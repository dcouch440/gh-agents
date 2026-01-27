# nexor Progress

> Work tracking for ROADMAP.md implementation

## Current Focus

**Active:** None
**Next:** 2.3 Cost Tracking (M2)
**Blocked:** None

**Milestone 1 Decomposition:** Complete - see `decomp/M1/`
**Milestone 2 Decomposition:** Complete - see `decomp/M2/`
**Milestone 3 Decomposition:** Complete - see `decomp/M3/`
**Milestone 4 Decomposition:** Complete - see `decomp/M4/`
**Milestone 5 Decomposition:** Complete - see `decomp/M5/`
**Milestone 6 Decomposition:** Complete - see `decomp/M6/`
**Milestone 7 Decomposition:** Complete - see `decomp/M7/`
**Milestone 8 Decomposition:** Complete - see `decomp/M8/`
**Milestone 9 Decomposition:** Complete - see `decomp/M9/`

---

## Milestone 1: Foundation

**Goal**: Project compiles, core types exist, config loads, database works.

| Ticket | Status | Progress | Notes |
|--------|--------|----------|-------|
| 1.1 Project Scaffolding | done | 3/3 | Cargo.toml, directory structure, main.rs with tokio |
| 1.2 Core Type Definitions | done | 6/6 | All types: task, agent, message, ticket, cost, config |
| 1.3 Configuration System | done | 4/4 | Global + project config loading, merging, validation |
| 1.4 Database Setup | done | 8/8 | SQLite init, 6 migrations, CRUD queries |
| 1.5 Logging Infrastructure | done | 3/3 | Tracing with env filter, file appender, helper spans/macros |

**Milestone Status:** Complete (5/5 tickets done)

---

## Milestone 2: LLM Layer

**Goal**: Can send prompts to Anthropic and get streaming responses.

| Ticket | Status | Progress | Notes |
|--------|--------|----------|-------|
| 2.1 Provider Abstraction | done | 3/3 | LLMProvider trait, types, streaming |
| 2.2 Anthropic Client | done | 4/4 | HTTP client, send_message, streaming, token counts |
| 2.3 Cost Tracking | pending | 0/3 | Needs 2.2 |
| 2.4 Retry Logic | done | 3/3 | ExponentialBackoff, RetryPolicy, RetryingProvider wrapper, 14 tests |

**Milestone Status:** In Progress (2/4 tickets done)

---

## Milestone 3: Agent Runtime

**Goal**: Agents can be spawned, receive tasks, execute them, and report back.

| Ticket | Status | Progress | Notes |
|--------|--------|----------|-------|
| 3.1 Agent Struct & Lifecycle | pending | 0/3 | Needs M1, M2 |
| 3.2 Agent Pool Manager | pending | 0/4 | Needs 3.1 |
| 3.3 Message Passing | pending | 0/4 | Needs 3.1 |
| 3.4 Persona System | pending | 0/3 | Needs M1 config |
| 3.5 Task Execution Loop | pending | 0/4 | Needs 3.1-3.4 |
| 3.6 Escalation Flow | pending | 0/3 | Needs 3.5 |
| 3.7 Inter-Agent Protocol | pending | 0/5 | Needs 3.3 |

**Milestone Status:** Not Started

---

## Milestone 4: Prompt Engineering & Agent Intelligence

**Goal**: Robust, tested prompts that drive reliable agent behavior.

| Ticket | Status | Progress | Notes |
|--------|--------|----------|-------|
| 4.1 Prompt Architecture Design | done | 4/4 | PromptBuilder, ContextInjector, PromptVersion, 23 tests |
| 4.2 Orchestrator Thinking Patterns | done | 5/5 | Decomposition, review, routing, conversation, recovery prompts, 25 tests |
| 4.3 Worker Thinking Patterns | pending | 0/5 | Needs 4.1 |
| 4.4 Utility Thinking Patterns | pending | 0/4 | Needs 4.1 |
| 4.5 Structured Output Design | pending | 0/5 | Needs 4.1 |
| 4.6 Few-Shot Examples Library | pending | 0/5 | Needs 4.2-4.4 |
| 4.7 Prompt Testing Framework | pending | 0/6 | Needs 4.5, M2 |
| 4.8 Context Management Strategy | done | 5/5 | ContextBudget, FileSelector, FileSummarizer, ContextRequestHandler, HistoryManager, 35 tests |
| 4.9 Self-Correction & Recovery Prompts | pending | 0/5 | Needs 4.5 |
| 4.10 Tool Definition & Selection | done | 6/6 | ToolDefinition, ToolRegistry, file/git/test tools, selection prompts, parser, 31 tests |
| 4.11 Context Window Validation | pending | 0/5 | Needs M2 |

