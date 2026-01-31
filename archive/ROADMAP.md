# nexor ROADMAP

> Living document for AI agent orchestration. Orchestrator reads this for context.

---

## Epic: nexor v1.0

Build a **Rust backend + React frontend** web application that orchestrates AI agents for GitHub workflows.

**Architecture**: Rust server (Axum) + React SPA + WebSocket for real-time updates.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     React Frontend                          │
│                     (ui/ directory)                         │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐       │
│  │  Chat   │  │  Feed   │  │  Tasks  │  │  Files  │       │
│  └─────────┘  └─────────┘  └─────────┘  └─────────┘       │
└─────────────────────────────────────────────────────────────┘
                         │ HTTP + WebSocket
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                     Rust Server (Axum)                      │
│                     src/server/                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │  REST API   │  │  WebSocket  │  │  Static file serve  │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│              Existing Orchestration Core                    │
│  agents/ │ orchestration/ │ llm/ │ execution/ │ github/    │
│                    (UNCHANGED from M1-M9)                   │
└─────────────────────────────────────────────────────────────┘
```

---

## Milestone Summary

| Milestone | Status | Description |
|-----------|--------|-------------|
| M1: Foundation | Complete | Core types, config, database, logging |
| M2: LLM Layer | Complete | Anthropic client, streaming, cost tracking |
| M3: Agent Runtime | Complete | Agent lifecycle, pools, messaging |
| M4: Prompt Engineering | Complete | Prompts, schemas, testing framework |
| M5: Orchestration Core | Complete | Planner, queue, router, scheduler |
| M6: TUI Basic | **DEPRECATED** | *Superseded by M10-M13* |
| M7: Execution Layer | Complete | File ops, git, test runner, sandbox |
| M8: GitHub Integration | Complete | Issues, PRs, merge queue |
| M9: Polish & Production | Complete | Error handling, observability |
| M10: Server Layer | **NEW** | Axum HTTP server + WebSocket |
| M11: React Foundation | **NEW** | Auth, layout, routing |
| M12: React Features | **NEW** | Chat, feed, tasks, files |
| M13: React Polish | **NEW** | Analytics, settings, mobile |
| M14: Dynamic Agent Selection | **NEW** | Difficulty-based model routing, prompt fixes |
| M15: Repo Mgmt & Workspace | **NEW** | Multi-repo, prompt library, Monaco editor, reports, pivots |
| M16: SaaS Foundation | **NEW** | Postgres, users/orgs, GitHub OAuth, cloud repos, collab chat, presence |

---

## Completed Milestones (M1-M5, M7-M9)

*These milestones are complete and their code is retained. See PROGRESS.md for details.*

### Milestone 1: Foundation - COMPLETE
Core types, configuration system, SQLite database, logging infrastructure.

### Milestone 2: LLM Layer - COMPLETE
Anthropic client with streaming, cost tracking, retry logic.

### Milestone 3: Agent Runtime - COMPLETE
Agent lifecycle, pool management, message passing, escalation.

### Milestone 4: Prompt Engineering - COMPLETE
Prompt architecture, thinking patterns, structured output, testing framework.

### Milestone 5: Orchestration Core - COMPLETE
Planner, task queue, router, dependency tracking, scheduler.

### Milestone 7: Execution Layer - COMPLETE
File operations, git operations, test runner, Docker sandbox, approval gates.

### Milestone 8: GitHub Integration - COMPLETE
GitHub API client, issue sync, PR creation, merge queue.

### Milestone 9: Polish & Production - COMPLETE
Error handling, observability, replay system, headless mode.

---

## DEPRECATED: Milestone 6 (TUI Basic)

> **Status**: DEPRECATED - Superseded by React frontend (M10-M13)
>
> The TUI code in `src/tui/` will be removed. Web UI provides better UX and broader reach.

---

## DEPRECATED: Milestone 10-11 (TUI File Editor & Analytics)

> **Status**: DEPRECATED - These were TUI-specific features
>
> File editing and analytics will be implemented in the React frontend instead.

---

## NEW: Milestone 10: Server Layer

**Goal**: Axum HTTP server exposing REST API and WebSocket for the React frontend.

**Checkpoint**: Can curl `/api/health`, send a chat message via API, receive streaming updates via WebSocket.

### Ticket 10.1: Axum Server Setup

Initialize HTTP server with basic routing.

| Slice | Description | Test |
|-------|-------------|------|
| 10.1.1 | Add axum, tower-http to Cargo.toml, create `src/server/mod.rs` | Module compiles |
| 10.1.2 | Implement server startup with configurable host/port | Server starts, logs listening address |
| 10.1.3 | Add CORS middleware for local development | React dev server can connect |
| 10.1.4 | Add graceful shutdown handling | Clean shutdown on SIGTERM |

### Ticket 10.2: REST API - Core Endpoints

Basic CRUD endpoints for tasks, agents, config.

| Slice | Description | Test |
|-------|-------------|------|
| 10.2.1 | Implement `GET /api/health` | Returns 200 OK |
| 10.2.2 | Implement `GET /api/tasks`, `GET /api/tasks/:id` | Returns task list/details |
| 10.2.3 | Implement `POST /api/tasks` (create manual task) | Creates task, returns ID |
| 10.2.4 | Implement `GET /api/agents` | Returns agent pool status |
| 10.2.5 | Implement `GET /api/config`, `PATCH /api/config` | Read/update config |

### Ticket 10.3: REST API - Chat Endpoint

Chat with orchestrator via HTTP.

| Slice | Description | Test |
|-------|-------------|------|
| 10.3.1 | Implement `POST /api/chat` with message body | Message queued to orchestrator |
| 10.3.2 | Implement `GET /api/chat/history` | Returns conversation history |
| 10.3.3 | Return streaming response using SSE | Tokens stream to client |
| 10.3.4 | Implement `DELETE /api/chat/history` | Clears history |

### Ticket 10.4: WebSocket Gateway

Real-time updates for feed, tasks, agents.

| Slice | Description | Test |
|-------|-------------|------|
| 10.4.1 | Add tokio-tungstenite, implement `/ws` endpoint | WebSocket connects |
| 10.4.2 | Implement subscription protocol (subscribe to channels) | Can subscribe to "feed", "tasks" |
| 10.4.3 | Broadcast feed items to subscribed clients | Feed updates reach clients |
| 10.4.4 | Broadcast task/agent status changes | Status updates reach clients |
| 10.4.5 | Handle client disconnect/reconnect gracefully | No server crash on disconnect |

### Ticket 10.5: Authentication

Local password auth + optional account linking.

| Slice | Description | Test |
|-------|-------------|------|
| 10.5.1 | Create `sessions` table, add argon2 for password hashing | Migration runs |
| 10.5.2 | Implement `POST /api/auth/setup` (first-run password creation) | Password stored hashed |
| 10.5.3 | Implement `POST /api/auth/login` returning JWT | Returns valid JWT |
| 10.5.4 | Add auth middleware, protect API routes | Unauthenticated requests rejected |
| 10.5.5 | Implement `GET /api/auth/me` | Returns current user info |

### Ticket 10.6: Static File Serving

Serve React build in production.

| Slice | Description | Test |
|-------|-------------|------|
| 10.6.1 | Serve static files from `ui/dist` directory | Index.html served at `/` |
| 10.6.2 | Handle SPA routing (return index.html for unknown routes) | `/chat` returns index.html |
| 10.6.3 | Add cache headers for assets | Assets cached appropriately |

---

## NEW: Milestone 11: React Foundation

**Goal**: React app scaffold with auth, routing, and layout.

**Checkpoint**: Can login, see layout with sidebar, navigate between views.

### Ticket 11.1: Project Setup

Initialize React project with tooling.

| Slice | Description | Test |
|-------|-------------|------|
| 11.1.1 | Create Vite + React + TypeScript project in `ui/` | `npm run dev` works |
| 11.1.2 | Add TailwindCSS for styling | Styles apply |
| 11.1.3 | Add React Router for navigation | Routes work |
| 11.1.4 | Add Zustand for state management | Store works |
| 11.1.5 | Configure proxy to Rust backend in dev mode | API calls work in dev |

### Ticket 11.2: API Client

TypeScript client for Rust API.

| Slice | Description | Test |
|-------|-------------|------|
| 11.2.1 | Create typed API client with fetch wrapper | Client compiles |
| 11.2.2 | Add auth token handling (storage, injection) | Token sent with requests |
| 11.2.3 | Add WebSocket client with reconnection | WebSocket connects |
| 11.2.4 | Create React hooks: `useChat`, `useFeed`, `useTasks` | Hooks return data |

### Ticket 11.3: Authentication UI

Login and setup screens.

| Slice | Description | Test |
|-------|-------------|------|
| 11.3.1 | Create `LoginPage` component | Renders login form |
| 11.3.2 | Create `SetupPage` component (first-run password) | Renders setup form |
| 11.3.3 | Implement auth flow (login → store token → redirect) | Can login successfully |
| 11.3.4 | Add protected route wrapper | Unauthenticated users redirected |

### Ticket 11.4: Layout Components

App shell with sidebar and header.

| Slice | Description | Test |
|-------|-------------|------|
| 11.4.1 | Create `AppLayout` with sidebar, header, main area | Layout renders |
| 11.4.2 | Create `Sidebar` with navigation links | Links work |
| 11.4.3 | Create `Header` with agent status indicators | Shows agent counts |
| 11.4.4 | Make layout responsive (collapse sidebar on mobile) | Mobile layout works |

---

## NEW: Milestone 12: React Features

**Goal**: Core feature views - chat, feed, tasks, files.

**Checkpoint**: Full feature parity with original TUI vision.

### Ticket 12.1: Chat View

Orchestrator conversation interface.

| Slice | Description | Test |
|-------|-------------|------|
| 12.1.1 | Create `ChatPage` with message list | Messages display |
| 12.1.2 | Create `MessageInput` component | Can type and submit |
| 12.1.3 | Implement streaming response display | Tokens appear as received |
| 12.1.4 | Add markdown rendering for responses | Markdown formatted |
| 12.1.5 | Add code syntax highlighting in messages | Code blocks highlighted |

### Ticket 12.2: Feed View

Real-time agent activity.

| Slice | Description | Test |
|-------|-------------|------|
| 12.2.1 | Create `FeedPage` with scrollable feed | Feed displays |
| 12.2.2 | Create `FeedItem` component with type variants | Different types styled |
| 12.2.3 | Subscribe to WebSocket feed channel | Real-time updates appear |
| 12.2.4 | Add auto-scroll with "new messages" indicator | UX works smoothly |

### Ticket 12.3: Tasks View

Task list and detail.

| Slice | Description | Test |
|-------|-------------|------|
| 12.3.1 | Create `TasksPage` with task list | Tasks display |
| 12.3.2 | Create `TaskCard` component with status badge | Status visible |
| 12.3.3 | Create `TaskDetail` modal/drawer | Details viewable |
| 12.3.4 | Add task actions (cancel, retry) | Actions work |
| 12.3.5 | Subscribe to task updates via WebSocket | Real-time status |

### Ticket 12.4: Agents View

Agent pool status.

| Slice | Description | Test |
|-------|-------------|------|
| 12.4.1 | Create `AgentsPage` with agent cards | Agents display |
| 12.4.2 | Show agent status (idle, working, task info) | Status accurate |
| 12.4.3 | Add agent actions (stop) | Actions work |
| 12.4.4 | Subscribe to agent updates via WebSocket | Real-time status |

### Ticket 12.5: File Browser & Editor

Browse and edit project files.

| Slice | Description | Test |
|-------|-------------|------|
| 12.5.1 | Add file API endpoints to Rust (`GET /api/files`, `GET /api/files/*path`) | Endpoints work |
| 12.5.2 | Create `FileBrowser` component with tree view | Tree displays |
| 12.5.3 | Create `FileViewer` with syntax highlighting (Prism/Shiki) | Code highlighted |
| 12.5.4 | Create `FileEditor` using Monaco or CodeMirror | Can edit files |
| 12.5.5 | Implement save with confirmation | Save works |

### Ticket 12.6: Diff Viewer

View changes made by agents.

| Slice | Description | Test |
|-------|-------------|------|
| 12.6.1 | Add diff API endpoint (`GET /api/git/diff`) | Returns diff |
| 12.6.2 | Create `DiffViewer` component | Diff displays |
| 12.6.3 | Highlight additions/deletions | Colors correct |

---

## NEW: Milestone 13: React Polish

**Goal**: Analytics, settings, mobile responsiveness, production readiness.

**Checkpoint**: Production-ready web application.

### Ticket 13.1: Analytics Dashboard

Usage statistics and cost tracking.

| Slice | Description | Test |
|-------|-------------|------|
| 13.1.1 | Add analytics API endpoints (`GET /api/stats`, `GET /api/costs`) | Endpoints return data |
| 13.1.2 | Create `StatsPage` with key metrics | Metrics display |
| 13.1.3 | Create cost breakdown charts (by tier, model, time) | Charts render |
| 13.1.4 | Add date range selector | Filter works |

### Ticket 13.2: Settings Page

Configuration UI.

| Slice | Description | Test |
|-------|-------------|------|
| 13.2.1 | Create `SettingsPage` with sections | Page renders |
| 13.2.2 | Add API key management (Anthropic, GitHub) | Keys saveable |
| 13.2.3 | Add model configuration per tier | Config saves |
| 13.2.4 | Add theme toggle (light/dark) | Theme switches |

### Ticket 13.3: Mobile Responsiveness

Optimize for mobile devices.

| Slice | Description | Test |
|-------|-------------|------|
| 13.3.1 | Audit all components for mobile | Issues identified |
| 13.3.2 | Fix layout issues on small screens | Layout works on mobile |
| 13.3.3 | Add touch-friendly interactions | Buttons/inputs sized correctly |
| 13.3.4 | Test on iOS Safari, Android Chrome | Works on real devices |

### Ticket 13.4: Production Build

Optimize for deployment.

| Slice | Description | Test |
|-------|-------------|------|
| 13.4.1 | Configure Vite production build | Build succeeds |
| 13.4.2 | Add bundle size optimization (code splitting, tree shaking) | Bundle under 500KB |
| 13.4.3 | Update Dockerfile to build and serve React | Docker image works |
| 13.4.4 | Add health check endpoint verification | Health check passes |

### Ticket 13.5: Documentation Update

Update all docs for new architecture.

| Slice | Description | Test |
|-------|-------------|------|
| 13.5.1 | Update README with new architecture | README accurate |
| 13.5.2 | Update installation docs | Install works |
| 13.5.3 | Update configuration docs | Config docs accurate |
| 13.5.4 | Add API documentation | API documented |

---

## NEW: Milestone 14: Dynamic Agent Selection

**Goal**: Route tasks to the right model based on difficulty. Fix prompt verbosity.

**Checkpoint**: Orchestrator tags slices with difficulty. Complex → Opus, standard/simple → Sonnet.

| Ticket | Title | Slices |
|--------|-------|--------|
| 14.1 | Fix prompt verbosity | 2 (orchestrator + worker prompts) |
| 14.2 | Add difficulty metadata routing | 2 (router rules + orchestrator instruction) |
| 14.3 | Wire model override through agent pool | 2 (config defaults + pool spawn) |

**Note**: Can be done independently of M10-M13. Only touches `src/prompts/`, `src/orchestration/router.rs`, `src/agents/pool.rs`, `src/types/config.rs`, and `templates/orchestrator.md`.

---

## NEW: Milestone 15: Repo Management & Power User Workspace

**Goal**: Transform nexor into a standalone daily-driver workspace with multi-repo management, prompt library, full code editor, report review/submission, and pivotal points tracking.

**Checkpoint**: Can manage multiple repos, save/launch prompts, edit code in Monaco with VS Code shortcuts, review/submit agent reports, and track key decisions on a timeline.

| Ticket | Title | Slices | Priority |
|--------|-------|--------|----------|
| 15.1 | Multi-Repo Backend | 6 | P0 |
| 15.2 | Prompt Library Backend | 5 | P0 |
| 15.3 | Multi-Repo Frontend | 5 | P0 |
| 15.4 | Prompt Library Frontend | 5 | P1 |
| 15.5 | Full Code Editor (Monaco) | 7 | P0 |
| 15.6 | Report Management Backend | 5 | P1 |
| 15.7 | Report Viewer & Submission UI | 6 | P1 |
| 15.8 | Pivotal Points Dashboard | 5 | P2 |
| 15.9 | System Prompt Admin | 6 | P1 |

**Key Features**:
- **Multi-Repo**: Add/clone repos, switch active repo from header, per-repo config, git status indicators
- **Prompt Library**: CRUD with tagging, versioning, categories, `{{variable}}` templates, one-click launch to chat
- **Code Editor**: Monaco with VS Code keybindings (Ctrl+P, Ctrl+Shift+P, Ctrl+D, etc.), tabs, split panes, file tree, minimap, git gutter, global search
- **Reports**: Agent-generated reports with review lifecycle (draft → pending → approved → submitted), inline editing, comment threads
- **Pivotal Points**: Bookmark decisions/milestones/branch points, timeline view, cross-repo, linked to commits/PRs/reports

**Dependencies**: M10 complete (for backend tickets), M11.4 complete (for frontend tickets). Backend tickets (15.1, 15.2, 15.6) can start immediately.

See `decomp/M15/` for detailed ticket breakdowns.

---

## NEW: Milestone 16: SaaS Foundation

**Goal**: Refactor nexor from a local single-user app to a cloud-hosted multi-tenant SaaS platform at nexor.io. Real user accounts, GitHub OAuth, Postgres, cloud repos, collaborative chat with AI, and an onboarding wizard.

**Checkpoint**: User visits nexor.io, signs in with GitHub, imports repos, invites teammates, chats in shared rooms where AI participates, edits code with presence awareness.

| Ticket | Title | Slices | Priority |
|--------|-------|--------|----------|
| 16.1 | Postgres Migration | 6 | P0 |
| 16.2 | User Accounts & Org Model | 7 | P0 |
| 16.3 | GitHub OAuth & Account Connect | 6 | P0 |
| 16.4 | Cloud Repo Management | 7 | P0 |
| 16.5 | Multi-Tenant Data Isolation | 5 | P0 |
| 16.6 | Encrypted Secrets Storage | 4 | P0 |
| 16.7 | Collaborative Chat Rooms | 8 | P1 |
| 16.8 | Presence & User Awareness | 5 | P1 |
| 16.9 | Onboarding Wizard | 6 | P1 |

**Key Features**:
- **Postgres**: Replace SQLite for multi-tenant concurrent access, sqlx with compile-time query checking
- **Users & Orgs**: Real identities, organizations, roles (owner/admin/member/viewer), invitations via email or link
- **GitHub OAuth**: Sign in with GitHub, connect account, browse and import repos
- **Cloud Repos**: Server-side clones per org, sandboxed storage, disk quotas, GitHub sync
- **Tenant Isolation**: org_id on every table, middleware-enforced, integration-tested
- **Encrypted Secrets**: AES-256-GCM encrypted API keys per org, master key on server
- **Collaborative Chat**: Shared rooms where team + AI participate. AI listens, responds to @mentions, posts agent updates, contributes relevant observations
- **Presence**: Who's online, what they're viewing, typing indicators, collaborative cursors in editor
- **Onboarding**: Sign up → GitHub connect → import repos → invite team → start working in under 2 minutes

**Dependencies**: M10 complete. M15 features layer on top of M16 tenant model. Strict internal ordering: 16.1 → 16.2 → 16.3/16.5/16.6 → 16.4 → 16.7/16.8 → 16.9.

See `decomp/M16/` for detailed ticket breakdowns.

---

## Parallelization Notes

### Can be parallelized:

**M10 (Server) + M11 (React Foundation)**:
- 10.1-10.4 (server setup, API, WebSocket) in parallel with 11.1-11.2 (React setup, API client)
- Auth (10.5 + 11.3) should be done together

**M12 (React Features)**:
- All view tickets (12.1-12.6) can be done in parallel once foundation is ready

### Dependencies:

```
M10.1 (Server Setup)
  → M10.2-10.4 (API endpoints)
  → M10.5 (Auth)
  → M10.6 (Static serving)

M11.1 (React Setup)
  → M11.2 (API Client)
  → M11.3 (Auth UI) [needs M10.5]
  → M11.4 (Layout)

M11 complete → M12 (Features)
M12 complete → M13 (Polish)
```

---

## NEW: Milestone 22: Multi-Agent Docker Isolation

**Goal**: Per-agent Docker containers with git worktree isolation, enabling multiple agents to work on different branches simultaneously on the same machine.

**Checkpoint**: Orchestrator assigns 3 tasks on different branches. Each spawns a container with its own worktree. Agents edit files, run tests, commit — all in parallel with zero conflicts.

| Ticket | Title | Slices |
|--------|-------|--------|
| 22.1 | Git Worktree Manager | 5 |
| 22.2 | Agent Worker Dockerfile | 3 |
| 22.3 | Container Lifecycle Management | 5 |
| 22.4 | Agent Worker Mode | 5 |
| 22.5 | Worker Client Protocol | 4 |
| 22.6 | Container Pool Integration | 5 |
| 22.7 | Cleanup, Monitoring & Health | 3 |

**Key ideas**: Git worktrees (not full clones) for instant, disk-efficient branch isolation. Orchestrator stays on host. Workers connect back via WebSocket. Falls back to in-process if Docker unavailable.

**Dependencies**: M7 (Execution Layer), M18 (Typed Subagent System) recommended but not required. Can start 22.1 + 22.2 immediately.

See `decomp/M22/` for detailed ticket breakdowns.

---

## Status

- [x] Milestone 1: Foundation
- [x] Milestone 2: LLM Layer
- [x] Milestone 3: Agent Runtime
- [x] Milestone 4: Prompt Engineering
- [x] Milestone 5: Orchestration Core
- [x] ~~Milestone 6: TUI Basic~~ DEPRECATED
- [x] Milestone 7: Execution Layer
- [x] Milestone 8: GitHub Integration
- [x] Milestone 9: Polish & Production
- [ ] Milestone 10: Server Layer ← **NEXT**
- [ ] Milestone 11: React Foundation
- [ ] Milestone 12: React Features
- [ ] Milestone 13: React Polish
- [ ] Milestone 14: Dynamic Agent Selection
- [ ] Milestone 15: Repo Mgmt & Power User Workspace
- [ ] Milestone 16: SaaS Foundation
- [ ] Milestone 22: Multi-Agent Docker Isolation

---

## Code to Remove

The following will be deleted as part of the architectural pivot:

```
src/tui/           # All TUI code
├── mod.rs
├── app.rs
├── commands.rs
├── errors.rs
├── mode.rs
├── menu/
├── theme.rs
└── views/
    ├── chat.rs
    ├── feed.rs
    ├── logs.rs
    ├── file_viewer.rs
    └── ...
```

Dependencies to remove from Cargo.toml:
- `ratatui`
- `crossterm`
- `tui-textarea` (if added)
- `syntect` (will use JS-based highlighting in React)

---

## New Dependencies

Add to Cargo.toml:
```toml
# HTTP Server
axum = "0.7"
tower-http = { version = "0.5", features = ["cors", "fs", "trace"] }

# WebSocket
tokio-tungstenite = "0.21"

# Auth
argon2 = "0.5"
jsonwebtoken = "9"
```

New directory:
```
ui/                    # React frontend
├── src/
│   ├── App.tsx
│   ├── api/          # API client
│   ├── components/   # Shared components
│   ├── pages/        # Route pages
│   ├── hooks/        # Custom hooks
│   └── store/        # Zustand stores
├── package.json
├── vite.config.ts
└── tailwind.config.js
```

---

*Last updated: 2026-01-27 - Architectural pivot from TUI to Rust + React*
