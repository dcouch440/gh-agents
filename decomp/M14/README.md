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

> Teach the router to read a `difficulty` metadata key and map it to model tiers.

### Changes

**`src/orchestration/router.rs`**:
- Add three new routing rules (before the default fallback):

```
Priority 75: HasMetadata("difficulty=simple") → Utility tier
Priority 65: HasMetadata("difficulty=complex") → Orchestrator tier
Priority 0 (existing default): → Worker tier (covers "standard" implicitly)
```

This uses the existing `RuleMatcher::HasMetadata` and `RoutingRule` types. No new structs needed.

**`src/prompts/templates/orchestrator.rs`**:
- In the decomposition prompt, add instruction for the orchestrator to tag each slice:
  - `simple` — mechanical, follows existing patterns, low ambiguity
  - `standard` — typical implementation work, some decisions needed
  - `complex` — architectural, cross-cutting, high judgment required

### Verify
- `cargo test` — router tests pass
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
- No new types, no new traits, no new modules. Uses existing metadata, routing rules, and tier config.
- Future enhancement: add extended thinking (`budget_tokens`) for complex tasks via the Anthropic API. Not in scope here.
