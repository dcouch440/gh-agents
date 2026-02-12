# Belief-Oriented Context System for DAG Execution and Meetings

**Type:** feat
**Scope:** backend, frontend, database
**Priority:** high

---

## Problem

A single node on the canvas can have 40+ upstream connections. When a room/meeting step starts, it needs context from all upstream nodes plus growing conversation history. Current approaches:

- **Port resolution** passes raw structured data between steps — works for exact values but doesn't provide semantic understanding
- **Variable interpolation** (`{variable.path}`) injects specific fields into prompts — doesn't scale to 40 upstream sources
- **Room transcript** accumulates full message history — grows unbounded, eventually exceeds context window

None of these provide **curated, relevance-filtered context** that stays bounded regardless of upstream fan-in or conversation length.

## Solution

Implement a **belief system** that:

1. Decomposes each step's output into tagged belief slices at completion time (static, computed once)
2. Stores beliefs in a queryable store keyed by step execution
3. At meeting/chat time, a lightweight gatekeeper selects relevant beliefs per turn
4. Human and agent messages are absorbed as beliefs, blending into the same store
5. Each agent turn receives a small, curated belief window instead of raw history

Beliefs propagate **up the DAG as a side effect of execution**. By the time a high-fan-in meeting node starts, all upstream beliefs already exist. The meeting gatekeeper only does selection — never decomposition.

---

## Database Schema

### Migration: `0025_belief_system.sql`

```sql
-- Belief slices generated from step outputs or conversation messages
CREATE TABLE beliefs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Origin: which execution produced this belief
    workflow_id         UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    workflow_execution_id UUID REFERENCES workflow_executions(id) ON DELETE SET NULL,
    source_step_id      UUID REFERENCES workflow_steps(id) ON DELETE CASCADE,

    -- For beliefs generated from human/agent messages in a room
    source_session_id   UUID REFERENCES room_sessions(id) ON DELETE CASCADE,
    source_message_role  TEXT,          -- 'human', 'agent', 'system', NULL for step output beliefs
    source_agent_id     UUID REFERENCES agents(id) ON DELETE SET NULL,

    -- Belief content
    semantic_tag        TEXT NOT NULL,
    confidence          TEXT NOT NULL CHECK (confidence IN ('high', 'medium', 'low')),
    emotional_tone      TEXT NOT NULL,
    content             TEXT NOT NULL,  -- Dense understanding, not raw quote
    cross_step_tension  TEXT,           -- Tension/coupling with other steps, NULL if none

    -- Metadata
    belief_type         TEXT NOT NULL DEFAULT 'step_output'
                        CHECK (belief_type IN ('step_output', 'human_message', 'agent_message', 'revision')),
    superseded_by       UUID REFERENCES beliefs(id),  -- For revised beliefs: points to replacement
    is_killed           BOOLEAN NOT NULL DEFAULT FALSE,

    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Primary query: get all active beliefs from upstream steps for a given step
CREATE INDEX idx_beliefs_workflow_step ON beliefs(workflow_id, source_step_id)
    WHERE NOT is_killed AND superseded_by IS NULL;

-- Query: get beliefs from a room session (conversation beliefs)
CREATE INDEX idx_beliefs_session ON beliefs(source_session_id, created_at)
    WHERE NOT is_killed AND superseded_by IS NULL;

-- Query: get beliefs by semantic tag for cross-step search
CREATE INDEX idx_beliefs_semantic ON beliefs(workflow_id, semantic_tag)
    WHERE NOT is_killed AND superseded_by IS NULL;


-- Tracks which beliefs were selected for a specific agent turn
-- Enables debugging and belief effectiveness analysis
CREATE TABLE belief_selections (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_execution_id  UUID NOT NULL REFERENCES agent_executions(id) ON DELETE CASCADE,
    belief_id           UUID NOT NULL REFERENCES beliefs(id) ON DELETE CASCADE,
    relevance_score     REAL,           -- Optional: gatekeeper's relevance estimate
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_belief_selections_execution ON belief_selections(agent_execution_id);
```

### Key design decisions:

- **`superseded_by` + `is_killed`**: Supports the confirm/revise/kill lifecycle. Active beliefs are those where `superseded_by IS NULL AND NOT is_killed`. Revised beliefs point to their replacement. Killed beliefs are marked but retained for audit.
- **`belief_type`**: Distinguishes step-output beliefs (static, generated at step completion) from conversation beliefs (dynamic, generated during meetings). Same table, same query patterns, same selection mechanism.
- **`source_step_id` nullable with `source_session_id`**: A belief comes from either a step output OR a conversation message, never both. Enforced at application layer.
- **`belief_selections`**: Records which beliefs were actually used for each agent turn. Enables analysis of belief effectiveness and debugging context quality.

