# Agent Rooms — Full Implementation Plan

## Design Decisions (from discussion)

- **Rooms are pipeline-scoped** — defined within a pipeline, not standalone top-level entities
- **Gatekeeper is optional and system-managed** — hardcoded static prompt in Rust, fed roster/tools from DB at runtime. If no gatekeeper, all agents speak every turn in display_order
- **Agents use room mode if available**, otherwise default system prompt + room context layer on top. Not all agents will have modes — that's fine, the room context layer handles the meeting behavior
- **Transcript format**: labeled transcript block injected as system context (best LLM performance — the agent sees its identity in the system prompt, the meeting transcript as context, and the latest user message as the actual prompt)
- **Room output**: room agents are discussion-oriented, no structured output schemas. For pipeline integration, a post-room summarizer call or full transcript passdown handles downstream consumption
- **Tools configurable per room** — `tools_enabled` flag. Default off (conversational)
- **@ mentions supported** — parsed from user message, passed to gatekeeper as hints, mentioned agents speak first
- **Room size target**: 5-8 agents. Transcript compression after ~5 turns
- **DAG integration**: `execution_mode='room'` on workflow_steps (phase 2, standalone first)

---

## Step 1: Migration 055 — Room Tables

**File**: `migrations/055_create_rooms.sql`

Creates `rooms` and `room_members` tables. Rooms are pipeline-scoped.

```sql
-- Room definitions (pipeline-scoped)
CREATE TABLE rooms (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    pipeline_id UUID NOT NULL REFERENCES pipelines(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    gatekeeper_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    gatekeeper_model_id TEXT NOT NULL DEFAULT 'claude-haiku-4-20250414',
    max_speakers_per_turn INTEGER NOT NULL DEFAULT 4,
    max_turns INTEGER NOT NULL DEFAULT 20,
    tools_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_rooms_user ON rooms(user_id);
CREATE INDEX idx_rooms_pipeline ON rooms(pipeline_id);

-- Room membership (join table)
CREATE TABLE room_members (
    room_id UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    display_name TEXT,                    -- override agent name for room context
    role_description TEXT NOT NULL,       -- what gatekeeper sees: "Security specialist, OWASP expert"
    display_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (room_id, agent_id)
);

CREATE INDEX idx_room_members_agent ON room_members(agent_id);
```

**Why no gatekeeper_agent_id**: The gatekeeper is system-managed with a hardcoded prompt. We only store `gatekeeper_model_id` to control which model runs it. `gatekeeper_enabled` defaults to FALSE — without it, all agents speak every turn in `display_order`. Enable it for larger rooms where selective turn-taking matters.

---

## Step 2: Migration 056 — Room Sessions and Execution Columns

**File**: `migrations/056_room_sessions.sql`

Runtime tracking for active room conversations.

```sql
-- Room sessions (runtime — one per active room conversation)
CREATE TABLE room_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_id UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    run_id UUID REFERENCES pipeline_runs(id) ON DELETE CASCADE,  -- NULL for ad-hoc rooms
    status TEXT NOT NULL DEFAULT 'active',           -- 'active', 'completed', 'cancelled'
    current_turn INTEGER NOT NULL DEFAULT 0,
    transcript_summary TEXT,                         -- compressed older turns
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX idx_room_sessions_room ON room_sessions(room_id);
CREATE INDEX idx_room_sessions_run ON room_sessions(run_id) WHERE run_id IS NOT NULL;
CREATE INDEX idx_room_sessions_status ON room_sessions(status);

-- Link agent_executions to room sessions
ALTER TABLE agent_executions ADD COLUMN room_session_id UUID REFERENCES room_sessions(id);
ALTER TABLE agent_executions ADD COLUMN speaker_order INTEGER;

CREATE INDEX idx_agent_executions_room ON agent_executions(room_session_id)
    WHERE room_session_id IS NOT NULL;

-- Optional: link workflow_steps to rooms for DAG integration (phase 2)
ALTER TABLE workflow_steps ADD COLUMN room_id UUID REFERENCES rooms(id);
```

---

## Step 3: DB Row Types

**File**: `src/db/mod.rs`

Add row structs following existing patterns (`#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]`):

