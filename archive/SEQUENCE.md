# Ticket Sequence Guide

> Quick reference for ticket execution order. Check `PROGRESS.md` for current status.

---

## Milestone Overview

```
M1 Foundation ──────► M2 LLM Layer ──────► M3 Agent Runtime ──────┐
                            │                                      │
                            ▼                                      ▼
                      M4 Prompts ─────────────────────────► M5 Orchestration
                                                                   │
                      M6 TUI Basic ◄───────────────────────────────┤
                            │                                      │
                            ▼                                      ▼
                      M7 Execution ──────► M8 GitHub ──────► M9 Polish
```

---

## M1: Foundation

```
1.1 Project Scaffolding
 │
 ├──► 1.2 Core Type Definitions
 │     │
 │     ├──► 1.3 Configuration System
 │     │
 │     └──► 1.4 Database Setup
 │
 └──► 1.5 Logging Infrastructure (independent)
```

| Ticket | Title | Dependencies | Slices |
|--------|-------|--------------|--------|
| 1.1 | Project Scaffolding | None | 3 |
| 1.2 | Core Type Definitions | 1.1 | 6 |
| 1.3 | Configuration System | 1.2 | 4 |
| 1.4 | Database Setup | 1.2 | 8 |
| 1.5 | Logging Infrastructure | 1.1 | 3 |

---

## M2: LLM Layer

```
2.1 Provider Abstraction
 │
 └──► 2.2 Anthropic Client
       │
       ├──► 2.3 Cost Tracking
       │
       └──► 2.4 Retry Logic
```

| Ticket | Title | Dependencies | Slices |
|--------|-------|--------------|--------|
| 2.1 | Provider Abstraction | M1 | 3 |
| 2.2 | Anthropic Client | 2.1 | 4 |
| 2.3 | Cost Tracking | 2.2 | 3 |
| 2.4 | Retry Logic | 2.2 | 3 |

---

## M3: Agent Runtime

```
3.1 Agent Struct & Lifecycle
 │
 ├──► 3.2 Agent Pool Manager
 │
 ├──► 3.3 Message Passing ──────► 3.7 Inter-Agent Protocol
 │
 └──► 3.4 Persona System
       │
       └──► 3.5 Task Execution Loop
             │
             └──► 3.6 Escalation Flow
```

| Ticket | Title | Dependencies | Slices |
|--------|-------|--------------|--------|
| 3.1 | Agent Struct & Lifecycle | M1, M2 | 3 |
| 3.2 | Agent Pool Manager | 3.1 | 4 |
| 3.3 | Message Passing | 3.1 | 4 |
| 3.4 | Persona System | M1 config | 3 |
| 3.5 | Task Execution Loop | 3.1-3.4 | 4 |
| 3.6 | Escalation Flow | 3.5 | 3 |
| 3.7 | Inter-Agent Protocol | 3.3 | 5 |

---

## M4: Prompt Engineering

```
4.1 Prompt Architecture Design
 │
 ├──► 4.2 Orchestrator Thinking ─────┐
 │                                    │
 ├──► 4.3 Worker Thinking ───────────┼──► 4.6 Few-Shot Examples
 │                                    │
 ├──► 4.4 Utility Thinking ──────────┘
 │
 ├──► 4.5 Structured Output ──────────┬──► 4.9 Self-Correction
 │                                    │
 │                                    └──► 4.7 Prompt Testing (+ M2)
 │
 ├──► 4.8 Context Management
 │
 └──► 4.10 Tool Definition

4.11 Context Window Validation (needs M2 only)
```

| Ticket | Title | Dependencies | Slices |
|--------|-------|--------------|--------|
| 4.1 | Prompt Architecture Design | None | 4 |
| 4.2 | Orchestrator Thinking Patterns | 4.1 | 5 |
| 4.3 | Worker Thinking Patterns | 4.1 | 5 |
| 4.4 | Utility Thinking Patterns | 4.1 | 4 |
| 4.5 | Structured Output Design | 4.1 | 5 |
| 4.6 | Few-Shot Examples Library | 4.2, 4.3, 4.4 | 5 |
| 4.7 | Prompt Testing Framework | 4.5, M2 | 6 |
| 4.8 | Context Management Strategy | 4.1 | 5 |
| 4.9 | Self-Correction & Recovery | 4.5 | 5 |
| 4.10 | Tool Definition & Selection | 4.1 | 6 |
| 4.11 | Context Window Validation | M2 | 5 |

**Parallel after 4.1:** 4.2, 4.3, 4.4, 4.5, 4.8, 4.10

---

## M5: Orchestration Core

```
5.0 Planner Bot (PRD Creation)
 │
 └──► 5.1 Planner (Ticket → Slices)
       │
       └──► 5.3 Router (Task → Tier)

5.2 Task Queue
 │
 └──► 5.4 Dependency Tracking
       │
       └──► 5.5 Scheduler
```

| Ticket | Title | Dependencies | Slices |
|--------|-------|--------------|--------|
| 5.0 | Planner Bot (Interactive PRD) | M3, M4 | 5 |
| 5.1 | Planner (Ticket → Slices) | M3, M4 | 5 |
| 5.2 | Task Queue | M1 | 4 |
| 5.3 | Router (Task → Tier) | 5.1 | 3 |
| 5.4 | Dependency Tracking | 5.2 | 3 |
| 5.5 | Scheduler | 5.2-5.4 | 3 |

---

## M6: TUI Basic