---

## Backend Implementation

### Phase 1: Belief Generation at Step Completion

#### 1.1 Belief Generator Service

**File:** `src/server/hub/beliefs/mod.rs`

```
src/server/hub/beliefs/
├── mod.rs          # BeliefGenerator service + types
├── generator.rs    # LLM-based belief decomposition
├── selector.rs     # Gatekeeper belief selection
├── absorber.rs     # Human/agent message → belief conversion
└── tests.rs
```

**Core types:**

```rust
pub struct Belief {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub source_step_id: Option<Uuid>,
    pub source_session_id: Option<Uuid>,
    pub semantic_tag: String,
    pub confidence: BeliefConfidence,
    pub emotional_tone: String,
    pub content: String,
    pub cross_step_tension: Option<String>,
    pub belief_type: BeliefType,
}

pub enum BeliefConfidence { High, Medium, Low }

pub enum BeliefType {
    StepOutput,
    HumanMessage,
    AgentMessage,
    Revision,
}

pub struct BeliefSelection {
    pub beliefs: Vec<Belief>,
    pub reasoning: String,
}
```

#### 1.2 Integration Point: Step Completion Hook

**File:** `src/server/hub/dag/single.rs` — after `execute_single_step` stores output

After a step completes and its output is stored in `completed` + `completed_envelopes` + `var_outputs`, generate beliefs:

```rust
// After line 163: completed.insert(step.id, output);
if state.belief_generation_enabled() {
    let beliefs = state.belief_generator()
        .generate_from_step_output(
            ctx.workflow_id,
            ctx.stage_execution_id,
            step,
            &output,
            completed_envelopes,  // upstream context for cross-step tension
        )
        .await;
    state.belief_repo().store_beliefs(&beliefs).await;
}
```

Same integration in:
- `for_each.rs` — after for-each step completes (aggregate beliefs from iteration results)
- `room_step.rs` — after room session completes (beliefs from final outputs + key transcript moments)

#### 1.3 Belief Generator (LLM Call)

**File:** `src/server/hub/beliefs/generator.rs`

Uses a fast model (Haiku) with tool_use for structured output:

```rust
impl BeliefGenerator {
    pub async fn generate_from_step_output(
        &self,
        workflow_id: Uuid,
        execution_id: Uuid,
        step: &WorkflowStepRow,
        output: &StepOutput,
        upstream_envelopes: &HashMap<Uuid, StepExecutionEnvelope>,
    ) -> Vec<Belief> {
        // System prompt: gatekeeper decomposition instruction
        // User prompt: step output + step description + upstream context summary
        // Tool: structured_output with belief schema
        // Model: haiku (fast, cheap — beliefs are generated per step)

        // Returns 3-6 beliefs per step output
    }
}
```

**Design constraint:** Belief generation uses Haiku, not the step's model. It's a metadata operation, not a reasoning operation. Cost per step: ~$0.001.

#### 1.4 Belief Absorber (Message → Belief)

**File:** `src/server/hub/beliefs/absorber.rs`

Converts room messages into beliefs. Runs after each turn in a room session:

```rust
impl BeliefAbsorber {
    pub async fn absorb_message(
        &self,
        workflow_id: Uuid,
        session_id: Uuid,
        message_role: &str,     // "human" or "agent"
        agent_id: Option<Uuid>,
        content: &str,
    ) -> Vec<Belief> {
        // For human messages: extract semantic meaning, tag with emotional tone
        // For agent messages: extract key insights, decisions, or findings
        // Short messages may produce 0-1 beliefs
        // Dense messages may produce 2-3 beliefs

        // Model: haiku
    }
}
```

**When absorber runs:**
- After each human message in an interactive room session
- After each agent turn in a room session
- NOT retroactively on old messages — only on new messages during active sessions

### Phase 2: Belief Selection for Meeting Context

#### 2.1 Belief Selector (Gatekeeper)

**File:** `src/server/hub/beliefs/selector.rs`

At each agent turn in a room, the selector curates the belief window:

```rust
impl BeliefSelector {
    pub async fn select_for_turn(
        &self,
        workflow_id: Uuid,
        step_id: Uuid,
        session_id: Uuid,
        current_message: &str,    // The triggering message or turn topic
        upstream_step_ids: &[Uuid],
    ) -> BeliefSelection {
        // 1. Load all active beliefs from upstream steps
        //    (pre-computed, static — just a DB query)
        let upstream_beliefs = self.repo.get_active_beliefs_for_steps(
            workflow_id, upstream_step_ids
        ).await;

        // 2. Load conversation beliefs from this session
        let session_beliefs = self.repo.get_session_beliefs(session_id).await;

        // 3. Combine into candidate pool
        let candidates = [upstream_beliefs, session_beliefs].concat();

        // 4. Gatekeeper selects relevant subset
        //    Input: candidate beliefs (semantic_tag + content summary) + current context
        //    Output: selected indices + reasoning
        //    Model: haiku (selection is cheap)

        // 5. Record selection in belief_selections table

        // Returns: curated beliefs for this turn
    }
}
```

