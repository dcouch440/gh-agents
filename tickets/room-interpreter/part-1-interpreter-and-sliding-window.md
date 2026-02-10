# Part 1: Interpreter + Sliding Window (Backend Core)

**Scope:** Backend only — add the interpreter function, sliding window logic, prefixed message building, and user message tracking to room execution.

---

## 1.1 Interpreter Function

A new utility that calls Haiku to maintain a rolling summary of the conversation.

### What

The interpreter takes a previous summary (if any) and a batch of new transcript entries, then produces an updated rolling summary. It only runs when the message count exceeds the window size (lazy trigger). The summary is stored in the existing `room_sessions.transcript_summary` field.

### New file: `src/server/executors/room/interpreter.rs`

```rust
use crate::llm::{AnthropicClient, AnthropicConfig, LLMRequest, Message as LlmMessage};

const INTERPRETER_SYSTEM_PROMPT: &str = "\
You are a conversation summarizer for a multi-agent discussion room. \
Your job is to maintain a rolling summary of the discussion so far.\n\n\
You will receive:\n\
1. The previous summary (if any)\n\
2. A batch of new messages to incorporate\n\n\
Produce an updated summary that:\n\
- Captures key points, decisions, and disagreements\n\
- Identifies who said what (use participant names)\n\
- Is 3-6 sentences long\n\
- Preserves important details that participants might reference later\n\
- Drops small talk and redundant exchanges\n\
- Maintains chronological flow of the discussion";

const MAX_INTERPRETER_INPUT_CHARS: usize = 6000;
const MAX_INTERPRETER_OUTPUT_TOKENS: u32 = 512;

/// Run the interpreter to produce an updated rolling summary.
///
/// Takes the previous summary (if any) and a batch of new transcript entries,
/// returns an updated summary incorporating both.
pub async fn run_interpreter(
    previous_summary: Option<&str>,
    new_entries: &[FormattedTranscriptEntry],
    model_id: &str,
) -> Option<String> {
    let config = AnthropicConfig::from_env().ok()?;
    let client = AnthropicClient::new(config).ok()?;

    let mut user_text = String::new();

    // Previous summary
    match previous_summary {
        Some(s) if !s.is_empty() => {
            user_text.push_str("Previous summary:\n");
            user_text.push_str(s);
            user_text.push_str("\n\n");
        }
        _ => {
            user_text.push_str(
                "Previous summary:\n(No previous summary — this is the start of the conversation)\n\n"
            );
        }
    }

    // New messages to incorporate
    user_text.push_str("New messages to incorporate:\n");
    for entry in new_entries {
        user_text.push_str(&format!("[{}]: {}\n", entry.speaker_name, entry.content));
    }
    user_text.push_str("\nWrite an updated rolling summary.");

    // Truncate if too long
    let truncated: String = user_text.chars().take(MAX_INTERPRETER_INPUT_CHARS).collect();

    let request = LLMRequest::new(model_id, vec![LlmMessage::user(truncated)])
        .with_system(INTERPRETER_SYSTEM_PROMPT)
        .with_max_tokens(MAX_INTERPRETER_OUTPUT_TOKENS);

    match client.send_message(request).await {
        Ok(resp) => Some(resp.content),
        Err(e) => {
            tracing::warn!("Room interpreter failed: {}", e);
            None
        }
    }
}
```

### Shared type: `FormattedTranscriptEntry`

Used by both the interpreter and message builder:

```rust
/// A transcript entry with resolved speaker name.
pub struct FormattedTranscriptEntry {
    pub speaker_name: String,
    pub agent_id: Option<Uuid>,  // None for user messages
    pub content: String,
    pub is_user_message: bool,
}
```

### Constants

Add to `src/constants.rs`:

```rust
// ── Room Interpreter ────────────────────────────────────────────────────
pub const ROOM_INTERPRETER_WINDOW_SIZE: usize = 6;
pub const ROOM_INTERPRETER_MODEL: &str = MODEL_HAIKU;
```

---

## 1.2 Sliding Window Logic

### What

Integrated into `execute_room_turn()`. After loading the transcript, check if the message count exceeds the window size. If so, run the interpreter on the overflow messages that haven't been summarized yet (tracked by `interpreter_cursor`).

### Implementation

In `execute_room_turn()`, between loading the transcript (step 2) and the gatekeeper call (step 3), insert the window check:

```rust
// 2b. Sliding window — summarize old messages if window overflows
let interpreter_model = room.interpreter_model_id.as_deref()
    .unwrap_or(crate::constants::ROOM_INTERPRETER_MODEL);
let window_size = room.window_size.unwrap_or(
    crate::constants::ROOM_INTERPRETER_WINDOW_SIZE as i32
) as usize;

let interpreter_cursor = session.interpreter_cursor as usize;
let transcript_count = transcript.len();
let window_start = transcript_count.saturating_sub(window_size);

if room.interpreter_enabled && window_start > interpreter_cursor {
    // Messages [interpreter_cursor..window_start] need summarizing
    let entries_to_summarize: Vec<FormattedTranscriptEntry> = transcript
        [interpreter_cursor..window_start]
        .iter()
        .map(|e| format_entry(e, &members))
        .collect();

    if let Some(new_summary) = interpreter::run_interpreter(
        session.transcript_summary.as_deref(),
        &entries_to_summarize,
        interpreter_model,
    ).await {
        // Update session with new summary and cursor position
        room_repo.update_room_session_interpreter(
            session.id,
            &new_summary,
            window_start as i32,
        ).await.ok();
    }
}

// Window entries for message building
let window_entries = &transcript[window_start..];
```

### Cursor tracking

The `interpreter_cursor` integer on `room_sessions` tracks how many messages from the start of the transcript have been summarized. This avoids re-summarizing already-covered messages:

- Start: cursor=0, 4 messages → no overflow (4 < 6)
- Turn 4: cursor=0, 8 messages → overflow, summarize [0..2], cursor=2
- Turn 5: cursor=2, 10 messages → overflow, summarize [2..4], cursor=4
- Each run feeds the previous summary + new batch to the interpreter

---

## 1.3 Prefixed Message Building

### What

Replace the current `format_transcript()` + `build_speaker_prompt()` approach with a new function that builds individual LLM messages with speaker prefixes when interpreter mode is active.

### The key insight

For each agent, their own past messages become `assistant` role (natural continuation for the LLM), and everyone else's become `user` role with `[Name]:` prefix. This gives the LLM a first-person conversation perspective where it naturally continues as itself.

Consecutive non-agent messages are combined into a single `user` message to prevent API-level merging from losing structure.

### New function: `build_speaker_messages()`

```rust
/// Build the LLM message array for a speaker using the sliding window.
///
/// The agent's own past responses become `assistant` role messages.
/// All other speakers' messages (other agents + user) become `user` role
/// messages with `[SpeakerName]: ` prefix.
///
/// Consecutive non-agent messages are combined into a single user message
/// to avoid API-level merging losing the structure.
pub fn build_speaker_messages(
    summary: Option<&str>,
    window_entries: &[FormattedTranscriptEntry],
    current_user_message: &str,
    speaking_agent_id: Uuid,
    followup_context: &str,
) -> Vec<Message> {
    let mut messages: Vec<Message> = Vec::new();

    // 1. If there's an interpreter summary, inject it as the opening context
    if let Some(s) = summary {
        if !s.is_empty() {
            messages.push(Message::user(format!(
                "[Prior Discussion Summary]\n{}\n\n[Recent messages follow]",
                s
            )));
            messages.push(Message::assistant(
                "I understand the discussion context. I'll build on what's been discussed."
                    .to_string(),
            ));
        }
    }

    // 2. Recent window messages with speaker identity
    let mut pending_user_lines: Vec<String> = Vec::new();

    for entry in window_entries {
        let is_self = entry.agent_id == Some(speaking_agent_id);

        if is_self {
            // Flush any pending non-agent messages first
            if !pending_user_lines.is_empty() {
                messages.push(Message::user(pending_user_lines.join("\n")));
                pending_user_lines.clear();
            }
            // Agent's own past response -> assistant role (no prefix needed)
            messages.push(Message::assistant(entry.content.clone()));
        } else {
            // Other speaker -> accumulate as prefixed user message
            let prefix = if entry.is_user_message { "User" } else { &entry.speaker_name };
            pending_user_lines.push(format!("[{}]: {}", prefix, entry.content));
        }
    }

    // Flush remaining non-agent messages
    if !pending_user_lines.is_empty() {
        messages.push(Message::user(pending_user_lines.join("\n")));
    }

    // 3. Current user message
    let mut current = format!("[User]: {}", current_user_message);
    if !followup_context.is_empty() {
        current.push_str(&format!("\n\n[Facilitator note]: {}", followup_context));
    }
    messages.push(Message::user(current));

    messages
}
```

### Example output for agent "Bob"

Given a room with Bob, Gen, and a User — after the interpreter has summarized older messages:

```
user: "[Prior Discussion Summary]
Gen proposed focusing on authentication. The user asked about costs.
Bob suggested checking error logs. Gen agreed pooling is likely.

[Recent messages follow]"

A: "I understand the discussion context. I'll build on what's been discussed."

user: "[Gen]: Good point about the database, let me think about connection patterns."

A: "I've seen similar issues in the connection pooling layer. We should check the pool configuration."

user: "[User]: Can you elaborate on the connection pooling issue?
[Gen]: I agree with Bob, pooling is likely the root cause."
```

