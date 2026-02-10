# Phase 2: Session Management and Context Injection

**Scope:** Backend — tie chat sessions to documenter steps, auto-inject context into the system prompt via the port manifest, and support session clearing.

## 2.1 Session Lifecycle

### What

Each documenter step gets exactly one chat session. The session is created lazily (on first Assistant tab open) and persists indefinitely. Users can clear the conversation to start fresh (same session ID, messages wiped).

### Implementation

**New endpoint: `POST /api/workflows/{workflow_id}/steps/{step_id}/assistant/session`**

Find-or-create semantics:

```rust
async fn get_or_create_assistant_session(
    state: AppState,
    workflow_id: Uuid,
    step_id: Uuid,
    user_id: UserId,
) -> Result<ChatSession> {
    // 1. Look for existing session with this step_id in draft_config
    if let Some(session) = repo.find_session_by_step(step_id).await? {
        return Ok(session);
    }

    // 2. Create new session
    let session = repo.create_session(CreateSession {
        user_id,
        agent_id: DOCUMENTER_ASSISTANT_AGENT_ID,
        mode_id: "documenter-assistant".into(),
        title: format!("Documenter Assistant"),
        draft_config: json!({
            "type": "documenter_assistant",
            "workflow_id": workflow_id,
            "step_id": step_id,
        }),
    }).await?;

    Ok(session)
}
```

**Database index:**

```sql
-- Fast lookup of session by step_id stored in draft_config
CREATE INDEX idx_chat_sessions_step_id
ON chat_sessions ((draft_config->>'step_id'))
WHERE draft_config->>'type' = 'documenter_assistant';
```

**New endpoint: `DELETE /api/workflows/{workflow_id}/steps/{step_id}/assistant/session/messages`**

Clears all messages for the session but keeps the session row. The next conversation starts fresh with context re-injected.

```rust
async fn clear_assistant_messages(
    state: AppState,
    workflow_id: Uuid,
    step_id: Uuid,
    user_id: UserId,
) -> Result<StatusCode> {
    let session = repo.find_session_by_step(step_id).await?
        .ok_or(AppError::NotFound)?;
    repo.clear_session_messages(session.id).await?;
    Ok(StatusCode::NO_CONTENT)
}
```

### Chat and streaming

The existing session-scoped endpoints handle the actual chat:

- `POST /api/sessions/{session_id}/chat` — send message
- `GET /api/sessions/{session_id}/chat/{message_id}/stream` — SSE stream
- `GET /api/sessions/{session_id}/chat/history` — load history

No changes needed to these. The frontend calls them directly once it has the session ID.

---

## 2.2 Context Injection

### What

When the chat consumer picks up a message for a documenter assistant session, the system prompt is dynamically enriched with the port manifest from the workflow. This happens on every message, not just the first — so the agent always sees the latest state.

### Implementation

**In `run_chat()` (or strategy construction), detect documenter assistant sessions:**

```rust
// In run_chat() or ChatStrategy::new()
let strategy = if session.draft_config_type() == "documenter_assistant" {
    let ctx = DocumenterToolContext::from_draft_config(&session.draft_config, &state).await?;
    build_documenter_strategy(ctx, &state).await?
} else {
    // existing chat strategy logic
    build_standard_strategy(...)
};
```

**`build_documenter_strategy` assembles:**

1. **Base system prompt** from the agent row (the carefully crafted prompt from Phase 1)
2. **Port manifest** appended to system prompt:

```
## Current Documenter State

### Step: "{step_name}"
Workflow: "{workflow_name}"

### Instruction Prompt
{prompt_template or "(No prompt set)"}

### Existing Document Definitions
{for each def:}
- **{name}** (id: {id})
  Description: {description or "none"}
  Target length: {target_length or "not set"}
{end for, or "(No documents defined yet)"}

### Incoming Context
{for each upstream step:}
- **{source_name}** (type: {execution_mode}, status: {content_status})
  Description: {step.description}
  {if content_status == "populated":}
  Preview: {first 500 chars of prompt_template}...
  Size: ~{word_count} words
  {else if content_status == "empty":}
  Note: Context node exists but has no content yet.
  {else if content_status == "pending":}
  Note: Will provide context at runtime when the workflow executes.
  {end if}
{end for, or "(No incoming context sources)"}
```

