# Step Activity Stream — Real-Time Agent Milestone Broadcasting

## Context

When a workflow runs, the frontend only knows two things about each step: **it started** and **it finished** (or failed). Between `StepStarted` and `StepCompleted`, the user sees a pulsing blue dot and nothing else. For steps that take 30-60+ seconds (LLM calls, multi-round tool use, complex routing), this is a black box.

Production multi-agent frameworks (OpenAI Agents SDK, LangGraph, AG-UI protocol) solve this with tiered event granularity — raw tokens at the bottom, semantic milestones in the middle, lifecycle events at the top. The milestone tier is the sweet spot: enough to tell a human what's happening, sparse enough to not be noise.

**Key decisions:**
- **Milestone model, not action log.** Each step emits 2-4 milestone events, not one per internal action. The frontend shows one status line per step that updates in place.
- **Closed milestone enum.** Four milestone types: `preparing`, `thinking`, `acting`, `decided`. No open strings. This prevents feature creep.
- **No extra LLM calls.** All messages derived from execution context (step names, tool names, model IDs, upstream step names, routing labels).
- **No changes to ExecutionFilter or ExecutionEngine.** Milestones are emitted at the DAG orchestrator level. Tool names captured via a lightweight `StreamSink` implementation.
- **Events only reach clients subscribed to the `workflow` topic.** Users not on the workflow page receive nothing — zero wasted bandwidth.
- **Persisted for historical replay.** Only 2-4 rows per step, so storage is negligible.

---

## Milestone Types (Closed Enum)

| Milestone | When Emitted | Example Message | Frequency |
|-----------|-------------|-----------------|-----------|
| `preparing` | After inputs resolved + prompt composed | "Resolving inputs from Planner, Reviewer" | Always (1x per step) |
| `thinking` | LLM call starts | "Thinking..." | Always (1x per step) |
| `acting` | First tool call (or first NEW tool name) | "Running tool: github_search" | Only if tools used |
| `decided` | Routing label chosen | "Routing to 'needs_revision'" | Only if routing happens |

**Deduplication rules:**
- `acting` emits once per **distinct tool name** per step execution. If `github_search` is called 5 times across rounds, only the first emits.
- For-each steps emit `thinking` with periodic progress: "Processing items... (3 of 10 complete)" — updated every N items or ~5 seconds, not per-item.
- `preparing` always fires exactly once, even if the step has no upstream inputs (message becomes "Composing prompt...").

**Budget:** 2-4 milestones per step. A 10-step workflow produces 20-40 events total.

---

## What the User Sees

A 5-step workflow with parallel steps and a routing decision:

```
12:00:01  Planner        · Resolving inputs...
12:00:01  Planner        · Thinking...
12:00:04  Planner        · Completed (890 tokens, 3.1s)
12:00:04  Reviewer       · Resolving inputs from Planner
12:00:04  Code Analyzer  · Resolving inputs from Planner
12:00:05  Reviewer       · Thinking...
12:00:05  Code Analyzer  · Thinking...
12:00:05  Code Analyzer  · Running tool: github_search
12:00:08  Reviewer       · Completed (1,200 tokens, 4.0s)
12:00:09  Code Analyzer  · Completed (2,100 tokens, 5.2s)
12:00:09  Summarizer     · Resolving inputs from Reviewer, Code Analyzer
12:00:10  Summarizer     · Thinking...
12:00:10  Summarizer     · Routing to 'needs_revision'
12:00:12  Summarizer     · Completed (650 tokens, 2.8s)
```

15 lines for a 5-step workflow. A narrative, not a log.

---

## Part 1: Backend Event + Database Foundation

> **Risk:** LOW — Additive only, no behavior changes to existing code.
> **Effort:** Small
> **Dependencies:** None

### 1A. New `WorkflowEventKind::StepActivity` variant

**File:** `src/server/ws/events.rs`

Add a new variant to `WorkflowEventKind`:

```rust
StepActivity {
    step_id: Uuid,
    step_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent_name: Option<String>,
    milestone: StepMilestone,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    iteration_progress: Option<(usize, usize)>,  // (completed, total)
}
```

