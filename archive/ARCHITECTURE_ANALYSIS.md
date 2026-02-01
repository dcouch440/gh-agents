# Architecture Analysis

> An honest breakdown of the nexor agent orchestration system.

---

## System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                          YOU (Human)                            │
│                              │                                  │
│                         /main chat                              │
│                              ▼                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                    ORCHESTRATOR                           │  │
│  │            "Arch" - Senior Architect                      │  │
│  │    Plans, decomposes, reviews, makes decisions            │  │
│  └──────────────────────────────────────────────────────────┘  │
│                    │                    │                       │
│           delegates work         delegates work                 │
│                    ▼                    ▼                       │
│  ┌─────────────────────┐   ┌─────────────────────┐             │
│  │      WORKERS        │   │     UTILITIES       │             │
│  │  "Dev" - Developers │   │ "Helper" - Quick    │             │
│  │   Write code        │   │   tasks             │             │
│  │   Implement slices  │   │   Format, lint      │             │
│  └─────────────────────┘   └─────────────────────┘             │
└─────────────────────────────────────────────────────────────────┘
```

---

## The Full Conversation Flow

```
YOU                    ORCHESTRATOR              WORKER              UTILITY
 │                          │                      │                    │
 │  "Add auth"              │                      │                    │
 │─────────────────────────►│                      │                    │
 │                          │                      │                    │
 │  "I'll break into 4      │                      │                    │
 │   slices..."             │                      │                    │
 │◄─────────────────────────│                      │                    │
 │                          │                      │                    │
 │  "Go ahead"              │                      │                    │
 │─────────────────────────►│                      │                    │
 │                          │                      │                    │
 │                          │  AssignTask(slice1)  │                    │
 │                          │─────────────────────►│                    │
 │                          │                      │                    │
 │                          │  TaskStarted         │                    │
 │                          │◄─────────────────────│                    │
 │                          │                      │                    │
 │                          │  ProgressUpdate      │                    │
 │                          │◄─────────────────────│  (shows in /feed)  │
 │                          │                      │                    │
 │                          │  ContextRequest      │                    │
 │                          │◄─────────────────────│                    │
 │                          │                      │                    │
 │                          │  ProvideContext      │                    │
 │                          │─────────────────────►│                    │
 │                          │                      │                    │
 │                          │                      │  AssignTask(fmt)   │
 │                          │                      │───────────────────►│
 │                          │                      │                    │
 │                          │                      │  TaskCompleted     │
 │                          │                      │◄───────────────────│
 │                          │                      │                    │
 │                          │  TaskCompleted       │                    │
 │                          │◄─────────────────────│                    │
 │                          │                      │                    │
 │                          │  (reviews)           │                    │
 │                          │                      │                    │
 │  ApprovalRequest         │                      │                    │
 │◄─────────────────────────│                      │                    │
 │                          │                      │                    │
 │  "Approved"              │                      │                    │
 │─────────────────────────►│                      │                    │
 │                          │                      │                    │
 │  "Auth complete! ✓"      │                      │                    │
 │◄─────────────────────────│                      │                    │
```

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         TUI Layer                                │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐           │
│  │   Feed   │ │  Input   │ │  Status  │ │  Agents  │           │
│  │   View   │ │   Bar    │ │   Bar    │ │   Bar    │           │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘           │
│                        ratatui                                   │
├─────────────────────────────────────────────────────────────────┤
│                     Command Router                               │
│            /main  /logs  /tasks  /agents  /costs                │
├─────────────────────────────────────────────────────────────────┤
│                   Orchestration Core                             │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐           │
│  │ Planner  │ │Scheduler │ │  Router  │ │ Priority │           │
│  │(slicing) │ │ (queue)  │ │ (tier)   │ │ Manager  │           │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘           │
├─────────────────────────────────────────────────────────────────┤
│                     Agent Runtime                                │
│  ┌────────────────┐ ┌────────────────┐ ┌────────────────┐      │
│  │  Orchestrator  │ │    Workers     │ │   Utilities    │      │
│  │   (1 max)      │ │   (N pool)     │ │   (M pool)     │      │
│  └────────────────┘ └────────────────┘ └────────────────┘      │
│              tokio channels (mpsc)                               │
├─────────────────────────────────────────────────────────────────┤
│                   LLM Provider Layer                             │
│  ┌──────────┐ ┌──────────┐                                      │
│  │Anthropic │ │  (more)  │              ← Thin HTTP wrappers   │
│  └──────────┘ └──────────┘                                      │
├─────────────────────────────────────────────────────────────────┤
│                    Execution Layer                               │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐           │
│  │   Git    │ │  Files   │ │  Tests   │ │  Docker  │           │
│  │   Ops    │ │   R/W    │ │  Runner  │ │ Sandbox  │           │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘           │
├─────────────────────────────────────────────────────────────────┤
│                     Persistence                                  │
│                      SQLite                                      │
│            .nexor/state.db (append-only log)                    │
└─────────────────────────────────────────────────────────────────┘
```

