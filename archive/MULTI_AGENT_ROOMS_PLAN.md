# Interactive Review Queue & Multi-Agent Rooms Plan

> **Priority**: Post-router-modes implementation
> **Depends on**: Phases 1-5 of Router Modes System (see ROUTER_MODES_DESIGN.md)
> **Created**: 2026-02-04

---

## Strategy

**Phase A** — Interactive Review Queue (1-on-1 agent chats in a pipeline)
**Phase B** — Multi-Agent Rooms (group discussions in a pipeline)

Phase A first. It's simpler, covers the primary use case (review and refine results), and builds the pause/resume infrastructure that rooms need anyway.

---

# Phase A: Interactive Review Queue

## Goal

A pipeline step produces output → DAG pauses → user chats 1-on-1 with an agent about the result → user approves → DAG continues. When multiple steps need review, they appear in a queue. Finish one, auto-advance to the next.

```
4 workflows run in parallel
    ↓           ↓           ↓           ↓
  Result A    Result B    Result C    Result D
    ↓           ↓           ↓           ↓
┌─────────────────────────────────────────────┐
│  REVIEW QUEUE                               │
│                                             │
│  ● Chat: Agent re Result A         [ACTIVE] │
│  ○ Chat: Agent re Result B        [PENDING] │
│  ○ Chat: Agent re Result C        [PENDING] │
│  ○ Chat: Agent re Result D        [PENDING] │
│                                             │
│  [Approve] → captures output, next loads   │
└─────────────────────────────────────────────┘
    ↓
All 4 approved → pipeline continues
```

## What Already Exists

| Piece | Status | Location |
|-------|--------|----------|
| `interactive_agent_id` field on steps | ✅ Exists | `WorkflowStepRow.interactive_agent_id` |
| `is_interactive` flag on executions | ✅ Exists | `AgentExecutionRow.is_interactive` |
| Interactive agent makes initial LLM review | ✅ Exists | `dag_executor/mod.rs:700-817` |
| Status set to `"awaiting_user"` + WS broadcast | ✅ Exists | `dag_executor/mod.rs:789-812` |
| Send message API | ✅ Exists | `POST /api/agent-executions/:id/messages` |
| Approve API | ✅ Exists | `POST /api/agent-executions/:id/approve` |
| DAG pauses on interactive step | ❌ Broken | Result ignored: `let _ = execute_interactive_review(...)` |
| LLM responds to user messages | ❌ Missing | Send message only records, doesn't call LLM |
| Approval resumes pipeline | ❌ Missing | Sets status but nothing continues DAG |
| Queue page UI | ❌ Missing | Frontend feature |

## Feature A1: Fix DAG Pause on Interactive Steps

**Problem**: The DAG executor calls `execute_interactive_review()` but ignores the result and continues immediately.

**Files**: `src/server/dag_executor/mod.rs`

**What to do**:
- When a step has `interactive_agent_id`, after the initial LLM review call:
  - Return `HubError::AwaitingUser { step_id, execution_id: interactive_ae_id }`
  - This halts the DAG at that step (same pattern as room steps)
- Store the interactive execution ID and step ID so we know where to resume

**Also fix in `collection_dag_executor.rs`**:
- Catch `HubError::AwaitingUser` separately from other errors
- Set workflow execution status to `"paused"` (not `"failed"`)
- Set collection run status to `"paused"` (not `"failed"`)

---

## Feature A2: Interactive Chat — LLM Responds to Messages

**Problem**: `POST /api/agent-executions/:id/messages` records the user message but doesn't trigger an LLM response. The agent never replies.

**Files**: `src/server/api/agent_executions/mod.rs`