Define `StepMilestone` as a closed enum in the same file:

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepMilestone {
    Preparing,
    Thinking,
    Acting,
    Decided,
}
```

Update `event_name()` match arm to return `"step_activity"`.

Wire format on the WebSocket:
```json
{
  "topic": "workflow",
  "event": "step_activity",
  "ts": "2024-01-01T00:00:00.123Z",
  "run_id": "abc-123",
  "user_id": "def-456",
  "data": {
    "workflow_id": "...",
    "step_id": "...",
    "step_name": "code_reviewer",
    "agent_name": "Code Reviewer",
    "milestone": "acting",
    "message": "Running tool: github_search",
    "iteration_progress": null
  }
}
```

### 1B. Database migration

**File:** `migrations/0020_step_activity_log.sql`

```sql
CREATE TABLE step_activity_log (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_execution_id UUID NOT NULL REFERENCES workflow_executions(id) ON DELETE CASCADE,
    step_id               UUID NOT NULL,
    agent_name            TEXT,
    milestone             VARCHAR(20) NOT NULL,    -- preparing, thinking, acting, decided
    message               TEXT NOT NULL,
    iteration_progress    JSONB,                   -- {"completed": 3, "total": 10} or null
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_step_activity_execution ON step_activity_log(workflow_execution_id, created_at);
```

No index on `step_id` alone — queries always filter by `workflow_execution_id` first.

### 1C. Database row type + repo methods

**File:** `src/db/` — add `StepActivityRow` struct and repo trait methods.

```rust
pub struct StepActivityRow {
    pub id: Uuid,
    pub workflow_execution_id: Uuid,
    pub step_id: Uuid,
    pub agent_name: Option<String>,
    pub milestone: String,
    pub message: String,
    pub iteration_progress: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}
```

Repo methods:
- `insert_step_activity(row: &StepActivityRow) -> Result<()>`
- `get_activity_for_execution(workflow_execution_id: Uuid) -> Result<Vec<StepActivityRow>>`

### 1D. Activity emitter utility

**File:** `src/server/hub/dag/activity.rs` (new module)

A lightweight struct that combines persist + broadcast into one call:

```rust
pub struct ActivityEmitter<'a> {
    state: &'a AppState,
    ctx: &'a WorkflowExecutionContext,
    workflow_id: Uuid,
}

impl<'a> ActivityEmitter<'a> {
    pub fn new(state: &'a AppState, ctx: &'a WorkflowExecutionContext, workflow_id: Uuid) -> Self { ... }

    pub async fn emit(
        &self,
        step: &WorkflowStepRow,
        agent_name: Option<&str>,
        milestone: StepMilestone,
        message: String,
        iteration_progress: Option<(usize, usize)>,
    ) {
        // 1. Insert to step_activity_log (fire-and-forget, log errors on failure)
        // 2. Broadcast WorkflowEventKind::StepActivity via EventBus
    }
}
```

The insert is fire-and-forget: if the DB write fails, log the error but don't block execution. The broadcast happens regardless.

Add `pub mod activity;` to `src/server/hub/dag/mod.rs`.

### Tests

- Unit test: `StepMilestone` serializes to expected strings (`"preparing"`, `"thinking"`, etc.)
- Unit test: `StepActivity` variant converts to correct `WireMessage` shape
- Integration test: `ActivityEmitter::emit()` inserts a row and broadcasts an event
- Unit test: `get_activity_for_execution()` returns rows in `created_at` order

---

## Part 2: DAG Orchestrator Integration

> **Risk:** LOW — Adding emit calls at existing boundaries. No logic changes.
> **Effort:** Medium
> **Dependencies:** Part 1

### 2A. Emit milestones in `execute_single_step`

**File:** `src/server/hub/dag/mod.rs` — `execute_single_step()` (line 881)

Insert milestone emit calls at natural boundaries:

```
fn execute_single_step(...) {
    // ... existing StepStarted broadcast ...

    // MILESTONE: preparing
    emitter.emit(step, agent_name, Preparing, "Resolving inputs from ...").await;

    // ... existing port resolution + prompt composition ...

    // MILESTONE: thinking
    emitter.emit(step, agent_name, Thinking, format!("Calling {}...", model_id)).await;

    // ... existing engine.execute() call ...

    // MILESTONE: decided (only if routing happened)
    if let Some(label) = routing_label {
        emitter.emit(step, agent_name, Decided, format!("Routing to '{}'", label)).await;
    }

    // ... existing StepCompleted broadcast ...
}
```

The `preparing` message should be contextual:
- If the step has upstream port inputs: `"Resolving inputs from {upstream_step_names}"`
- If the step has no upstream inputs: `"Composing prompt..."`
- Names come from `WorkflowStepRow.name` or `output_variable_name` of parent steps

### 2B. Emit milestones in `execute_for_each_step`

**File:** `src/server/hub/dag/mod.rs` — `execute_for_each_step()`

For-each steps use a different milestone pattern:

```
preparing  → "Splitting into {N} items"
thinking   → "Processing items... (3 of 10 complete)"  // periodic update
decided    → only if routing within iterations
```

The `thinking` milestone with `iteration_progress` updates periodically — **not per-item**. Use a counter + threshold:

```rust
let update_interval = std::cmp::max(1, total_items / 5);  // Update ~5 times during execution
if completed_count % update_interval == 0 {
    emitter.emit(step, agent_name, Thinking,
        format!("Processing items... ({} of {} complete)", completed_count, total_items),
        Some((completed_count, total_items))
    ).await;
}
```

### 2C. Emit milestones in `execute_for_each_chain`

**File:** `src/server/hub/dag/mod.rs` — `execute_for_each_chain()`

Chained for-each pipelines emit at the chain level, not per-stage:

```
preparing  → "Starting pipeline: {chain_step_count} stages, {item_count} items"
thinking   → "Pipeline processing... (3 of 10 items complete)"
```

Individual pipeline stages within `execute_pipeline_item()` do NOT emit milestones — the chain-level updates are sufficient.

### 2D. `ActivitySink` for tool name capture

**File:** `src/server/hub/dag/activity.rs`

The `NullSink` currently discards tool events in DAG execution. Replace it with an `ActivitySink` that captures the first distinct tool name and emits an `acting` milestone:

```rust
pub struct ActivitySink {
    emitter: ActivityEmitter,       // Owns an emitter (cloned from the step-level one)
    step_id: Uuid,
    step_name: String,
    agent_name: Option<String>,
    iteration_index: Option<i32>,
    seen_tools: Mutex<HashSet<String>>,  // Track which tool names we've already emitted for
}

