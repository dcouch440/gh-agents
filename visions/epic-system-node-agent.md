# Epic: System Node Agent

> Vision: [vision-system-node-agent.md](./vision-system-node-agent.md)
> The vision is the source of truth for design decisions, prompt formats, file schemas, and examples. This document is the implementation plan.

## Overview

Replace the two-phase builder → designer pipeline with a single containerized system node agent that writes JSON config files. Six slices, each testable independently.

```
Slice 1: Strategy runs               Slice 2: Files become agents
(new strategy + tools)                (file reader + execution bridge)
 │                                     │
 └──────────┬──────────────────────────┘
            │
Slice 3: Sync to DB
(files → DB projection)
            │
Slice 4: Wire into dispatch
(board serializer + sessions)
            │
Slice 5: Cascade
(description propagation)
            │
Slice 6: Cleanup
(delete old machinery)
```

Slices 1 and 2 can be built in parallel. Each subsequent slice depends on the one above it.

---

## Slice 1 — The Strategy Runs

The system node agent can be dispatched, writes files, and completes successfully.

### 1.1 — System prompt config

Create `config/archetype/workforce/system_agent/` with:
- `config.yaml` — model tier, max_tokens, temperature, max_rounds, context_budget
- `system.md` — role, runtime, schema, guide, examples, completion (from vision doc)

Register in `src/config/protocols.rs` as a new `Lazy` static alongside `WORKFORCE_BUILDER` and `WORKFORCE`.

**Files:**
- `config/archetype/workforce/system_agent/config.yaml` (new)
- `config/archetype/workforce/system_agent/system.md` (new)
- `src/config/protocols.rs` (add new static)

**Test:** config loads without panic, template variables resolve.

### 1.2 — SystemNodeStrategy

New `ExecutionStrategy` implementation. Minimal first pass:

- `system_prompt()` — resolved from system_agent config
- `tools()` — `run_command` + `complete_system`
- `model_id()` / `max_rounds()` / `temperature()` — from config
- `rebuild_system_prompt()` — reads filesystem, builds `<current_state>` XML
- `build_messages()` — loads session history, calls `build_pruned_instruction`
- `should_stop()` — true when `complete_system` captured
- `execute_tool()` — routes `run_command` and `complete_system`

**Files:**
- `src/server/hub/execution/strategies/system_node/mod.rs` (new)
- `src/server/hub/execution/strategies/system_node/tests.rs` (new)
- `src/server/hub/execution/strategies/mod.rs` (add module)

**Depends on:** 1.1

### 1.3 — complete_system tool

Tool definition + handler:
- Parameters: `summary` (string), `verify` (object with `topology_complete`, `agents_complete`, `config_accurate` booleans)
- Validate verify claims against filesystem
- On success: capture summary, signal stop
- On failure: return structured errors

Register in tool registry alongside `run_command`.

**Files:**
- `src/server/tools/system_node/mod.rs` (new)
- `src/server/tools/system_node/tests.rs` (new)
- `src/tools/registry/mod.rs` (add `complete_system` definition)

**Depends on:** 1.4 (validator)

### 1.4 — Write-time JSON validation

Intercept file writes in `run_command` execution for the system node agent. When the agent writes to `config.json`, `topology.json`, or `agents/*.json`:
- Parse JSON
- Validate required fields per file type
- Accept → file hits disk
- Reject → file not written, error in tool response

No cross-reference validation at write time (topology vs agent files). That happens in `complete_system`.

**Files:**
- `src/server/tools/system_node/validate.rs` (new)
- `src/server/tools/system_node/validate_tests.rs` (new)

**Depends on:** nothing — pure function

### 1.5 — current_state builder

Read the system node agent's filesystem and produce `<current_state>` XML:
- Read `topology.json` → extract agent slugs and depends_on
- Check each `agents/{slug}.json` exists → `configured` or `missing`
- Read `config.json` → extract name, check status
- Render as XML for system prompt injection

**Files:**
- `src/server/hub/board/system_node_state.rs` (new)
- `src/server/hub/board/system_node_state_tests.rs` (new)

**Depends on:** nothing — reads files, returns string

### Slice 1 acceptance test

