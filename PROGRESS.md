# nexor Progress

> Work tracking for ROADMAP.md implementation

## Current Focus

**Active:** M12 Terminal CLI (Ink)
**Next:** M13 Agent View
**Planned:** M14 Dynamic Agent Selection (can run in parallel with M10-M13)
**Blocked:** None

---

## Architectural Pivot (2026-01-27)

**Decision**: Migrate from ratatui TUI to Rust backend + React frontend.

**Rationale**:

- Web UI provides broader reach (browser, mobile)
- React ecosystem for faster UI development
- Rust backend keeps performance where it matters (agents, LLM, orchestration)
- Path to SaaS deployment

**What's Retained** (83% of work):

- M1-M5: Foundation, LLM, Agents, Prompts, Orchestration
- M7-M9: Execution, GitHub, Polish

**What's Deprecated**:

- M6: TUI Basic (replaced by React frontend)
- M10-old: TUI File Editor
- M11-old: TUI Analytics

**New Milestones**:

- M10: Server Layer (Axum HTTP + WebSocket)
- M11: React Foundation
- M12: Terminal CLI (Ink)
- M13: Agent View (Factorio-Inspired Monitor)

---

## Milestone 1: Foundation - COMPLETE

**Goal**: Project compiles, core types exist, config loads, database works.

| Ticket                     | Status | Progress | Notes                                                       |
| -------------------------- | ------ | -------- | ----------------------------------------------------------- |
| 1.1 Project Scaffolding    | done   | 3/3      | Cargo.toml, directory structure, main.rs with tokio         |
| 1.2 Core Type Definitions  | done   | 6/6      | All types: task, agent, message, ticket, cost, config       |
| 1.3 Configuration System   | done   | 4/4      | Global + project config loading, merging, validation        |
| 1.4 Database Setup         | done   | 8/8      | SQLite init, 6 migrations, CRUD queries                     |
| 1.5 Logging Infrastructure | done   | 3/3      | Tracing with env filter, file appender, helper spans/macros |

**Milestone Status:** Complete (5/5 tickets done)

---

## Milestone 2: LLM Layer - COMPLETE

**Goal**: Can send prompts to Anthropic and get streaming responses.

| Ticket                   | Status | Progress | Notes                                                               |
| ------------------------ | ------ | -------- | ------------------------------------------------------------------- |
| 2.1 Provider Abstraction | done   | 3/3      | LLMProvider trait, types, streaming                                 |
| 2.2 Anthropic Client     | done   | 4/4      | HTTP client, send_message, streaming, token counts                  |
| 2.3 Cost Tracking        | done   | 3/3      | ModelPricing, CostTracker, aggregation methods                      |
| 2.4 Retry Logic          | done   | 3/3      | ExponentialBackoff, RetryPolicy, RetryingProvider wrapper, 14 tests |

**Milestone Status:** Complete (4/4 tickets done)

---

## Milestone 3: Agent Runtime - COMPLETE

**Goal**: Agents can be spawned, receive tasks, execute them, and report back.

| Ticket                       | Status | Progress | Notes                                                                           |
| ---------------------------- | ------ | -------- | ------------------------------------------------------------------------------- |
| 3.1 Agent Struct & Lifecycle | done   | 3/3      | Agent struct with state transitions, shutdown, 9 tests                          |
| 3.2 Agent Pool Manager       | done   | 4/4      | AgentPool, spawn/release/remove, PoolStats, 19 tests                            |
| 3.3 Message Passing          | done   | 4/4      | Channels, AgentHandle, Dispatcher, response/command flow                        |
| 3.4 Role System              | done   | 5/5      | RoleLibrary, prompts, RequiredReadingLoader, RoleManager, 12 tests              |
| 3.5 Task Execution Loop      | done   | 4/4      | Run loop, LLM integration with role context, progress updates, timeout handling |
| 3.6 Escalation Flow          | done   | 3/3      | EscalationPolicy, EscalationManager, HumanReview types, 12 tests                |
| 3.7 Inter-Agent Protocol     | done   | 5/5      | Protocol types, serialization, validation, DelegationContext, 19 tests          |

