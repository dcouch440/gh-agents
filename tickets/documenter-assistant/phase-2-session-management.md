# Phase 2: Generic Step Chat Pipeline

**Scope:** Backend — create a unified chat pipeline that any workflow step can use. Replace the documenter-specific assistant concept with a generic `run_step_chat()` entry point. Prerequisite cleanup removes dead strategies and agent-centric plumbing.

## Background

Chat and workflow protocols were originally two completely different systems. The chat pipeline (`run_chat()`) is built around the "agent as standalone chatbot" paradigm — it loads an agent from DB, resolves modes, applies overlays. Workflow execution runs DAG steps with no session awareness.

We don't need most of the agent-centric infrastructure. Many strategies and systems (Cavernous, Router, ModeResolver, DraftConfig, InteractiveChat, agent_modes) were concepts along the way that never reached production. The patterns they encode (routing, mode selection, config classification) are handled better by the workflow DAG and protocol roles (e.g., the gatekeeper handles routing for rooms).

### What stays

- `ExecutionEngine` + `ExecutionStrategy` trait + `StreamSink` + `ExecutionRecorder` — the core engine
- `ChatStrategy` — becomes the one interactive chat path (reused for step chat)
- `DagStepStrategy` — pipeline step execution
- `RoomSpeakerStrategy` — speaker turns in multi-agent rooms
- Documenter pipeline strategies (Coordinator/Research/Writer) — DAG execution phases

### What goes (prerequisite cleanup)

| Component | Why |
|-----------|-----|
| `CavernousStepStrategy` | Two-phase LLM routing. Just use two DAG steps. |
| `RouterStrategy` | Gatekeeper does this better with structured output. |
| `InteractiveChatStrategy` | Agent execution review queue. Not workflow. |
| `ModeResolver` + `classify_mode()` + `apply_mode_overlay()` | Agent mode routing. Gatekeeper handles this for rooms. |
| `DraftConfig` + `run_chat_with_config()` | Workshop agent design sessions. Not needed. |
| Legacy `agent_modes` DB system | Underlying data for mode routing. |

---

## 2.1 Step Chat Pipeline (`run_step_chat`)

### What

A new entry point alongside `run_chat()` that builds `ChatConfig` from workflow step context instead of from an agent row. Any step type can have an interactive session — the system prompt, tools, and context are determined by the step's position in the DAG and its execution mode.

### Implementation

```rust
pub async fn run_step_chat(
    state: &AppState,
    provider: Arc<dyn LLMProvider + Send + Sync>,
    session_id: Uuid,
    workflow_id: Uuid,
    step_id: Uuid,
    message_id: Uuid,
    content: &str,
    user_id: UserId,
    cancel: Option<&CancellationToken>,
) -> Result<ExecutionResult, HubError> {
    // 1. Load step to determine execution_mode / protocol
    // 2. Build system prompt from protocol role definition
    // 3. Append live context (build_config_snapshot already exists)
    // 4. Resolve tools by step type
    // 5. Build ChatConfig, create ChatStrategy, execute
}
```

**No new strategy type.** `ChatStrategy` already handles session history, streaming, save response, auto-naming, and compaction. The only difference is how `ChatConfig` gets built.

### Context injection

The system prompt is rebuilt on **every user message** by calling `build_config_snapshot()` (already exists in `src/server/tools/documenter/mod.rs`). This ensures the agent always sees the latest state even if the user edits things manually between messages.

For step types that don't have a snapshot builder yet, the system prompt comes from the protocol role definition alone.

### Tool resolution by step type

Tools are determined by the step's `execution_mode`:

| execution_mode | Tools | Source |
|---------------|-------|--------|
| `documenter` | `create_doc_def`, `update_doc_def`, `delete_doc_def`, `update_config`, `think` | `execute_documenter_tool` |
| (future types) | TBD | Same pattern — register tool sets per mode |

The `execute_tool` override on `ChatStrategy` routes to the appropriate dispatcher based on step context stored in the strategy.

