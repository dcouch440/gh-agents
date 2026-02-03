# Agent Rooms — Implementation Gameplan

Bridge the existing DAG/pipeline orchestrator to support multi-agent room sessions with curated turn-taking.

---

## What We Have

- **DAG executor** — runs workflow steps with edges defining execution order and parallelism
- **Pipeline orchestrator** — sequential stages, each containing parallel workflow members
- **Agent executions** — full conversation logging per agent invocation (execution_messages)
- **Interactive agent pattern** — agent_executions with `is_interactive=true`, parent linking, and user chat
- **Tool router design** — gatekeeper concept, context window assembly, async passdowns (doc only, not built)

## What We Need

A room is a **shared, sequential conversation loop** where multiple agents participate turn-by-turn within a single user interaction. The existing pipeline model runs agents in isolation (each gets its own execution_messages). Rooms need agents to see each other's messages in a shared transcript.

---

## Phase 1: Room Data Model

No new tables required. Rooms map onto existing structures with one addition.

### Option A: Room as a pipeline stage execution (minimal schema change)

A room session is a `stage_execution` where multiple `agent_executions` share the same conversation context. The key change: agent_executions in a room read from a **shared message pool** instead of only their own execution_messages.

Add to `agent_executions`:

```sql
ALTER TABLE agent_executions ADD COLUMN room_session_id UUID;
ALTER TABLE agent_executions ADD COLUMN speaker_order INTEGER;

CREATE INDEX idx_agent_executions_room ON agent_executions(room_session_id)
    WHERE room_session_id IS NOT NULL;
```

- `room_session_id` — groups agent_executions that share a conversation. NULL for normal (non-room) executions.
- `speaker_order` — the order this agent spoke within the current turn. Set by the gatekeeper.

When `room_session_id` is set, the executor builds the LLM prompt using **all** execution_messages from executions sharing that room_session_id, not just the current agent_execution's messages.

### Room configuration on the workflow step

```sql
ALTER TABLE workflow_steps ADD COLUMN room_config JSONB;
```

Shape:

```json
{
    "enabled": true,
    "gatekeeper_agent_id": "uuid",
    "max_speakers_per_turn": 4,
    "turn_strategy": "gatekeeper",
    "agent_ids": ["uuid", "uuid", "uuid"]
}
```

This keeps rooms as a workflow step feature — no new top-level entity. A step with `room_config` runs as a room instead of a single agent execution.

---

## Phase 2: The Gatekeeper (Router Agent for Turn Selection)

The gatekeeper is a lightweight agent (Haiku-class) that decides who speaks and in what order for each user message.

### Input to gatekeeper

```json
{
    "user_message": "Should we refactor the auth module?",
    "conversation_history": [ ... ],
    "available_agents": [
        {
            "id": "uuid",
            "name": "Security Lead",
            "description": "Specializes in OWASP, CVE analysis, auth patterns",
            "recent_context": "Has been auditing the auth module for 3 days"
        },
        {
            "id": "uuid",
            "name": "Architecture Lead",
            "description": "System design, module boundaries, migration planning",
            "recent_context": "Designed the current service layer"
        },
        {
            "id": "uuid",
            "name": "Frontend Lead",
            "description": "React components, UI state, API integration",
            "recent_context": "Building the new agent dashboard"
        }
    ]
}
```

### Output from gatekeeper

```json
{
    "speakers": [
        {
            "agent_id": "uuid",
            "reason": "Security expertise directly relevant to auth refactor",
            "followup_context": "Focus on the CVEs you found in the JWT validation"
        },
        {
            "agent_id": "uuid",
            "reason": "Architecture decisions depend on security findings",
            "followup_context": "Consider the security agent's findings when proposing the new structure"
        }
    ],
    "skipped": [
        {
            "agent_id": "uuid",
            "reason": "Frontend not relevant to backend auth refactor discussion"
        }
    ]
}
```

The `followup_context` field is the key innovation you described — the gatekeeper doesn't just pick who talks, it **extends the prompt** with directed context for each speaker. This is how you steer the meeting.

