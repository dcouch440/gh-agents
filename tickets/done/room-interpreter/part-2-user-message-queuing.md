# Part 2: User Message Queuing

**Scope:** Backend + frontend — handle user messages sent while a room turn is in progress. Messages queue, display after the current speaker, and get batched into the next turn.

---

## 2.1 Turn Lock

### What

Currently `send_room_message()` spawns a background task for `execute_room_turn()` and returns immediately. If the user sends another message while the turn is executing, a second turn would spawn concurrently — agents could see partial/racing transcripts.

We need a turn-level lock per session so concurrent messages queue instead of spawning parallel turns.

### Implementation

**In-memory lock via `AppState`:**

Room turns are single-server (no distributed execution yet), so an in-memory `DashMap` is the simplest approach.

```rust
// New: src/server/state/room_turns.rs (or inline in state/mod.rs)
use dashmap::DashMap;

pub struct RoomTurnManager {
    /// Active turns by session_id. If present, a turn is in progress.
    /// The Vec holds messages that arrived while the turn was running.
    active_turns: DashMap<Uuid, Vec<QueuedMessage>>,
}

#[derive(Debug, Clone)]
pub struct QueuedMessage {
    pub user_id: Uuid,
    pub content: String,
    pub queued_at: DateTime<Utc>,
}

impl RoomTurnManager {
    pub fn new() -> Self {
        Self { active_turns: DashMap::new() }
    }

    /// Try to acquire the turn lock. Returns true if this caller starts the turn.
    /// Returns false if a turn is already in progress (message was queued).
    pub fn try_start_turn(&self, session_id: Uuid) -> bool {
        // entry() is atomic — only one caller gets to insert
        self.active_turns.entry(session_id).or_insert_with(Vec::new);
        // If we just inserted (was absent), we own the turn
        // DashMap doesn't distinguish insert vs get — use a different approach:
        // Try insert with empty vec. If key already existed, it's a queue operation.
        !self.active_turns.contains_key(&session_id)
        // Better: use try_insert or check-then-insert pattern
    }

    /// Queue a message for an active turn.
    pub fn queue_message(&self, session_id: Uuid, msg: QueuedMessage) {
        if let Some(mut queue) = self.active_turns.get_mut(&session_id) {
            queue.push(msg);
        }
    }

    /// Drain queued messages. Returns None if no messages queued.
    pub fn drain_queue(&self, session_id: Uuid) -> Option<Vec<QueuedMessage>> {
        self.active_turns.get_mut(&session_id)
            .map(|mut q| std::mem::take(q.value_mut()))
            .filter(|q| !q.is_empty())
    }

    /// Release the turn lock.
    pub fn release_turn(&self, session_id: Uuid) {
        self.active_turns.remove(&session_id);
    }

    /// Check if a turn is active.
    pub fn is_turn_active(&self, session_id: Uuid) -> bool {
        self.active_turns.contains_key(&session_id)
    }
}
```

**Wire into `AppState`:**

```rust
// In AppStateInner:
pub room_turns: RoomTurnManager,
```

---

## 2.2 Updated `send_room_message()` Flow

### What

The handler checks the turn lock before spawning. If a turn is active, the message is queued and a WS event is broadcast. The background turn task drains the queue after each turn completes.

### Implementation

```rust
pub async fn send_room_message(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(session_id): Path<Uuid>,
    Json(request): Json<RoomMessageRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // ... existing validation (empty check, session status, load room/members) ...

    let user_id = auth.user_id.0;

    // Always record the user message in the transcript (Part 1)
    let room_repo = &state.repos().rooms;
    let session = /* loaded above */;
    room_repo.insert_room_message(
        session.id,
        user_id,
        &request.content,
        session.current_turn + 1,
    ).await?;

    // Check if a turn is already in progress
    if state.room_turns().is_turn_active(session_id) {
        // Queue the message for the next turn
        state.room_turns().queue_message(session_id, QueuedMessage {
            user_id,
            content: request.content.clone(),
            queued_at: Utc::now(),
        });

        // Broadcast queued event so frontend can display it
        state.broadcast_room(RoomEvent {
            room_session_id: session_id,
            run_id: None,
            user_id: Some(user_id),
            kind: RoomEventKind::MessageQueued {
                content: request.content,
                user_id,
            },
        });

        return Ok(Json(json!({
            "status": "queued",
            "session_id": session_id,
        })));
    }

    // No turn in progress — acquire lock and start
    state.room_turns().start_turn(session_id);

    // Spawn background turn with queue drain loop
    let state_clone = state.clone();
    let room_clone = room.clone();
    let user_message = request.content.clone();
    tokio::spawn(async move {
        // Execute the initial turn
        run_turn_and_drain(&state_clone, provider, &room_clone, session_id, &user_message, user_id).await;
    });

    Ok(Json(json!({
        "status": "processing",
        "session_id": session_id,
    })))
}
```

