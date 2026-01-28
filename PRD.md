# nexor Product Requirements Document

> AI Agent Orchestration TUI for GitHub Workflows

---

## Table of Contents

1. [Overview](#overview)
2. [Technical Decisions](#technical-decisions)
3. [Interface & UX](#interface--ux)
4. [Agent Architecture](#agent-architecture)
5. [Data Models](#data-models)
6. [System Architecture](#system-architecture)
7. [Implementation Phases](#implementation-phases)
8. [UI Design](#ui-design)
9. [Project Hierarchy & ROADMAP](#project-hierarchy--roadmap)
10. [Decomposition Guide](#decomposition-guide)
11. [Agent Personas](#agent-personas)
12. [Configuration System](#configuration-system)
13. [Error Handling](#error-handling)
14. [Communication Model](#communication-model)
15. [File Structure](#file-structure)
16. [Future Considerations](#future-considerations)

---

## Overview

**nexor** is a Rust-based terminal application that orchestrates multiple AI agents to handle software engineering tasks. It provides a rich TUI interface for managing GitHub workflows, breaking down tickets into vertical slices, and coordinating work across agent tiers.

### Core Value Proposition

- **Single point of contact**: Chat with one orchestrator who manages everything
- **Intelligent decomposition**: Automatically break tickets into deployable vertical slices
- **Cost-aware execution**: Route tasks to appropriate AI tiers (expensive → cheap)
- **Full visibility**: Real-time feed of agent activity in natural language
- **Safety first**: Configurable approval gates and sandboxed execution

---

## Technical Decisions

### Language & Stack

| Component | Choice | Rationale |
|-----------|--------|-----------|
| **Core Language** | Rust | Performance, safety, excellent async support |
| **TUI Framework** | ratatui | Modern, well-maintained, flexible |
| **Database** | SQLite | Embedded, zero-config, stored in `.nexor/state.db` |
| **Config Format** | TOML | Rust-native, readable, well-supported |
| **Async Runtime** | tokio | Industry standard for Rust async |
| **LLM Integration** | Pure Rust HTTP | No SDK dependencies, full control |

### Deployment

- **Primary**: Local execution with Docker sandboxing
- **Headless Mode**: Supported for CI/CD and automation pipelines
- **State**: Project-local in `.nexor/` directory

---

## Interface & UX

### Design Principles

- **Rich TUI** with real-time updates (like Claude Code)
- **Fixed layout** for predictable, consistent experience
- **Standard keybindings**: Arrow keys, Tab, Enter, Ctrl+C (not Vim-style)
- **Minimal chrome**: Content-first design

### Slash Commands

| Command | Description |
|---------|-------------|
| `/home` | Return to home screen |
| `/main` | Chat with orchestrator (your single point of contact) |
| `/feed` | View agent activity (read-only, natural language reports) |
| `/logs` | View detailed technical logs |
| `/tasks` | View/manage task queue |
| `/agents` | View agent status and pool |
| `/costs` | View cost breakdown |

---

## Agent Architecture

### Agent Tiers

| Tier | Role | Default Model | Responsibilities |
|------|------|---------------|------------------|
| **Orchestrator** | Expensive | User configurable | Planning, review, decisions, `/main` chat |
| **Worker** | Mid-tier | User configurable | Code implementation, features, bug fixes |
| **Utility** | Cheap | User configurable | Formatting, linting, boilerplate, docs |

### Concurrency Model

- **In-process**: All agents run in one process, communicate via tokio channels
- **Configurable pools**: e.g., 1 orchestrator, 3 workers, 5 utilities
- **Code context**: Hybrid scoping - task-focused with ability to request more

### Communication

- **Reports**: Natural language style ("I'm looking at the auth module...")
- **Verbosity**: Configurable levels (quiet/normal/verbose)
- **`/main` Chat**: Both task-focused and conversational

---

## Data Models

### Task Lifecycle

```rust
enum TaskStatus {
    Pending,
    InProgress,
    Review,
    Completed,
    Failed,
}

enum Priority {
    Low,
    Normal,
    High,
    Urgent,  // Can preempt other work
}

enum AgentTier {
    Orchestrator,
    Worker,
    Utility,
}
```

### Core Structures

```rust
/// A vertical slice of work
struct VerticalSlice {
    id: Uuid,
    ticket_id: Uuid,
    title: String,
    description: String,
    tasks: Vec<Task>,
    status: TaskStatus,
    created_at: DateTime<Utc>,
}

/// Individual task assigned to an agent
struct Task {
    id: Uuid,
    slice_id: Option<Uuid>,
    title: String,
    description: String,
    assigned_tier: AgentTier,
    assigned_agent: Option<AgentId>,
    status: TaskStatus,
    priority: Priority,
    context_files: Vec<PathBuf>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Task state change log entry (append-only)
struct TaskEvent {
    id: Uuid,
    task_id: Uuid,
    event_type: TaskEventType,
    agent_id: Option<AgentId>,
    details: String,
    timestamp: DateTime<Utc>,
}

enum TaskEventType {
    Created,
    Assigned,
    Started,
    ProgressUpdate,
    ContextRequested,
    SubmittedForReview,
    ReviewFeedback,
    Completed,
    Failed,
    Cancelled,
    Escalated,
}
```

### Agent Types

```rust
struct Agent {
    id: AgentId,
    tier: AgentTier,
    persona: AgentPersona,
    model_config: ModelConfig,
    current_task: Option<Uuid>,
    status: AgentStatus,
}

enum AgentStatus {
    Idle,
    Working,
    WaitingForContext,
    WaitingForApproval,
}

struct AgentPersona {
    name: String,
    system_prompt: String,
    style: CommunicationStyle,
}

enum CommunicationStyle {
    Formal,
    Casual,
    Technical,
    Friendly,
}

struct ModelConfig {
    provider: LLMProvider,
    model_id: String,
    max_tokens: u32,
    temperature: f32,
}

enum LLMProvider {
    Anthropic,
}
```

### Messages & Feed

```rust
struct AgentMessage {
    id: Uuid,
    from: AgentId,
    to: AgentId,
    message_type: MessageType,
    content: String,
    context: Option<TaskContext>,
    timestamp: DateTime<Utc>,
}

enum MessageType {
    TaskAssignment,
    TaskResult,
    ReviewRequest,
    ReviewFeedback,
    ContextRequest,
    ContextResponse,
    Escalation,
    StatusUpdate,
}

struct FeedItem {
    id: Uuid,
    agent_id: AgentId,
    content: String,
    item_type: FeedItemType,
    verbosity_level: VerbosityLevel,
    timestamp: DateTime<Utc>,
}

enum FeedItemType {
    AgentReport,
    TaskStarted,
    TaskCompleted,
    Error,
    UserMessage,
    SystemNotice,
}

enum VerbosityLevel {
    Quiet,
    Normal,
    Verbose,
}
```

### GitHub Integration

```rust
struct Ticket {
    id: Uuid,
    source: TicketSource,
    title: String,
    description: String,
    labels: Vec<String>,
    slices: Vec<VerticalSlice>,
    status: TicketStatus,
    created_at: DateTime<Utc>,
}

enum TicketSource {
    GitHub { owner: String, repo: String, issue_number: u32 },
    Manual,
}

enum TicketStatus {
    New,
    Planning,
    InProgress,
    Review,
    Completed,
    Closed,
}
```

### Cost Tracking

```rust
struct CostRecord {
    id: Uuid,
    task_id: Option<Uuid>,
    agent_id: AgentId,
    agent_tier: AgentTier,
    model_id: String,
    input_tokens: u32,
    output_tokens: u32,
    cost_usd: f64,
    timestamp: DateTime<Utc>,
}

struct CostSummary {
    session_total: f64,
    by_tier: HashMap<AgentTier, f64>,
    by_task: HashMap<Uuid, f64>,
    by_model: HashMap<String, f64>,
}
```

### Configuration

```rust
struct GlobalConfig {
    default_models: TierModels,
    api_keys: ApiKeys,
    verbosity: VerbosityLevel,
}

struct ProjectConfig {
    models: Option<TierModels>,
    autonomy: AutonomyLevel,
    approval_gates: ApprovalGates,
    git_strategy: GitStrategy,
    sandbox_mode: SandboxMode,
    agent_pool: AgentPoolConfig,
}

struct TierModels {
    orchestrator: ModelConfig,
    worker: ModelConfig,
    utility: ModelConfig,
}

enum AutonomyLevel {
    FullAuto,
    ApprovalGates,
    Supervised,
}

struct ApprovalGates {
    before_commit: bool,
    before_pr: bool,
    before_merge: bool,
}

enum GitStrategy {
    BranchPerSlice,
    BranchPerTicket,
}

enum SandboxMode {
    Docker,
    LocalRestricted,
    None,
}

struct AgentPoolConfig {
    max_orchestrators: u8,
    max_workers: u8,
    max_utilities: u8,
}
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
│            .nexor/state.db (append-only log)                │
└─────────────────────────────────────────────────────────────────┘
```

---

## Implementation Phases

### Phase 1: Foundation

**Goal**: Project setup, core types, basic infrastructure

| Task | Description |
|------|-------------|
| Initialize Cargo workspace | Set up project structure with dependencies |
| Define core data types | All structs and enums from data models |
| Set up SQLite | Schema, migrations, connection pool |
| Implement config loading | Global + project config with layered merge |
| Create logging infrastructure | tracing setup with configurable levels |
| Set up tokio async runtime | Basic app skeleton |

### Phase 2: LLM Layer

**Goal**: Working LLM API integration

| Task | Description |
|------|-------------|
| Implement Anthropic HTTP client | Messages API with streaming |
| Create provider abstraction trait | Unified interface for all providers |
| Add streaming response support | Real-time token output |
| Implement cost tracking | Token counting and cost calculation |
| Add retry logic with backoff | Exponential backoff for rate limits |

### Phase 3: Agent Runtime

**Goal**: Agents that can execute tasks

| Task | Description |
|------|-------------|
| Implement Agent struct and lifecycle | Creation, state management, cleanup |
| Create agent pool manager | Spawn/despawn agents by tier |
| Build message passing | tokio mpsc channels for inter-agent comms |
| Implement persona/system prompts | Configurable agent personalities |
| Add task assignment logic | Match tasks to available agents |
| Implement escalation flow | cheap → mid → expensive → human |

### Phase 4: Orchestration Core

**Goal**: Ticket decomposition and task routing

| Task | Description |
|------|-------------|
| Implement Planner | ticket → vertical slices decomposition |
| Build task queue with priority | Priority-ordered work queue |
| Create Router | task → appropriate tier routing |
| Add dependency tracking | Task dependencies and blocking |
| Implement interrupt/preemption | Urgent tasks preempt lower priority |

### Phase 5: TUI - Basic

**Goal**: Functional terminal interface

| Task | Description |
|------|-------------|
| Set up ratatui with crossterm | Terminal initialization and cleanup |
| Implement fixed layout panels | Consistent panel arrangement |
| Create main feed view | Real-time agent activity |
| Add input bar | Command input with history |
| Implement slash command router | Parse and route commands |
| Add `/main` chat view | Orchestrator conversation |
| Add `/logs` view | Detailed technical logs |

### Phase 6: Execution Layer

**Goal**: Agents can modify code safely

| Task | Description |
|------|-------------|
| Implement file read/write | Scoped file access per task |
| Add Git operations | Branch, commit, diff, push |
| Implement test runner | Run project tests, capture output |
| Add Docker sandbox support | Isolated execution environment |
| Create approval gate system | User confirmation for dangerous ops |

### Phase 7: GitHub Integration

**Goal**: Pull tickets from GitHub, create PRs

| Task | Description |
|------|-------------|
| Implement GitHub API client | REST API with auth |
| Add issue fetching and sync | Pull issues as tickets |
| Implement PR creation | Create PRs from completed slices |
| Add comment/update support | Update issues with progress |

### Phase 8: Polish & Production

**Goal**: Production-ready quality

| Task | Description |
|------|-------------|
| Implement remaining views | `/tasks`, `/agents`, `/costs` |
| Add startup screen design | Logo, status indicators |
| Comprehensive error handling | Graceful failures, recovery |
| Add headless mode | Non-interactive operation |
| Docker packaging | Containerized deployment |
| Documentation | README, user guide, API docs |

---

## UI Design

### Home Screen (Startup/Idle)

```
┌─────────────────────────────────────────────────────────────┐
│ w[0/6] o[0/2]                                               │
│                                                             │
│                                                             │
│            ██████╗ ██╗  ██╗     █████╗                      │
│           ██╔════╝ ██║  ██║    ██╔══██╗                     │
│           ██║  ███╗███████║    ███████║                     │
│           ██║   ██║██╔══██║    ██╔══██║                     │
│           ╚██████╔╝██║  ██║    ██║  ██║                     │
│            ╚═════╝ ╚═╝  ╚═╝    ╚═╝  ╚═╝                     │
│                                                             │
│                                                             │
│                                                             │
│  > _                                                        │
│  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │
│  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │
└─────────────────────────────────────────────────────────────┘
```

- `w[0/6]` = workers active/total
- `o[0/2]` = orchestrators active/total
- Clean aesthetic with prominent branding
- System messages slide in at bottom

### Chat View (/main)

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│ You: Add user authentication to the API                     │
│                                                             │
│ Orchestrator: I'll break this into vertical slices:         │
│   1. User model + database migration                        │
│   2. Auth endpoints (register, login, logout)               │
│   3. JWT middleware                                         │
│   4. Tests for auth flow                                    │
│                                                             │
│ Should I proceed with this plan?                            │
│                                                             │
│                                                             │
│                                                             │
│  > _                                                        │
└─────────────────────────────────────────────────────────────┘
```

- Minimal chrome, content-focused
- Transitions from home when typing begins
- Use `/home` to return

### Feed View (/feed)

```
┌─────────────────────────────────────────────────────────────┐
│ w[2/6] o[1/2]                                    /feed      │
│─────────────────────────────────────────────────────────────│
│ ● Worker 1: Looking at the existing user model...           │
│                                                             │
│ ● Worker 1: Found the schema in src/models/. Adding         │
│   password_hash and email fields to the User struct.        │
│                                                             │
│ ★ MILESTONE: User model updated                             │
│                                                             │
│ ● Worker 2: Now implementing the auth endpoints...          │
│                                                             │
│ ● Utility 1: Formatting src/models/user.rs                  │
│                                                             │
│  > _                                                        │
└─────────────────────────────────────────────────────────────┘
```

- Natural language agent activity
- Milestone markers for completions
- Conversational, spoken-word style

---

## Project Hierarchy & ROADMAP

### The Flow

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

### ROADMAP.md as Living Document

- **You** write the high-level milestones and direction
- **Orchestrator** reads it to understand context
- **Orchestrator** adds notes, questions, decisions as it works
- **Rolling process** - gets updated continuously
- **Accuracy through granularity** - high-level in ROADMAP, details in linked files

### File Structure

```
project/
├── ROADMAP.md                      ← High-level milestones (always loaded)
└── .nexor/
    ├── config.toml                 ← Project config
    ├── state.db                    ← SQLite state
    └── slices/                     ← Detailed slice breakdowns
        ├── sqlite-setup.md
        ├── config-system.md
        └── ...
```

### Context Management

| What | Where | When Loaded |
|------|-------|-------------|
| Big picture | ROADMAP.md | Always (small file) |
| Slice details | .nexor/slices/*.md | On-demand |
| Task state | SQLite | Queried as needed |
| Old notes | Archived/deleted | After resolved |

---

## Decomposition Guide

### The Decomposition Flow

```
Epic (vision)
  ↓ break into...
Milestones (usable checkpoints)
  ↓ break into...
Tickets (features/stories)
  ↓ break into...
Slices (smallest deployable units)
```

### Epic → Milestones

**Question**: "What are the major checkpoints where I have something usable?"

**Technique**:
1. List everything the final product needs
2. Group into chunks that each deliver standalone value
3. Order by dependencies
4. Each milestone should be demo-able

**Rule of thumb**: 5-10 milestones for a medium project.

### Milestone → Tickets

**Question**: "What distinct features or components make up this milestone?"

**Technique**:
1. List all pieces needed for this milestone
2. Each ticket = one logical feature/component
3. Tickets can often be worked in parallel
4. Done when feature works end-to-end

**Rule of thumb**: 3-8 tickets per milestone.

### Ticket → Slices (Vertical Slicing)

**Question**: "What's the smallest piece I can complete and deploy independently?"

**The Vertical Slice Principle**: Each slice touches ALL layers needed to work.

**BAD (horizontal)**:
```
Slice 1: Write all database code
Slice 2: Write all API code
Slice 3: Write all tests
→ Nothing works until ALL slices are done
```

**GOOD (vertical)**:
```
Slice 1: Users table + insert user + test insert
Slice 2: Sessions table + create session + test session
Slice 3: Query user by email + test query
→ Each slice works independently
```

**Rule of thumb**: 2-5 slices per ticket. Each takes hours, not days.

### Slice Checklist

Before a slice is "ready to work":

- [ ] **Clear scope** - I know exactly what code to write
- [ ] **Testable** - I know how to verify it works
- [ ] **Independent** - Doesn't require other unfinished slices
- [ ] **Small** - Can complete in one focused session
- [ ] **Valuable** - Adds real functionality

### Common Patterns

**API Endpoint Slice**:
1. Request/response types
2. Route + handler (mock data)
3. Connect to real data
4. Add validation
5. Add tests

**Database Feature Slice**:
1. Schema + migration
2. Insert operation + test
3. Query operation + test
4. Update/delete + tests

**UI Component Slice**:
1. Static render (hardcoded)
2. Wire to real data
3. Add interactivity
4. Handle edge cases

### Quick Reference

| Level | Size | Duration | Deliverable |
|-------|------|----------|-------------|
| Epic | Whole project | Months | The vision |
| Milestone | Major chunk | 1-4 weeks | Something usable |
| Ticket | One feature | 1-5 days | Feature complete |
| Slice | Smallest unit | 1-4 hours | One PR, tests pass |

---

## Agent Personas

### Configuration Approach

- **Presets** for easy start (pick a style)
- **Full customization** for power users
- Global defaults + project overrides

### Default Personas

**Orchestrator (Expensive AI)**
```toml
[personas.orchestrator]
name = "Arch"
style = "collaborative"
system_prompt = """
You are a senior software architect working with a team of AI agents.
Your role is to:
- Break down complex problems into vertical slices
- Each slice should be independently deployable and valuable
- Explain your reasoning and ask clarifying questions
- Review work from other agents before approval
- Make architectural decisions and resolve conflicts

Communicate in a collaborative, thoughtful manner. Brainstorm with
the user when needed. Always consider the bigger picture.
"""
```

**Worker (Mid-tier AI)**
```toml
[personas.worker]
name = "Dev"
style = "focused"
system_prompt = """
You are a focused software developer. Your role is to:
- Implement code changes as specified in your task
- Write clean, well-tested code
- Report progress naturally as you work
- Ask for more context only when truly needed
- Submit work for review when complete

Stay heads-down on your task. Keep updates brief but informative.
"""
```

**Utility (Cheap AI)**
```toml
[personas.utility]
name = "Helper"
style = "quick"
system_prompt = """
You handle quick, well-defined tasks:
- Format code according to project style
- Run linters and fix issues
- Generate boilerplate from templates
- Update documentation

Be brief. Report only completions and errors.
Example: "Formatted 3 files" or "Lint error in src/main.rs:42"
"""
```

---

## Configuration System

### Layered Config

```
~/.config/nexor/config.toml     ← Global defaults
     ↓ merged with
.nexor/config.toml              ← Project overrides
     ↓ merged with
CLI flags / env vars                ← Runtime overrides
```

### Example Global Config

```toml
# ~/.config/nexor/config.toml

[models]
orchestrator = { provider = "anthropic", model = "claude-sonnet-4-20250514", max_tokens = 8192 }
worker = { provider = "anthropic", model = "claude-sonnet-4-20250514", max_tokens = 4096 }
utility = { provider = "anthropic", model = "claude-haiku", max_tokens = 2048 }

[pool]
max_orchestrators = 2
max_workers = 6
max_utilities = 4

[ui]
verbosity = "normal"  # quiet, normal, verbose
```

### Example Project Config

```toml
# .nexor/config.toml

[models]
worker = { provider = "anthropic", model = "claude-sonnet-4-20250514", max_tokens = 4096 }

[autonomy]
level = "approval_gates"  # full_auto, approval_gates, supervised

[approval_gates]
before_commit = false
before_pr = true
before_merge = true

[git]
strategy = "branch_per_slice"  # or "branch_per_ticket"

[sandbox]
mode = "docker"  # docker, local_restricted, none

[personas.orchestrator]
system_prompt = """
You are working on a Rust TUI application. Prioritize:
- Memory safety and proper error handling
- Clean async patterns with tokio
- Modular architecture
"""
```

### Environment Variables

```bash
# API keys (never in config files)
export ANTHROPIC_API_KEY="sk-ant-..."
export GITHUB_TOKEN="ghp_..."
```

---

## Error Handling

### Error Display

- **Minimized section** in TUI (doesn't clutter feed)
- **Expandable** - short message default, expand for details
- Errors in `/feed` appear inline as red messages

### Agent Failure Recovery

1. **Retry same tier first** - Try again before escalating
2. If retry fails: Utility → Worker → Orchestrator → Human
3. Configurable retry count per task type

### Error Types

| Error | Display | Action |
|-------|---------|--------|
| API rate limit | Status bar | Auto-retry with backoff |
| API auth failure | Modal | Prompt for new key |
| Agent confused | Feed inline | Retry or escalate |
| Code won't compile | Feed inline | Agent fixes or escalates |
| Tests failing | Feed inline | Agent investigates |
| Network down | Modal | Pause all work, wait |

---

## Communication Model

### Simple Hierarchy

```
You ←→ Orchestrator ←→ Workers/Utilities
```

- **You only talk to the Orchestrator** via `/main`
- **Orchestrator delegates** to workers and utilities
- **Orchestrator reports back** with updates and questions
- **`/feed`** shows real-time activity (read-only)

If you need info from a worker, ask the orchestrator:
> "What approach is the worker taking on auth?"

The orchestrator handles internal coordination.

---

## File Structure

```
nexor/
├── Cargo.toml                    # Workspace root
├── Cargo.lock
├── .nexor/
│   └── config.toml.example
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── config/
│   │   ├── mod.rs
│   │   ├── global.rs
│   │   └── project.rs
│   ├── types/
│   │   ├── mod.rs
│   │   ├── task.rs
│   │   ├── agent.rs
│   │   ├── message.rs
│   │   ├── ticket.rs
│   │   └── cost.rs
│   ├── db/
│   │   ├── mod.rs
│   │   ├── migrations.rs
│   │   └── queries.rs
│   ├── llm/
│   │   ├── mod.rs
│   │   ├── provider.rs
│   │   └── anthropic.rs
│   ├── agents/
│   │   ├── mod.rs
│   │   ├── runtime.rs
│   │   ├── pool.rs
│   │   ├── orchestrator.rs
│   │   ├── worker.rs
│   │   └── utility.rs
│   ├── orchestration/
│   │   ├── mod.rs
│   │   ├── planner.rs
│   │   ├── scheduler.rs
│   │   └── router.rs
│   ├── execution/
│   │   ├── mod.rs
│   │   ├── files.rs
│   │   ├── git.rs
│   │   ├── tests.rs
│   │   └── sandbox.rs
│   ├── github/
│   │   ├── mod.rs
│   │   └── client.rs
│   └── tui/
│       ├── mod.rs
│       ├── app.rs
│       ├── layout.rs
│       ├── views/
│       │   ├── mod.rs
│       │   ├── feed.rs
│       │   ├── main_chat.rs
│       │   ├── logs.rs
│       │   ├── tasks.rs
│       │   ├── agents.rs
│       │   └── costs.rs
│       └── input.rs
├── tests/
│   └── ...
└── docker/
    └── Dockerfile
```

---

## In-TUI File Editor

**Vision**: Users can view and edit files directly within the TUI while agents are working, providing a seamless development experience without leaving nexor.

### Core Features

| Feature | Description |
|---------|-------------|
| **File Viewer** | Read-only view with syntax highlighting for any project file |
| **File Editor** | Full in-app editing with nano-style keybindings (Ctrl+X to exit) |
| **Agent File Access** | Open files that agents are currently working on |
| **Save & Commit** | Save changes and optionally commit to current branch |
| **Diff View** | See before/after for agent modifications |

### User Flow

```
┌─────────────────────────────────────────────────────────┐
│  Agent working on: src/auth/login.rs                    │
│  Status: in_progress                                    │
│                                                         │
│  [View File]  [Edit File]  [View Diff]                  │
└─────────────────────────────────────────────────────────┘
                    │
                    ▼ (user presses Edit or /edit path)
┌─────────────────────────────────────────────────────────┐
│  src/auth/login.rs                        [Ctrl+X Exit] │
├─────────────────────────────────────────────────────────┤
│  1 │ use crate::auth::Session;                          │
│  2 │ use crate::db::UserRepo;                           │
│  3 │                                                    │
│  4 │ pub async fn login(creds: Credentials) -> Result { │
│  5 │     let user = UserRepo::find_by_email(&creds.em   │
│  6 │ ...                                                │
├─────────────────────────────────────────────────────────┤
│  Ln 4, Col 12 | Modified                                │
└─────────────────────────────────────────────────────────┘
                    │
                    ▼ (Ctrl+X to exit)
┌─────────────────────────────────────────────────────────┐
│  Save changes to src/auth/login.rs?                     │
│                                                         │
│  [Save]  [Save & Commit]  [Discard]  [Cancel]           │
└─────────────────────────────────────────────────────────┘
```

### Slash Commands

| Command | Description |
|---------|-------------|
| `/view <path>` | Open file in read-only viewer |
| `/edit <path>` | Open file in editor |
| `/diff <path>` | Show diff for file (if modified by agent) |
| `/files` | Browse project files with tree view |

### Technical Components

| Component | Crate/Approach |
|-----------|----------------|
| Text editor widget | `tui-textarea` or `edtui` |
| Syntax highlighting | `syntect` |
| File tree browser | Custom widget with `tui-tree-widget` |
| Git integration | `git2` crate (from M7) |

### Keybindings (nano-style)

| Key | Action |
|-----|--------|
| `Ctrl+X` | Exit (prompt to save if modified) |
| `Ctrl+O` | Save file |
| `Ctrl+G` | Go to line |
| `Ctrl+W` | Search |
| `Ctrl+K` | Cut line |
| `Ctrl+U` | Paste |
| `Arrow keys` | Navigate |
| `Page Up/Down` | Scroll |

---

## Future Considerations

Features to consider after v1:

- **Team collaboration** - Multiple users, shared agent pools, collaborative roadmaps
- **Pause/resume agents** - Save and restore agent state mid-task
- **Multi-repo support** - Orchestrate across multiple repositories
- **Learning system** - Improve prompts based on success/failure patterns
- **Plugin architecture** - Extensible integrations beyond GitHub

---

## Appendix: Ticket & Task Management

### Ticket Input
- GitHub Issues (automatic sync)
- Manual entry via `/main` chat

### Vertical Slicing
- Templates for common patterns
- Orchestrator for novel work

### Task States
```
pending → in_progress → review → completed
                              ↘ failed
```

### Task History
- Append-only log (full history)
- Replayable for debugging

### Interrupts
- Priority system
- Urgent tasks can preempt lower priority work

### Autonomy & Safety

| Level | Description |
|-------|-------------|
| FullAuto | No human approval needed |
| ApprovalGates | Approval at configured points |
| Supervised | Human reviews each step |

### Dangerous Actions
- Blocked automatically
- User must explicitly confirm

### Code Validation
- Generate tests for new code
- Run existing test suite before commit