**Milestone Status:** Complete (7/7 tickets done)

---

## Milestone 4: Prompt Engineering - COMPLETE

**Goal**: Robust, tested prompts that drive reliable agent behavior.

| Ticket                                 | Status | Progress | Notes                                                                                           |
| -------------------------------------- | ------ | -------- | ----------------------------------------------------------------------------------------------- |
| 4.1 Prompt Architecture Design         | done   | 4/4      | PromptBuilder, ContextInjector, PromptVersion, 23 tests                                         |
| 4.2 Orchestrator Thinking Patterns     | done   | 5/5      | Decomposition, review, routing, conversation, recovery prompts, 25 tests                        |
| 4.3 Worker Thinking Patterns           | done   | 5/5      | Implementation, context-gathering, progress, self-check, stuck-detection prompts                |
| 4.4 Utility Thinking Patterns          | done   | 4/4      | Task recognition, execution, reporting, escalation prompts, 20 tests                            |
| 4.5 Structured Output Design           | done   | 5/5      | DecompositionOutput, TaskResultOutput, ReviewOutput, ErrorOutput, OutputValidator with 69 tests |
| 4.6 Few-Shot Examples Library          | done   | 5/5      | Decomposition, implementation, review, recovery examples + selector, 30 tests                   |
| 4.7 Prompt Testing Framework           | done   | 6/6      | Harness, assertions, decomp/impl tests, diff tooling, confusion detection, 74 tests             |
| 4.8 Context Management Strategy        | done   | 5/5      | ContextBudget, FileSelector, FileSummarizer, ContextRequestHandler, HistoryManager, 35 tests    |
| 4.9 Self-Correction & Recovery Prompts | done   | 5/5      | Parse error, test failure, review rejection, stuck loop, conflict resolution prompts, 29 tests  |
| 4.10 Tool Definition & Selection       | done   | 6/6      | ToolDefinition, ToolRegistry, file/git/test tools, selection prompts, parser, 31 tests          |
| 4.11 Context Window Validation         | done   | 5/5      | TokenCounter, ModelLimits, ContextValidator, ContextTruncator, ContextPressureWarning, 38 tests |

**Milestone Status:** Complete (11/11 tickets done)

---

## Milestone 5: Orchestration Core - COMPLETE

**Goal**: Orchestrator can decompose tickets into slices, route to appropriate agents.

| Ticket                        | Status | Progress | Notes                                                                                   |
| ----------------------------- | ------ | -------- | --------------------------------------------------------------------------------------- |
| 5.1 Planner (Ticket → Slices) | done   | 5/5      | Planner, PlannerConfig, DecompositionError, PlannerOutput, retry logic, 13 tests        |
| 5.2 Task Queue                | done   | 4/4      | TaskQueue, PersistentTaskQueue, RequeuePolicy, priority ordering, 20 tests              |
| 5.3 Router (Task → Tier)      | done   | 3/3      | Router, RouterConfig, RoutingRule, RuleMatcher, metadata field added to Task, 21 tests  |
| 5.4 Dependency Tracking       | done   | 3/3      | DependencyTracker, DependencyAwareQueue, depends_on field, circular detection, 11 tests |
| 5.5 Scheduler                 | done   | 3/3      | TaskScheduler, SchedulerConfig, agent wait with Notify, preemption check, 2 tests       |

**Milestone Status:** Complete (5/5 tickets done)

---

## Milestone 6: TUI Basic - DEPRECATED

> **Status**: DEPRECATED - Superseded by React frontend (M10-M13)
>
> This milestone's code will be removed. See "Code Cleanup" section below.

| Ticket  | Status     | Notes              |
| ------- | ---------- | ------------------ |
| 6.1-6.7 | deprecated | Code to be removed |

---

## Milestone 7: Execution Layer - COMPLETE

**Goal**: Agents can read/write files, run git commands, execute tests.