---

## 2.2 Session Lifecycle

### What

Any workflow step can have exactly one chat session. Created lazily on first interaction, persists indefinitely. Users can clear the conversation to start fresh.

### Implementation

**New endpoint: `POST /api/workflows/{workflow_id}/steps/{step_id}/chat/session`**

Find-or-create semantics. Session stores `workflow_id` and `step_id` for lookup.

**New DB method: `find_session_by_step(step_id)`**

```sql
CREATE INDEX idx_chat_sessions_step_id
ON chat_sessions ((draft_config->>'step_id'))
WHERE draft_config->>'step_id' IS NOT NULL;
```

**Message clearing:** `DELETE /api/workflows/{workflow_id}/steps/{step_id}/chat/messages`

Uses existing `clear_session_messages()` trait method.

**Step deletion cleanup:** When a step is deleted, its chat session is deleted too.

### Chat and streaming

The existing session-scoped endpoints handle the actual chat:

- `POST /api/sessions/{session_id}/chat` — send message
- `GET /api/sessions/{session_id}/chat/{message_id}/stream` — SSE stream
- `GET /api/sessions/{session_id}/chat/history` — load history

No changes needed. The frontend calls them directly once it has the session ID.

### Consumer routing

The chat consumer (`handle_message`) needs a new branch: if the session has a `step_id` in its config, call `run_step_chat()` instead of `run_chat()`.

---

## 2.3 Route Registration

```rust
.route(
    "/api/workflows/:workflow_id/steps/:step_id/chat/session",
    post(get_or_create_step_session)
)
.route(
    "/api/workflows/:workflow_id/steps/:step_id/chat/messages",
    delete(clear_step_messages)
)
```

### Frontend API additions

```typescript
workflows.getOrCreateStepSession(
  workflowId: string,
  stepId: string,
): Promise<ChatSession>

workflows.clearStepMessages(
  workflowId: string,
  stepId: string,
): Promise<void>
```

---

## Files to create/modify

| File | Change |
|------|--------|
| `src/server/hub/mod.rs` | Add `run_step_chat()` entry point |
| `src/server/hub/strategies/chat/mod.rs` | Support step-context tool routing in `execute_tool` |
| `src/server/executors/chat/mod.rs` | Add consumer branch for step sessions |
| `src/db/traits/mod.rs` | Add `find_session_by_step(step_id)` |
| `src/db/pg/sessions.rs` | Implement `find_session_by_step` |
| `migrations/NNNN_step_chat_sessions.sql` | Index on `draft_config->>'step_id'` |
| `src/server/api/workflows/mod.rs` | New handlers: `get_or_create_step_session`, `clear_step_messages` |
| `src/server/api/workflows/tests.rs` | Tests for new endpoints |
| `src/server/mod.rs` | Register new routes |
| `frontend/src/api/api.ts` | Add typed endpoint methods |

## Tests

- Find-or-create: first call creates, second returns same session
- Clear messages: history empty after clear, session still exists
- Context injection: system prompt includes step config + upstream context
- Context refresh: after manual edit, next message sees updated context
- Step deletion cascades to session cleanup
- Consumer routes step sessions to `run_step_chat()`, not `run_chat()`
- Tool resolution: documenter step gets documenter tools, not server tools

## Acceptance Criteria

- [ ] `run_step_chat()` builds ChatConfig from workflow step context
- [ ] System prompt rebuilt every message with live step state
- [ ] Tools resolved by step execution_mode
- [ ] `POST .../chat/session` returns existing or creates new step session
- [ ] `DELETE .../chat/messages` clears messages, keeps session
- [ ] Consumer detects step sessions and routes to `run_step_chat()`
- [ ] Existing session chat/stream endpoints work with step sessions
- [ ] Step deletion cascades to session cleanup
- [ ] Frontend API client has typed methods for both endpoints
- [ ] Prerequisite cleanup complete (dead strategies removed)