#[async_trait]
impl StreamSink for ActivitySink {
    async fn tool_start(&self, name: &str, _id: &str) {
        let mut seen = self.seen_tools.lock().await;
        if seen.insert(name.to_string()) {
            // First time seeing this tool name — emit milestone
            self.emitter.emit(..., Acting, format!("Running tool: {}", name), ...).await;
        }
        // Subsequent calls with same tool name: silent
    }

    async fn tool_end(&self, _name: &str, _id: &str) {
        // No-op. We don't emit on tool completion.
    }

    async fn token(&self, _text: &str) {
        // No-op. DAG steps don't stream tokens.
    }

    async fn error(&self, _msg: &str) {
        // No-op. Errors handled by the engine.
    }
}
```

**Integration point:** In `run_step_via_engine()`, replace `NullSink` with `ActivitySink`. This is the only place `NullSink` is used for DAG steps.

### 2E. Room step milestones

**File:** `src/server/hub/dag/mod.rs` — room step execution section

Room steps get one milestone before entering the room:

```
preparing  → "Entering room discussion with {N} agents"
```

Room-internal events (`SpeakerStart`, `SpeakerToken`, `SpeakerEnd`) already flow through `RoomEvent` on the WebSocket. No duplication needed.

### Tests

- Integration test: Run a mock single step through `execute_single_step`, verify 2 milestones emitted (`preparing` + `thinking`)
- Integration test: Run a mock step with tool use, verify `acting` milestone emitted for first tool only
- Integration test: Run a for-each with 10 items, verify `thinking` milestone emitted ~5 times (not 10)
- Unit test: `ActivitySink` deduplicates tool names (call `tool_start("search")` 3x, verify 1 emit)

---

## Part 3: Frontend Store + Inline Display

> **Risk:** LOW — Additive UI change, no existing behavior modified.
> **Effort:** Medium
> **Dependencies:** Part 2

### 3A. WS event types

**File:** `frontend/src/types/ws.ts`

Add to workflow event enum:

```typescript
type StepMilestone = 'preparing' | 'thinking' | 'acting' | 'decided'

type StepActivityData = {
    workflow_id: string
    step_id: string
    step_name: string
    agent_name: string | null
    milestone: StepMilestone
    message: string
    iteration_progress: { completed: number; total: number } | null
}
```

Add `STEP_ACTIVITY = 'step_activity'` to the workflow event constants.

### 3B. Execution store update

**File:** `frontend/src/stores/workflowExecutionStore.ts`

Extend `StepExecutionState` with one field:

```typescript
latestActivity: string | null  // Most recent milestone message
```

Handle `step_activity` in the event handler:

```typescript
case WORKFLOW_EVENT.STEP_ACTIVITY: {
    const { step_id, message } = data as StepActivityData
    const step = state.stepStates[step_id]
    if (step) {
        step.latestActivity = message
    }
    break
}
```

Clear `latestActivity` when `step_completed` or `step_failed` arrives (the completion message replaces it).

On `WORKFLOW_EVENT.STARTED`, reset all `latestActivity` to `null`.

### 3C. Timeline entry inline display

**File:** `frontend/src/components/panels/execution/ExecutionTimelineEntry.tsx`

When a step is in `running` status and `latestActivity` is non-null, show it as a subtle secondary line:

```
[blue pulse] Code Reviewer
             Running tool: github_search     ← latestActivity