| Ticket                   | Status | Progress | Notes                                                              |
| ------------------------ | ------ | -------- | ------------------------------------------------------------------ |
| 7.1 File Operations      | done   | 3/4      | FileOps with read/write/delete/list, path validation, 13 tests     |
| 7.2 Git Operations       | done   | 6/6      | GitOps with status/branch/commit/diff/push/merge, 16 tests         |
| 7.3 Test Runner          | done   | 4/4      | TestRunner with framework detection, run/parse/streaming, 11 tests |
| 7.4 Docker Sandbox       | done   | 4/4      | Sandbox with config builder, mounts, resource limits, 3 tests      |
| 7.5 Approval Gates       | done   | 4/4      | ApprovalGate, DangerousOperation, InteractiveApprovalGate, 8 tests |
| 7.6 Git Merge Operations | done   | 6/6      | fetch/merge/pull, conflict detection/resolution/abort, 7 tests     |

**Milestone Status:** Complete (6/6 tickets done)

---

## Milestone 8: GitHub Integration - COMPLETE

**Goal**: Can pull issues from GitHub, create PRs, and manage PR merge queue.

| Ticket                                   | Status | Progress | Notes                                                                                                   |
| ---------------------------------------- | ------ | -------- | ------------------------------------------------------------------------------------------------------- |
| 8.0 GitHub Authentication                | done   | 5/5      | Device flow, token storage, git config, 9 tests                                                         |
| 8.1 GitHub API Client                    | done   | 4/4      | Client, issues, PRs, rate limiting, 6 tests                                                             |
| 8.2 Issue Sync                           | done   | 3/3      | IssueRef parsing, conversion, IssueSync service, 9 tests                                                |
| 8.3 PR Creation                          | done   | 3/3      | PrBodyGenerator, PrService, PrResult, 6 tests                                                           |
| 8.4 Progress Updates                     | done   | 3/3      | ProgressSummary, CommentService, update comments, 6 tests                                               |
| 8.5 PR Retrieval & Review                | done   | 4/4      | PrFile, reviews, approve/request changes, 3 tests                                                       |
| 8.6 PR Merge Operations                  | done   | 4/4      | MergeMethod, MergePrRequest/Response/Result, MergeableStatus, MergeError, client methods, 7 tests       |
| 8.7 PR Merge Queue & Conflict Resolution | done   | 6/6      | MergeQueue, QueueStatus, MergeQueueProcessor, conflict resolution flow, progress notifications, 8 tests |

**Milestone Status:** Complete (8/8 tickets done)

---

## Milestone 9: Polish & Production - COMPLETE

**Goal**: Production-ready, fully-featured.

| Ticket                       | Status | Progress | Notes                                                                                        |
| ---------------------------- | ------ | -------- | -------------------------------------------------------------------------------------------- |
| 9.1 Remaining TUI Views      | done   | 3/3      | TasksView, AgentsView, CostsView widgets, 49 tests - **TO BE REMOVED**                       |
| 9.2 Headless Mode            | done   | 4/4      | cli.rs, headless.rs, task input parsing, 13 tests                                            |
| 9.3 Error Handling Polish    | done   | 3/3      | NexorError, ErrorDisplay, suggestions, 34 tests                                              |
| 9.4 Docker Packaging         | done   | 3/3      | Dockerfile, docker-compose.yml, docs/docker.md                                               |
| 9.5 Documentation            | done   | 4/4      | installation.md, configuration.md, usage.md, commands.md                                     |
| 9.6 Observability & Replay   | done   | 5/5      | LlmCallLogger, DecisionReplay, SessionExporter, ReplayView, 29 tests                         |
| 9.7 Refactor Mode Foundation | done   | 4/4      | Types, DB, scheduler pause/resume                                                            |
| 9.8 Refactor Agent           | done   | 4/4      | Intent detection, change proposals, apply changes                                            |
| 9.9 TUI Integration          | done   | 3/3      | /refactor command, mode switching, status bar - **TO BE REMOVED**                            |
| 9.10 Menu Types & Data       | done   | 3/3      | MenuItem, MenuAction, Menu types, MenuState, build_menu_tree(), milestone limit DB, 24 tests |
| 9.11 Menu Widget & Rendering | done   | 3/3      | MenuWidget, MenuController, centered_rect, 22 tests - **TO BE REMOVED**                      |
| 9.12 App Integration         | done   | 3/3      | Menu command, status, actions, Esc trigger, milestone limit - **TO BE REMOVED**              |