Manually dispatch `SystemNodeStrategy` with a test instruction. Agent writes config.json, topology.json, agents/*.json. Calls `complete_system` with verify all true. Backend validates, tool returns success. Files on disk are valid.

---

## Slice 2 — Files Become Agents

Read the system node agent's output files and feed them into the existing execution pipeline.

### 2.1 — File reader

Parse `config.json` + `topology.json` + `agents/*.json` from the filesystem into `DesignedAgentPrompt` structs.

Mapping:
- `agents/{slug}.json` → `DesignedAgentPrompt`
  - `name` → `agent_name`
  - `system_prompt` → `system_prompt`
  - `assignment` → `assignment`
  - `expected_output` → `expected_output`
  - `capabilities` → `tools`
- `topology.json` agents map → `depends_on` → `receives_from`
- Execution order computed from topology
- `agent_roster_entry_id` → placeholder UUID (real ID assigned in slice 3 sync)

**Files:**
- `src/server/hub/dag/pipeline/file_reader.rs` (new)
- `src/server/hub/dag/pipeline/file_reader_tests.rs` (new)

**Depends on:** nothing — pure function from files to structs

### 2.2 — Execution bridge

Wire the file reader output into `execute_agent_levels`:
- Call file reader to get `Vec<DesignedAgentPrompt>`
- Call `compute_execution_levels` (existing)
- Call `execute_agent_levels` (existing)

This is an alternative entry point to the pipeline, alongside the existing designer phase → agent executor path.

**Files:**
- `src/server/hub/dag/pipeline/file_executor.rs` (new)
- `src/server/hub/dag/pipeline/file_executor_tests.rs` (new)
- `src/server/hub/dag/pipeline/mod.rs` (add module)

**Depends on:** 2.1

### Slice 2 acceptance test

Place valid config.json + topology.json + agents/*.json on disk. File reader parses them. `compute_execution_levels` produces correct levels. Agents execute via `WorkforceAgentStrategy` with the designed prompts.

---

## Slice 3 — Sync to DB

After `complete_system` succeeds, diff files against DB state and apply minimal mutations. The DB becomes a projection of the files.

### 3.1 — Topology sync

Diff `topology.json` agents map against `TaskAgentRosterRow` entries:
- New slugs → create roster entry + child step via pipeline service
- Missing slugs → remove roster entry + child step
- Changed depends_on → add/remove child workflow edges
- Recompute execution order

Pattern: same diff logic as `configure_team` in `configure.rs`, driven by file contents instead of tool input.

**Files:**
- `src/server/services/system_node_sync/mod.rs` (new)
- `src/server/services/system_node_sync/topology.rs` (new)
- `src/server/services/system_node_sync/topology_tests.rs` (new)

**Depends on:** slice 1 (files exist on disk)

### 3.2 — Agent config sync

Diff `agents/*.json` against stored prompt data:
- Update roster `role_description` from agent `name`
- Sync `capabilities` on roster entries
- Store `system_prompt`, `assignment`, `expected_output`

**Files:**
- `src/server/services/system_node_sync/agents.rs` (new)
- `src/server/services/system_node_sync/agents_tests.rs` (new)

**Depends on:** 3.1 (roster entries exist)

### 3.3 — Config sync

Diff `config.json` against step row:
- Write `name` to step display name
- Write `description` to `designer_handoff` field
- Diff `description` against previous → set `description_changed` flag

**Files:**
- `src/server/services/system_node_sync/config.rs` (new)
- `src/server/services/system_node_sync/config_tests.rs` (new)

**Depends on:** nothing — reads config.json, updates step row

### Slice 3 acceptance test

System node agent completes. Sync runs. DB has correct roster entries, edges, mission brief, step name, designer_handoff. Frontend shows updated agents and topology without knowing the source changed.

---

## Slice 4 — Wire into Dispatch

Connect the system node agent to the board serializer dispatch path.

### 4.1 — Dispatch routing

Board serializer currently dispatches to `DispatchStrategy` (builder). Route workforce nodes to `SystemNodeStrategy` instead.

**What changes:**
- `src/server/services/board/executor.rs` — dispatch to system node agent
- `src/server/executors/dispatch.rs` — new runner function for system node tasks (or adapt existing)

**Files:**
- `src/server/services/board/executor.rs` (modify)
- `src/server/executors/dispatch.rs` (modify or new runner)

**Depends on:** slices 1, 2, 3

### 4.2 — Instruction formatting

Use existing `instruction.rs` for formatting (new node / updated node). The instruction becomes the user message. No changes to the format — `format_new_node` and `format_updated_node` already produce the right shape.

One addition: upstream description trigger. When a previous step's config description changes, format an instruction like "The upstream step changed what it produces." with the new `<previous_step>` block.

**Files:**
- `src/server/services/board/instruction.rs` (add upstream trigger format)

**Depends on:** nothing

### 4.3 — Session persistence

Reuse `find_or_create_builder_session` with a new role (`system_agent` instead of `builder`). Session accumulates history across dispatches. `complete_system` summary persisted as assistant message.

**Files:**
- `src/server/services/dispatch/mod.rs` (add system_agent session role)

**Depends on:** nothing

### 4.4 — End-to-end pipeline

Wire the full flow: board submit → serializer → instruction → dispatch → system node agent → complete_system → sync → execute agents.

**Depends on:** 4.1, 4.2, 4.3

### Slice 4 acceptance test

User draws a box on canvas, submits. System node agent is dispatched, configures agents, calls complete_system. Sync updates DB. Runtime agents execute. Full loop from canvas to execution.

---

## Slice 5 — Cascade

When a step's config description changes, re-run downstream system node agents.

### 5.1 — Description diff

After sync, compare new `config.json.description` against the previous value stored in `designer_handoff`. Return `description_changed` boolean.

Already partially implemented in 3.3. This slice adds the propagation trigger.

**Files:**
- `src/server/services/system_node_sync/config.rs` (extend)

### 5.2 — Cascade dispatcher

If `description_changed`, walk the downstream DAG in topological order. For each downstream step:
1. Format instruction: "The upstream step changed what it produces." + `<previous_step>`
2. Dispatch system node agent
3. Wait for completion
4. Check if that step's description changed
5. If changed → continue to next downstream step
6. If unchanged → stop

**Files:**
- `src/server/services/system_node_sync/cascade.rs` (new)
- `src/server/services/system_node_sync/cascade_tests.rs` (new)

**Depends on:** 5.1, slice 4

### Slice 5 acceptance test

Three-step workflow: A → B → C. Change step A's node text. System agent for A re-runs, updates config description. Cascade dispatches B's system agent with new `<previous_step>`. B adjusts agents but its description doesn't change. Cascade stops — C is not dispatched.

---

## Slice 6 — Cleanup

Remove the old builder + designer machinery.

### 6.1 — Delete strategies
- `src/server/hub/execution/strategies/dispatch/` (DispatchStrategy)
- `src/server/hub/execution/strategies/react_designer/` (ReactDesignerStrategy)
- `src/server/hub/execution/strategies/agent_designer/` (AgentDesignerStrategy)

### 6.2 — Delete tools
- Designer tools: `write_file`, `read_file`, `complete_design` (from system_store tool context)
- Builder tools: `configure_team`, `complete_task` (from workforce tools)
- Remove from tool registry

### 6.3 — Delete designer infrastructure
- `src/server/hub/dag/pipeline/designer.rs` — designer phase
- `src/server/hub/dag/agent_designer/` — one-shot designer
- `src/server/hub/board/state/enrich.rs` — design status enrichment
- `parse_store_configs()` — S3 config reader
- Designer-specific S3 paths (`design/{step_id}/agents/`)

### 6.4 — Simplify board state
- Remove L4 `Dispatch` variant (or repurpose for system node state)
- Remove `AgentDesignStatus` enrichment
- Remove `design_status` / `config_path` from agent rendering

### 6.5 — Consider DB table cleanup
- `agent_designer_runs` — may keep for historical data
- `agent_designer_outputs` — may keep for historical data
- Builder prompt configs in `config/archetype/workforce/builder/` — delete
- Designer prompt configs in `config/designer/` — delete

### Slice 6 acceptance test

`cargo clippy` + `cargo test` pass with no dead code warnings from deleted modules. Frontend still displays workflow correctly using data from the sync step.

---

## Risk & Open Questions

| Risk | Mitigation |
|------|------------|
| Write-time validation interceptor adds latency to `run_command` | Only intercept writes to known paths (config.json, topology.json, agents/*.json). Pass-through everything else. |
| Agent writes all files in one `run_command` — first validation failure breaks the chain | Acceptable. Agent sees which file failed, fixes it, writes again. Usually 1 retry. |
| Filesystem persistence between dispatches | System node agent's files live on JuiceFS, same as runtime agent files. Persist across dispatches within a workflow. |
| Session history for new role | Reuse existing session infrastructure with `role = "system_agent"`. |
| Cascade infinite loop | DAG is acyclic. Cascade follows topological order. Each step either propagates or absorbs. Finite by construction. |
| Frontend expects roster rows | Sync step (slice 3) creates them. Frontend unchanged. |

## Open questions

- **Model tier for system node agent** — tier:1 (fast, cheap, like the builder) or tier:2 (smart, like the designer)? The agent does both jobs now. Probably tier:2 since prompt quality matters.
- **Max rounds** — builder is 5, designer is 20. System node agent probably needs 5-10. First run is 2 tool calls. Re-runs with reads might be 3-4.
- **Capabilities seed file** — do we seed `capabilities.json` into the repository at dispatch time, or keep capabilities in the system prompt? Only `database_query` exists today.
- **Pen strokes / images** — runtime agents get images. Does the system node agent need to know this? Probably a one-liner in the system prompt.