**Key:** The `Description:` line reads from `workflow_steps.description` — the column seeded in Phase 1. This is the step's identity, not its content. The `Preview:` line (only for populated context nodes) shows actual content from `prompt_template`.

3. **Tool list** — only the 6 documenter tools: `create_doc_def`, `update_doc_def`, `delete_doc_def`, `read_context`, `update_prompt`, `think`
4. **Model** — from the agent row (Sonnet by default)

### Context refresh strategy

The system prompt is rebuilt on **every user message**. This is the mechanism that keeps the agent aware of manual edits:

1. User adds a doc def manually in the Documents tab
2. User sends a message in the Assistant tab
3. `build_documenter_strategy()` re-reads from DB
4. System prompt now includes the manually-added def
5. Agent sees it and won't duplicate it

For mid-conversation awareness (agent wants to check state between tool calls within a single turn), the `read_context` tool provides the same data on demand.

---

## 2.3 Route Registration

### New routes

```rust
// In src/server/mod.rs route registration
.route(
    "/api/workflows/:workflow_id/steps/:step_id/assistant/session",
    post(get_or_create_assistant_session)
)
.route(
    "/api/workflows/:workflow_id/steps/:step_id/assistant/session/messages",
    delete(clear_assistant_messages)
)
```

### Frontend API additions

```typescript
// In api.ts, under workflows namespace
workflows.getOrCreateAssistantSession(
  workflowId: string,
  stepId: string,
): Promise<ChatSession>
// POST /workflows/{workflowId}/steps/{stepId}/assistant/session

workflows.clearAssistantMessages(
  workflowId: string,
  stepId: string,
): Promise<void>
// DELETE /workflows/{workflowId}/steps/{stepId}/assistant/session/messages
```

---

## 2.4 Session Cleanup

When a documenter step is deleted, its assistant session should be cleaned up:

```rust
// In delete_step handler (existing), add:
if step.execution_mode == "documenter" {
    if let Some(session) = repo.find_session_by_step(step.id).await? {
        repo.delete_session(session.id).await?;
    }
}
```

---

### Files to create/modify

| File | Change |
|------|--------|
| `migrations/0022_documenter_assistant.sql` | Index on `draft_config->>'step_id'` (combined with Phase 1 migration) |
| `src/db/traits/mod.rs` | Add `find_session_by_step(step_id)`, `clear_session_messages(session_id)` |
| `src/db/pg/sessions.rs` | Implement new trait methods |
| `src/server/api/workflows/mod.rs` | New handlers: `get_or_create_assistant_session`, `clear_assistant_messages` |
| `src/server/api/workflows/tests.rs` | Tests for new endpoints |
| `src/server/hub/mod.rs` | Detect documenter sessions in `run_chat()` |
| `src/server/hub/strategies/chat/mod.rs` | `build_documenter_strategy()` with port manifest injection |
| `src/server/mod.rs` | Register new routes |
| `frontend/src/api/api.ts` | Add typed endpoint methods |

### Tests

- Test find-or-create: first call creates, second call returns same session
- Test clear messages: history empty after clear, session still exists
- Test context injection: system prompt includes step name, prompt, doc defs, port manifest
- Test context refresh: after manual doc def change, next message sees updated context
- Test cleanup: deleting documenter step removes its assistant session
- Test port manifest construction:
  - Context node with content → `content_status: "populated"` with preview and word count
  - Context node without content → `content_status: "empty"`
  - Non-context step (single, for_each, etc.) → `content_status: "pending"`
  - All sources include `description` from `workflow_steps.description` column
- Test port manifest is rebuilt on every user message (not cached from first message)

## Acceptance Criteria

- [ ] `POST .../assistant/session` returns existing or creates new session
- [ ] Session `draft_config` contains `workflow_id`, `step_id`, `type`
- [ ] `DELETE .../assistant/session/messages` clears messages, keeps session
- [ ] System prompt dynamically built with port manifest on every message
- [ ] Port manifest includes: step name, description (from `workflow_steps.description`), content_status
- [ ] Populated sources include content preview (first 500 chars of `prompt_template`) and word count
- [ ] Pending sources include step description and "will provide context at runtime" note
- [ ] Empty sources flagged as "context node exists but has no content yet"
- [ ] Only documenter tools injected (not general tools)
- [ ] Existing session chat/stream endpoints work with the documenter session
- [ ] Step deletion cascades to session cleanup
- [ ] Frontend API client has typed methods for both endpoints
- [ ] All new handlers have integration tests