```rust
// After existing row types

pub struct RoomRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub pipeline_id: Uuid,
    pub name: String,
    pub gatekeeper_enabled: bool,
    pub gatekeeper_model_id: String,
    pub max_speakers_per_turn: i32,
    pub max_turns: i32,
    pub tools_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct RoomMemberRow {
    pub room_id: Uuid,
    pub agent_id: Uuid,
    pub display_name: Option<String>,
    pub role_description: String,
    pub display_order: i32,
}

pub struct RoomSessionRow {
    pub id: Uuid,
    pub room_id: Uuid,
    pub run_id: Option<Uuid>,
    pub status: String,
    pub current_turn: i32,
    pub transcript_summary: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}
```

Update `AgentExecutionRow` to include new optional fields:
```rust
// Add to existing AgentExecutionRow
pub room_session_id: Option<Uuid>,
pub speaker_order: Option<i32>,
```

Update `WorkflowStepRow` to include:
```rust
pub room_id: Option<Uuid>,
```

---

## Step 4: Repository Traits

**File**: `src/db/traits.rs`

Add `RoomRepo` trait following existing patterns:

```rust
#[async_trait]
pub trait RoomRepo: Send + Sync {
    // Room CRUD
    async fn create_room(&self, user_id: Uuid, pipeline_id: Uuid, name: &str, gatekeeper_model_id: &str, max_speakers: i32, max_turns: i32, tools_enabled: bool) -> Result<RoomRow>;
    async fn get_room(&self, id: Uuid) -> Result<Option<RoomRow>>;
    async fn list_rooms_for_pipeline(&self, pipeline_id: Uuid) -> Result<Vec<RoomRow>>;
    async fn update_room(&self, id: Uuid, name: &str, gatekeeper_model_id: &str, max_speakers: i32, max_turns: i32, tools_enabled: bool) -> Result<RoomRow>;
    async fn delete_room(&self, id: Uuid) -> Result<()>;

    // Room members (join table — follows agent_tools pattern)
    async fn list_room_members(&self, room_id: Uuid) -> Result<Vec<RoomMemberRow>>;
    async fn add_room_member(&self, room_id: Uuid, agent_id: Uuid, display_name: Option<&str>, role_description: &str, display_order: i32) -> Result<()>;
    async fn remove_room_member(&self, room_id: Uuid, agent_id: Uuid) -> Result<()>;
    async fn set_room_members(&self, room_id: Uuid, members: Vec<RoomMemberInput>) -> Result<()>;

    // Room sessions (runtime)
    async fn create_room_session(&self, room_id: Uuid, run_id: Option<Uuid>) -> Result<RoomSessionRow>;
    async fn get_room_session(&self, id: Uuid) -> Result<Option<RoomSessionRow>>;
    async fn update_room_session_status(&self, id: Uuid, status: &str) -> Result<()>;
    async fn increment_room_session_turn(&self, id: Uuid) -> Result<i32>;
    async fn set_transcript_summary(&self, id: Uuid, summary: &str) -> Result<()>;

    // Room transcript (cross-execution message loading)
    async fn get_room_transcript(&self, room_session_id: Uuid) -> Result<Vec<RoomTranscriptEntry>>;
}
```

Where `RoomTranscriptEntry` is:
```rust
pub struct RoomTranscriptEntry {
    pub agent_name: String,
    pub role_description: String,
    pub content: String,
    pub speaker_order: i32,
    pub turn_number: i32,
    pub created_at: DateTime<Utc>,
}
```

The transcript query joins `execution_messages` → `agent_executions` → `agents` + `room_members` to produce labeled entries.

---

## Step 5: Repository Implementation

**File**: `src/db/pg_repo.rs`

Implement `RoomRepo` on `PgRepo`. Key queries:

**Room transcript** (the most important query):
```sql
SELECT
    COALESCE(rm.display_name, a.name) AS agent_name,
    rm.role_description,
    em.content,
    ae.speaker_order,
    ae.room_session_id,
    em.created_at
FROM execution_messages em
JOIN agent_executions ae ON em.agent_execution_id = ae.id
JOIN agents a ON ae.agent_id = a.id
LEFT JOIN room_members rm ON rm.agent_id = ae.agent_id
    AND rm.room_id = (SELECT room_id FROM room_sessions WHERE id = $1)
WHERE ae.room_session_id = $1
    AND em.role IN ('user', 'assistant')
ORDER BY em.created_at ASC
```

