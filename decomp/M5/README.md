# Milestone 5: Orchestration Core

> Orchestrator can decompose tickets into slices, route tasks to appropriate agents, and coordinate execution.

## Goal

The orchestration layer transforms high-level ticket descriptions into actionable tasks, routes them to the correct agent tier, tracks dependencies, and schedules execution across the agent pool.

**Checkpoint**: Give orchestrator a ticket description, see it create slices, assign to workers.

---

## Tickets

| Ticket | Title | Slices | Dependencies |
|--------|-------|--------|--------------|
| 5.1 | Planner (Ticket → Slices) | 5 | M3 (Agent Runtime), M4 (Prompt Engineering) |
| 5.2 | Task Queue | 4 | M1 (Foundation - types, db) |
| 5.3 | Router (Task → Tier) | 3 | 5.1 |
| 5.4 | Dependency Tracking | 3 | 5.2 |
| 5.5 | Scheduler | 3 | 5.2, 5.3, 5.4 |

---

## Dependency Graph

```
M1 ─────────────────────────────────────┐
                                        │
M3 (Agent Runtime) ──┐                  │
                     ├──→ 5.1 (Planner) ──→ 5.3 (Router) ──┐
M4 (Prompts) ────────┘                                     │
                                                           │
M1 ──→ 5.2 (Task Queue) ──→ 5.4 (Dependency Tracking) ─────┼──→ 5.5 (Scheduler)
                                                           │
                           5.3 (Router) ───────────────────┘
```

---

## Parallelization

**Can run in parallel**:
- 5.1 (Planner) and 5.2 (Task Queue) - no dependencies between them
- Once both complete: 5.3 and 5.4 can potentially overlap if workers coordinate

**Must be sequential**:
- 5.1 → 5.3 (Router needs Planner's slice output format)
- 5.2 → 5.4 (Dependency tracking extends the queue)
- 5.3 + 5.4 → 5.5 (Scheduler ties everything together)

---

## Key Files

All orchestration code lives in `src/orchestration/`:

```
src/orchestration/
├── mod.rs           ← Module exports, shared types
├── planner.rs       ← Ticket decomposition logic
├── queue.rs         ← Priority task queue
├── router.rs        ← Task → tier routing
├── dependency.rs    ← Dependency tracking
└── scheduler.rs     ← Work assignment loop
```

---

## External Dependencies

This milestone builds on:

- **M1 Types**: `Task`, `TaskStatus`, `Priority`, `VerticalSlice`, `AgentTier`
- **M1 Database**: SQLite connection pool, task/slice persistence
- **M2 LLM**: Provider for orchestrator decomposition calls
- **M3 Agents**: Agent pool for availability checks
- **M4 Prompts**: Decomposition prompt templates, output schemas

---

## Notes

- The Planner is the brain - it uses the orchestrator LLM to think through decomposition
- Task Queue is the heart - all work flows through it
- Router and Dependency Tracking are supporting systems
- Scheduler is the coordinator that brings it all together
- Design for testability: each component should work in isolation with mocks
