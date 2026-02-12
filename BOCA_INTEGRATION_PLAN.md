# BOCA in Nexor: Belief Capture + Mask Agent

## Context

Seven BOCA experiments validated that structured beliefs outperform raw transcripts for question answering (flat converged 15/20 beat full context 14/20). The product goal: after a workflow executes, users can have a conversation with a **mask agent** that knows everything the workflow learned — contradictions, plans, decisions, findings. Domain-agnostic: code, scripts, business plans, anything.

Two layers: **Belief Capture** (how beliefs get into the system) and **Mask Agent** (the conversational interface).

## Part 1: Belief Capture

### Approach: Post-Execution Gatekeeper

After each step completes, one LLM call extracts beliefs from the step's assistant messages. No agent behavior changes. Proven in BOCA Phase 6.

```
Step executes normally
    ↓
Step completes → execution_messages in DB
    ↓
If step.extract_beliefs = true:
    ↓
Load assistant-role messages (skip tool results — that's noise)
    ↓
One gatekeeper LLM call → structured belief slices
    ↓
Beliefs stored in `beliefs` table
```

**Why assistant messages only**: Tool results are file contents, command outputs — bulk noise. The agent's reasoning, decisions, and observations live in its assistant messages. That's where beliefs are.

**Non-fatal**: If gatekeeper fails, log warning and continue. Workflow never breaks due to belief extraction.

**Future enhancement**: Add `report_finding` tool for agents to self-report beliefs inline. Milestone/window system for real-time capture. These are optimizations on top of the base gatekeeper.

### Migration: `0025_beliefs.sql`

```sql
CREATE TABLE beliefs (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_id uuid NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    workflow_execution_id uuid NOT NULL,
    workflow_step_id uuid NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    agent_execution_id uuid NOT NULL REFERENCES agent_executions(id) ON DELETE CASCADE,
    content text NOT NULL,
    reasoning text NOT NULL,
    belief_type text NOT NULL DEFAULT 'fact',
    confidence text NOT NULL DEFAULT 'medium',
    confidence_justification text,
    semantic_tag text NOT NULL,
    emotional_tone text,
    cross_source_tension text,
    source_step_name text NOT NULL,
    extraction_model text NOT NULL,
    extraction_tokens_in integer NOT NULL DEFAULT 0,
    extraction_tokens_out integer NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_beliefs_workflow ON beliefs(workflow_id);
CREATE INDEX idx_beliefs_workflow_execution ON beliefs(workflow_execution_id);
CREATE INDEX idx_beliefs_step ON beliefs(workflow_step_id);
CREATE INDEX idx_beliefs_semantic_tag ON beliefs(semantic_tag);
CREATE INDEX idx_beliefs_type ON beliefs(belief_type);

ALTER TABLE workflow_steps ADD COLUMN extract_beliefs boolean NOT NULL DEFAULT false;
```

### DB Layer

**Row types** in `src/db/mod.rs`: `BeliefRow`, `NewBelief`

**Trait** in `src/db/traits/mod.rs`:
```rust
pub trait BeliefRepo: Send + Sync {
    async fn insert_beliefs(&self, beliefs: &[NewBelief]) -> Result<Vec<BeliefRow>>;
    async fn list_beliefs_for_workflow(&self, workflow_id: Uuid) -> Result<Vec<BeliefRow>>;
    async fn list_beliefs_for_execution(&self, workflow_execution_id: Uuid) -> Result<Vec<BeliefRow>>;
    async fn list_beliefs_for_step(&self, step_id: Uuid) -> Result<Vec<BeliefRow>>;
    async fn delete_beliefs_for_execution(&self, workflow_execution_id: Uuid) -> Result<u64>;
}
```

**Implementation** in `src/db/pg_repo/mod.rs`.

### Gatekeeper Module: `src/server/hub/beliefs/`

```
src/server/hub/beliefs/
├── mod.rs       # extract_beliefs_from_execution(), format_assistant_messages()
└── tests.rs     # Unit tests
```

**Core function**:
```rust
pub async fn extract_beliefs_from_execution(
    state: &AppState,
    agent_execution_id: Uuid,
    step: &WorkflowStepRow,
    workflow_execution_id: Uuid,
) -> Result<Vec<BeliefRow>, HubError>
```

1. `list_execution_messages(agent_execution_id)` — load all messages
2. Filter to `role = "assistant"` only
3. Format into transcript string
4. Call LLM via `provider.send_message()` with tool_use (belief schema)
5. Parse response into `NewBelief` structs
6. `insert_beliefs()` to DB
7. Return inserted rows

**Gatekeeper prompt** (adapted from BOCA v2 — proven best):
```
You are the Belief Gatekeeper. Decompose this execution transcript into
BELIEF SLICES — atomic claims about what was learned, decided, observed,
or done.

Rules:
1. Preserve ALL NUMBERS exactly
2. Each belief = one atomic claim
3. Extract 5-20 beliefs covering significant facts, decisions, observations
4. Classify: fact | policy | opinion | observation
5. Fill reasoning FIRST
```

### DAG Hook: `src/server/hub/dag/single.rs`

Modify `run_step_via_engine` to return `agent_execution_id` in its tuple.

After step completion (line ~163), before StepCompleted broadcast:
```rust
if step.extract_beliefs {
    if let Some(provider) = state.provider() {
        match beliefs::extract_beliefs_from_execution(
            state, ae_id, step, ctx.stage_execution_id,
        ).await {
            Ok(extracted) => debug!(count = extracted.len(), "Beliefs extracted"),
            Err(e) => warn!("Belief extraction failed: {}", e),
        }
    }
}
```