**Set room members** (follows `set_agent_tools` transaction pattern):
```rust
// DELETE all existing + INSERT batch in a transaction
```

---

## Step 6: Gatekeeper Prompt (Hardcoded)

**File**: `src/agents/gatekeeper.rs` (new file)

```rust
pub const GATEKEEPER_SYSTEM_PROMPT: &str = r#"
You manage a multi-agent conversation room. You decide which agents should
respond to the user's latest message and in what order.

## Rules

1. Only include agents whose expertise is relevant to the current topic.
2. Order matters — put the agent whose input others should build on FIRST.
3. Provide followup_context to steer each agent toward a productive response.
4. If the user @mentions an agent by name, that agent MUST speak first.
5. If only one agent is relevant, return just that one.
6. Never include more agents than max_speakers_per_turn.
7. Consider the full conversation history, not just the latest message.

## Response Format (JSON only)

{
    "speakers": [
        {
            "agent_id": "<uuid>",
            "followup_context": "<directed prompt for this speaker>"
        }
    ]
}
"#;
```

**Gatekeeper input assembly** (also in this file):

```rust
pub struct GatekeeperInput {
    pub user_message: String,
    pub mentions: Vec<String>,          // parsed @ mentions
    pub transcript_summary: String,     // recent turns
    pub roster: Vec<RosterEntry>,       // from room_members
    pub max_speakers: i32,
}

pub struct RosterEntry {
    pub agent_id: Uuid,
    pub name: String,
    pub role_description: String,
}

pub struct GatekeeperOutput {
    pub speakers: Vec<SpeakerSelection>,
}

pub struct SpeakerSelection {
    pub agent_id: Uuid,
    pub followup_context: String,
}
```

**Gatekeeper call**: builds an LLM request with:
- System: `GATEKEEPER_SYSTEM_PROMPT`
- User message: JSON blob with `user_message`, `mentions`, `roster`, `max_speakers`, and last 3-4 turns of transcript

Parses the JSON response into `GatekeeperOutput`. If parsing fails, falls back to all agents in display_order.

**@ mention parsing**: simple regex `@(\w+)` matched against `room_members.display_name` or `agents.name`.

---

## Step 7: Room Executor

**File**: `src/server/room_executor.rs` (new file)

This is the core loop. Handles one user turn in a room session.

```rust
pub async fn execute_room_turn(
    state: &AppState,
    provider: &dyn LLMProvider,
    room: &RoomRow,
    session: &RoomSessionRow,
    members: &[RoomMemberWithAgent],  // room_members joined with agents
    user_message: &str,
) -> Result<Vec<RoomTurnResponse>, HubError>
```

**Flow per turn:**

1. **Parse @ mentions** from `user_message`
2. **Record user message** as an `execution_message` (role='user') linked to a room-level agent_execution
3. **Determine speaker order:**
   - If `room.gatekeeper_enabled` → call gatekeeper LLM with roster + transcript + mentions → parse speaker order
   - If gatekeeper disabled → all members speak in `display_order`. If @ mentions present, mentioned agents go first, rest follow in display_order
4. **For each speaker in order:**
   a. Load agent row + optional room mode from `agent_modes`
   b. **Assemble context**:
      - Layer 1: Agent's system prompt (or room mode's `system_prompt_suffix` appended)
      - Layer 2: Room context injection — hardcoded preamble: "You are in a group meeting with other agents..."
      - Layer 3: Gatekeeper's `followup_context` for this speaker
      - Layer 4: Transcript so far (labeled, from `get_room_transcript`)
   c. **Create agent_execution** row with `room_session_id`, `speaker_order`
   d. **Call LLM** — streaming, with tools if `room.tools_enabled` + agent has tools
   e. **Record response** as execution_messages
   f. **Broadcast WS event**: `room_speaker_start`, streamed tokens, `room_speaker_end`
   g. **Create token_ledger entry**