**Milestone Status:** In Progress (2/11 tickets done)

---

## Milestone 5: Orchestration Core

**Goal**: Orchestrator can decompose tickets into slices, route to appropriate agents.

| Ticket | Status | Progress | Notes |
|--------|--------|----------|-------|
| 5.1 Planner (Ticket → Slices) | pending | 0/5 | Needs M3, M4 |
| 5.2 Task Queue | pending | 0/4 | Needs M1 |
| 5.3 Router (Task → Tier) | pending | 0/3 | Needs 5.1 |
| 5.4 Dependency Tracking | pending | 0/3 | Needs 5.2 |
| 5.5 Scheduler | pending | 0/3 | Needs 5.2-5.4 |

**Milestone Status:** Not Started

---

## Milestone 6: TUI Basic

**Goal**: Functional terminal interface with feed, chat, and navigation.

| Ticket | Status | Progress | Notes |
|--------|--------|----------|-------|
| 6.1 Terminal Setup | pending | 0/3 | Needs M1 |
| 6.2 Layout System | pending | 0/3 | Needs 6.1 |
| 6.3 Home Screen | pending | 0/3 | Needs 6.2 |
| 6.4 Feed View (/feed) | pending | 0/4 | Needs 6.2 |
| 6.5 Chat View (/main) | pending | 0/5 | Needs 6.2, M3 |
| 6.6 Slash Command Router | pending | 0/4 | Needs 6.2 |
| 6.7 Logs View (/logs) | pending | 0/3 | Needs 6.2, M1.5 |

**Milestone Status:** Not Started

---

## Milestone 7: Execution Layer

**Goal**: Agents can read/write files, run git commands, execute tests.

| Ticket | Status | Progress | Notes |
|--------|--------|----------|-------|
| 7.1 File Operations | pending | 0/4 | Needs M1 |
| 7.2 Git Operations | pending | 0/6 | Needs M1 |
| 7.3 Test Runner | pending | 0/4 | Needs M1 |
| 7.4 Docker Sandbox | pending | 0/4 | Needs 7.1-7.3 |
| 7.5 Approval Gates | pending | 0/4 | Needs M6 (TUI) |

**Milestone Status:** Not Started

---

## Milestone 8: GitHub Integration

**Goal**: Can pull issues from GitHub, create PRs.

| Ticket | Status | Progress | Notes |
|--------|--------|----------|-------|
| 8.1 GitHub API Client | pending | 0/4 | Needs M1 |
| 8.2 Issue Sync | pending | 0/3 | Needs 8.1 |
| 8.3 PR Creation | pending | 0/3 | Needs 8.1, M7 |
| 8.4 Progress Updates | pending | 0/3 | Needs 8.1 |

**Milestone Status:** Not Started

---

## Milestone 9: Polish & Production

**Goal**: Production-ready, fully-featured.

| Ticket | Status | Progress | Notes |
|--------|--------|----------|-------|
| 9.1 Remaining TUI Views | pending | 0/3 | Needs M6 |
| 9.2 Headless Mode | pending | 0/4 | Needs M5 |
| 9.3 Error Handling Polish | pending | 0/3 | Needs M1-M8 |
| 9.4 Docker Packaging | pending | 0/3 | Needs M1-M8 |
| 9.5 Documentation | pending | 0/4 | Needs M1-M8 |
| 9.6 Observability & Replay | pending | 0/5 | Needs M2, M5 |

**Milestone Status:** Not Started

---

## Summary

| Milestone | Tickets | Slices | Status |
|-----------|---------|--------|--------|
| M1: Foundation | 5 | 24 | Complete |
| M2: LLM Layer | 4 | 13 | Not Started |
| M3: Agent Runtime | 7 | 26 | Not Started |
| M4: Prompt Engineering | 11 | 55 | Not Started |
| M5: Orchestration Core | 5 | 18 | Not Started |
| M6: TUI Basic | 7 | 25 | Not Started |
| M7: Execution Layer | 5 | 22 | Not Started |
| M8: GitHub Integration | 4 | 13 | Not Started |
| M9: Polish & Production | 6 | 22 | Not Started |
| **Total** | **55** | **222** | |

---

## Decisions Log

| Date | Decision | Rationale |
|------|----------|-----------|
| | | |

---

## Agent Workflow

When picking up work:
1. Read `PROGRESS.md` to find next unblocked ticket
2. Read corresponding section in `ROADMAP.md` for full spec
3. Update `PROGRESS.md`: set status to `in_progress`
4. Do the work (implement slices)
5. Update `PROGRESS.md`: mark slices done, add notes
6. When ticket complete: set status to `done`

**Status values:**
- `pending` - Not started
- `in_progress` - Being worked on
- `blocked` - Waiting on dependency
- `done` - Complete and verified
