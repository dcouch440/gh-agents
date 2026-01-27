# gh-agents ROADMAP

> Living document for AI agent orchestration. Orchestrator reads this for context.

---

## Epic: gh-agents v1.0

Build a Rust TUI that orchestrates AI agents for GitHub workflows.

---

## Milestone 1: Foundation

**Goal**: Project compiles, core types exist, config loads, database works.

**Checkpoint**: Can run `cargo run`, load config, connect to SQLite.

### Ticket 1.1: Project Scaffolding

Set up Cargo workspace with all dependencies.

| Slice | Description | Test |
|-------|-------------|------|
| 1.1.1 | Create `Cargo.toml` with workspace config and core dependencies (tokio, ratatui, sqlx, serde, toml, uuid, chrono, tracing) | `cargo check` passes |
| 1.1.2 | Create directory structure (`src/`, `src/types/`, `src/config/`, `src/db/`, etc.) with `mod.rs` files | All modules resolve |
| 1.1.3 | Set up `main.rs` with tokio runtime and basic error handling | `cargo run` starts and exits cleanly |

### Ticket 1.2: Core Type Definitions

Define all structs and enums from data models.

| Slice | Description | Test |
|-------|-------------|------|
| 1.2.1 | Task types: `TaskStatus`, `Priority`, `Task`, `VerticalSlice`, `TaskEvent`, `TaskEventType` | Types compile, can instantiate |
| 1.2.2 | Agent types: `AgentTier`, `Agent`, `AgentStatus`, `AgentPersona`, `CommunicationStyle`, `ModelConfig`, `LLMProvider` | Types compile, can instantiate |
| 1.2.3 | Message types: `AgentMessage`, `MessageType`, `FeedItem`, `FeedItemType`, `VerbosityLevel` | Types compile, can instantiate |
| 1.2.4 | GitHub types: `Ticket`, `TicketSource`, `TicketStatus` | Types compile, can instantiate |
| 1.2.5 | Cost types: `CostRecord`, `CostSummary` | Types compile, can instantiate |
| 1.2.6 | Config types: `GlobalConfig`, `ProjectConfig`, `TierModels`, `AutonomyLevel`, `ApprovalGates`, `GitStrategy`, `SandboxMode`, `AgentPoolConfig` | Types compile, can instantiate |

### Ticket 1.3: Configuration System

Load and merge global + project configs.

| Slice | Description | Test |
|-------|-------------|------|
| 1.3.1 | Implement `config/global.rs`: load from `~/.config/gh-agents/config.toml`, return defaults if missing | Unit test: loads file or returns defaults |
| 1.3.2 | Implement `config/project.rs`: load from `.gh-agents/config.toml`, return None if missing | Unit test: loads file or returns None |
| 1.3.3 | Implement config merge logic: global ← project overrides | Unit test: project values override global |
| 1.3.4 | Add config validation (required fields, valid enum values) | Unit test: invalid config returns error |

### Ticket 1.4: Database Setup

SQLite with migrations and connection pooling.

| Slice | Description | Test |
|-------|-------------|------|
| 1.4.1 | Set up sqlx with SQLite, create `.gh-agents/state.db` on startup | `cargo run` creates database file |
| 1.4.2 | Create migration for `tasks` table (id, slice_id, title, description, assigned_tier, status, priority, created_at, updated_at) | Migration runs, table exists |
| 1.4.3 | Create migration for `task_events` table (append-only log) | Migration runs, table exists |
| 1.4.4 | Create migration for `agents` table | Migration runs, table exists |
| 1.4.5 | Create migration for `messages` table | Migration runs, table exists |
| 1.4.6 | Create migration for `cost_records` table | Migration runs, table exists |
| 1.4.7 | Create migration for `tickets` and `vertical_slices` tables | Migration runs, tables exist |
| 1.4.8 | Implement connection pool and basic query helpers in `db/mod.rs` | Can insert and query a task |

### Ticket 1.5: Logging Infrastructure

Set up tracing with configurable levels.

| Slice | Description | Test |
|-------|-------------|------|
| 1.5.1 | Initialize tracing-subscriber with env filter | `RUST_LOG=debug cargo run` shows debug logs |
| 1.5.2 | Add file appender for `.gh-agents/logs/` | Logs written to file |
| 1.5.3 | Create log macros/helpers for consistent formatting | Logs show module, level, message |

---

## Milestone 2: LLM Layer

**Goal**: Can send prompts to Anthropic/OpenAI and get streaming responses.

**Checkpoint**: Can chat with Claude via CLI, see tokens stream in, see cost tracked.

