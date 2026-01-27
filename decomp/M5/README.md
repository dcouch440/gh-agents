# Milestone 5: Orchestration Core

> Specialized bots for planning and orchestration, plus the infrastructure to decompose and schedule work.

## Goal

The orchestration layer provides:
1. **Planner Bot** (5.0) - Interactive PRD creation in `/plan` mode
2. **Planner** (5.1) - Automatic ticket → slice decomposition for execution
3. **Task management** - Queue, routing, dependencies, scheduling

**Checkpoint**: Create a PRD in `/plan` mode with Planner Bot, then execute it via `/main` where the orchestrator uses the Planner to decompose milestones into slices.

---

## Tickets

| Ticket | Title | Slices | Dependencies |
|--------|-------|--------|--------------|
| 5.0 | Planner Bot (Interactive PRD Creation) | 5 | M2 (LLM Layer), M3 (Agent Runtime), M4 (Prompts) |
| 5.1 | Planner (Ticket → Slices) | 5 | M3 (Agent Runtime), M4 (Prompts) |
| 5.2 | Task Queue | 4 | M1 (Foundation - types, db) |
| 5.3 | Router (Task → Tier) | 3 | 5.1 |
| 5.4 | Dependency Tracking | 3 | 5.2 |
| 5.5 | Scheduler | 3 | 5.2, 5.3, 5.4 |

**Total**: 6 tickets, 23 slices

---

## Two Types of Planning

### Planner Bot (5.0) - Interactive, User-Driven

```
User enters /plan mode
        ↓
Planner Bot activated (specialized persona)
        ↓
Multi-turn conversation through phases:
  Discovery → Scoping → Technical → Milestones → Review
        ↓
User approves PRD
        ↓
PRD saved to database
```

- **When**: User explicitly enters `/plan` mode
- **How**: Conversational, asks questions, builds PRD collaboratively
- **Output**: Structured PRD document with milestones

### Planner (5.1) - Automatic, System-Driven

```
Orchestrator receives ticket (from PRD milestone or direct)
        ↓
Planner called automatically
        ↓
Single LLM call decomposes ticket → slices
        ↓
Slices queued for execution
```

- **When**: Orchestrator needs to break down work
- **How**: Single prompt, structured output, no user interaction
- **Output**: List of VerticalSlices with tasks

---

## Dependency Graph

```
                           /plan mode
                               │
M2 (LLM) ──────┐               │
               ├──→ 5.0 (Planner Bot) ──→ PRD Document
M3 (Agents) ───┤                              │
               │                              │ (user approves, sends to /main)
M4 (Prompts) ──┘                              ▼
                                         Orchestrator
                                              │
M3 (Agent Runtime) ──┐                        │
                     ├──→ 5.1 (Planner) ←─────┘
M4 (Prompts) ────────┘         │
                               ▼
                          VerticalSlices
                               │
                               ▼
                         5.3 (Router) ──────────────────┐
                                                        │
M1 ──→ 5.2 (Task Queue) ──→ 5.4 (Dependency) ──────────┼──→ 5.5 (Scheduler)
                                                        │
                              5.3 ──────────────────────┘
```

---

## Parallelization

**Can run in parallel**:
- 5.0 (Planner Bot) and 5.1 (Planner) - different purposes
- 5.0 and 5.2 (Task Queue) - no dependencies
- Once 5.1 and 5.2 complete: 5.3 and 5.4 can overlap

**Must be sequential**:
- 5.1 → 5.3 (Router needs Planner's slice output format)
- 5.2 → 5.4 (Dependency tracking extends the queue)
- 5.3 + 5.4 → 5.5 (Scheduler ties everything together)

---

## Key Files

```
src/agents/
├── planner_bot.rs      ← 5.0: Interactive Planner Bot persona
└── mod.rs

src/types/
├── prd.rs              ← 5.0: PRD document types
└── mod.rs

src/orchestration/
├── mod.rs              ← Module exports
├── planner.rs          ← 5.1: Ticket decomposition logic
├── queue.rs            ← 5.2: Priority task queue
├── router.rs           ← 5.3: Task → tier routing
├── dependency.rs       ← 5.4: Dependency tracking
└── scheduler.rs        ← 5.5: Work assignment loop

src/db/
├── prd.rs              ← 5.0: PRD persistence
└── ...
```

---

## External Dependencies

This milestone builds on:

- **M1 Types**: `Task`, `TaskStatus`, `Priority`, `VerticalSlice`, `AgentTier`
- **M1 Database**: SQLite connection pool, persistence
- **M2 LLM**: Provider for both Planner Bot conversations and Planner decomposition
- **M3 Agents**: Agent runtime for bot lifecycle
- **M4 Prompts**: Planner Bot persona, decomposition templates, output schemas

---

## Notes

- **Planner Bot** has a distinct persona - methodical, asks questions, guides through phases
- **Planner** is utilitarian - takes ticket, returns slices, no conversation
- PRDs created in `/plan` can be executed via `/main`
- Task Queue is the heart - all executable work flows through it
- Design for testability: each component works in isolation with mocks
