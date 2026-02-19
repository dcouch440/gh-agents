# Eliminate Duplicated Patterns Across Server Codebase

## Overview

An audit of `src/server/` and `src/db/` revealed **10 categories of duplicated boilerplate** that could benefit from trait-based abstraction, shared helpers, or macro extraction. This is the same class of problem we just solved for LLM providers with `SseProviderAdapter` — strategy pattern + shared generic executor.

Ordered by impact.

---

## 1. `verify_ownership` — 10 near-identical functions across services

**Pattern:** Every service module re-implements ownership verification: fetch row -> 404 if missing -> check `user_id` -> 403 if mismatch -> return row.

| File | Function |
|------|----------|
| `services/workflows/mod.rs` | `verify_workflow_ownership` |
| `services/sessions/mod.rs` | `verify_ownership` |
| `services/agents/mod.rs` | `verify_ownership` |
| `services/collections/mod.rs` | `verify_ownership` |
| `services/rooms/mod.rs` | `verify_ownership` |
| `services/documents/mod.rs` | `verify_ownership` |
| `services/prompt_templates/mod.rs` | `verify_ownership` |
| `services/output_schemas/mod.rs` | `verify_ownership` |
| `services/tools/mod.rs` | `verify_agent_ownership` |
| `api/ownership.rs` | `verify_agent_ownership` |

The agent ownership check (`user_id.is_some() && user_id != Some(caller)`) is itself written **3 times**.

**Fix:** A generic `verify_ownership<R: OwnedResource>(repo_fn, entity_name, id, user_id)` helper, plus a `verify_user_editable` variant for nullable-owner resources (prompt templates, output schemas).

---

## 2. Step lifecycle wrapper — 9+ identical start/record/snapshot/complete sequences

**Pattern:** Every DAG execution mode repeats the same 4-phase boilerplate:
1. `let step_start = Instant::now()`
2. `broadcast_workflow_event(StepStarted { ... })`
3. Execute the step
4. `record_step_output(...)` + `versioning::snapshot_content(envelope, "output")` + `broadcast_workflow_event(StepCompleted { ... })`

The `record_step_output` + `snapshot_content(ENVELOPE, "output")` sequence appears verbatim in **9 call sites** across:
- `dag/single/mod.rs`
- `dag/belief_capture/mod.rs`
- `dag/workforce/mod.rs`
- `dag/sub_workflow/mod.rs`
- `dag/for_each/iteration.rs`
- `dag/mod.rs`
- `dag/room_step/mod.rs`

**Fix:** A `record_and_snapshot_output(dag_state, dag, step, output, envelope)` helper collapses the 8-line sequence. A `StepLifecycleGuard` or `run_step_with_lifecycle(dag, step, agent, fut)` wrapper handles started/completed broadcast + timing.

---

## 3. Execution mode dispatch via `if` chains (not `match`)

**Pattern:** In `hub/dag/mod.rs:440-598`, the main DAG loop dispatches to execution mode handlers via `if step.execution_mode == "..."` chains. 8 modes handled as separate `if` blocks rather than a unified dispatch table.

This makes:
- Adding new modes require editing the loop body
- `spawn_summarizer_if_completed` is inconsistently placed (not called after `"room"` or `"for_each"`)
- No compile-time exhaustiveness checking

**Fix:** A `StepDispatcher` trait or match + function-pointer table. Aligns with the `ProtocolCompiler` registration pattern already used in `hub/protocols/mod.rs`.

---

## 4. Tool parameter extraction — 47 identical guard patterns

**Pattern:** Every tool handler opens with the same guard:
```rust
let Some(val) = input["field"].as_str() else {
    return json!({ "error": "Missing required parameter: field" });
};
```

**47 occurrences** of "Missing required parameter" across 4 tool modules (`node_assistant`, `belief_capture`, `room_config`, `workforce`).

The step-loading pattern also repeats at 7 call sites:
```rust
match repo.get_step(ctx.step_id).await {
    Ok(Some(s)) => s,
    Ok(None) => return json!({ "error": "Step not found" }),
    Err(e) => return json!({ "error": ... }),
}
```