---

## Decomposition Flow

```
Epic (vision)
  ↓ break into...
Milestones (usable checkpoints)
  ↓ break into...
Tickets (features/stories)
  ↓ break into...
Slices (smallest deployable units)
```

```
ROADMAP.md (you write high-level milestones)
     │
     ▼
Orchestrator reads it, breaks down into slices
     │
     ▼
Workers execute each slice
     │
     ▼
Orchestrator adds notes, questions, updates back to ROADMAP.md
```

---

## Example: Complete Task Flow

### Phase 1: User Initiates Work

```
┌─────────────────────────────────────────────────────────────────┐
│ /main                                                           │
│─────────────────────────────────────────────────────────────────│
│                                                                 │
│ You: Add user authentication to the API                         │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Phase 2: Orchestrator Decomposes

```
┌─────────────────────────────────────────────────────────────────┐
│ /main                                                           │
│─────────────────────────────────────────────────────────────────│
│                                                                 │
│ Orchestrator: I'll break this into vertical slices. Each        │
│ slice will be independently deployable:                         │
│                                                                 │
│   Slice 1: User model + database migration                      │
│     - Create User struct with email, password_hash              │
│     - Create SQLite migration                                   │
│     - Add insert_user, get_user_by_email queries                │
│     - Tests for all operations                                  │
│                                                                 │
│   Slice 2: Auth endpoints (register, login)                     │
│     - POST /register - create new user                          │
│     - POST /login - validate credentials, return token          │
│     - Tests for success and failure cases                       │
│                                                                 │
│   Slice 3: JWT middleware                                       │
│     - Token generation on login                                 │
│     - Token validation middleware                               │
│     - Protected route example                                   │
│                                                                 │
│   Slice 4: Logout + session management                          │
│     - POST /logout - invalidate token                           │
│     - Token refresh mechanism                                   │
│                                                                 │
│ Should I proceed with this plan?                                │
│                                                                 │
│ You: Yes, go ahead                                              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Phase 3: Worker Executes

```
┌─────────────────────────────────────────────────────────────────┐
│ /feed                                                 w[1/6]    │
│─────────────────────────────────────────────────────────────────│
│                                                                 │
│ ● Worker 1: Starting work on: User Model + Database             │
│                                                                 │
│ ● Worker 1: Analyzing task and building context...              │
│                                                                 │
│ ● Worker 1: Looking at the existing schema in src/db/...        │
│   Found the migration pattern. Creating user table migration.   │
│                                                                 │
│ ● Worker 1: Writing User struct with email, password_hash,      │
│   created_at fields. Adding to src/types/user.rs                │
│                                                                 │
│ ● Worker 1: Implementing insert_user query with password        │
│   hashing using argon2.                                         │
│                                                                 │
│ ★ MILESTONE: User model complete                                │
│                                                                 │
│ ● Worker 1: Writing tests... get_user_by_email, insert with     │
│   duplicate email (should fail), password validation.           │
│                                                                 │
│ ● Worker 1: Running cargo test... All 5 tests passing.          │
│                                                                 │
│ ● Worker 1: Task complete. Submitting for review.               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Phase 4: Worker Needs Context

```
┌─────────────────────────────────────────────────────────────────┐
│ /feed                                                           │
│─────────────────────────────────────────────────────────────────│
│                                                                 │
│ ● Worker 2: Starting work on: JWT Middleware                    │
│                                                                 │
│ ● Worker 2: Implementing token generation... having trouble     │
│   with the jsonwebtoken crate's async support.                  │
│                                                                 │
│ ● Worker 2: Tried sync wrapper, getting lifetime issues.        │
│   Need to understand the existing async patterns better.        │
│                                                                 │
│ ⚠ Worker 2: Requesting context - need to see how other          │
│   middleware is structured in this project.                     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Phase 5: Approval Gate

