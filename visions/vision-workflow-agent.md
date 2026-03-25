# Workflow Agent — Vision (Draft)

## What It Is

A conversational agent that helps users design workflow topology — nodes, edges, and node descriptions — by living in a file-based repository that mirrors the canvas. Like Claude Code for workflow design: the user talks, the agent edits, the user sees changes live.

## How It Works

The agent's repository IS the board state, serialized as files. Every user message triggers a sync: canvas → repo. The agent reads the files, makes changes (add nodes, wire edges, update descriptions), and the backend pushes changes back to the frontend as a draft.

```
User message + canvas state
    ↓
Board → repo sync (canvas elements → files in agent's workspace)
    ↓
Agent sees: updated files + user message + conversation history
    ↓
Agent writes changes (add/remove/edit nodes and edges via run_command)
    ↓
Repo → board sync (file changes → canvas draft pushed to frontend)
    ↓
User sees draft overlay, accepts/modifies/continues talking
    ↓
(loop — agent persists across messages)
```

## Key Design Principles

**The user owns the canvas.** The agent suggests; it doesn't commit. Changes appear as drafts that the user can accept, reject, or modify before they become real.

**The canvas is the repo.** The agent doesn't have a separate data model. It reads and writes the same structure the canvas uses. Board submit is the sync mechanism — same path that already exists.

**Conversational, not fire-and-forget.** The current manager dispatches instructions and disconnects. This agent has a persistent session — it remembers what it discussed, what the user asked for, what it already tried.

**Same infrastructure as system node agent.** Container, JuiceFS pinned path, run_command, per-turn state rebuild, session history. The system node agent proved the pattern; this agent operates one level up.

## Repository Structure (Sketch)

```
./
├── board.json          # Serialized canvas state (nodes + edges + positions)
├── nodes/
│   ├── {node_id}.json  # Per-node config (name, description, execution_mode)
│   └── ...
└── edges.json          # Edge list (from, to, conditions)
```

Or simpler — a single `workflow.json` that the agent reads and writes:
```json
{
  "nodes": [
    { "id": "abc", "name": "Research", "description": "Find competitor data", "position": [100, 200] }
  ],
  "edges": [
    { "from": "abc", "to": "def" }
  ]
}
```

The exact schema depends on what the frontend needs to render a draft overlay.

## What Changes vs System Node Agent

| Aspect | System Node Agent | Workflow Agent |
|--------|-------------------|----------------|
| **Scope** | Agents within one node | Nodes + edges across the workflow |
| **Trigger** | Board submit (node text changed) | User message in conversation |
| **Input** | Node text + upstream context | Full board state + user message |
| **Output** | config.json, topology.json, agents/*.json | Updated board state (nodes, edges, descriptions) |
| **Consumer** | file_executor → runtime agents | Frontend renders draft overlay |
| **Session** | Per-step, across dispatches | Per-workflow, across messages |
| **Lifecycle** | Fires on submit, completes, exits | Persistent — lives across the conversation |

## What Stays The Same

- Container + JuiceFS workspace (pinned path per workflow)
- run_command for file writes
- Per-turn `<current_state>` rebuild from filesystem
- Session history with `build_pruned_instruction`
- Write-time JSON validation
- ExecutionStrategy trait

## Open Questions

- **Draft UX**: How does the frontend show the agent's proposed changes? Ghost nodes? Side panel diff? Inline with accept/reject buttons?
- **Conflict resolution**: If the user edits the canvas while the agent is thinking, whose changes win?
- **Completion signal**: System node agent has `complete_system`. What's the equivalent? Maybe just the agent's text response — no explicit tool call needed since every response can include file changes.
- **Scope boundary**: Does this agent also dispatch to system node agents (configure the agents within each node it creates)? Or is that a separate step?
- **Board → repo fidelity**: How much canvas state goes into the repo? Just logical structure (nodes, edges, text)? Or also visual layout (positions, colors, grouping)?
- **Manager relationship**: Does this replace the L2 manager? Or coexist as a different interaction mode?
