# Enhanced Room Execution — Rooms as Conversation Nodes in Workflow DAGs

## Overview

Enhanced Room Execution introduces rooms as first-class conversation nodes within workflow DAGs. Instead of a step executing a single agent, a **room step** creates a multi-agent conversation where agents discuss a topic, debate, and synthesize findings. Each agent's output then flows downstream via ports to subsequent steps.

This is distinct from **cavernous routing** (document-based dynamic execution). Cavernous routing creates a "super agent" that discovers and executes tailored subtask plans. Enhanced rooms are about **communication lines between nodes** — like a synthesizer where agents collaborate, then each relay their findings downstream.

## Mental Model

```
                    ┌──────────────────────────────┐
                    │        Room Step              │
                    │                               │
 Upstream ─────────►│  Bob (agent): "I learned the  │──── Bob's output ──────► Next Step A
 context            │   website should be red."     │
                    │                               │
                    │  Jim (agent): "I learned we   │──── Jim's output ──────► Next Step B
                    │   need good authentication."  │
                    │                               │
                    │  [Optional] User can join     │
                    │   the conversation too         │
                    └──────────────────────────────┘
```

Key idea: The room is a conversation between agents. Each agent in the room is purpose-built for room conversations. After the conversation concludes, each agent's findings flow downstream independently through their respective output ports.

## Design Goals

1. **Multi-agent conversation** — 2+ agents discuss within a room, with configurable turn order and round limits
2. **Per-agent output ports** — Each agent's conclusion flows to different downstream steps via port-based routing
3. **Optional user participation** — The DAG can pause for user input in the room conversation, then resume
4. **Room-specialized agents** — Agents assigned to rooms are expected to have personas suited for collaborative discussion
5. **DAG integration** — Rooms participate in the existing topological execution order, receiving upstream context and feeding downstream steps

## Architecture

### Room Step Configuration

A workflow step with `execution_mode = "room"` references a `room_id`. The room defines:

- **Agents**: Which agents participate (via `room_agents` join table)
- **Speaker order**: Turn-taking policy (round-robin, priority-based, or LLM-directed)
- **Max rounds**: Conversation length limit
- **User participation**: Whether the DAG pauses for user input between turns

### Execution Flow

```
1. DAG reaches a room step
2. Create room session with upstream context as initial prompt
3. For each round:
   a. Each agent takes a turn (sees full conversation history)
   b. Agent produces a message (visible to all) + optional structured output
   c. If user participation enabled: pause DAG, wait for user message, resume
4. After max_rounds or convergence:
   a. Each agent's final structured output is extracted
   b. Outputs are wrapped in StepExecutionEnvelopes (one per agent)
   c. Port routing maps agent outputs to downstream step inputs
5. DAG continues with downstream steps
```

### Per-Agent Output Routing

The key difference from a single-step execution: a room step produces **multiple outputs**, one per agent. Downstream steps connect to specific agent outputs via ports:

```
Room Step
├── output_port: "bob_output"  ──► Edge ──► Step A (input_port: "analysis")
├── output_port: "jim_output"  ──► Edge ──► Step B (input_port: "security_review")
└── output_port: "combined"    ──► Edge ──► Step C (input_port: "summary")
```

The `combined` port could be an automatic aggregation of all agent outputs.

### DAG Pause/Resume for User Participation

When `user_participation = true`:

1. Room step begins, agents take turns
2. After each agent round (or at configured intervals), DAG emits `HubError::AwaitingUser`
3. Frontend shows room conversation + input field
4. User submits message → `resume_workflow_via_engine` injects it and continues
5. Agents respond to user's input in subsequent rounds
6. Cycle repeats until max_rounds reached or user signals "done"

This reuses the existing `AwaitingUser` pause mechanism already used for interactive steps.

## Data Model Considerations

### Existing Tables (already in place)

- `rooms` — room configuration (name, description, max_rounds, speaker_order)
- `room_agents` — agents assigned to a room with speaker_order
- `room_sessions` — active conversation instances
- `room_messages` — conversation messages with agent attribution

### Potential Additions

- `room_step_outputs` — per-agent structured outputs from a room step execution
- Port definitions on room steps linking agent IDs to output port names
- `workflow_step_edges` may need agent-specific port references (e.g., `from_output_port = "agent:<agent_id>"`)

## Open Questions

1. **Convergence detection**: Should rooms have an LLM-based "moderator" that decides when the conversation has reached a conclusion, or rely solely on max_rounds?
2. **Agent output extraction**: Should each agent produce structured output on every turn, or only on their final turn? Leaning toward final-turn extraction for simplicity.
3. **Turn ordering**: Round-robin is simplest, but some conversations benefit from dynamic ordering (e.g., agent A responds to agent B's point). Worth considering an LLM-directed "facilitator" role.
4. **User turn frequency**: When user participates, do they get a turn every round, or only when they signal they want to speak?

## Relationship to Existing Features

- **Cavernous routing** (Phase 7): Completely separate feature. Cavernous routing is about dynamic subtask discovery via document search. Rooms are about inter-agent communication.
- **Existing room execution** (`src/server/executors/room/`): Current room execution handles basic multi-agent turns. Enhanced rooms extend this with per-agent output ports and DAG integration.
- **Port-based data flow** (Phase 5): Enhanced rooms build directly on the port system — each agent's output becomes a named port that downstream steps can connect to.

## Implementation Phases (Future)

1. **Phase A**: Per-agent output extraction — room sessions produce per-agent `StepExecutionEnvelope`s
2. **Phase B**: Port routing for room outputs — map agent outputs to downstream step input ports
3. **Phase C**: User participation with DAG pause/resume
4. **Phase D**: Convergence detection and dynamic turn ordering