```

Style: smaller font, muted color, single line with text-overflow ellipsis. Disappears when step completes.

### Tests

- Store test: `step_activity` event updates `latestActivity` for the correct step
- Store test: `step_completed` clears `latestActivity`
- Store test: Multiple rapid `step_activity` events — only latest message retained
- Store test: `step_activity` for unknown `step_id` is silently ignored

---

## Part 4: Activity Stream Panel

> **Risk:** LOW — New component, no existing code modified.
> **Effort:** Medium
> **Dependencies:** Part 3

### 4A. Activity stream store state

**File:** `frontend/src/stores/workflowExecutionStore.ts`

Add a chronological activity log alongside per-step state:

```typescript
// Add to WorkflowExecutionState:
activityStream: ActivityStreamEntry[]

type ActivityStreamEntry = {
    ts: string              // ISO timestamp from WS event
    stepName: string
    agentName: string | null
    milestone: StepMilestone
    message: string
}
```

`step_activity` handler appends to `activityStream` (in addition to updating `latestActivity`).

Cap at a reasonable max (200 entries) to prevent unbounded growth during very large workflows. Oldest entries dropped when cap is hit.

On `WORKFLOW_EVENT.STARTED`, clear `activityStream`.

### 4B. `ActivityStream` component

**File:** `frontend/src/components/panels/execution/ActivityStream.tsx`

A scrollable list that renders `activityStream` entries chronologically:

```
12:00:01  Planner        · Resolving inputs...
12:00:01  Planner        · Thinking...
12:00:04  Reviewer       · Resolving inputs from Planner
12:00:04  Code Analyzer  · Resolving inputs from Planner
12:00:05  Reviewer       · Thinking...
12:00:05  Code Analyzer  · Running tool: github_search
```

Design requirements:
- Auto-scrolls to bottom while workflow is running (stop auto-scroll if user scrolls up)
- Relative timestamps ("2s ago") while running, absolute when complete
- Each row: `[timestamp] [agent name] · [message]`
- Agent name gets a consistent color per-agent (hash agent name to color)
- Monospace or tabular layout so columns align

### 4C. Integration into ExecutionPanel

**File:** `frontend/src/components/panels/execution/ExecutionPanel.tsx`

Add an "Activity" tab alongside the existing timeline/history tabs. The activity tab renders `<ActivityStream />`. This tab is the "full screen stream channeled at the root."

When the workflow is not running and no historical activity is loaded, the tab shows an empty state: "Activity will appear here when a workflow runs."

### Tests

- Component test: `ActivityStream` renders entries in chronological order
- Component test: Auto-scroll behavior (scrolls to bottom on new entry, stops if user scrolled up)
- Store test: `activityStream` is capped at 200 entries
- Store test: `activityStream` clears on `WORKFLOW_EVENT.STARTED`

---

## Part 5: REST API + Historical Replay

> **Risk:** LOW — New endpoint, read-only.
> **Effort:** Small
> **Dependencies:** Part 1 (database), Part 4 (frontend component)

### 5A. API endpoint

**File:** `src/server/api/workflows/mod.rs`

```
GET /api/workflows/executions/{execution_id}/activity
```

Returns all `step_activity_log` rows for a workflow execution, ordered by `created_at`:

```json
{
  "activity": [
    {
      "id": "uuid",
      "step_id": "uuid",
      "agent_name": "Code Reviewer",
      "milestone": "preparing",
      "message": "Resolving inputs from Planner",
      "iteration_progress": null,
      "created_at": "2024-01-01T00:00:01.123Z"
    }
  ]
}
```

Auth: requires valid JWT, scoped to the user who owns the workflow.

### 5B. API client method

**File:** `frontend/src/api/api.ts`

```typescript
getExecutionActivity: (executionId: string, config?: RequestConfig) =>
    baseApi.get<ExecutionActivityResponse>(API.WORKFLOW_EXECUTION_ACTIVITY(executionId), config)
