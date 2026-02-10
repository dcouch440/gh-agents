# Part 3: Room Configuration + Frontend

**Scope:** Backend API exposure + frontend settings UI for interpreter configuration and queued message display.

---

## 3.1 Room API Updates

### What

Expose the new interpreter fields in room create/update endpoints so users can configure the interpreter per room.

### CreateRoomRequest additions

```rust
pub struct CreateRoomRequest {
    // ... existing fields ...
    #[serde(default = "default_interpreter_enabled")]
    pub interpreter_enabled: bool,              // default: true
    pub interpreter_model_id: Option<String>,   // default: None (use Haiku)
    pub window_size: Option<i32>,               // default: None (use 6)
}

fn default_interpreter_enabled() -> bool { true }
```

### UpdateRoomRequest additions

```rust
pub struct UpdateRoomRequest {
    // ... existing fields ...
    pub interpreter_enabled: Option<bool>,
    pub interpreter_model_id: Option<String>,
    pub window_size: Option<i32>,
}
```

### Room response

`GET /api/rooms/:id` response includes the new fields. The `create_room()` and `update_room()` handlers pass the new fields through to the repo layer.

### SQL adjustments

The `create_room` query needs the three new columns:

```sql
INSERT INTO rooms (id, user_id, collection_id, name, gatekeeper_enabled,
    gatekeeper_model_id, max_speakers_per_turn, max_turns, tools_enabled,
    interpreter_enabled, interpreter_model_id, window_size)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
```

The `update_room` query conditionally sets the new fields (same pattern as existing nullable updates).

---

## 3.2 Frontend Types

### What

Update the TypeScript Room type to include interpreter configuration fields.

### In `frontend/src/types/room.ts`

```typescript
type Room = {
  // ... existing fields ...
  interpreter_enabled: boolean
  interpreter_model_id: string | null
  window_size: number | null
}

type CreateRoomRequest = {
  // ... existing fields ...
  interpreter_enabled?: boolean
  interpreter_model_id?: string | null
  window_size?: number | null
}

type UpdateRoomRequest = {
  // ... existing fields ...
  interpreter_enabled?: boolean
  interpreter_model_id?: string | null
  window_size?: number | null
}
```

---

## 3.3 Frontend Room Settings

### What

In the room configuration UI, add interpreter-related controls.

### Controls

- **Interpreter toggle** — Checkbox: "Enable conversation interpreter"
  - When on: interpreter summarizes old messages, agents see sliding window
  - When off: agents see full transcript (legacy behavior)
  - Default: on for new rooms

- **Window size** — Number input
  - Label: "Recent messages to keep verbatim"
  - Default: 6
  - Range: 2–20
  - Only shown when interpreter is enabled
  - Help text: "Messages older than this are summarized by the interpreter"

- **Interpreter model** — Dropdown
  - Options: Haiku (default), Sonnet (higher quality summaries)
  - Only shown when interpreter is enabled
  - Help text: "Model used to summarize older conversation"

### Placement

These controls go in the room settings panel, grouped under an "Interpreter" section below the existing Gatekeeper section. The visual grouping mirrors the gatekeeper pattern (toggle + dependent settings).

---

## 3.4 Frontend Queued Message Display

### What

When a `MessageQueued` WS event arrives, the frontend should display the queued user message in the transcript with visual distinction.

### Implementation

**In `roomStore.ts`:**

Add a `queuedMessages` state field:

```typescript
type RoomStoreState = {
  // ... existing fields ...
  queuedMessages: Map<string, QueuedMessage[]>  // keyed by session_id
}

type QueuedMessage = {
  content: string
  user_id: string
  queued_at: string
}
```

**WS handler:**

```typescript
// On ROOM_EVENT.MESSAGE_QUEUED:
case 'message_queued': {
    const sessionId = event.room_session_id;
    const existing = state.queuedMessages.get(sessionId) ?? [];
    existing.push({
        content: event.data.content,
        user_id: event.data.user_id,
        queued_at: new Date().toISOString(),
    });
    state.queuedMessages.set(sessionId, existing);
    break;
}

// On ROOM_EVENT.SPEAKER_START (next turn beginning):
// Clear queued messages — they're now being processed
case 'speaker_start': {
    state.queuedMessages.delete(event.room_session_id);
    break;
}
```

**Visual display:**

Queued messages appear in the transcript area with:
- Lighter text color (e.g., `text-gray-400` / 50% opacity)
- Small "queued" badge or "(waiting for current turn to finish)" note
- Positioned after the current speaker's streaming output
- Transitions to normal display when the next turn starts

---

## 3.5 API Client Updates

### In `frontend/src/api/api.ts`

The room create/update methods already accept the full request body, so no changes needed to the API client methods themselves — the types drive the payload.

Verify that the room response type in the API layer matches the updated `Room` type.

---

## Files to create/modify

| File | Change |
|------|--------|
| `src/server/api/rooms/mod.rs` | Add interpreter fields to `CreateRoomRequest`, `UpdateRoomRequest`, wire through to repo |
| `src/db/pg_repo/mod.rs` | Update `create_room()` and `update_room()` queries with new columns |
| `frontend/src/types/room.ts` | Add interpreter fields to `Room`, `CreateRoomRequest`, `UpdateRoomRequest` |
| `frontend/src/stores/roomStore.ts` | Add `queuedMessages` state, handle `MessageQueued` and `SpeakerStart` events |
| `frontend/src/api/api.ts` | Verify room response type includes new fields |
| Room settings component (TBD) | Add interpreter configuration section |

---

## Tests

### Backend
- **API**: Create room with `interpreter_enabled: true` → stored correctly, returned in GET
- **API**: Create room without interpreter fields → defaults applied (`interpreter_enabled: true`, NULL model/window)
- **API**: Update room `window_size: 10` → reflected in GET response
- **API**: Update room `interpreter_enabled: false` → field updated, model/window fields preserved

### Frontend
- **Types**: Room type includes interpreter fields
- **Store**: `MessageQueued` event adds entry to `queuedMessages` map
- **Store**: `SpeakerStart` event clears queued messages for that session
- **Settings**: Interpreter toggle shows/hides dependent controls
- **Settings**: Window size input validates range (2-20)

---

## Acceptance Criteria

- [ ] `interpreter_enabled`, `interpreter_model_id`, `window_size` accepted in room create/update API
- [ ] New fields returned in room GET response
- [ ] Default values applied when fields are omitted
- [ ] Room settings UI shows interpreter configuration section
- [ ] Interpreter toggle shows/hides window size and model controls
- [ ] Queued messages displayed with visual distinction in room transcript
- [ ] Queued messages cleared when next turn starts
- [ ] Frontend room type updated with new fields
- [ ] All API changes have integration tests