**Milestone Status:** Complete (12/12 tickets done)

---

## Milestone 10: Server Layer - IN PROGRESS

**Goal**: Axum HTTP server exposing REST API and WebSocket for React frontend.

| Ticket                         | Status | Progress | Notes                                                       |
| ------------------------------ | ------ | -------- | ----------------------------------------------------------- |
| 10.1 Axum Server Setup         | done   | 4/4      | Server, state, graceful shutdown, CLI port flag             |
| 10.2 REST API - Core Endpoints | done   | 5/5      | Health, tasks CRUD, agents, config endpoints                |
| 10.3 REST API - Chat Endpoint  | done   | 4/4      | POST /chat, GET/DELETE /chat/history, SSE streaming         |
| 10.4 WebSocket Gateway         | done   | 5/5      | WS handler, subscriptions, broadcast channels, ping/pong    |
| 10.5 Authentication            | done   | 5/5      | Password auth, JWT tokens, auth middleware, /auth endpoints |
| 10.6 Static File Serving       | done   | 3/3      | ServeDir, SPA fallback, cache headers, 6 tests              |

**Milestone Status:** Complete (6/6 tickets done)

---

## Milestone 11: React Foundation - COMPLETE

**Goal**: React app scaffold with auth, routing, and layout.

| Ticket                 | Status | Progress | Notes                                                                                                       |
| ---------------------- | ------ | -------- | ----------------------------------------------------------------------------------------------------------- |
| 11.1 Project Setup     | done   | 5/5      | Vite + React + TypeScript, TailwindCSS v4, React Router, Zustand, proxy configured                          |
| 11.2 API Client        | done   | 4/4      | Typed HTTP client, auth token expiry, WebSocket with reconnection, React hooks (useChat, useFeed, useTasks) |
| 11.3 Authentication UI | done   | 5/5      | Input, Button components, LoginPage, SetupPage, auth flow routing, ProtectedRoute                           |
| 11.4 Layout Components | done   | 4/4      | Layout, Sidebar, Header, StatusDot, responsive mobile overlay, placeholder pages, nested routes             |

**Milestone Status:** Complete (4/4 tickets done)

---

## Milestone 12: Terminal CLI (Ink) - COMPLETE

**Goal**: Claude Code-style terminal CLI using TypeScript + Ink. Chat-only scope.

| Ticket                    | Status  | Progress | Notes                                                       |
| ------------------------- | ------- | -------- | ----------------------------------------------------------- |
| 12.1 CLI Scaffolding      | done    | 4/4      | Package setup, tsconfig, Ink app entry, directory structure |
| 12.2 API Client           | done    | 4/4      | HTTP fetch wrapper, types, API methods, 16 tests            |
| 12.3 Auth Flow            | done    | 3/3      | Login prompt, token persistence, auth gate                  |
| 12.4 Chat UI Components   | done    | 5/5      | Message, MessageList, Input, useChat, ChatView, 13 tests    |
| 12.5 Streaming & SSE      | done    | 3/3      | SSE client, StreamingMessage component, wired into useChat, 21 tests |
| 12.6 Polish & Integration | done    | 3/3      | Markdown rendering, error handling, Makefile, 161 tests     |

**Milestone Status:** Complete (6/6 tickets done)

**Note:** Previous M12 (React Features) web UI work remains in `ui/` — chat and feed views are functional. This milestone adds a terminal-native CLI as the primary interface.

---

## Milestone 13: Agent View (Factorio-Inspired Monitor) - NOT STARTED