```
┌─────────────────────────────────────────────────────────────────┐
│ /main                                                           │
│─────────────────────────────────────────────────────────────────│
│                                                                 │
│ Orchestrator: All 4 slices complete. Ready to commit.           │
│                                                                 │
│ Files to be committed:                                          │
│   + src/types/user.rs (new)                                     │
│   + src/db/migrations/002_users.sql (new)                       │
│   + src/db/queries/user.rs (new)                                │
│   M src/routes/mod.rs (modified)                                │
│   + src/routes/auth.rs (new)                                    │
│   + src/middleware/jwt.rs (new)                                 │
│                                                                 │
│ Commit message:                                                 │
│   feat(auth): implement user authentication                     │
│                                                                 │
│ [Approve] [Reject] [View Diff]                                  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Message Types

| Message | Direction | Purpose |
|---------|-----------|---------|
| `AssignTask` | Dispatcher → Agent | Give agent work |
| `TaskStarted` | Agent → Dispatcher | Acknowledge work started |
| `ProgressUpdate` | Agent → Dispatcher | Feed updates |
| `ContextRequest` | Agent → Dispatcher | "I need more info" |
| `ProvideContext` | Dispatcher → Agent | "Here's what you asked for" |
| `ApprovalRequest` | Agent → Dispatcher | "Can I do this?" |
| `GrantApproval` | Dispatcher → Agent | "Yes, proceed" |
| `DenyApproval` | Dispatcher → Agent | "No, don't do that" |
| `TaskCompleted` | Agent → Dispatcher | "Done, here's the result" |
| `TaskFailed` | Agent → Dispatcher | "I couldn't do it" |
| `Shutdown` | Dispatcher → Agent | "Stop gracefully" |

---

## What's Smart

### 1. The Vertical Slice Principle

Most AI coding tools generate horizontal layers (all models, then all routes, then all tests). This system enforces vertical slices — each piece works end-to-end. This means:

- Partial progress is still useful
- Easier to review and test
- Natural git boundaries

### 2. Cost-Aware Tiering

Routing simple tasks to cheap models and complex tasks to expensive ones is economically sound. Most systems use one model for everything and burn money on formatting tasks.

| Tier | Role | Cost | Use Case |
|------|------|------|----------|
| Orchestrator | Planning, review | High | Architecture decisions |
| Worker | Implementation | Medium | Writing code |
| Utility | Quick tasks | Low | Formatting, linting |

### 3. Single Point of Contact

You only talk to the Orchestrator. This prevents the chaos of managing multiple agents directly. It mirrors how real teams work — you talk to a tech lead, not every developer.

```
You ←→ Orchestrator ←→ Workers/Utilities
```

### 4. The Escalation Path

`Utility → Worker → Orchestrator → Human` is a natural failure recovery mechanism. Agents know when they're stuck and can ask for help rather than spinning.

### 5. Structured Prompts with Schemas

Forcing JSON output schemas means you can actually parse and act on agent responses programmatically, not just hope the LLM outputs something usable.

```json
{
  "phase": "planning | implementing | complete",
  "code_changes": [...],
  "status": "needs_context | in_progress | ready_for_review"
}
```

---

## What's Challenging

### 1. Context Window Management

As tasks get complex, the context (files, history, conventions) can exceed model limits. You'll need smart summarization and selective loading.

### 2. Agent Coordination Overhead

Message passing between agents adds latency. A simple task might bounce between 3 agents before completing.

### 3. State Recovery

If the process crashes mid-task, reconstructing agent state from the SQLite log is non-trivial.

### 4. Prompt Brittleness

The system relies heavily on LLMs following instructions precisely. One model update could break assumptions.

---

## The Real Innovation

The combination of:

- **Document-driven planning** (PRD → ROADMAP → decomp files)
- **Role-based agents** with different personalities/capabilities
- **Append-only event log** for full traceability
- **TUI for visibility** into what's actually happening

This is closer to how a real software team operates than most "AI coding assistant" approaches that treat the LLM as a magic autocomplete.

---

## Comparison to Other Approaches

| Approach | How It Works | Limitation |
|----------|--------------|------------|
| Copilot | Single model, autocomplete | No planning, no memory |
| ChatGPT | Single conversation | Context limit, no execution |
| AutoGPT | Autonomous loop | Runaway costs, no control |
| **nexor** | Tiered agents, human oversight | Coordination complexity |

---

## Key Design Principles

1. **You only talk to Orchestrator** — Single point of contact via `/main`
2. **Vertical slices** — Each piece is independently deployable
3. **Cost-aware routing** — Simple tasks → cheap agents, complex → expensive
4. **Escalation path** — Utility → Worker → Orchestrator → Human
5. **Approval gates** — Human confirms dangerous operations
6. **Natural language feed** — `/feed` shows what's happening in plain English

---

## Honest Take

It's not revolutionary in any single dimension — tiered agents, task queues, and approval gates all exist elsewhere. What's clever is the *integration*: the way the pieces fit together into a coherent workflow that mirrors actual software development practices.

Whether it works in practice depends on execution. The architecture is sound. The hard part is prompt engineering, context management, and handling the inevitable edge cases where agents get confused.

---

## Key Success Factors

1. **Prompt Quality** — The system is only as good as its prompts
2. **Context Selection** — Knowing what files to include (and exclude)
3. **Failure Recovery** — Graceful handling when agents get stuck
4. **User Trust** — The TUI must make the system's behavior transparent

---

*Generated during architecture review discussion.*
