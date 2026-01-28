# nexor Progress

> Work tracking for ROADMAP.md implementation

## Current Focus

**Active:** None
**Next:** M6 TUI Basic or M7 Execution Layer
**Blocked:** None
**Completed:** M1 Foundation, M2 LLM Layer, M3 Agent Runtime, M4 Prompt Engineering, M5 Orchestration Core

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
| 2.3 Cost Tracking | done | 3/3 | ModelPricing, CostTracker, aggregation methods |
| 2.4 Retry Logic | done | 3/3 | ExponentialBackoff, RetryPolicy, RetryingProvider wrapper, 14 tests |

**Milestone Status:** Complete (4/4 tickets done)

---

## Milestone 3: Agent Runtime

**Goal**: Agents can be spawned, receive tasks, execute them, and report back.

| Ticket | Status | Progress | Notes |
|--------|--------|----------|-------|
| 3.1 Agent Struct & Lifecycle | done | 3/3 | Agent struct with state transitions, shutdown, 9 tests |
| 3.2 Agent Pool Manager | done | 4/4 | AgentPool, spawn/release/remove, PoolStats, 19 tests |
| 3.3 Message Passing | done | 4/4 | Channels, AgentHandle, Dispatcher, response/command flow |
| 3.4 Role System | done | 5/5 | RoleLibrary, prompts, RequiredReadingLoader, RoleManager, 12 tests |
| 3.5 Task Execution Loop | done | 4/4 | Run loop, LLM integration with role context, progress updates, timeout handling |
| 3.6 Escalation Flow | done | 3/3 | EscalationPolicy, EscalationManager, HumanReview types, 12 tests |
| 3.7 Inter-Agent Protocol | done | 5/5 | Protocol types, serialization, validation, DelegationContext, 19 tests |

**Milestone Status:** Complete (7/7 tickets done)

---

## Milestone 4: Prompt Engineering & Agent Intelligence

**Goal**: Robust, tested prompts that drive reliable agent behavior.

| Ticket | Status | Progress | Notes |
|--------|--------|----------|-------|
| 4.1 Prompt Architecture Design | done | 4/4 | PromptBuilder, ContextInjector, PromptVersion, 23 tests |
| 4.2 Orchestrator Thinking Patterns | done | 5/5 | Decomposition, review, routing, conversation, recovery prompts, 25 tests |
| 4.3 Worker Thinking Patterns | done | 5/5 | Implementation, context-gathering, progress, self-check, stuck-detection prompts |
| 4.4 Utility Thinking Patterns | done | 4/4 | Task recognition, execution, reporting, escalation prompts, 20 tests |
| 4.5 Structured Output Design | done | 5/5 | DecompositionOutput, TaskResultOutput, ReviewOutput, ErrorOutput, OutputValidator with 69 tests |
| 4.6 Few-Shot Examples Library | done | 5/5 | Decomposition, implementation, review, recovery examples + selector, 30 tests |
| 4.7 Prompt Testing Framework | done | 6/6 | Harness, assertions, decomp/impl tests, diff tooling, confusion detection, 74 tests |
| 4.8 Context Management Strategy | done | 5/5 | ContextBudget, FileSelector, FileSummarizer, ContextRequestHandler, HistoryManager, 35 tests |
| 4.9 Self-Correction & Recovery Prompts | done | 5/5 | Parse error, test failure, review rejection, stuck loop, conflict resolution prompts, 29 tests |
| 4.10 Tool Definition & Selection | done | 6/6 | ToolDefinition, ToolRegistry, file/git/test tools, selection prompts, parser, 31 tests |
| 4.11 Context Window Validation | done | 5/5 | TokenCounter, ModelLimits, ContextValidator, ContextTruncator, ContextPressureWarning, 38 tests |

**Milestone Status:** Complete (11/11 tickets done)

---

## Milestone 5: Orchestration Core

**Goal**: Orchestrator can decompose tickets into slices, route to appropriate agents.

