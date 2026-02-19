# Meeting Protocol — Vision

## What It Is

A meeting is a protocol — just like workforce — that creates a team of agents with **roles** (not tasks) who gather information, then enter a turn-based conversation where users and agents discuss, debate, and reach conclusions together.

## How It Works

### Configuration (same pattern as workforce)

1. User creates a node on the canvas, sets archetype to "meeting"
2. User talks to the node assistant: "I want a meeting to review what the research team found and decide next steps"
3. The assistant helps design the meeting:
   - Who are the participants? What are their roles? (e.g., "Devil's Advocate", "Domain Expert", "Facilitator")
   - What prep work needs to happen first? (e.g., "Pull up the workforce execution logs from the research step and summarize what each agent did")
   - What's the meeting purpose? How should it conclude?
4. Assistant dispatches to configure the roster and pipeline

### What gets built (the pipeline)

The meeting protocol creates a child workflow (pipeline) with two phases:

```
Phase 1: Information Gathering
  ├── Step A: "single" — Pull context from connected upstream steps
  ├── Step B: "single" — Summarize workforce execution logs step-by-step
  └── Step C: "single" — Prepare briefing documents for meeting members

Phase 2: The Meeting
  └── Step D: "meeting" — Turn-based conversation (special activation)
```

The designer creates the prep steps dynamically based on what the meeting needs. It can create agents with capabilities to pull execution logs (Tier 3), query connected steps, or synthesize information — whatever the meeting requires.

### Execution

Phase 1 runs like any pipeline — single steps execute in topological order, gathering and preparing information.

Phase 2 enters the **meeting primitive** — a fundamentally different execution model:

- **Turn-based conversation**: Agents speak in rounds based on their roles. The facilitator may guide discussion, the domain expert provides analysis, the devil's advocate challenges assumptions.
- **Gatekeeper**: Between turn cycles, a gatekeeper activates — it manages who speaks next, whether new participants should be brought in, and whether the conversation is productive.
- **User participation**: Users can join the meeting. In a multi-user scenario, the gatekeeper manages turn order for humans too — activating between cycles to allow new users to speak.
- **Long-running activation**: The meeting step stays alive across many turns. This isn't fire-and-forget — it's a persistent session that can run for extended periods with many rounds of interaction.
- **Conclude system**: After each cycle, the system evaluates whether the meeting has reached its goals. Users or the facilitator can signal "we need another round" or "we're done." The meeting can go through multiple conclude-and-continue cycles.

### Output

When the meeting concludes, it produces structured output — decisions made, action items, key insights, unresolved questions. This output flows downstream through the DAG like any other step output.

## How It Differs From Workforce

| | Workforce | Meeting |
|---|---|---|
| Agents have | **Tasks** — each agent works independently on their assignment | **Roles** — agents interact with each other in conversation |
| Execution | Parallel within levels, sequential across levels. Each agent gets one shot. | Turn-based rounds. Agents respond to each other. Multiple cycles. |
| User involvement | User configures beforehand, execution is autonomous | Users can actively participate in the meeting |
| Duration | Short — each agent runs once | Long — many turns, pause/resume, extended sessions |
| Output | Per-agent task outputs merged | Collective decisions, synthesized from discussion |

## What Already Exists

- **DB tables**: `rooms`, `room_members`, `room_sessions`, `room_execution_outputs`, `room_step_configs`, `room_step_members` — good schema for turn tracking, per-speaker outputs, gatekeeper config
- **Pipeline service**: Creates child workflows, manages steps and edges, computes execution order
- **Pipeline execution**: Runs child workflows through the DAG loop — Phase 1 prep steps work today
- **Workforce pattern**: Assistant conversation → dispatch → roster configuration. Meeting reuses this exact pattern with roles instead of tasks.

## What Needs To Be Built

1. **Meeting archetype**: Node assistant block, chat tools for configuring meeting participants and their roles (similar to workforce's add_agent/update_agent but role-oriented)
2. **Meeting primitive** (`execution_mode = "meeting"`): The turn-based conversation loop — gatekeeper, turn management, user participation, conclude system. Lives in `dag/meeting/` or similar.
3. **Long-running step activation**: The ability for a step to pause, wait for user input, and resume across many interactions. This is the hardest piece — the current DAG loop expects steps to complete.
4. **Meeting designer input**: Formats roster + prep step outputs into meeting context for each participant