```

Add the URL constant to the API constants.

### 5C. Historical activity loading

**File:** `frontend/src/stores/workflowExecutionStore.ts`

When viewing a historical run (user selects a past execution from the history panel):
1. Fetch activity from the API
2. Populate `activityStream` with the response data
3. The `ActivityStream` component renders identically for live and historical data

When a client reconnects mid-execution:
1. Detect the active `run_id` from the store
2. Fetch activity from the API to backfill missed milestones
3. New WS events append from there — no duplicates because milestones have unique `(step_id, milestone)` pairs

### Tests

- API test: Endpoint returns activity in chronological order
- API test: Endpoint returns 404 for non-existent execution
- API test: Endpoint returns empty array for execution with no activity
- Integration test: Historical load populates `activityStream` correctly
- Integration test: Reconnect backfill doesn't duplicate entries

---

## Edge Cases

| Case | Handling |
|------|----------|
| **User not on workflow page** | No `workflow` topic subscription. WS handler drops events at the topic filter. Zero wasted compute or bandwidth. |
| **Client reconnects mid-run** | Frontend calls activity API for current `run_id` to backfill. New WS events append. Deduplicate by `(step_id, milestone, created_at)`. |
| **Concurrent for-each iterations** | `iteration_progress` field tracks `(completed, total)`. Updates throttled to ~5 per for-each step, not per-item. |
| **Chained for-each pipelines** | Chain-level milestones only. Individual pipeline stages within `execute_pipeline_item()` do not emit — chain progress is sufficient. |
| **Room steps (multi-speaker)** | One `preparing` milestone at DAG level. Room-internal events flow through existing `RoomEvent` channel. No duplication. |
| **Cancelled workflow** | Cancellation token stops execution. No more milestones emitted after cancellation. Partial activity persisted for debugging. |
| **Step with no upstream inputs** | `preparing` message becomes "Composing prompt..." instead of "Resolving inputs from ...". Always emits. |
| **Tool-heavy agents (20+ tool calls)** | `ActivitySink` deduplicates by tool name. 20 calls to `github_search` = 1 `acting` milestone. 3 different tools = 3 milestones. Still bounded. |
| **Very large workflows (50+ steps)** | Activity stream capped at 200 entries. At 3 milestones/step, that covers ~65 steps. Older entries rotate out. Historical API returns all. |
| **DB write latency** | Activity insert is fire-and-forget. Slow DB never blocks execution. Broadcast happens immediately regardless of DB success. |
| **Broadcast channel lag** | At 3 milestones/step, a 20-step workflow produces ~60 events. Well within the 256-message broadcast buffer. |
| **Step fails before any milestone** | `StepFailed` event already handled. No activity to clean up. Partial milestones (e.g., `preparing` emitted but step fails during LLM call) remain in the log — useful for debugging. |
| **Multiple browser tabs** | Each tab has its own WS connection and topic subscriptions. Each receives events independently. No interference. |

---

## Implementation Order

| Part | Ships Independently? | What You Get |
|------|---------------------|--------------|
| **Part 1** | Yes | Backend emits milestones + persists. Nothing visible yet but foundation is solid. |
| **Part 2** | Yes (with Part 1) | DAG steps emit real milestones during execution. Visible in server logs. |
| **Part 3** | Yes (with Parts 1-2) | **Inline status on existing timeline entries.** Immediate UX improvement — users see what each step is doing. |
| **Part 4** | Yes (with Parts 1-3) | **Full activity stream panel.** The "full screen stream" view across all agents. |
| **Part 5** | Yes (with Parts 1-4) | **Historical replay.** View activity for past runs. Reconnect backfill. Feature complete. |

After Part 3, users already have a meaningful improvement. Parts 4 and 5 add polish and completeness.

---

## Files Changed (Summary)

**Backend — New:**
- `migrations/0020_step_activity_log.sql`
- `src/server/hub/dag/activity.rs` (ActivityEmitter + ActivitySink)
- `src/server/hub/dag/activity/tests.rs`

**Backend — Modified:**
- `src/server/ws/events.rs` (StepMilestone enum, StepActivity variant)
- `src/db/` (StepActivityRow, repo trait + impl)
- `src/server/hub/dag/mod.rs` (emit calls in execute_single_step, execute_for_each_step, execute_for_each_chain, room step execution; replace NullSink with ActivitySink)
- `src/server/api/workflows/mod.rs` (GET activity endpoint)

**Frontend — New:**
- `frontend/src/components/panels/execution/ActivityStream.tsx`

**Frontend — Modified:**
- `frontend/src/types/ws.ts` (StepActivityData, StepMilestone)
- `frontend/src/stores/workflowExecutionStore.ts` (latestActivity, activityStream, event handler)
- `frontend/src/components/panels/execution/ExecutionTimelineEntry.tsx` (inline activity display)
- `frontend/src/components/panels/execution/ExecutionPanel.tsx` (Activity tab)
- `frontend/src/api/api.ts` (getExecutionActivity method)
- `frontend/src/constants.ts` (API URL constant)