5. **Increment turn counter** on room_session
6. **Check turn limit** — if `current_turn >= max_turns`, set session status to 'completed'
7. **Transcript compression** — if turn > 5, summarize turns 1 through (current-3) into `transcript_summary` using a Haiku call. Recent 3 turns stay verbatim.

**Transcript format injected into each agent's prompt:**

```
## Meeting Transcript

[Summary of earlier discussion]
{session.transcript_summary}

[Recent turns]
---
**User**: Should we refactor the auth module?

**Security Lead** (turn 1, speaker 1):
The current implementation has three CVEs...

**Architecture Lead** (turn 1, speaker 2):
Given those vulnerabilities, I'd restructure into...
---
```

This goes into the system prompt as context. The actual user message for the LLM call is the gatekeeper's `followup_context` + the original user message.

---

## Step 8: WebSocket Events

**File**: `src/server/ws.rs`

Add `RoomUpdate` to the `ServerMessage` enum:

```rust
pub enum ServerMessage {
    // ... existing variants
    RoomUpdate { data: RoomUpdateEvent },
}

pub struct RoomUpdateEvent {
    pub room_session_id: Uuid,
    pub run_id: Option<Uuid>,
    pub event: String,              // "speaker_start", "speaker_token", "speaker_end", "turn_complete", "session_complete"
    pub agent_id: Option<Uuid>,
    pub agent_name: Option<String>,
    pub content: Option<String>,    // token content for streaming
    pub speaker_order: Option<i32>,
    pub turn_number: Option<i32>,
    pub user_id: Option<Uuid>,
    pub timestamp: DateTime<Utc>,
}
```

Add a `rooms` channel to the subscription system. Filter by `user_id` like existing channels.

If the room is pipeline-scoped (`run_id` is set), also broadcast on the `pipelines` channel so the execution tree UI picks it up.

---

## Step 9: API Endpoints

**File**: `src/server/api.rs`

New routes under `/api/rooms/`:

```
POST   /api/rooms                           — Create room (within a pipeline)
GET    /api/rooms/:id                       — Get room definition
PUT    /api/rooms/:id                       — Update room config
DELETE /api/rooms/:id                       — Delete room
GET    /api/pipelines/:id/rooms             — List rooms for a pipeline

POST   /api/rooms/:id/members               — Add member
DELETE /api/rooms/:id/members/:agent_id      — Remove member
PUT    /api/rooms/:id/members                — Set all members (replace)

POST   /api/rooms/:id/sessions               — Start a room session
GET    /api/room-sessions/:id                 — Get session status
POST   /api/room-sessions/:id/messages        — Send user message (triggers room turn)
GET    /api/room-sessions/:id/transcript       — Get full transcript
POST   /api/room-sessions/:id/close           — End session
```

The key endpoint is `POST /api/room-sessions/:id/messages`. This:
1. Accepts `{ "content": "..." }`
2. Calls `execute_room_turn()`
3. Streams responses via WebSocket (the HTTP response returns immediately with a turn ID)
4. Returns `{ "turn_id": "...", "status": "processing" }`

---

## Step 10: DAG Integration (Phase 2)

**File**: `src/server/dag_executor.rs`

Add room dispatch in the execution mode check:

```rust
if step.execution_mode == "for_each" {
    // ... existing for_each logic
} else if step.execution_mode == "room" {
    // Room execution
    let room_id = step.room_id.ok_or(HubError::MissingRoomId)?;
    let room = state.repo.get_room(room_id).await?;
    let members = state.repo.list_room_members_with_agents(room_id).await?;
    let session = state.repo.create_room_session(room_id, Some(ctx.run_id)).await?;

    // The room runs as an interactive session — pipeline pauses
    // User interacts via POST /api/room-sessions/:id/messages
    // When user closes room, output = full transcript or summary
    // Pipeline continues with next step
} else {
    // ... existing single execution logic
}
```

Room output for downstream steps: the full transcript as a JSON array of `{ speaker, content }` objects stored in `agent_executions.output` for the room's parent execution. Downstream `{variable}` refs access it.

---

## Step 11: Update `create_agent_execution` Signature