#### 2.2 Integration with Room Execution

**File:** `src/server/hub/dag/room_step/mod.rs`

Before each speaker turn, inject curated beliefs into the prompt:

```rust
// In execute_room_turn(), before building the speaker prompt:
let belief_selection = state.belief_selector()
    .select_for_turn(
        workflow_id,
        step.id,
        session.id,
        &current_context,
        &upstream_step_ids,
    )
    .await;

// Format beliefs into a context block
let belief_context = format_belief_context(&belief_selection.beliefs);

// Prepend to the speaker's prompt
let prompt = format!("{}\n\n{}", belief_context, speaker_prompt);
```

**Belief context format in prompt:**

```
<context>
The following represents curated understanding from upstream work and this conversation.
Each belief carries a confidence level and emotional assessment.

[auth_design] (high confidence, careful tone)
The authentication system uses JWT with refresh token rotation...

[user_concern_performance] (medium confidence, skeptical tone)
A reviewer raised concerns about query performance under load...

[data_model_coupling] (high confidence, fragile tone)
Cross-step tension: The user model and session model share a UUID...
</context>
```

#### 2.3 Integration with Step Chat Sessions

**File:** `src/server/api/workflows/step_chat_handlers.rs`

Step chat sessions (non-room chats) also benefit. When a user chats with a step:

1. Load beliefs from upstream steps (same as room)
2. Inject as context for the chat LLM call
3. Absorb user messages as beliefs if they carry semantic weight

### Phase 3: Belief Revision (Optional, Future)

For critical meetings, enable a revision loop:

1. After a meeting produces a conclusion, the gatekeeper evaluates it against upstream beliefs
2. If gaps found, revise beliefs and re-prompt
3. Store revised beliefs with `superseded_by` pointing to replacements

This is the Phase 2 mechanism from the prototype. Defer to a follow-up ticket unless needed immediately.

---

## Database Repository

**File:** `src/db/traits/beliefs.rs` + `src/db/pg_repo/beliefs.rs`

```rust
#[async_trait]
pub trait BeliefRepo: Send + Sync {
    /// Store beliefs generated from step output or message absorption
    async fn store_beliefs(&self, beliefs: &[NewBelief]) -> Result<Vec<Belief>>;

    /// Get all active beliefs for a set of upstream step IDs
    /// Active = not killed AND not superseded
    async fn get_active_beliefs_for_steps(
        &self,
        workflow_id: Uuid,
        step_ids: &[Uuid],
    ) -> Result<Vec<Belief>>;

    /// Get conversation beliefs from a room session
    async fn get_session_beliefs(&self, session_id: Uuid) -> Result<Vec<Belief>>;

    /// Get all active beliefs for a workflow (for debugging/UI)
    async fn get_workflow_beliefs(&self, workflow_id: Uuid) -> Result<Vec<Belief>>;

    /// Mark a belief as killed (soft delete)
    async fn kill_belief(&self, belief_id: Uuid) -> Result<()>;

    /// Revise a belief: mark original as superseded, store replacement
    async fn revise_belief(
        &self,
        original_id: Uuid,
        replacement: &NewBelief,
    ) -> Result<Belief>;

    /// Record which beliefs were selected for an agent execution
    async fn record_selection(
        &self,
        agent_execution_id: Uuid,
        belief_ids: &[Uuid],
    ) -> Result<()>;
}
```

---

## Frontend

### Phase 1: Belief Visibility

#### API Endpoints

```
GET  /api/workflows/:wid/beliefs                    — All active beliefs for workflow
GET  /api/workflows/:wid/steps/:sid/beliefs          — Beliefs generated by a step
GET  /api/workflows/:wid/steps/:sid/beliefs/upstream  — Beliefs from all upstream steps
GET  /api/rooms/sessions/:sid/beliefs                 — Conversation beliefs for a room session
```

#### Belief Store

**File:** `frontend/src/stores/beliefStore.ts`