### Gatekeeper system prompt

```
You manage a multi-agent conversation room. Your job is to decide which agents
should respond to the user's latest message and in what order.

Rules:
- Only include agents whose expertise is relevant to the current topic
- Order matters: put the agent whose input others should build on FIRST
- Provide followup_context to steer each agent toward the most productive response
- If the user addresses a specific agent by name, that agent speaks first
- If only one agent is needed, return just that one — don't waste tokens
- Consider the full conversation history, not just the latest message
```

### Implementation

1. User sends message to room
2. Append user message to shared transcript (execution_messages with room_session_id)
3. Call gatekeeper agent with roster + history + latest message
4. For each speaker in the returned order:
   a. Build context: agent's system prompt + shared transcript + gatekeeper's followup_context
   b. Call LLM
   c. Append response to shared transcript
   d. Stream response to frontend via WS
   e. Create token_ledger entry
5. Turn complete — wait for next user message

---

## Phase 3: Context Assembly for Room Participants

This is where the "mode switch" concept matters. Each agent enters the room with their project history but gets a **room-mode system prompt** layered on top.

### System prompt composition for a room participant

```
[Layer 1 — Agent identity]
{agent.system_prompt}
// "You are the Security Lead. You specialize in..."

[Layer 2 — Room mode injection]
You are participating in a group discussion with other AI agents and a human
facilitator. Other agents have their own expertise — build on their responses
rather than repeating what they said. Be concise. If you disagree with another
agent, say so directly and explain why.

[Layer 3 — Project context summary]
// Summarized from the agent's recent agent_executions
// "You've been working on: auth module audit. Key findings: 3 CVEs found..."

[Layer 4 — Gatekeeper direction]
// From the gatekeeper's followup_context for this speaker
// "Focus on the CVEs you found in the JWT validation"

[Layer 5 — Shared transcript]
// All execution_messages for this room_session_id
```

### Project context summarization

Before entering a room, each agent's recent work needs to be compressed. Raw execution_messages from their project work would blow the context window.

Strategy:
1. Query the agent's last N completed agent_executions
2. Pull only assistant messages with `structured_output` (the conclusions, not the reasoning)
3. Run a summarization call (Haiku) to produce a ~500 token briefing
4. Cache the briefing on the agent_execution row or in context_store

```sql
ALTER TABLE agent_executions ADD COLUMN context_summary TEXT;
```

This summary is what the gatekeeper sees as `recent_context` and what gets injected as Layer 3 in the room prompt.

---

## Phase 4: Shared Transcript Storage

Room messages need to be queryable as a unified conversation across multiple agent_executions.

### Option: Shared via room_session_id

All execution_messages in a room share the same `room_session_id` (derived from their parent agent_execution). To load the full transcript:

```sql
SELECT em.* FROM execution_messages em
JOIN agent_executions ae ON em.agent_execution_id = ae.id
WHERE ae.room_session_id = :room_session_id
ORDER BY em.created_at ASC;
```

Each message row still belongs to a specific agent_execution (for token accounting), but the room query joins them into a single thread.

### Frontend rendering

The frontend groups messages by agent_execution_id for visual distinction (name, color) but displays them in created_at order as a single conversation.

WS events for room turns:

```
{ "type": "room_speaker_start", "agent_id": "...", "agent_name": "Security Lead" }
{ "type": "room_token", "agent_id": "...", "content": "The current..." }
{ "type": "room_speaker_end", "agent_id": "..." }
{ "type": "room_turn_complete" }
```

---

## Phase 5: Meeting Lifecycle

### Starting a room

Two entry points:

1. **From a pipeline step** — the DAG executor hits a step with `room_config`, creates agent_executions for each participant with a shared `room_session_id`, and enters the room loop
2. **Ad-hoc from the UI** — user picks agents from a roster, creates a room session directly (powered chat with room mode)

### During the meeting

The room loop is:

```
while room is active:
    wait for user message
    call gatekeeper → get speaker order
    for each speaker:
        assemble context (identity + room mode + project summary + direction + transcript)
        call LLM
        append to shared transcript
        stream to frontend
    end for
end while
```

### Ending the meeting

The user closes the room or the pipeline advances to the next stage. On close:

1. Run a summarization pass over the full transcript → store as a document
2. Each agent's context_summary is updated with meeting outcomes
3. The summary document can be attached to downstream workflow steps via step_documents

---

## Phase 6: DAG/Pipeline Integration

### Room as a workflow step type

The existing DAG executor dispatches on `execution_mode`:
- `single` — one agent, one execution
- `for_each` — one agent per array element

Add:
- `room` — multiple agents, shared sequential conversation

When the executor encounters a `room` step:

1. Read `room_config` from the workflow_step
2. Create agent_executions for each participant + the gatekeeper
3. Set all to the same `room_session_id`
4. Enter the room loop (Phase 5)
5. On completion, the room's final summary becomes the step's `structured_output`
6. Downstream steps reference it via `output_variable_name` like any other step

### Output schema for rooms

The room's output isn't a single agent's structured response — it's a synthesis. Two options:

1. **Designated summarizer** — one agent in the room is tagged as the "closer" who produces the final structured output matching the step's output_schema
2. **Post-room summarization** — after the room loop ends, a separate LLM call reads the full transcript and produces structured output

Option 1 is simpler and keeps the room self-contained. The closer agent speaks last on the final turn and its response is parsed against the output_schema.

---

## Phase 7: Token Economics

Rooms are expensive. Each speaker on each turn sees the full transcript. With 4 agents and 10 turns, that's 40 LLM calls with growing context.

### Mitigations

1. **Gatekeeper filtering** — the whole point. 3 agents in the room but only 2 speak per turn = 33% savings
2. **Transcript summarization** — after N turns, summarize older messages and replace them in the context window. Keep the last 3-4 turns verbatim, summarize the rest.
3. **Model tiering** — gatekeeper runs on Haiku. Participants run on Sonnet. Only use Opus for the final synthesis.
4. **Max turns** — room_config includes `max_turns` to prevent runaway conversations. The gatekeeper can also signal "meeting complete" when the discussion has converged.

### Token tracking

Each agent_execution in the room gets its own token_ledger entries. The room's total cost is:

```sql
SELECT SUM(tl.cost_usd) FROM token_ledger tl
JOIN agent_executions ae ON tl.agent_execution_id = ae.id
WHERE ae.room_session_id = :room_session_id;
```

---

## Implementation Order

1. **Schema changes** — `room_session_id`, `speaker_order`, `room_config`, `context_summary` columns
2. **Gatekeeper agent** — system prompt, input/output schema, integration with agent execution
3. **Room executor** — the loop: user message → gatekeeper → sequential speaker calls → transcript append
4. **Context assembly** — project summary generation, room-mode prompt layering
5. **Shared transcript queries** — cross-execution message loading for room participants
6. **WS events** — room-specific events for frontend streaming
7. **Frontend** — group chat UI with per-agent visual distinction, gatekeeper transparency panel
8. **DAG integration** — `execution_mode: 'room'` in the workflow step executor
9. **Pipeline integration** — room output as step output, downstream variable resolution
10. **Token controls** — transcript summarization, max turns, model tiering

---

## What Already Works

- Agent definitions with system prompts and tool assignments → room participants
- execution_messages with role-based logging → shared transcript storage
- token_ledger per agent_execution → per-agent cost tracking in rooms
- Interactive agent pattern (is_interactive, parent linking) → precedent for multi-agent coordination on a step
- WS streaming for agent responses → extend to room speaker streaming
- Pipeline stage sequential execution → rooms are sequential within a turn, pipeline handles stage ordering

## What Needs Building

- Gatekeeper agent type and routing logic
- Room executor loop (the core new code)
- Cross-execution context assembly (shared transcript loading)
- Project context summarization for room entry
- Room-mode system prompt injection
- Frontend group chat rendering
- `execution_mode: 'room'` in the DAG executor dispatch