### API: `src/server/api/workflows/belief_handlers.rs`

- `GET /api/workflows/:wid/beliefs` — all beliefs for workflow
- `GET /api/workflows/:wid/executions/:eid/beliefs` — beliefs from one execution
- `GET /api/workflows/:wid/steps/:sid/beliefs` — beliefs from one step
- `DELETE /api/workflows/:wid/executions/:eid/beliefs` — clear execution beliefs

### WebSocket Event

```rust
WorkflowEventKind::BeliefsExtracted {
    step_id: Uuid,
    step_name: String,
    belief_count: usize,
}
```

---

## Part 2: Mask Agent

### Concept

A conversational agent that users chat with about a completed workflow. The mask has the workflow's belief store injected into its context. Users ask questions, the mask answers from beliefs.

This uses the **existing chat infrastructure** (ChatStrategy, chat_sessions, chat_messages) — no new execution engine needed.

### How It Works

1. User opens mask conversation for a workflow execution
2. System loads all beliefs for that execution
3. Beliefs are formatted and injected into the mask's system prompt
4. User chats normally — mask answers from beliefs
5. Conversation persists in `chat_sessions` like any other chat

### Mask System Prompt

```
You are a Mask agent. You have access to a belief store containing
everything learned during this workflow execution. Answer questions
using ONLY the beliefs provided.

<beliefs>
[b01] (Architect, fact, high) Auth uses JWT with 15-min token expiry
[b02] (Developer, fact, high) No rate limiting on login endpoint
[b03] (QA, opinion, medium) Should add rate limiting before launch
[b04] (Security, fact, high) mTLS between services, cert pinning enabled
...
</beliefs>

Rules:
1. Answer from beliefs only. If beliefs don't cover a topic, say so.
2. When beliefs contradict, note the contradiction and sources.
3. Include exact numbers from beliefs.
4. Cite belief IDs in answers.
```

### Implementation

**New endpoint**: `POST /api/workflows/:wid/executions/:eid/mask/chat`
- Creates or finds a mask chat session for this execution
- Loads beliefs, formats them, builds system prompt
- Routes to existing `ChatStrategy` with the belief-enriched system prompt

**Session management**: Store mask sessions in `chat_sessions` with `mode_id = "mask"` and `draft_config = {"workflow_execution_id": eid}`.

**Belief refresh**: If new beliefs are added (workflow still running or re-run), the mask's system prompt is rebuilt on next message.

### Frontend

- "Ask about this workflow" button on workflow execution view
- Opens a chat panel (reuse existing chat components)
- Chat messages persist across sessions

---

## Files to Create/Modify

### Create:
1. `migrations/0025_beliefs.sql`
2. `src/server/hub/beliefs/mod.rs` — gatekeeper extraction
3. `src/server/hub/beliefs/tests.rs`
4. `src/server/api/workflows/belief_handlers.rs` — belief CRUD endpoints
5. `src/server/api/workflows/mask_handlers.rs` — mask chat endpoints

### Modify:
6. `src/db/mod.rs` — BeliefRow, NewBelief, extract_beliefs on WorkflowStepRow
7. `src/db/traits/mod.rs` — BeliefRepo trait
8. `src/db/pg_repo/mod.rs` — PgRepo impl for BeliefRepo
9. `src/server/hub/mod.rs` — `pub mod beliefs;`
10. `src/server/hub/dag/single.rs` — gatekeeper hook + return ae_id
11. `src/server/api/workflows/mod.rs` — register belief + mask routes
12. `src/server/ws/events.rs` — BeliefsExtracted event

---

## Implementation Order

### Phase A: Belief Capture (backend)
1. Migration `0025_beliefs.sql`
2. DB layer: BeliefRow, NewBelief, BeliefRepo trait, PgRepo impl
3. Gatekeeper module: `src/server/hub/beliefs/`
4. DAG hook in `single.rs`
5. Belief API endpoints
6. WebSocket event
7. Tests

### Phase B: Mask Agent (backend)
8. Mask chat endpoint (`mask_handlers.rs`)
9. Belief formatting for mask system prompt
10. Session management (mode_id = "mask")
11. Tests

### Phase C: Frontend
12. Belief extraction toggle on step config
13. "Ask about this workflow" button
14. Mask chat panel (reuse chat components)
15. Beliefs list view (optional)

---

## Verification

1. `cargo check` + `cargo test` — no regressions
2. Run migration, create workflow with `extract_beliefs=true`
3. Execute workflow → verify beliefs in DB
4. `GET /api/workflows/:wid/beliefs` → returns beliefs
5. Open mask chat → ask questions → mask answers from beliefs
6. Verify mask cites belief IDs and handles contradictions

---

## Future Enhancements (not in this plan)

- **`report_finding` tool**: Agents self-report beliefs inline (no gatekeeper needed for those)
- **Milestone + window system**: Real-time belief capture during long executions
- **Convergence step type**: Automatic belief deduplication and contradiction resolution
- **Taxonomy management**: Controlled tag vocabulary for cross-workflow queries
- **Cross-workflow mask**: Query beliefs across multiple workflow executions
- **Belief-type authority**: Facts override opinions in convergence (BOCA Phase 7 finding)
