# Milestone 23: Human-in-the-Loop Gate System

> Structured sync points between decomposition and execution — agents queue up to talk to the user before starting work.

## Goal

Insert conversational gates at every decomposition level (milestone, slice, task). Before an agent begins work, it presents its understanding to the user in a CLI conversation. The user corrects, clarifies, or skips. A full-screen tree overlay in the Ink CLI lets the user navigate the project hierarchy and manage gates.

**Checkpoint**: Decompose a PRD with milestone gates enabled → tree overlay shows pending gates → open a gate, chat with the agent → resolve it → scheduler allows work to proceed.

---

## Context

Agents lose context at each decomposition boundary. The planner reads the PRD and produces milestones, but nuance gets lost. Each downstream agent works from an increasingly compressed version of the original intent.

Gates solve this by creating a structured handoff: the assigned agent shows what it understood, the user catches what was lost in translation, and work proceeds with shared understanding.

```
PRD
 └→ Planner decomposes into milestones (titles + descriptions)
     └→ Gates created (if enabled per config)
         └→ User syncs with each agent via CLI tree overlay
             └→ Gate resolved/skipped → work proceeds
                 └→ Scheduler assigns tasks
```

The system integrates with:
- **Planner** (`src/orchestration/planner.rs`) — creates gates after decomposition
- **Scheduler** (`src/orchestration/scheduler.rs`) — checks gate status before assigning work
- **Config** (`src/types/config.rs`) — per-level gate toggles
- **Ink CLI** (`cli/`) — tree overlay + gate conversation UI

---

## Scope

- 11 tickets, ~34 slices
- 1 new migration (gates + gate_messages tables)
- 3 new Rust modules (types, repo, service)
- 6 new CLI components/hooks
- Modifications to planner, scheduler, config, API, CLI app

## Dependency Graph

```
23.1 (Schema & Types)
  ├→ 23.2 (Repository Layer)
  │    └→ 23.3 (Gate Service)
  │         ├→ 23.4 (Planner & Scheduler Integration)
  │         └→ 23.5 (REST API Endpoints)
  │              └→ 23.7 (CLI API Client)
  │                   ├→ 23.8 (CLI Tree Overlay)
  │                   │    └→ 23.11 (CLI Spec Detail View)
  │                   └→ 23.9 (CLI Gate Conversation View)
  │                        └→ 23.10 (CLI Navigation & Integration)
  └→ 23.6 (Config Loading)
```

## Tickets

| Ticket | Title | Slices | Dependencies |
|--------|-------|--------|-------------|
| 23.1 | Database Schema & Core Types | 3 | None |
| 23.2 | Repository Layer | 3 | 23.1 |
| 23.3 | Gate Service | 4 | 23.2 |
| 23.4 | Planner & Scheduler Integration | 3 | 23.3 |
| 23.5 | REST API Endpoints | 4 | 23.3 |
| 23.6 | Config Loading | 2 | 23.1 |
| 23.7 | CLI API Client | 2 | 23.5 |
| 23.8 | CLI Tree Overlay | 4 | 23.7 |
| 23.9 | CLI Gate Conversation View | 3 | 23.7 |
| 23.10 | CLI Navigation & Integration | 3 | 23.8, 23.9 |
| 23.11 | CLI Spec Detail View | 3 | 23.8 |

## Key Design Decisions

1. **Polymorphic target** — Gates use `(level, target_id)` to reference any decomposition level. No separate FK per level, simpler schema.
2. **Dedicated tables** — `gates` + `gate_messages` rather than reusing `chat_messages`. Clean separation, future-proofed for summarization back into records.
3. **Full-screen tree overlay** — CLI renders a colored tree with box-drawing characters over the main chat view. Not a separate screen — chat stays mounted underneath.
4. **Per-level config toggles** — `milestone_gates`, `slice_gates`, `task_gates` in `[approval_gates]`. Independent booleans, milestone gates on by default.
5. **Agent persona in gate** — The gate conversation uses the agent tier assigned to that work level, so the user talks to the "right person."
6. **Ancestor blocking** — Unresolved milestone gate blocks all descendant slice/task work. Skip unblocks immediately.
7. **LLM context bundle** — Gate agent receives PRD summary + target description + parent context for informed conversation.