**Goal**: Factorio-inspired agent monitoring screen in the CLI. Unicode station grid with real-time WebSocket updates, progress animations, and Claude Code aesthetic.

| Ticket | Status | Progress | Notes |
|--------|--------|----------|-------|
| 13.1 WebSocket Client | done | 4/4 | ws package, typed messages, reconnect, tests |
| 13.2 Agent Types & State | done | 3/3 | Types, REST fetch, useAgents hook |
| 13.3 Agent Station Widget | pending | 0/4 | Unicode box, status icons, progress bar |
| 13.4 Agent Grid Layout | pending | 0/3 | Tier sections, stats summary, responsive rows |
| 13.5 Screen Navigation | pending | 0/3 | Tab to switch Chat/Agents, status bar |
| 13.6 Polish & Animations | pending | 0/4 | Spinner, smooth progress, flash, loading states |

**Milestone Status:** Not Started (0/6 tickets done)

---

## Milestone 15: Repo Management & Power User Workspace - DEPRECATED

> **Status**: DEPRECATED — Scope was premature. Features may be revisited individually in future milestones.

| Ticket    | Status     | Notes              |
| --------- | ---------- | ------------------ |
| 15.1-15.9 | deprecated | No work started    |

---

## Milestone 16: SaaS Foundation - DEPRECATED

> **Status**: DEPRECATED — Multi-tenant SaaS pivot shelved. Postgres migration extracted to M17 as a standalone effort.

| Ticket    | Status     | Notes                                    |
| --------- | ---------- | ---------------------------------------- |
| 16.1-16.9 | deprecated | 16.1 (Postgres) superseded by M17       |

---

## Milestone 17: SQLite to PostgreSQL Migration - NOT STARTED

**Goal**: Pure database swap — replace SQLite with PostgreSQL. Same schema, no new features. Hard cut, no dual-driver.

| Ticket | Title | Status | Progress | Notes |
|--------|-------|--------|----------|-------|
| 17.1 | Infrastructure & Dependencies | done | 4/4 | Docker Compose Postgres, sqlx postgres features, DATABASE_URL config, .env.example |
| 17.2 | Migration Files Rewrite | done | 4/4 | All 14 migrations rewritten: TEXT→UUID, TEXT→TIMESTAMPTZ, INTEGER→BOOLEAN, TEXT→JSONB, datetime('now')→NOW(), INSERT OR IGNORE→ON CONFLICT DO NOTHING |
| 17.3 | Connection Pool & Init | done | 3/3 | PgPool, sqlx::migrate!(), removed custom migration runner |
| 17.4 | Query Rewrites: Tasks/Chat/Auth | done | 5/5 | All queries use $N params, native Uuid/DateTime/JsonValue types, PgPool |
| 17.5 | Query Rewrites: Refactor/PRD | done | 4/4 | refactor.rs and prd.rs converted to PgPool with native types |
| 17.6 | Consumer File Updates | done | 4/4 | All 10+ consumer files updated: SqlitePool→PgPool, ?→$N, string parsing→native types |
| 17.7 | Test Infrastructure | pending | 0/4 | Per-test Postgres databases |
| 17.8 | Cleanup & Documentation | pending | 0/4 | Remove SQLite, data migration tool |

**Milestone Status:** In Progress (6/8 tickets done)

---

## Milestone 22: Multi-Agent Docker Isolation - NOT STARTED

**Goal**: Per-agent Docker containers with git worktree isolation for concurrent multi-branch work.

| Ticket | Title | Status | Progress | Notes |
|--------|-------|--------|----------|-------|
| 22.1 | Git Worktree Manager | pending | 0/5 | WorktreeManager CRUD for isolated checkouts |
| 22.2 | Agent Worker Dockerfile | pending | 0/3 | Container image with runtimes |
| 22.3 | Container Lifecycle Management | pending | 0/5 | Spawn, stop, status, health check |
| 22.4 | Agent Worker Mode | pending | 0/5 | `--worker` binary mode, WS connection |
| 22.5 | Worker Client Protocol | pending | 0/4 | Host-side client for container communication |
| 22.6 | Container Pool Integration | pending | 0/5 | Full lifecycle: worktree + container + merge |
| 22.7 | Cleanup, Monitoring & Health | pending | 0/3 | Orphan cleanup, health monitoring, drain |