### Ticket 2.1: Provider Abstraction

Create unified trait for LLM providers.

| Slice | Description | Test |
|-------|-------------|------|
| 2.1.1 | Define `LLMProvider` trait with `send_message()` async method | Trait compiles |
| 2.1.2 | Define `LLMRequest` and `LLMResponse` types | Types compile |
| 2.1.3 | Define streaming types: `StreamChunk`, `StreamHandle` | Types compile |

### Ticket 2.2: Anthropic Client

Implement Anthropic Messages API.

| Slice | Description | Test |
|-------|-------------|------|
| 2.2.1 | Implement basic HTTP client with reqwest, auth headers | Can make authenticated request |
| 2.2.2 | Implement `send_message()` for non-streaming | Unit test with mock, integration test with real API |
| 2.2.3 | Implement streaming response parsing (SSE) | Can receive and parse stream chunks |
| 2.2.4 | Extract token counts from response for cost tracking | Token counts captured correctly |

### Ticket 2.3: OpenAI Client

Implement OpenAI Chat Completions API.

| Slice | Description | Test |
|-------|-------------|------|
| 2.3.1 | Implement basic HTTP client with auth headers | Can make authenticated request |
| 2.3.2 | Implement `send_message()` for non-streaming | Unit test with mock |
| 2.3.3 | Implement streaming response parsing (SSE) | Can receive and parse stream chunks |
| 2.3.4 | Extract token counts from response | Token counts captured correctly |

### Ticket 2.4: Cost Tracking

Track token usage and calculate costs.

| Slice | Description | Test |
|-------|-------------|------|
| 2.4.1 | Create cost-per-token lookup table for known models | Lookup returns correct rates |
| 2.4.2 | Implement `CostTracker` that records each API call | Costs recorded to database |
| 2.4.3 | Implement `get_summary()` for cost aggregation | Summary shows by-tier, by-task, by-model |

### Ticket 2.5: Retry Logic

Handle rate limits and transient failures.

| Slice | Description | Test |
|-------|-------------|------|
| 2.5.1 | Implement exponential backoff helper | Backoff increases correctly |
| 2.5.2 | Wrap provider calls with retry logic | Retries on 429, 500, 503 |
| 2.5.3 | Add configurable max retries | Respects config limit |

---

## Milestone 3: Agent Runtime

**Goal**: Agents can be spawned, receive tasks, execute them, and report back.

**Checkpoint**: Can spawn a worker agent, give it a task, see it "work" (call LLM), report completion.

### Ticket 3.1: Agent Struct & Lifecycle

Basic agent creation and state management.

| Slice | Description | Test |
|-------|-------------|------|
| 3.1.1 | Implement `Agent::new()` with tier, persona, model config | Can create agent instance |
| 3.1.2 | Implement agent state transitions (Idle → Working → etc.) | State changes correctly |
| 3.1.3 | Implement `Agent::shutdown()` for clean cleanup | Agent releases resources |

### Ticket 3.2: Agent Pool Manager

Manage pools of agents by tier.

| Slice | Description | Test |
|-------|-------------|------|
| 3.2.1 | Implement `AgentPool` struct with configurable max per tier | Pool respects limits |
| 3.2.2 | Implement `spawn_agent(tier)` that creates and tracks agent | Agent added to pool |
| 3.2.3 | Implement `get_available_agent(tier)` that returns idle agent | Returns idle agent or None |
| 3.2.4 | Implement `release_agent(id)` that marks agent as available | Agent becomes idle |

### Ticket 3.3: Message Passing

Inter-agent communication via channels.

| Slice | Description | Test |
|-------|-------------|------|
| 3.3.1 | Create channel types: `AgentCommand`, `AgentResponse` | Types compile |
| 3.3.2 | Give each agent an mpsc receiver for commands | Agent can receive commands |
| 3.3.3 | Create central dispatcher with sender handles | Dispatcher can send to any agent |
| 3.3.4 | Implement response channel back to dispatcher | Agent responses reach dispatcher |

### Ticket 3.4: Persona System

Configurable agent personalities.

| Slice | Description | Test |
|-------|-------------|------|
| 3.4.1 | Load default personas from embedded config | Default personas available |
| 3.4.2 | Override personas from project config | Project persona overrides default |
| 3.4.3 | Build system prompt from persona + task context | System prompt correctly composed |

### Ticket 3.5: Task Execution Loop

Agent main loop for processing tasks.