### Turn + drain loop

```rust
/// Execute a turn, then drain any queued messages into subsequent turns.
async fn run_turn_and_drain(
    state: &AppState,
    provider: Arc<dyn LLMProvider>,
    room: &RoomRow,
    session_id: Uuid,
    initial_message: &str,
    user_id: Uuid,
) {
    let mut current_message = initial_message.to_string();

    loop {
        // Reload session (turn counter may have changed)
        let session = match state.repos().rooms.get_room_session(session_id).await {
            Ok(Some(s)) => s,
            _ => break,
        };

        if session.status != "active" {
            break;
        }

        // Load members
        let members = /* load members as before */;

        // Execute the turn
        if let Err(e) = execute_room_turn(
            state,
            provider.clone(),
            room,
            &session,
            &members,
            &current_message,
            user_id,
            None,
        ).await {
            tracing::error!("Room turn execution error: {}", e);
            break;
        }

        // Check for queued messages
        match state.room_turns().drain_queue(session_id) {
            Some(messages) if !messages.is_empty() => {
                // Combine queued messages
                current_message = if messages.len() == 1 {
                    messages[0].content.clone()
                } else {
                    messages.iter()
                        .enumerate()
                        .map(|(i, m)| format!("{}. {}", i + 1, m.content))
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                // Continue loop — execute another turn
            }
            _ => {
                // No more queued messages — release the lock
                state.room_turns().release_turn(session_id);
                break;
            }
        }
    }

    // Safety: always release on exit
    state.room_turns().release_turn(session_id);
}
```

---

## 2.3 WebSocket Events

### New event kind

Add `MessageQueued` to `RoomEventKind` in `src/server/ws/events.rs`:

```rust
pub enum RoomEventKind {
    // ... existing variants ...

    /// A user message was queued because a turn is already in progress.
    /// The frontend should display this message after the current speaker.
    MessageQueued {
        content: String,
        user_id: Uuid,
    },
}
```

The frontend receives this event and:
1. Appends a "queued" entry to the local transcript state
2. Displays it with visual distinction (lighter text, "queued" badge)
3. Positioned after the current speaker's streaming output
4. When the next `SpeakerStart` event arrives, the queued visual transitions to normal

---

## 2.4 Message Combination Strategy

When multiple user messages are queued between turns, they're combined into a single message for the next turn:

**Single queued message:** Used as-is.

**Multiple queued messages:** Numbered list format:

```
1. Just kidding about the previous comment
2. Can we focus on the authentication module?
```

The interpreter naturally handles multi-part user messages — they appear as one user turn in the transcript and get summarized when they age out of the window.

Each individual queued message was already recorded in `room_messages` by `send_room_message()`, so the full transcript preserves the individual messages even though the combined version is what triggers the turn.

---

## Files to create/modify

| File | Change |
|------|--------|
| `src/server/state/mod.rs` | Add `RoomTurnManager` to `AppStateInner`, expose via `room_turns()` method |
| `src/server/api/rooms/mod.rs` | Updated `send_room_message()` with turn lock + queue check, new `run_turn_and_drain()` |
| `src/server/ws/events.rs` | Add `MessageQueued` variant to `RoomEventKind` |
| `frontend/src/stores/roomStore.ts` | Handle `MessageQueued` WS event, add queued message state |
| `frontend/src/types/room.ts` | Add `QueuedMessage` type or extend transcript entry type |

---

## Tests

- **Turn lock**: Second message while turn active → returns `{ "status": "queued" }`, first turn completes → queued messages trigger next turn
- **Queue drain**: 3 messages queued → combined into single numbered turn → queue empty → lock released
- **Lock release**: Turn completes with empty queue → lock released, no extra turn
- **Lock safety**: Turn errors → lock still released (no deadlock)
- **WS event**: `MessageQueued` broadcast with content and user_id when message is queued
- **No race conditions**: Concurrent `send_room_message()` calls don't corrupt the queue (DashMap is thread-safe)
- **Message recording**: Queued messages are still recorded in `room_messages` table even before the turn processes them
- **Session completion**: If session completes during drain loop, no further turns execute

---

## Acceptance Criteria

- [ ] Second message during active turn returns `{ "status": "queued" }`
- [ ] Queued messages combined and processed as next turn after current completes
- [ ] Queue drains completely before lock is released
- [ ] Lock always released even on error (no deadlock)
- [ ] `MessageQueued` WS event broadcast when message is queued
- [ ] No race conditions — concurrent sends don't corrupt transcript
- [ ] Queued messages recorded in `room_messages` immediately (not deferred)
- [ ] Frontend displays queued messages with visual distinction
- [ ] All new handlers have unit tests