Bob's own prior responses are `assistant` role. Gen's and User's messages are `user` role with `[Name]:` prefix. Consecutive non-Bob messages are combined into one `user` message.

---

## 1.4 RoomSpeakerStrategy Adaptation

### What

The `RoomSpeakerStrategy` currently receives a pre-built `user_prompt` string and returns it as a single user message in `build_messages()`. We need it to optionally accept a pre-built message array for interpreter mode while keeping backward compatibility.

### Changes to `src/server/hub/strategies/room_speaker/mod.rs`

Add a new field to `RoomSpeakerConfig`:

```rust
pub struct RoomSpeakerConfig {
    // ... existing fields ...
    /// Pre-built message array (interpreter mode). If set, overrides user_prompt.
    pub messages: Option<Vec<Message>>,
}
```

Update `build_messages()`:

```rust
async fn build_messages(&self, _input: &str) -> Result<Vec<Message>, HubError> {
    if let Some(ref messages) = self.config.messages {
        Ok(messages.clone())
    } else {
        // Fallback: legacy single-prompt mode (interpreter_enabled = false)
        Ok(vec![Message::user(&self.config.user_prompt)])
    }
}
```

### Integration in `execute_room_turn()`

When building the speaker strategy:

```rust
// If interpreter mode is active, use build_speaker_messages()
let speaker_messages = if room.interpreter_enabled {
    let summary = session.transcript_summary.as_deref()
        .or_else(|| /* freshly computed summary from step 2b */);
    Some(build_speaker_messages(
        summary,
        &window_entries_formatted,
        user_message,
        selection.agent_id,
        &selection.followup_context,
    ))
} else {
    None // Legacy path — user_prompt used instead
};

let strategy = RoomSpeakerStrategy::new(
    RoomSpeakerConfig {
        // ... existing fields ...
        messages: speaker_messages,
    },
    state.clone(),
);
```

---

## 1.5 User Messages in Transcript

### What

Currently, user messages are NOT in the `room_transcript` view — only agent execution messages are. The transcript is built from `agent_executions` + `execution_messages`. User messages exist only as the `input` field of the first agent's execution.

For the sliding window to work properly, **user messages must be in the transcript** so the interpreter can summarize them and the window includes them.

### New table: `room_messages`

```sql
CREATE TABLE room_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_session_id UUID NOT NULL REFERENCES room_sessions(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    content TEXT NOT NULL,
    turn_number INT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_room_messages_session ON room_messages(room_session_id, turn_number);
```

### Updated send flow

In `send_room_message()`, insert the user message into `room_messages` BEFORE spawning the turn execution:

```rust
// Record the user message in the transcript
room_repo.insert_room_message(
    session.id,
    user_id,
    &request.content,
    session.current_turn + 1,
).await?;

// Then spawn execute_room_turn(...)
```

### Updated transcript loading

New repo method that returns a unified transcript with user messages interleaved:

```rust
/// Load the full transcript including user messages.
/// Returns FormattedTranscriptEntry entries ordered by created_at.
pub async fn get_room_transcript_with_user_messages(
    session_id: Uuid,
) -> Result<Vec<FormattedTranscriptEntry>>
```

Implementation: UNION query of agent transcript entries + room_messages, ordered by `created_at ASC`. User messages get `is_user_message = true` and `agent_id = None`.

```sql
SELECT agent_name, content, speaker_order, created_at, false AS is_user_message
FROM room_transcript_view WHERE room_session_id = $1
UNION ALL
SELECT 'User' AS agent_name, content, NULL AS speaker_order, created_at, true AS is_user_message
FROM room_messages WHERE room_session_id = $1
ORDER BY created_at ASC
```

---

## 1.6 Database Changes (Migration)

### Migration: `0024_room_interpreter.sql`

```sql
-- Room interpreter configuration
ALTER TABLE rooms
    ADD COLUMN interpreter_enabled BOOLEAN NOT NULL DEFAULT true,
    ADD COLUMN interpreter_model_id TEXT,
    ADD COLUMN window_size INT;

-- Interpreter cursor tracking on sessions
ALTER TABLE room_sessions
    ADD COLUMN interpreter_cursor INT NOT NULL DEFAULT 0;

-- User messages in room sessions (for transcript interleaving)
CREATE TABLE room_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_session_id UUID NOT NULL REFERENCES room_sessions(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    content TEXT NOT NULL,
    turn_number INT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_room_messages_session
    ON room_messages(room_session_id, turn_number);

-- Backfill: existing rooms get interpreter disabled (preserve current behavior)
-- New rooms will default to interpreter_enabled = true
UPDATE rooms SET interpreter_enabled = false WHERE interpreter_enabled = true;
```

