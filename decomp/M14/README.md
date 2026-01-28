# Milestone 14: Dynamic Agent Selection

> Minimal changes to enable difficulty-based model routing in the pipeline.

## Goal

Let the orchestrator tag each slice with a difficulty level, and have the router pick the right model accordingly. Fix prompt verbosity that causes 1.5k-line decomps.

**Checkpoint**: Orchestrator produces concise milestone specs with difficulty tags. Router assigns Opus for complex slices, Sonnet for standard/simple ones.

---

## Tickets

| Ticket | Title | Difficulty |
|--------|-------|------------|
| 14.1 | Fix prompt verbosity | simple |
| 14.2 | Add difficulty metadata to task routing | standard |
| 14.3 | Wire model override through agent pool | standard |

---

## Ticket 14.1: Fix Prompt Verbosity

> Remove instructions that cause over-detailed decompositions and excessive narration.

### Changes

**`templates/orchestrator.md` (~line 51)**:
- Replace: "If your decomp requires the worker to make decisions, it's not detailed enough."
- With: "Define what to build, why, and how slices connect. The worker decides implementation details. Each slice spec should fit on one screen."

**`src/prompts/templates/orchestrator.rs`**:
- Remove "Show your thinking for each step before providing the final slices" from `DECOMPOSITION_THINKING`
- Remove "Show your thinking for each step before giving your verdict" from `REVIEW_THINKING`

**`src/prompts/templates/worker.rs`**:
- In `IMPLEMENTATION_THINKING`, remove: ANNOUNCE, EXPLAIN decisions, REPORT progress every few minutes
- Keep: READ task, IDENTIFY files, PLAN approach, test/verify steps

### Verify
- `cargo check` passes
- Prompts still compile and render

---

## Ticket 14.2: Add Difficulty Metadata to Task Routing

> Wire the existing `estimated_complexity` field through to task metadata so the router can use it.

### Key Discovery

The planner already parses `estimated_complexity` (low/medium/high) per task from the LLM's JSON output via `TaskOutput` in `src/prompts/schemas/decomposition.rs:53`. But `convert_to_planner_output` in `src/orchestration/planner.rs:316` sets `metadata: None`, throwing this data away. The LLM output format doesn't need to change.

### Changes

**`src/orchestration/planner.rs` (~line 299, in `convert_to_planner_output`)**:
- Map `estimated_complexity` into task metadata instead of discarding it:
```rust
let mut metadata = HashMap::new();
metadata.insert("difficulty".to_string(), match task_output.estimated_complexity {
    ComplexityOutput::Low => "simple",
    ComplexityOutput::Medium => "standard",
    ComplexityOutput::High => "complex",
}.to_string());

// then in the Task struct:
metadata: Some(metadata),  // was: metadata: None,
```

**`src/orchestration/router.rs`**:
- Add a new `RuleMatcher` variant:
```rust
/// Match if task metadata key equals a specific value
MetadataEquals(String, String),  // (key, value)
```
- Add matching logic in `evaluate_rule`:
```rust
RuleMatcher::MetadataEquals(key, value) => {
    if self.get_metadata(task, key).as_deref() == Some(value.as_str()) {
        Some(rule.target_tier)
    } else {
        None
    }
}
```
- Add two new routing rules to `RouterConfig::default()` (before the default fallback):
```
Priority 75: MetadataEquals("difficulty", "simple") → Utility tier
Priority 65: MetadataEquals("difficulty", "complex") → Orchestrator tier
Priority 0 (existing default): → Worker tier (covers "standard" implicitly)
```

**Why not use existing matchers:** `HasMetadata` only checks key existence, not value. `ComplexityAbove` uses a numeric threshold which doesn't cleanly map to exact difficulty levels. `MetadataEquals` is ~10 lines and reusable for future metadata-driven routing.

**No prompt changes needed** — the LLM already outputs `estimated_complexity` per task. The orchestrator prompt in `src/prompts/templates/orchestrator.rs` already asks for this field as part of the JSON schema.

### Verify
- `cargo test` — router tests pass
- `cargo test planner` — planner conversion test shows metadata populated
- New routing rules resolve correctly for each difficulty level

---

## Ticket 14.3: Wire Model Override Through Agent Pool

> Allow tasks to carry a model override so the spawned agent uses the right model.

### Changes

**`src/types/config.rs`**:
- Update `TierModels` defaults:
  - Orchestrator model → `claude-opus-4-5-20251101` (was Sonnet)
  - Worker model → `claude-sonnet-4-20250514` (unchanged)
  - Utility model → `claude-sonnet-4-20250514` (was Haiku — Sonnet for reliability)

This means:
- `difficulty=complex` → routed to Orchestrator tier → Opus
- `difficulty=standard` → routed to Worker tier → Sonnet
- `difficulty=simple` → routed to Utility tier → Sonnet (low token budget)

**`src/agents/pool.rs`**:
- In `spawn_agent()`, check if the task's metadata contains `model_override`. If present, use that model config instead of the tier default. This is the escape hatch for edge cases.

### Verify
- `cargo check` passes
- `cargo test` — pool tests pass
- Agent spawned for a "complex" task uses Opus model

---

## Notes

- This is intentionally minimal. Three files touched for routing, three for prompts, one config default change.
- The orchestrator's role stays the same — it still writes the full plan in context. It just writes intent instead of implementation.
- One new enum variant (`MetadataEquals`) in `RuleMatcher`. No new types, traits, or modules otherwise.
- Future enhancement: add extended thinking (`budget_tokens`) for complex tasks via the Anthropic API. Not in scope here.