```typescript
type Belief = {
    id: string
    workflow_id: string
    source_step_id: string | null
    source_session_id: string | null
    source_agent_id: string | null
    semantic_tag: string
    confidence: 'high' | 'medium' | 'low'
    emotional_tone: string
    content: string
    cross_step_tension: string | null
    belief_type: 'step_output' | 'human_message' | 'agent_message' | 'revision'
    is_killed: boolean
    created_at: string
}
```

#### Canvas Integration

On the canvas, each completed step node shows a belief indicator:

- Small badge showing belief count (e.g., "5 beliefs")
- Click to expand belief list in the step's detail panel
- Beliefs colored by confidence: green (high), yellow (medium), red (low)
- Cross-step tension beliefs highlighted with a link icon

When viewing a room/meeting node:
- Show which upstream beliefs were selected for the current/last turn
- Show conversation beliefs accumulating as the meeting progresses
- Belief selection reasoning visible in a debug panel

### Phase 2: Belief Explorer (Future)

A dedicated panel showing the full belief graph:
- All beliefs across the workflow, grouped by step
- Filter by semantic_tag, confidence, emotional_tone
- Visual connections between beliefs and the steps that consumed them
- Revision history (superseded chains)

---

## Execution Flow Summary

```
Step 1 completes
  → BeliefGenerator.generate_from_step_output() [Haiku, ~200ms]
  → 4 beliefs stored in DB

Step 2 completes
  → BeliefGenerator... → 3 beliefs stored

... 38 more upstream steps complete, each generating 3-6 beliefs ...

Meeting node (Step 41) starts — 40 upstream steps, ~160 beliefs in store

  Turn 1:
    → Human sends message
    → BeliefAbsorber.absorb_message() → 1 belief stored [Haiku, ~100ms]
    → BeliefSelector.select_for_turn() → selects 12/161 beliefs [Haiku, ~200ms]
    → Agent receives 12 curated beliefs + current message → responds
    → BeliefAbsorber.absorb_message(agent) → 2 beliefs stored

  Turn 2:
    → Human sends message → 1 belief
    → BeliefSelector selects 14/164 beliefs (different subset)
    → Agent responds with belief context → 1 belief

  ... meeting continues with bounded context window ...
```

**Cost per meeting turn:** ~3 Haiku calls (absorb human, select, absorb agent) = ~$0.003
**Context window per turn:** ~12-15 beliefs ≈ 1,500-2,000 tokens (vs 40 upstream outputs at ~50,000+ tokens)

---

## Configuration

Add to `WorkflowStepRow` or workflow-level config:

```sql
ALTER TABLE workflows ADD COLUMN belief_generation_enabled BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE workflow_steps ADD COLUMN belief_generation_enabled BOOLEAN DEFAULT NULL;
-- NULL = inherit from workflow, TRUE/FALSE = step-level override
```

This keeps the system opt-in. Existing workflows are unaffected.

---

## Testing

### Backend Tests

1. **Belief generation:** Mock LLM, verify beliefs are stored with correct metadata after step completion
2. **Belief selection:** Given N candidate beliefs and a query, verify gatekeeper selects relevant subset
3. **Belief absorption:** Verify human messages produce beliefs with correct type and session linkage
4. **Belief revision:** Verify superseded_by chain and is_killed filtering
5. **Integration:** Run a 3-step DAG with belief generation enabled, verify downstream step receives curated beliefs

### Frontend Tests

1. **Belief store:** Verify fetch, normalization, and filtering
2. **Belief display:** Verify confidence badges, cross-step tension highlighting
3. **Room integration:** Verify beliefs appear in room context panel during active sessions

---

## Rollout

1. **Migration + repo layer** — schema and CRUD
2. **Belief generator** — LLM-based decomposition at step completion
3. **Belief selector** — curated belief window for room turns
4. **Belief absorber** — human/agent message conversion
5. **Frontend: belief visibility** — badges, detail panel, debug view
6. **Opt-in config** — workflow-level and step-level toggles

Each phase is independently shippable. Phase 1-2 are the minimum viable belief system. Phase 3-4 make meetings context-aware. Phase 5-6 are polish.

---

## Open Questions

1. **Belief generation model:** Haiku is proposed for cost/speed. Should the gatekeeper model be configurable per workflow? Some users may want Sonnet-quality beliefs.

2. **Belief TTL:** Should beliefs expire? For long-running workflows, old beliefs may become stale. Consider a `max_belief_age` config.

3. **Belief count cap:** Should there be a max beliefs per step? Prevents runaway generation on steps with massive outputs.

4. **Port resolution coexistence:** Beliefs provide context; ports provide data. Both systems run in parallel. Should the belief context include a note about what exact data is available via ports?

5. **Retroactive generation:** When belief generation is enabled on an existing workflow with completed steps, should we retroactively generate beliefs from stored outputs?