### Model changes in `src/db/mod.rs`

```rust
pub struct RoomRow {
    // ... existing fields ...
    pub interpreter_enabled: bool,             // NEW
    pub interpreter_model_id: Option<String>,  // NEW — NULL = use default Haiku
    pub window_size: Option<i32>,              // NEW — NULL = use default (6)
}

pub struct RoomSessionRow {
    // ... existing fields ...
    pub interpreter_cursor: i32,               // NEW — tracks last summarized message index
}

/// Row type for user messages in a room session.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct RoomMessageRow {
    pub id: Uuid,
    pub room_session_id: Uuid,
    pub user_id: Uuid,
    pub content: String,
    pub turn_number: i32,
    pub created_at: DateTime<Utc>,
}
```

### New repo trait methods

```rust
// In room repo trait (src/db/traits/mod.rs):
async fn insert_room_message(
    session_id: Uuid, user_id: Uuid, content: &str, turn_number: i32
) -> Result<RoomMessageRow>;

async fn get_room_messages(session_id: Uuid) -> Result<Vec<RoomMessageRow>>;

async fn update_room_session_interpreter(
    session_id: Uuid, summary: &str, cursor: i32
) -> Result<()>;

async fn get_room_transcript_with_user_messages(
    session_id: Uuid
) -> Result<Vec<FormattedTranscriptEntry>>;
```

---

## Files to create/modify

| File | Change |
|------|--------|
| `migrations/0024_room_interpreter.sql` | New columns on rooms/room_sessions, room_messages table |
| `src/server/executors/room/interpreter.rs` | **New** — interpreter function, `FormattedTranscriptEntry` type |
| `src/server/executors/room/mod.rs` | Sliding window logic, `build_speaker_messages()`, updated turn flow, `pub mod interpreter;` |
| `src/server/hub/strategies/room_speaker/mod.rs` | Add `messages: Option<Vec<Message>>` to config, update `build_messages()` |
| `src/db/mod.rs` | New fields on `RoomRow`, `RoomSessionRow`, new `RoomMessageRow` struct |
| `src/db/pg_repo/mod.rs` | Implement new repo methods |
| `src/db/traits/mod.rs` | New trait methods for room messages and interpreter updates |
| `src/constants.rs` | `ROOM_INTERPRETER_WINDOW_SIZE`, `ROOM_INTERPRETER_MODEL` |
| `src/server/executors/room/tests.rs` | Tests for interpreter, window, message building |

---

## Tests

- **Interpreter function**: Given previous summary + new entries, produces updated summary (mock LLM)
- **Window overflow detection**: 4 messages + window_size=6 → no interpreter call; 8 messages → interpreter called on entries [0..2]
- **Cursor tracking**: After first overflow summarizes [0..2], cursor=2. After next overflow with 10 messages, summarizes [2..4], cursor=4.
- **`build_speaker_messages()`**:
  - Agent's own messages become `assistant` role
  - Other agents' messages get `[AgentName]:` prefix as `user` role
  - User messages get `[User]:` prefix
  - Consecutive non-agent messages combined into single `user` message
  - Summary injected as opening context with assistant acknowledgment
  - No summary → no opening context messages
  - Empty window → only current user message
- **Backward compatibility**: `interpreter_enabled = false` → uses legacy `format_transcript()` + `build_speaker_prompt()` path unchanged
- **User messages in transcript**: `insert_room_message()` stores message, unified transcript returns interleaved entries ordered by created_at
- **Interpreter failure**: If Haiku call fails, turn proceeds without summarization (graceful degradation)

---

## Acceptance Criteria

- [ ] `run_interpreter()` calls Haiku with previous summary + new entries, returns updated summary
- [ ] Interpreter only runs when transcript count exceeds window size AND cursor is behind
- [ ] `interpreter_cursor` updated after each interpreter run
- [ ] `transcript_summary` on `room_sessions` stores rolling interpreter output
- [ ] `build_speaker_messages()` produces correct LLM message array:
  - Agent's own past messages → `assistant` role (no prefix)
  - Other agents → `user` role with `[AgentName]:` prefix
  - User messages → `user` role with `[User]:` prefix
  - Consecutive non-agent messages combined into single `user` message
  - Summary injected as opening context pair
- [ ] User messages stored in `room_messages` table and interleaved in transcript
- [ ] `interpreter_enabled = false` → existing behavior unchanged (legacy path)
- [ ] New rooms default to `interpreter_enabled = true`
- [ ] Existing rooms migrated to `interpreter_enabled = false` (no behavior change)
- [ ] Interpreter failure does not block turn execution
- [ ] All new handlers have unit tests