| Slice | Description | Test |
|-------|-------------|------|
| 3.5.1 | Implement agent run loop: wait for command → process → respond | Agent processes one task |
| 3.5.2 | Integrate LLM calls into task processing | Agent calls LLM for task |
| 3.5.3 | Emit status updates during execution | Feed receives progress updates |
| 3.5.4 | Handle task completion and failure states | Correct status on success/failure |

### Ticket 3.6: Escalation Flow

Route failures up the tier hierarchy.

| Slice | Description | Test |
|-------|-------------|------|
| 3.6.1 | Define escalation policy (utility → worker → orchestrator → human) | Policy configured |
| 3.6.2 | Implement escalation trigger on repeated failure | Failed task escalates |
| 3.6.3 | Handle "needs human" terminal state | Task marked for human review |

---

## Milestone 4: Orchestration Core

**Goal**: Orchestrator can decompose tickets into slices, route to appropriate agents.

**Checkpoint**: Give orchestrator a ticket description, see it create slices, assign to workers.

### Ticket 4.1: Planner (Ticket → Slices)

Decompose tickets into vertical slices.

| Slice | Description | Test |
|-------|-------------|------|
| 4.1.1 | Create planner prompt template for decomposition | Prompt generates valid slices |
| 4.1.2 | Implement `Planner::decompose(ticket)` using orchestrator LLM | Returns list of VerticalSlice |
| 4.1.3 | Parse LLM response into structured slice data | Slices have title, description, tasks |
| 4.1.4 | Store slices in database | Slices persisted |

### Ticket 4.2: Task Queue

Priority-ordered work queue.

| Slice | Description | Test |
|-------|-------------|------|
| 4.2.1 | Implement `TaskQueue` with priority ordering | Higher priority dequeued first |
| 4.2.2 | Add `enqueue()`, `dequeue()`, `peek()` | Queue operations work correctly |
| 4.2.3 | Persist queue state to database | Queue survives restart |
| 4.2.4 | Implement `requeue()` for failed tasks | Failed tasks re-enter queue |

### Ticket 4.3: Router (Task → Tier)

Route tasks to appropriate agent tier.

| Slice | Description | Test |
|-------|-------------|------|
| 4.3.1 | Define routing rules (task type → tier) | Rules configured |
| 4.3.2 | Implement `Router::route(task)` returning target tier | Returns correct tier |
| 4.3.3 | Handle override hints in task metadata | Hints respected |

### Ticket 4.4: Dependency Tracking

Track task dependencies and blocking.

| Slice | Description | Test |
|-------|-------------|------|
| 4.4.1 | Add `depends_on: Vec<TaskId>` to Task | Field exists |
| 4.4.2 | Implement `is_blocked(task)` check | Returns true if deps incomplete |
| 4.4.3 | Filter blocked tasks from queue | Only unblocked tasks dequeued |

### Ticket 4.5: Scheduler

Coordinate task assignment to agents.

| Slice | Description | Test |
|-------|-------------|------|
| 4.5.1 | Implement scheduler loop: check queue → find agent → assign | Tasks assigned to idle agents |
| 4.5.2 | Handle "no available agent" by waiting | Scheduler waits, retries |
| 4.5.3 | Implement preemption for urgent tasks | Urgent task preempts lower priority |

---

## Milestone 5: TUI Basic

**Goal**: Functional terminal interface with feed, chat, and navigation.

**Checkpoint**: Can see agent activity in feed, chat with orchestrator, navigate with slash commands.

### Ticket 5.1: Terminal Setup

Initialize ratatui and crossterm.

| Slice | Description | Test |
|-------|-------------|------|
| 5.1.1 | Set up terminal initialization (raw mode, alternate screen) | Terminal enters TUI mode |
| 5.1.2 | Implement clean shutdown (restore terminal on exit/panic) | Terminal restored on Ctrl+C |
| 5.1.3 | Set up main event loop (input + tick) | App responds to input |

### Ticket 5.2: Layout System

Fixed panel arrangement.

| Slice | Description | Test |
|-------|-------------|------|
| 5.2.1 | Define layout constraints (header, main area, input bar) | Layout renders |
| 5.2.2 | Implement header bar with agent status (`w[0/6] o[0/2]`) | Shows agent counts |
| 5.2.3 | Implement input bar at bottom | Can type in input bar |

### Ticket 5.3: Home Screen

Startup/idle view with branding.

| Slice | Description | Test |
|-------|-------------|------|
| 5.3.1 | Render ASCII art logo centered | Logo displays |
| 5.3.2 | Show system status messages at bottom | Messages appear |
| 5.3.3 | Transition to chat when user types | Typing triggers view change |

### Ticket 5.4: Feed View (/feed)