**Fix:** A `require_str(input, "field")` / `require_value(input, "field")` helper that returns `Result<&str, Value>`. A `load_step_or_error(repo, step_id)` helper for the repeated step load.

---

## 5. Container -> local -> context-free tool dispatch cascade

**Pattern:** Both `strategies/dag_step/mod.rs` and `strategies/workforce_agent/mod.rs` implement the same 3-way tool dispatch:
1. If container handle -> `execute_tool_in_container`
2. If execution context -> `execute_execution_tool`
3. Otherwise -> `execute_context_free_tool`

Near line-for-line identical. `room_speaker/mod.rs` has the same minus the container branch.

**Fix:** A `dispatch_execution_tool(name, input, container, exec_ctx, tool_names)` free function.

---

## 6. `on_complete` in execution strategies — log tokens + update status

**Pattern:** 4 strategies duplicate the same `on_complete` body: `log_token_usage(...)` then `ae_repo.update_agent_execution_status(ae_id, "completed", Some(response), structured)`.

| File | Variation |
|------|-----------|
| `strategies/dag_step/mod.rs` | log + update with structured output |
| `strategies/workforce_agent/mod.rs` | identical |
| `strategies/agent_designer/mod.rs` | identical |
| `strategies/room_speaker/mod.rs` | same but `None` for structured output |

**Fix:** A `complete_agent_execution(state, ae_id, model_id, response, usage, include_structured)` shared helper.

---

## 7. `CreateStepInput` / `UpdateStepInput` field duplication

**Pattern:** `services/steps/mod.rs` defines two structs with **identical payload fields** (13+ `Option<T>` fields). The only difference: Create has `(workflow_id, user_id)`, Update adds `step_id`. Same duplication in `api/workflows/types.rs` with request types.

**Fix:** Extract a shared `StepPayload` struct:
```rust
struct CreateStepInput { workflow_id, user_id, payload: StepPayload }
struct UpdateStepInput { workflow_id, step_id, user_id, payload: StepPayload }
```

---

## 8. Read-preserve-write config upsert pattern

**Pattern:** Tool handlers that update a single config field must read all existing fields to preserve them, then call a full upsert. Appears 4+ times in `belief_capture/mod.rs` and 3+ times in `workforce/mod.rs`.

```rust
let existing = repo.get_extraction_plan(step_id).await.ok().flatten();
let (field_a, field_b) = match &existing {
    Some(plan) => (plan.a.clone(), plan.b.clone()),
    None => (default_a, default_b),
};
repo.upsert_extraction_plan(step_id, new_value, &field_a, &field_b).await
```

**Fix:** Either read-modify-write helpers per config type, or support partial column updates in the DB layer.

---

## 9. `verify_agent_ownership` at API vs service layer

Three separate implementations of agent ownership checking across `api/ownership.rs`, `services/agents/mod.rs`, and `services/tools/mod.rs`. Since `ServiceError` auto-converts to `AppError`, the API handler could call the service version directly.

**Fix:** Delete the API-layer duplicate, use the service version everywhere.

---

## 10. Service CRUD template (low priority)

All 8 service modules (`agents`, `sessions`, `workflows`, `rooms`, `collections`, `documents`, `prompt_templates`, `output_schemas`) follow the exact same list/get/create/update/delete template. This is intentional and correct — but any cross-cutting concern (audit logging, caching) must be applied to 8+ modules individually.

**Assessment:** Low priority. The uniformity is valuable as-is. A `CrudService<Repo, Row>` trait is feasible but adds type complexity. Worth revisiting only if a cross-cutting concern actually materializes.

---

## Suggested approach

Tackle in order of impact-to-effort ratio:
1. **#1 (ownership)** + **#9 (API duplicate)** — quick wins, pure extraction
2. **#4 (tool params)** — mechanical, high count
3. **#5 (tool dispatch cascade)** + **#6 (on_complete)** — small helpers
4. **#2 (step lifecycle)** + **#3 (mode dispatch)** — higher impact, more design needed
5. **#7 (step input)** + **#8 (config upsert)** — medium payoff