**Milestone Status:** Not Started (0/7 tickets done)

---

## Summary

| Milestone                  | Tickets | Status            |
| -------------------------- | ------- | ----------------- |
| M1: Foundation             | 5       | Complete          |
| M2: LLM Layer              | 4       | Complete          |
| M3: Agent Runtime          | 7       | Complete          |
| M4: Prompt Engineering     | 11      | Complete          |
| M5: Orchestration Core     | 5       | Complete          |
| M6: TUI Basic              | -       | **DEPRECATED**    |
| M7: Execution Layer        | 6       | Complete          |
| M8: GitHub Integration     | 8       | Complete          |
| M9: Polish & Production    | 12      | Complete          |
| M10: Server Layer          | 6       | Complete          |
| M11: React Foundation      | 4       | Complete          |
| M12: Terminal CLI (Ink)     | 6       | Complete          |
| M13: Agent View             | 6       | Not Started       |
| M15: Repo Mgmt & Workspace | -       | **DEPRECATED**    |
| M16: SaaS Foundation       | -       | **DEPRECATED**    |
| M17: SQLite → PostgreSQL   | 8       | In Progress       |
| M22: Docker Isolation      | 7       | Not Started       |
| **Total Active**           | **94**  |                   |

---

## Code Cleanup (COMPLETE)

The following code will be removed as part of the architectural pivot:

### Files to Delete

```
src/tui/                    # Entire directory
├── mod.rs
├── app.rs
├── commands.rs
├── errors.rs
├── mode.rs
├── theme.rs
├── menu/
│   ├── mod.rs
│   └── controller.rs
└── views/
    ├── mod.rs
    ├── chat.rs
    ├── feed.rs
    ├── logs.rs
    ├── file_viewer.rs
    └── ...
```

### Dependencies to Remove (Cargo.toml)

```toml
# Remove these:
ratatui = "0.28"
crossterm = "0.28"
syntect = "5"  # Will use JS-based highlighting in React
```

### Dependencies to Add (Cargo.toml)

```toml
# Add these:
axum = "0.7"
tower-http = { version = "0.5", features = ["cors", "fs", "trace"] }
tokio-tungstenite = "0.21"
argon2 = "0.5"
jsonwebtoken = "9"
```

---

## Decisions Log

| Date       | Decision                       | Rationale                              |
| ---------- | ------------------------------ | -------------------------------------- |
| 2026-01-27 | Pivot from TUI to Rust + React | Better UX, broader reach, path to SaaS |

---

## Next Steps

1. [x] Delete `src/tui/` directory - DONE
2. [x] Update `Cargo.toml` (remove TUI deps, add server deps) - DONE
3. [x] Update `src/main.rs` to launch server instead of TUI - DONE
4. [x] Create `src/server/mod.rs` - DONE (stub with health check)
5. [x] Ticket 10.1: Axum Server Setup - DONE (state module, graceful shutdown, port CLI flag)
6. [x] Ticket 10.2: REST API - Core Endpoints - DONE (health, tasks, agents, config endpoints)
7. [x] Ticket 10.3: REST API - Chat Endpoint - DONE (POST /chat, GET/DELETE /chat/history, SSE streaming)
8. [x] Ticket 10.4: WebSocket Gateway - DONE (ws handler, subscriptions, broadcast channels, ping/pong)
9. [x] Ticket 10.5: Authentication - DONE (password auth, JWT tokens, auth middleware, /auth endpoints)
10. [x] Ticket 10.6: Static File Serving - DONE (ServeDir, SPA fallback, cache headers)
11. [ ] Start Milestone 11: React Foundation