| Ticket | Status | Progress | Notes |
|--------|--------|----------|-------|
| 5.1 Planner (Ticket → Slices) | done | 5/5 | Planner, PlannerConfig, DecompositionError, PlannerOutput, retry logic, 13 tests |
| 5.2 Task Queue | done | 4/4 | TaskQueue, PersistentTaskQueue, RequeuePolicy, priority ordering, 20 tests |
| 5.3 Router (Task → Tier) | done | 3/3 | Router, RouterConfig, RoutingRule, RuleMatcher, metadata field added to Task, 21 tests |
| 5.4 Dependency Tracking | done | 3/3 | DependencyTracker, DependencyAwareQueue, depends_on field, circular detection, 11 tests |
| 5.5 Scheduler | done | 3/3 | TaskScheduler, SchedulerConfig, agent wait with Notify, preemption check, 2 tests |

**Milestone Status:** Complete (5/5 tickets done)

---

## Milestone 6: TUI Basic

**Goal**: Functional terminal interface with feed, chat, and navigation.

| Ticket | Status | Progress | Notes |
|--------|--------|----------|-------|
| 6.1 Terminal Setup | done | 3/3 | init_terminal, restore_terminal, install_panic_hook, App.run(), main.rs integration |
| 6.2 Layout System | done | 3/3 | AppLayout, HeaderBar, InputBar widgets, 8 tests |
| 6.3 Home Screen | done | 3/3 | HomeView widget, view state management, typing transitions to Main |
| 6.4 Feed View (/feed) | done | 4/4 | FeedView widget, FeedItem types, scrolling, App integration, 13 tests |
| 6.5 Chat View (/main) | done | 5/5 | ChatView widget, message types, submission, mock orchestrator, 9 tests |
| 6.6 Slash Command Router | pending | 0/4 | Needs 6.2 |
| 6.7 Logs View (/logs) | pending | 0/3 | Needs 6.2, M1.5 |

**Milestone Status:** In Progress (5/7 tickets done)

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
| 7.6 Git Merge Operations | pending | 0/6 | Needs 7.2 |

**Milestone Status:** Not Started

---

## Milestone 8: GitHub Integration

**Goal**: Can pull issues from GitHub, create PRs, and manage PR merge queue.

| Ticket | Status | Progress | Notes |
|--------|--------|----------|-------|
| 8.0 GitHub Authentication | pending | 0/5 | Needs M1 |
| 8.1 GitHub API Client | pending | 0/4 | Needs 8.0 |
| 8.2 Issue Sync | pending | 0/3 | Needs 8.1 |
| 8.3 PR Creation | pending | 0/3 | Needs 8.1, M7 |
| 8.4 Progress Updates | pending | 0/3 | Needs 8.1 |
| 8.5 PR Retrieval & Review | pending | 0/4 | Needs 8.1 |
| 8.6 PR Merge Operations | pending | 0/4 | Needs 8.1, 8.5 |
| 8.7 PR Merge Queue & Conflict Resolution | pending | 0/6 | Needs 7.6, 8.5, 8.6, 8.4 |

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
| 9.7 Refactor Mode Foundation | done | 4/4 | Types, DB, scheduler pause/resume |
| 9.8 Refactor Agent | done | 4/4 | Intent detection, change proposals, apply changes |
| 9.9 TUI Integration | done | 3/3 | /refactor command, mode switching, status bar |
| 9.10 Menu Types & Data | pending | 0/3 | Needs 9.7, 9.9 |
| 9.11 Menu Widget & Rendering | pending | 0/3 | Needs 9.10 |
| 9.12 App Integration | pending | 0/3 | Needs 9.10, 9.11, 9.9 |

**Milestone Status:** In Progress (3/12 tickets done)

---

## Summary

| Milestone | Tickets | Slices | Status |
|-----------|---------|--------|--------|
| M1: Foundation | 5 | 24 | Complete |
| M2: LLM Layer | 4 | 13 | Complete |
| M3: Agent Runtime | 7 | 26 | Complete |
| M4: Prompt Engineering | 11 | 55 | Complete |
| M5: Orchestration Core | 5 | 18 | Complete |
| M6: TUI Basic | 7 | 25 | Not Started |
| M7: Execution Layer | 6 | 28 | Not Started |
| M8: GitHub Integration | 8 | 32 | Not Started |
| M9: Polish & Production | 12 | 42 | In Progress (3/12 tickets) |
| **Total** | **65** | **263** | |

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