**File**: `src/db/pg_repo.rs` (line ~1379)

Add `room_session_id: Option<Uuid>` and `speaker_order: Option<i32>` parameters to `create_agent_execution`. Update the INSERT query to include these columns.

Update the trait in `src/db/traits.rs` (`AgentExecutionRepo`) to match.

Update all call sites in:
- `src/server/dag_executor.rs` — pass `None, None` for existing non-room calls
- `src/server/hub/dag.rs` — same
- `src/server/room_executor.rs` — pass actual values

---

## Step 12: Tests

### Unit tests (colocated):

- `src/agents/gatekeeper.rs` — test @ mention parsing, gatekeeper output parsing, fallback on parse failure
- `src/server/room_executor.rs` — test transcript formatting, context assembly layers, turn limit enforcement

### Integration tests:

- `tests/room_integration.rs`:
  - Create room with members → verify DB state
  - Start session → send message → verify gatekeeper called → verify speakers execute in order
  - Verify transcript query returns correct ordered messages
  - Verify WS events broadcast in correct sequence
  - Verify turn limit closes session
  - Verify transcript compression triggers after 5 turns
  - Verify @ mention forces agent to speak first
  - Verify tools_enabled=false prevents tool calls
  - Verify tools_enabled=true allows tool calls

### DB tests:

- `src/db/pg_repo.rs` (in `#[cfg(test)]` module):
  - Room CRUD
  - Room member set/add/remove (transaction atomicity)
  - Room transcript query joins correctly
  - Room session status transitions

---

## File Summary

| File | Action | Description |
|------|--------|-------------|
| `migrations/055_create_rooms.sql` | **Create** | rooms + room_members tables |
| `migrations/056_room_sessions.sql` | **Create** | room_sessions table + ALTER agent_executions + ALTER workflow_steps |
| `src/db/mod.rs` | **Edit** | Add RoomRow, RoomMemberRow, RoomSessionRow, RoomTranscriptEntry structs. Update AgentExecutionRow, WorkflowStepRow |
| `src/db/traits.rs` | **Edit** | Add RoomRepo trait. Update AgentExecutionRepo::create_agent_execution signature |
| `src/db/pg_repo.rs` | **Edit** | Implement RoomRepo. Update create_agent_execution |
| `src/agents/gatekeeper.rs` | **Create** | Hardcoded prompt, input/output types, gatekeeper LLM call, @ mention parsing |
| `src/server/room_executor.rs` | **Create** | Room turn loop, context assembly, transcript formatting, compression |
| `src/server/ws.rs` | **Edit** | Add RoomUpdate event, rooms channel |
| `src/server/api.rs` | **Edit** | Add room CRUD + session + message endpoints |
| `src/server/dag_executor.rs` | **Edit** | Add execution_mode='room' dispatch (phase 2) |
| `src/server/hub/dag.rs` | **Edit** | Same room dispatch for new hub architecture |
| `tests/room_integration.rs` | **Create** | Integration tests |

---

## Verification

1. `cargo check` — compiles with new types and traits
2. `cargo test` — all existing tests still pass + new room tests pass
3. `cargo clippy` — no warnings
4. Manual test flow:
   - Run migrations against local Postgres
   - Create a pipeline with a room via API
   - Add 3 agents as room members
   - Start a room session
   - Send a message → verify gatekeeper selects speakers → verify agents respond sequentially
   - Check WS events arrive in correct order
   - Verify transcript query returns full conversation
   - Send 6+ messages → verify transcript compression kicks in
   - Close session → verify status updates

---

## Implementation Order

Execute in this exact sequence — each step depends on the prior:

1. Migration 055 (rooms + room_members)
2. Migration 056 (room_sessions + agent_execution columns)
3. Row types in `src/db/mod.rs`
4. Repo traits in `src/db/traits.rs`
5. Repo implementation in `src/db/pg_repo.rs`
6. Gatekeeper module `src/agents/gatekeeper.rs`
7. Room executor `src/server/room_executor.rs`
8. WS events in `src/server/ws.rs`
9. API endpoints in `src/server/api.rs`
10. Update `create_agent_execution` signature + all call sites
11. DAG integration (phase 2)
12. Tests