```
6.1 Terminal Setup
 │
 └──► 6.2 Layout System
       │
       ├──► 6.3 Home Screen
       │
       ├──► 6.4 Feed View (/feed)
       │
       ├──► 6.5 Chat View (/main) ◄── needs M3
       │
       ├──► 6.6 Slash Command Router
       │
       ├──► 6.7 Logs View (/logs)
       │
       └──► 6.8 Plan View (/plan) ◄── needs 5.0
```

| Ticket | Title | Dependencies | Slices |
|--------|-------|--------------|--------|
| 6.1 | Terminal Setup | M1 | 3 |
| 6.2 | Layout System | 6.1 | 3 |
| 6.3 | Home Screen | 6.2 | 3 |
| 6.4 | Feed View (/feed) | 6.2 | 4 |
| 6.5 | Chat View (/main) | 6.2, M3 | 5 |
| 6.6 | Slash Command Router | 6.2 | 4 |
| 6.7 | Logs View (/logs) | 6.2, M1.5 | 3 |
| 6.8 | Plan View (/plan) | 6.2, 5.0 | 4 |

---

## M7: Execution Layer

```
7.1 File Operations ──┐
                      │
7.2 Git Operations ───┼──► 7.4 Docker Sandbox
                      │
7.3 Test Runner ──────┘

7.2 ──► 7.6 Git Merge Operations

7.5 Approval Gates ◄── needs M6
```

| Ticket | Title | Dependencies | Slices |
|--------|-------|--------------|--------|
| 7.1 | File Operations | M1 | 4 |
| 7.2 | Git Operations | M1 | 6 |
| 7.3 | Test Runner | M1 | 4 |
| 7.4 | Docker Sandbox | 7.1-7.3 | 4 |
| 7.5 | Approval Gates | M6 | 4 |
| 7.6 | Git Merge Operations | 7.2 | 6 |

---

## M8: GitHub Integration

```
8.0 GitHub Authentication
 │
 └──► 8.1 GitHub API Client
       │
       ├──► 8.2 Issue Sync
       │
       ├──► 8.3 PR Creation ◄── needs M7
       │
       ├──► 8.4 Progress Updates
       │
       ├──► 8.5 PR Retrieval & Review
       │     │
       │     └──► 8.6 PR Merge Operations
       │
       └──► 8.7 PR Merge Queue ◄── needs 7.6, 8.4, 8.5, 8.6
```

| Ticket | Title | Dependencies | Slices |
|--------|-------|--------------|--------|
| 8.0 | GitHub Authentication | M1 | 5 |
| 8.1 | GitHub API Client | 8.0 | 4 |
| 8.2 | Issue Sync | 8.1 | 3 |
| 8.3 | PR Creation | 8.1, M7 | 3 |
| 8.4 | Progress Updates | 8.1 | 3 |
| 8.5 | PR Retrieval & Review | 8.1 | 4 |
| 8.6 | PR Merge Operations | 8.1, 8.5 | 4 |
| 8.7 | PR Merge Queue | 7.6, 8.4, 8.5, 8.6 | 6 |

---

## M9: Polish & Production

```
9.1 Remaining TUI Views ◄── needs M6
9.2 Headless Mode ◄── needs M5
9.3 Error Handling Polish ◄── needs M1-M8
9.4 Docker Packaging ◄── needs M1-M8
9.5 Documentation ◄── needs M1-M8
9.6 Observability & Replay ◄── needs M2, M5

9.7 Refactor Mode Foundation
 │
 └──► 9.8 Refactor Agent
       │
       └──► 9.9 TUI Integration
             │
             └──► 9.10 Menu Types
                   │
                   └──► 9.11 Menu Widget
                         │
                         └──► 9.12 App Integration
```

| Ticket | Title | Dependencies | Slices |
|--------|-------|--------------|--------|
| 9.1 | Remaining TUI Views | M6 | 3 |
| 9.2 | Headless Mode | M5 | 4 |
| 9.3 | Error Handling Polish | M1-M8 | 3 |
| 9.4 | Docker Packaging | M1-M8 | 3 |
| 9.5 | Documentation | M1-M8 | 4 |
| 9.6 | Observability & Replay | M2, M5 | 5 |
| 9.7 | Refactor Mode Foundation | None | 4 |
| 9.8 | Refactor Agent | 9.7 | 4 |
| 9.9 | TUI Integration | 9.8 | 3 |
| 9.10 | Menu Types & Data | 9.7, 9.9 | 3 |
| 9.11 | Menu Widget & Rendering | 9.10 | 3 |
| 9.12 | App Integration | 9.10, 9.11, 9.9 | 3 |

---

## Quick Lookup

Find your ticket file:

| Ticket | File Path |
|--------|-----------|
| 1.x | `decomp/M1/1.x.md` |
| 2.x | `decomp/M2/2.x.md` |
| 3.x | `decomp/M3/3.x.md` |
| 4.x | `decomp/M4/4.x.md` |
| 5.x | `decomp/M5/5.x.md` |
| 6.x | `decomp/M6/6.x.md` |
| 7.x | `decomp/M7/7.x.md` |
| 8.x | `decomp/M8/8.x.md` |
| 9.x | `decomp/M9/9.x.md` |

---

## Parallelization Opportunities

**Can start immediately (no dependencies):**
- 4.1 Prompt Architecture
- 9.7 Refactor Mode Foundation

**Can run in parallel after M1:**
- M2 LLM Layer
- M6 TUI Basic (6.1-6.4, 6.6-6.7)
- M7 Execution (7.1-7.3)

**Can run in parallel after 4.1:**
- 4.2, 4.3, 4.4 (thinking patterns)
- 4.5 (structured output)
- 4.8 (context management)
- 4.10 (tool definitions)

---

## Total: 65 tickets, 263 slices
