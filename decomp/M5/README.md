# Milestone 5: Orchestration Core

> Orchestrator can classify requests, generate PRDs, decompose tickets into slices, route tasks to appropriate agents, and coordinate execution.

## Goal

The orchestration layer is the brain of nexor. It classifies incoming requests by scale, generates PRDs for large projects, transforms tickets into actionable tasks, routes them to the correct agent tier, tracks dependencies, and schedules execution across the agent pool.

**Checkpoint**: Give orchestrator a raw request like "build a billing system", see it classify as Project, generate a mini-PRD, create slices, and assign to workers.

---

## Tickets

| Ticket | Title | Slices | Dependencies |
|--------|-------|--------|--------------|
| 5.0 | Plan Mode (Request → Strategy) | 5 | M2 (LLM Layer), M4 (Prompt Engineering) |
| 5.1 | Planner (Ticket → Slices) | 5 | 5.0, M3 (Agent Runtime), M4 (Prompt Engineering) |
| 5.2 | Task Queue | 4 | M1 (Foundation - types, db) |
| 5.3 | Router (Task → Tier) | 3 | 5.1 |
| 5.4 | Dependency Tracking | 3 | 5.2 |
| 5.5 | Scheduler | 3 | 5.2, 5.3, 5.4 |

**Total**: 6 tickets, 23 slices

---

## Dependency Graph

```
                    Request
                       │
                       ▼
M2 (LLM) ──────┐   ┌──────────────────────────────────────────┐
               ├──→│ 5.0 (Plan Mode)                          │
M4 (Prompts) ──┘   │  ├─ Quick → Direct to Router             │
                   │  ├─ Task/Feature → 5.1 Planner           │
                   │  ├─ Project → PRD + 5.1 Planner          │
                   │  └─ Epic → PRD + Milestones + 5.1 Planner│
                   └──────────────────┬───────────────────────┘
                                      │
                                      ▼
M3 (Agent Runtime) ──┐   ┌────────────────────────┐
                     ├──→│ 5.1 (Planner)          │──→ 5.3 (Router) ──┐
M4 (Prompts) ────────┘   └────────────────────────┘                   │
                                                                      │
M1 ──→ 5.2 (Task Queue) ──→ 5.4 (Dependency Tracking) ────────────────┼──→ 5.5 (Scheduler)
                                                                      │
                              5.3 (Router) ───────────────────────────┘
```

---

## Request Scale Flow

Plan Mode (5.0) classifies requests and routes appropriately:

| Scale | Example | Decomposition Depth |
|-------|---------|---------------------|
| Quick | "Fix typo in README" | None - direct to agent |
| Task | "Add input validation" | Light - single task |
| Feature | "Add dark mode" | Standard - Planner (5.1) |
| Project | "Build auth system" | Mini-PRD → Planner |
| Epic | "Build entire platform" | Full PRD → Milestones → Planner |

---

## Parallelization

**Can run in parallel**:
- 5.0 (Plan Mode) and 5.2 (Task Queue) - no dependencies between them
- Once 5.0 complete: 5.1 can start
- Once 5.1 and 5.2 complete: 5.3 and 5.4 can potentially overlap

**Must be sequential**:
- 5.0 → 5.1 (Planner receives classified/PRD-enhanced tickets from Plan Mode)
- 5.1 → 5.3 (Router needs Planner's slice output format)
- 5.2 → 5.4 (Dependency tracking extends the queue)
- 5.3 + 5.4 → 5.5 (Scheduler ties everything together)

---

## Key Files

All orchestration code lives in `src/orchestration/`:

```
src/orchestration/
├── mod.rs           ← Module exports, shared types
├── plan_mode.rs     ← Request classification and PRD generation (NEW)
├── classifier.rs    ← Scale classification logic (NEW)
├── prd_generator.rs ← PRD generation for large projects (NEW)
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