Real-time agent activity.

| Slice | Description | Test |
|-------|-------------|------|
| 5.4.1 | Create scrollable feed widget | Feed scrolls |
| 5.4.2 | Subscribe to feed item channel | New items appear |
| 5.4.3 | Render different item types (report, milestone, error) | Types styled differently |
| 5.4.4 | Auto-scroll to bottom on new items | New items visible |

### Ticket 5.5: Chat View (/main)

Orchestrator conversation.

| Slice | Description | Test |
|-------|-------------|------|
| 5.5.1 | Create chat message list widget | Messages display |
| 5.5.2 | Implement message input and send | Can send message |
| 5.5.3 | Connect to orchestrator agent | Messages reach orchestrator |
| 5.5.4 | Display orchestrator responses | Responses appear in chat |
| 5.5.5 | Show streaming responses in real-time | Tokens appear as received |

### Ticket 5.6: Slash Command Router

Parse and route commands.

| Slice | Description | Test |
|-------|-------------|------|
| 5.6.1 | Detect `/` prefix in input | Commands identified |
| 5.6.2 | Parse command name and arguments | Parsed correctly |
| 5.6.3 | Route to appropriate view handler | View switches |
| 5.6.4 | Show error for unknown commands | Error displayed |

### Ticket 5.7: Logs View (/logs)

Technical log viewer.

| Slice | Description | Test |
|-------|-------------|------|
| 5.7.1 | Create log viewer widget | Logs display |
| 5.7.2 | Stream logs from tracing subscriber | New logs appear |
| 5.7.3 | Add log level filtering | Can filter by level |

---

## Milestone 6: Execution Layer

**Goal**: Agents can read/write files, run git commands, execute tests.

**Checkpoint**: Agent can modify a file, commit it, run tests.

### Ticket 6.1: File Operations

Scoped file read/write.

| Slice | Description | Test |
|-------|-------------|------|
| 6.1.1 | Implement `read_file(path)` with path validation | Can read file in project |
| 6.1.2 | Implement `write_file(path, content)` with path validation | Can write file in project |
| 6.1.3 | Implement path scoping (prevent escape from project dir) | Paths outside project rejected |
| 6.1.4 | Add file operation audit logging | Operations logged |

### Ticket 6.2: Git Operations

Branch, commit, diff, push.

| Slice | Description | Test |
|-------|-------------|------|
| 6.2.1 | Implement `git_status()` | Returns current status |
| 6.2.2 | Implement `git_branch(name)` | Creates branch |
| 6.2.3 | Implement `git_commit(message)` | Creates commit |
| 6.2.4 | Implement `git_diff()` | Returns diff output |
| 6.2.5 | Implement `git_push()` | Pushes to remote |
| 6.2.6 | Add git operation audit logging | Operations logged |

### Ticket 6.3: Test Runner

Run project tests and capture output.

| Slice | Description | Test |
|-------|-------------|------|
| 6.3.1 | Detect test framework (cargo test, npm test, pytest, etc.) | Detects correctly |
| 6.3.2 | Implement `run_tests()` that executes test command | Tests run, output captured |
| 6.3.3 | Parse test results (pass/fail count) | Results parsed |
| 6.3.4 | Stream test output to feed | Output appears in feed |

### Ticket 6.4: Docker Sandbox

Isolated execution environment.

| Slice | Description | Test |
|-------|-------------|------|
| 6.4.1 | Create Dockerfile for sandbox environment | Image builds |
| 6.4.2 | Implement `sandbox_exec(command)` that runs in container | Command runs in container |
| 6.4.3 | Mount project directory read-write | Files accessible in container |
| 6.4.4 | Implement resource limits (CPU, memory, time) | Limits enforced |

### Ticket 6.5: Approval Gates

User confirmation for dangerous operations.

| Slice | Description | Test |
|-------|-------------|------|
| 6.5.1 | Define "dangerous operation" categories | Categories defined |
| 6.5.2 | Check approval config before executing | Config respected |
| 6.5.3 | Implement approval prompt in TUI | Prompt appears |
| 6.5.4 | Block execution until approval received | Waits for user input |

---

## Milestone 7: GitHub Integration

**Goal**: Can pull issues from GitHub, create PRs.

**Checkpoint**: Fetch a GitHub issue, have agents work it, create a PR.

### Ticket 7.1: GitHub API Client

REST API with authentication.

| Slice | Description | Test |
|-------|-------------|------|
| 7.1.1 | Implement authenticated HTTP client | Auth header included |
| 7.1.2 | Implement `get_issue(owner, repo, number)` | Returns issue data |
| 7.1.3 | Implement `list_issues(owner, repo, filters)` | Returns issue list |
| 7.1.4 | Handle rate limiting | Respects rate limits |

