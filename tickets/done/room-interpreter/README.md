# Room Interpreter + Sliding Window — Enhanced Room Conversations

## Vision

Rooms currently give every agent the full transcript as a formatted text block in the user prompt. This works for short conversations but breaks down as discussions grow — context windows fill up, agents lose focus on recent points, and users can't send messages while agents are mid-generation without disrupting the flow.

The interpreter layer adds an intelligent compression mechanism: a lightweight LLM (Haiku) maintains a rolling summary of older conversation. Each agent sees this summary plus the last 6 raw messages with speaker identity preserved. The result is rooms that stay coherent over long discussions, cost less per turn, and handle async user participation naturally.

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Scope | Enhance existing room execution mode | Reuses proven infrastructure, backward compatible via `interpreter_enabled` flag |
| Interpreter model | Haiku (hardcoded default, configurable per room) | Cheapest/fastest, already used for summarization elsewhere |
| Window size | 6 messages (configurable per room) | ~2 full turn cycles of recent context |
| Speaker identity | `[Name]:` prefix per message in LLM array | Agent's own past responses as `assistant` role, everyone else as prefixed `user` role |
| Interpreter trigger | On window overflow only (lazy) | Cheaper — only runs when needed |
| Async user messages | Queue for next turn, display after current speaker | Natural flow, no mid-generation disruption |
| Summary storage | Existing `room_sessions.transcript_summary` + new `interpreter_cursor` | Minimal schema change, rolling summary updates in place |

## Architecture

```
User sends message
  |
  v
send_room_message() handler
  |
  +-- Is a turn already in progress?
  |     Yes -> Queue message, return { status: "queued" }
  |     No  -> Continue
  |
  v
execute_room_turn()
  |
  +-- Load full transcript (agent entries + user messages)
  +-- Check window overflow (count > window_size)
  |     Yes -> Run interpreter on messages [cursor..count-window_size]
  |             Update transcript_summary + interpreter_cursor
  |     No  -> Skip interpreter
  |
  +-- Gatekeeper selects speakers (unchanged)
  |
  +-- For each speaker:
  |     Build system prompt:
  |       agent base + room context
  |       + [Prior Discussion Summary] (interpreter summary)
  |     Build message array:
  |       Recent messages with [Name]: prefix
  |       Agent's own past responses as `assistant` role
  |       Current user message
  |     Execute via RoomSpeakerStrategy (unchanged engine)
  |     Stream tokens via WebSocket (unchanged)
  |
  +-- After all speakers complete:
        Broadcast TurnComplete
        Check for queued user messages -> if any, start next turn
```

## Parts

| Part | Ticket | Summary |
|------|--------|---------|
| 1 | [part-1-interpreter-and-sliding-window.md](./part-1-interpreter-and-sliding-window.md) | Backend: interpreter function, sliding window logic, prefixed message building, user messages in transcript |
| 2 | [part-2-user-message-queuing.md](./part-2-user-message-queuing.md) | Backend + frontend: turn lock, message queuing during active turns, WS events |
| 3 | [part-3-room-configuration.md](./part-3-room-configuration.md) | Backend API + frontend: interpreter settings in room config, queued message display |

## Message Format Example

For agent **Bob** in a room with Gen and a User, after the interpreter has summarized older messages:

```
system: "You are Bob, a security analyst...
         [room context: participants, roles, guidelines]"

user: "[Prior Discussion Summary]
Gen proposed focusing on authentication. The user asked about costs.
Bob suggested checking error logs. Gen agreed pooling is likely.

[Recent messages follow]"

assistant: "I understand the discussion context. I'll build on what's been discussed."

user: "[Gen]: Good point about the database, let me think about connection patterns."

assistant: "I've seen similar issues in the connection pooling layer. We should check the pool configuration."

user: "[User]: Can you elaborate on the connection pooling issue?
[Gen]: I agree with Bob, pooling is likely the root cause."
```

Bob's own past responses are `assistant` role (natural for the LLM). Gen's and User's messages are `user` role with `[Name]:` prefix. Consecutive non-Bob messages are combined into one `user` message.

## Implementation Order

Part 1 is the core — do this first. Part 2 (queuing) follows immediately as it depends on the turn execution flow. Part 3 (config + frontend) is independent and lower priority since defaults work out of the box.