**What to do**:
- After recording the user message, load the interactive agent + full message history for this execution
- Call the LLM (via ExecutionEngine or direct provider call) with the conversation so far
- Record the assistant response
- Stream tokens via WebSocket (reuse existing streaming infrastructure)
- Keep status as `"awaiting_user"` (user hasn't approved yet)

**ModeResolver integration**:
- Before calling LLM, resolve mode: `mode_resolver.resolve(&agent, user_message, Some(&conversation_summary))`
- Agent gets appropriate personality/tools for the review conversation
- Tools work if execution_context is provided (agent can research, check code, etc.)

---

## Feature A3: Approve Resumes Pipeline

**Problem**: `POST /api/agent-executions/:id/approve` sets status to "completed" but nothing picks up the paused DAG.

**Files**: `src/server/api/agent_executions/mod.rs`, `src/server/collection_dag_executor/mod.rs`

**What to do**:
- On approval, capture the final output:
  - If `structured_output` provided in request → use that (user revised the output)
  - Otherwise → use the agent's last response as the step output
- Find the paused workflow execution (via `parent_agent_execution_id` → `workflow_step_id` → `workflow_execution_id`)
- Trigger resume:
  - Reload the DAG from the paused step
  - Inject the approved output as the step's variable output
  - Continue executing downstream steps

**Resume endpoint** (alternative to auto-resume):
- `POST /api/workflow-executions/:id/resume`
- For cases where multiple interactive steps need approval before continuing
- Only resumes when ALL pending interactive steps for that workflow are approved

---

## Feature A4: Queue Page (Frontend)

**Files**: `frontend/src/pages/ReviewQueue/` (NEW)

**What it shows**:
- List of all `"awaiting_user"` agent executions for the current user
- Grouped by collection run (if applicable)
- Each item shows: agent name, step name, brief output preview, status
- Active chat opens inline or in a panel

**Behavior**:
- Click a pending item → opens chat with the interactive agent
- Chat shows: agent's initial review, full conversation history, the original output being reviewed
- User can send messages (Feature A2 provides LLM responses)
- "Approve" button → calls approve API (Feature A3)
- On approve → auto-advance to next pending item in the queue
- When all items in a collection run are approved → show "All reviews complete, pipeline continuing"

**WebSocket events to listen for**:
- `"awaiting_user"` — new item appears in queue
- `"agent_token"` — streaming LLM response in active chat
- `"execution_completed"` — item removed from queue
- `"workflow_resumed"` — pipeline continuing

---

## Feature A5: Wire ModeResolver into DAG Executor

**Status**: DAG executor ignores ModeResolver.

**Files**: `src/server/hub/dag/mod.rs`, `src/server/hub/strategies/dag_step/mod.rs`

**What to do**:
- In `hub/dag/mod.rs` `run_step_via_engine()`, call `mode_resolver.resolve(&agent, prompt, Some(&step_description))`
- Apply `ResolvedModeConfig` system prompt, then append schema enforcement ON TOP
- Add `temperature: f32` field to `DagStepConfig`
- Update `DagStepStrategy::temperature()` to use `self.config.temperature`

**Depends on**: Nothing (ModeResolver exists)

---

## Phase A Dependency Graph

```
A1 (Fix DAG pause) ─────────────┐
                                 ├── A3 (Approve resumes pipeline)
A2 (LLM responds to messages) ──┤
                                 └── A4 (Queue page UI)

A5 (DAG + ModeResolver) — independent, can be done anytime
```

A1 and A2 can be built in parallel. A3 and A4 depend on both.

---

## Phase A End-to-End Pipeline

```
Document
  → Agent breaks into 4 milestones (single step, structured JSON array)
  → For-each: agent writes plan per milestone (for_each step)
      → Each iteration has interactive_agent_id set
      → Agent reviews the plan, sets "awaiting_user"
      → DAG PAUSES (Feature A1)
  → Queue page shows 4 pending reviews (Feature A4)
  → User opens first review
      → Chats with agent about the plan (Feature A2)
      → "Can you add error handling to milestone 2?"
      → Agent responds with revised plan
      → User approves (Feature A3)
      → Next review auto-loads
  → All 4 approved → DAG RESUMES (Feature A3)
  → Next step: agents produce final documents
  → Collection continues downstream
```

---

---

# Phase B: Multi-Agent Rooms

> Build AFTER Phase A is complete. Phase A provides the pause/resume infrastructure that rooms also need.

## Goal

Multiple agents in a group conversation, each with their own context, discussing and coordinating. Used for cross-cutting review where agents need to hear each other.

---

## Feature B1: Wire ModeResolver into Room Executor

**Status**: ModeResolver only works in chat. Room executor ignores it.

**Files**: `src/server/room_executor/mod.rs`, `src/server/hub/strategies/room_speaker/mod.rs`

**What to do**:
- In `room_executor/mod.rs`, before building `RoomSpeakerStrategy`, call `mode_resolver.resolve(&agent, user_message, Some(&transcript_block))`
- Apply `ResolvedModeConfig` system prompt, then append room context ON TOP
- Apply mode tools, then respect `room.tools_enabled` as master switch
- Add `temperature: f32` field to `RoomSpeakerConfig`
- Update `RoomSpeakerStrategy::temperature()` to use `self.config.temperature`

**Depends on**: Nothing (ModeResolver exists)

---

## Feature B2: Wire Up Room Tool Execution

**Status**: `execution_context` is hardcoded to `None` on line 365 of `room_executor/mod.rs`. Agents see tools in the LLM request but tool calls return `"No execution context available"`.

**Files**: `src/server/room_executor/mod.rs`

**What to do**:
- Build an `ExecutionContext` for room speakers (repo handle, user_id, allowed tool names)
- Pass it into `RoomSpeakerConfig.execution_context`
- This enables agents to research/plan during their turn (web_search, read_file, etc.)
- Room speakers already have `max_rounds() = 5` for multi-round tool use

**Depends on**: Feature B1 (so mode tools are resolved correctly)

---

## Feature B3: DAG Output → Room Agent Context Bridge

**Status**: No code pipes DAG step outputs into room agent contexts. The `collection_id` FK on rooms exists but is unused during room execution.

**Files**: `src/server/room_executor/mod.rs`, `src/server/hub/dag/mod.rs`

When a room step starts in the DAG executor:
- Query `execution_variables` from the current `collection_run_id`
- For each room member agent, find relevant step outputs
- Inject these as context into the speaker's system prompt

### Approach A — Static (pre-load at room creation)

- When DAG creates the room session, also create temporary `agent_context` document entries from step outputs
- Room executor picks them up automatically via existing `get_agent_context()` path
- Clean up temp documents when room session closes

### Approach B — Dynamic (load at turn time)

- Room executor checks if room has a `collection_id`
- If yes, queries `execution_variables` for that collection run
- Injects relevant outputs into each speaker's prompt
- Requires a mapping table or naming convention (variable name matches agent)

**Depends on**: Phase A (pause/resume infrastructure)

---

## Feature B4: Room Completion → Pipeline Continuation

**Status**: When user finishes reviewing in the room, nothing triggers the DAG to continue.

**What to do**:
- Room session completion (close or max_turns) triggers resume (reuses Phase A infrastructure)
- Resume logic collects room transcript/summary as the step output
- Downstream DAG steps receive this output via normal variable resolution `{room_review.summary}`
- Downstream agents can have a different `router_id` or the same router with context that resolves to a different mode

**Depends on**: Phase A (pause/resume), Feature B3 (context bridge)

---

## Phase B Dependency Graph

```
B1 (Rooms + ModeResolver) ──── B2 (Room tool execution)

Phase A (pause/resume) ──┬── B3 (DAG→Room context bridge)
                         │
                         └── B4 (Room completion → continue)
```

---

## Phase B End-to-End Pipeline

```
Document
  → Agent breaks into milestones (single step, structured JSON array)
  → For-each: agent writes plans per milestone (for_each step)
  → Room step: DAG PAUSES, room session created
      → Each agent has their milestone loaded as context (Feature B3)
      → User joins, discusses with all agents
      → Agents use tools during turns — research, etc. (Feature B2)
      → ModeResolver gives personality/tools per agent (Feature B1)
  → User closes room → DAG RESUMES (Feature B4, reuses Phase A infra)
  → Next step: agents in "refactor" mode produce final documents
  → Collection continues downstream
```

---

## Notes

- Variable resolution supports array indexing: `{milestones.0.context}` works today
- For-each uses `$` syntax: `{milestones.content.$.title}` for current element
- Agent assignment per step is static (set at design time, not dynamic)
- Room gatekeeper selects speakers intelligently per turn
- Room speakers execute sequentially (not parallel)
- "Listening room" concept (agents think in real-time between messages) is a future enhancement — expensive due to persistent LLM contexts, not needed for Phase A or B