### Ticket 7.2: Issue Sync

Pull issues as tickets.

| Slice | Description | Test |
|-------|-------------|------|
| 7.2.1 | Convert GitHub issue to internal Ticket type | Ticket created with correct data |
| 7.2.2 | Implement `sync_issue(url)` command | Issue pulled and stored |
| 7.2.3 | Detect already-synced issues | No duplicates |

### Ticket 7.3: PR Creation

Create PRs from completed slices.

| Slice | Description | Test |
|-------|-------------|------|
| 7.3.1 | Implement `create_pr(title, body, branch, base)` | PR created |
| 7.3.2 | Generate PR description from slice info | Description includes slice details |
| 7.3.3 | Link PR to original issue | Issue referenced |

### Ticket 7.4: Progress Updates

Update issues with progress.

| Slice | Description | Test |
|-------|-------------|------|
| 7.4.1 | Implement `add_comment(issue, body)` | Comment added |
| 7.4.2 | Generate progress summary from task states | Summary accurate |
| 7.4.3 | Auto-comment on milestone completion | Comment posted automatically |

---

## Milestone 8: Polish & Production

**Goal**: Production-ready, fully-featured.

**Checkpoint**: All views work, headless mode works, documentation complete.

### Ticket 8.1: Remaining TUI Views

Complete all slash command views.

| Slice | Description | Test |
|-------|-------------|------|
| 8.1.1 | Implement `/tasks` view (task list with status) | Tasks display with status |
| 8.1.2 | Implement `/agents` view (agent pool status) | Agents display with status |
| 8.1.3 | Implement `/costs` view (cost breakdown) | Costs display by tier/task |

### Ticket 8.2: Headless Mode

Non-interactive operation.

| Slice | Description | Test |
|-------|-------------|------|
| 8.2.1 | Add `--headless` CLI flag | Flag parsed |
| 8.2.2 | Skip TUI initialization in headless mode | No terminal manipulation |
| 8.2.3 | Output to stdout/file instead of TUI | Output goes to file |
| 8.2.4 | Accept task input from stdin/file | Can process tasks without TUI |

### Ticket 8.3: Error Handling Polish

Graceful failures and recovery.

| Slice | Description | Test |
|-------|-------------|------|
| 8.3.1 | Add error boundaries around all async tasks | Errors don't crash app |
| 8.3.2 | Implement error display section in TUI | Errors visible |
| 8.3.3 | Add recovery suggestions for common errors | Suggestions helpful |

### Ticket 8.4: Docker Packaging

Containerized deployment.

| Slice | Description | Test |
|-------|-------------|------|
| 8.4.1 | Create production Dockerfile | Image builds |
| 8.4.2 | Add docker-compose for easy deployment | Compose works |
| 8.4.3 | Document Docker usage | Docs accurate |

### Ticket 8.5: Documentation

README and user guide.

| Slice | Description | Test |
|-------|-------------|------|
| 8.5.1 | Write installation instructions | User can install |
| 8.5.2 | Write configuration guide | User can configure |
| 8.5.3 | Write usage guide with examples | Examples work |
| 8.5.4 | Document all slash commands | Commands documented |

---

## Parallelization Notes

### Can be parallelized (no dependencies):

**Within Milestone 1:**
- Ticket 1.2 (types) can start immediately
- Ticket 1.3 (config) can start after 1.2.6 (config types)
- Ticket 1.4 (database) can start after 1.2.x (needs types)
- Ticket 1.5 (logging) is independent

**Across Milestones:**
- Milestone 2 (LLM) depends only on M1 types
- Milestone 5 (TUI) can start after M1, parallel with M2-M4
- Milestone 6 (Execution) can start after M1

**Agent tier assignments:**

| Tier | Best suited for |
|------|-----------------|
| **Orchestrator** | Planning, ticket decomposition, code review |
| **Worker** | Feature implementation, bug fixes, complex slices |
| **Utility** | Boilerplate, formatting, docs, simple migrations |

---

## Status

- [ ] Milestone 1: Foundation
- [ ] Milestone 2: LLM Layer
- [ ] Milestone 3: Agent Runtime
- [ ] Milestone 4: Orchestration Core
- [ ] Milestone 5: TUI Basic
- [ ] Milestone 6: Execution Layer
- [ ] Milestone 7: GitHub Integration
- [ ] Milestone 8: Polish & Production

---

*Last updated: Initial creation*
