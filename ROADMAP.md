# nexor ROADMAP

> Living document for AI agent orchestration. Orchestrator reads this for context.

---

## Epic: nexor v1.0

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
| 1.3.1 | Implement `config/global.rs`: load from `~/.config/nexor/config.toml`, return defaults if missing | Unit test: loads file or returns defaults |
| 1.3.2 | Implement `config/project.rs`: load from `.nexor/config.toml`, return None if missing | Unit test: loads file or returns None |
| 1.3.3 | Implement config merge logic: global ← project overrides | Unit test: project values override global |
| 1.3.4 | Add config validation (required fields, valid enum values) | Unit test: invalid config returns error |

### Ticket 1.4: Database Setup

SQLite with migrations and connection pooling.

| Slice | Description | Test |
|-------|-------------|------|
| 1.4.1 | Set up sqlx with SQLite, create `.nexor/state.db` on startup | `cargo run` creates database file |
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
| 1.5.2 | Add file appender for `.nexor/logs/` | Logs written to file |
| 1.5.3 | Create log macros/helpers for consistent formatting | Logs show module, level, message |

---

## Milestone 2: LLM Layer

**Goal**: Can send prompts to Anthropic and get streaming responses.

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

### Ticket 2.3: Cost Tracking

Track token usage and calculate costs.

| Slice | Description | Test |
|-------|-------------|------|
| 2.3.1 | Create cost-per-token lookup table for known models | Lookup returns correct rates |
| 2.3.2 | Implement `CostTracker` that records each API call | Costs recorded to database |
| 2.3.3 | Implement `get_summary()` for cost aggregation | Summary shows by-tier, by-task, by-model |

### Ticket 2.4: Retry Logic

Handle rate limits and transient failures.

| Slice | Description | Test |
|-------|-------------|------|
| 2.4.1 | Implement exponential backoff helper | Backoff increases correctly |
| 2.4.2 | Wrap provider calls with retry logic | Retries on 429, 500, 503 |
| 2.4.3 | Add configurable max retries | Respects config limit |

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

### Ticket 3.7: Inter-Agent Protocol

Define exactly how agents communicate.

| Slice | Description | Test |
|-------|-------------|------|
| 3.7.1 | Define `TaskAssignment` message format (task_id, description, context_files, constraints) | Message serializes/deserializes |
| 3.7.2 | Define `TaskResult` message format (task_id, status, output, files_modified, errors) | Message serializes/deserializes |
| 3.7.3 | Define `ContextRequest` / `ContextResponse` for agents requesting more info | Request/response cycle works |
| 3.7.4 | Define `ProgressUpdate` for feed streaming | Updates appear in feed |
| 3.7.5 | Implement message validation (reject malformed messages) | Invalid messages rejected with clear error |

**Message Schema Example**:
```rust
struct TaskAssignment {
    task_id: Uuid,
    title: String,
    description: String,
    context: TaskContext,
    constraints: TaskConstraints,
    timeout: Duration,
}

struct TaskContext {
    files: Vec<FileContent>,      // Pre-loaded file contents
    history: Vec<HistoryEntry>,   // Relevant prior work
    conventions: String,          // CLAUDE.md or similar
}

struct TaskConstraints {
    max_files_modified: Option<u32>,
    allowed_paths: Vec<PathPattern>,
    require_tests: bool,
    require_review: bool,
}
```

---

## Milestone 4: Prompt Engineering & Agent Intelligence

**Goal**: Robust, tested prompts that drive reliable agent behavior.

**Checkpoint**: Agents consistently produce structured output, think step-by-step, and recover from confusion.

> **Why a dedicated milestone?** Prompts ARE the behavior. A poorly-crafted prompt means an unreliable agent.
> This milestone is about engineering the thought process, not just writing text.

---

### Ticket 4.1: Prompt Architecture Design

Establish the foundational patterns all prompts will follow.

| Slice | Description | Test |
|-------|-------------|------|
| 4.1.1 | Define prompt template structure: `{role}`, `{context}`, `{task}`, `{constraints}`, `{output_format}`, `{examples}` | Template documented, team aligned |
| 4.1.2 | Create `PromptBuilder` struct that assembles prompts from components | Builder produces valid prompt strings |
| 4.1.3 | Implement context injection system (insert codebase info, task history, etc.) | Context correctly merged into prompts |
| 4.1.4 | Create prompt versioning system (track which prompt version produced which output) | Version stored with each LLM call |

**Design Rationale**: Every prompt follows the same skeleton. This makes them predictable, testable, and debuggable.

---

### Ticket 4.2: Orchestrator Thinking Patterns

The orchestrator is the "senior architect brain" - needs sophisticated reasoning.

| Slice | Description | Test |
|-------|-------------|------|
| 4.2.1 | Design orchestrator's **decomposition thinking**: ticket → mental model → slices | Decomposes sample tickets correctly |
| 4.2.2 | Design orchestrator's **review thinking**: examine work → identify issues → feedback | Catches intentional bugs in test code |
| 4.2.3 | Design orchestrator's **routing thinking**: task traits → tier selection reasoning | Routes tasks to correct tiers |
| 4.2.4 | Design orchestrator's **conversation thinking**: user intent → clarifying questions → plan | Asks good clarifying questions |
| 4.2.5 | Design orchestrator's **recovery thinking**: failure analysis → retry strategy → escalation decision | Makes sensible recovery choices |

**Orchestrator Thinking Template**:
```
## Your Role
You are Arch, a senior software architect coordinating a team of AI agents.

## How to Think

### When decomposing a ticket:
1. UNDERSTAND: What is the user actually trying to accomplish? What's the business value?
2. INVENTORY: What components/layers does this touch? (DB, API, UI, tests, etc.)
3. DEPENDENCIES: What must exist before other parts can work?
4. SLICE VERTICALLY: Each slice must be deployable alone. Ask: "If we stopped after this slice, would something work?"
5. SIZE CHECK: Each slice should be 1-4 hours of work. Too big? Split it. Too small? Combine with adjacent slice.

### When reviewing work:
1. CORRECTNESS: Does the code do what the task asked?
2. INTEGRATION: Will this break anything else?
3. QUALITY: Is this code maintainable? Any obvious issues?
4. COMPLETENESS: Are edge cases handled? Tests included?
5. VERDICT: Approve, request changes (be specific), or escalate.

### When stuck or confused:
1. STATE what you understand and what's unclear
2. ASK the user a specific clarifying question
3. NEVER guess at requirements - assumptions compound into wrong solutions

## Output Constraints
- Always explain your reasoning before giving conclusions
- Use the structured output format specified for each task type
- Prefix uncertainty with "I believe..." or "My understanding is..."
```

---

### Ticket 4.3: Worker Thinking Patterns

Workers are "focused developers" - need clear execution patterns.

| Slice | Description | Test |
|-------|-------------|------|
| 4.3.1 | Design worker's **implementation thinking**: task → plan → code → verify | Implements sample tasks correctly |
| 4.3.2 | Design worker's **context gathering**: what files do I need? → request specific context | Requests relevant files, not everything |
| 4.3.3 | Design worker's **progress reporting**: natural language updates on what they're doing | Reports are informative and human-readable |
| 4.3.4 | Design worker's **self-checking**: before submitting, verify against requirements | Catches own mistakes before review |
| 4.3.5 | Design worker's **stuck detection**: recognize when spinning wheels → escalate | Escalates after N failed attempts |

**Worker Thinking Template**:
```
## Your Role
You are Dev, a focused software developer. You receive well-scoped tasks and implement them.

## How to Think

### When starting a task:
1. READ the task description completely
2. IDENTIFY what files you'll need to read/modify
3. REQUEST context if needed (be specific: "I need to see src/auth/mod.rs")
4. PLAN your approach in 2-3 sentences before coding
5. ANNOUNCE: "Starting work on [task]. My approach: [brief plan]"

### When implementing:
1. WRITE code incrementally - don't try to do everything at once
2. EXPLAIN significant decisions: "Using X approach because Y"
3. TEST mentally: "If I call this with X, it should return Y"
4. REPORT progress every few minutes: "Completed the struct definition, now writing the impl block"

### Before submitting:
1. RE-READ the original task requirements
2. CHECK: Does my code satisfy each requirement?
3. LOOK for obvious bugs: off-by-one, null checks, error handling
4. VERIFY: Are there tests? Do they cover the happy path and edge cases?
5. If anything is incomplete, note it explicitly

### When stuck:
1. After 2-3 failed attempts at the same thing, STOP
2. SUMMARIZE: "I've tried X and Y, but I'm getting Z error"
3. ESCALATE: Request help rather than spinning in circles
4. NEVER submit broken code hoping it works - be honest about blockers

## Communication Style
- Brief, informative updates
- Focus on what you're doing, not philosophizing
- "Looking at auth module..." not "I shall now endeavor to examine..."
```

---

### Ticket 4.4: Utility Thinking Patterns

Utilities are "efficient helpers" - need speed and precision.

| Slice | Description | Test |
|-------|-------------|------|
| 4.4.1 | Design utility's **task recognition**: quick categorization of task type | Correctly identifies task type |
| 4.4.2 | Design utility's **templated execution**: apply known patterns rapidly | Formats code consistently |
| 4.4.3 | Design utility's **minimal reporting**: only report completion or errors | Reports are concise |
| 4.4.4 | Design utility's **escalation trigger**: recognize when task is too complex | Escalates complex tasks, doesn't attempt |

**Utility Thinking Template**:
```
## Your Role
You are Helper, handling quick well-defined tasks efficiently.

## How to Think

### Task categories you handle:
- FORMAT: Apply code formatter to files
- LINT: Run linter, fix auto-fixable issues
- BOILERPLATE: Generate code from templates
- DOCS: Update documentation, add docstrings
- RENAME: Find/replace identifiers

### Your process:
1. IDENTIFY task category (if unclear, escalate immediately)
2. EXECUTE the standard procedure for that category
3. REPORT result: "Formatted 3 files" or "Error: [specific error]"

### Escalate when:
- Task requires understanding business logic
- Task involves architectural decisions
- You're unsure what the right answer is
- Task would take more than a few minutes

## Communication Style
- Terse: "Done. Formatted src/*.rs (4 files)"
- Error format: "Failed: [file]: [error]"
- Never explain your reasoning unless asked
```

---

### Ticket 4.5: Structured Output Design

LLM outputs must be parseable. Design the output contracts.

| Slice | Description | Test |
|-------|-------------|------|
| 4.5.1 | Design slice output schema (JSON with title, description, tasks, dependencies) | Schema validates correctly |
| 4.5.2 | Design task output schema (status, result, files_modified, tests_added) | Schema validates correctly |
| 4.5.3 | Design review output schema (verdict, issues[], suggestions[], approved_files[]) | Schema validates correctly |
| 4.5.4 | Design error output schema (error_type, message, attempted_recovery, needs_human) | Schema validates correctly |
| 4.5.5 | Implement output validation with clear error messages on schema mismatch | Invalid output triggers retry with correction hint |

**Decomposition Output Schema**:
```json
{
  "thinking": "Brief explanation of decomposition reasoning",
  "slices": [
    {
      "title": "Short descriptive title",
      "description": "What this slice accomplishes",
      "tasks": [
        {
          "title": "Specific task",
          "tier": "worker|utility",
          "estimated_complexity": "low|medium|high",
          "context_files": ["paths the agent will need"]
        }
      ],
      "dependencies": ["slice_titles this depends on"],
      "acceptance_criteria": ["How to verify this slice is done"]
    }
  ],
  "questions": ["Clarifying questions if requirements unclear"],
  "risks": ["Potential issues or unknowns"]
}
```

---

### Ticket 4.6: Few-Shot Examples Library

Examples are often more powerful than instructions. Build the example bank.

| Slice | Description | Test |
|-------|-------------|------|
| 4.6.1 | Create 3-5 decomposition examples (ticket → slices) across different domains | Examples demonstrate good vertical slicing |
| 4.6.2 | Create 3-5 implementation examples (task → code) showing thinking process | Examples show plan-then-code pattern |
| 4.6.3 | Create 3-5 review examples (code → feedback) showing quality standards | Examples demonstrate constructive, specific feedback |
| 4.6.4 | Create 2-3 failure/recovery examples showing graceful degradation | Examples show when/how to escalate |
| 4.6.5 | Implement example selection logic (pick relevant examples based on task type) | Relevant examples injected into prompts |

**Example: Decomposition**
```
USER TICKET: "Add user authentication to the API"

ORCHESTRATOR THINKING:
I need to think about what "authentication" really means here and what vertical slices make sense.

Components touched: database (users table), API routes, middleware, tests
Dependencies: Users must exist before we can authenticate them

Slicing strategy: Each slice should result in something testable.

SLICES:
1. "User model + migration" - Creates users table, User struct, basic CRUD
   Why vertical: Can test user creation independently

2. "Password hashing + validation" - Adds password field, hashing on create, validation
   Why vertical: Can test password handling without full auth flow

3. "Login endpoint" - POST /login, verify credentials, return token
   Why vertical: With users and passwords working, can test actual login

4. "Auth middleware" - Validate token, inject user into request context
   Why vertical: Can test middleware independently with mock tokens

5. "Protected route example" - One route that requires auth
   Why vertical: Proves the whole flow works end-to-end
```

---

### Ticket 4.7: Prompt Testing Framework

Prompts need tests like code does.

| Slice | Description | Test |
|-------|-------------|------|
| 4.7.1 | Create test harness that runs prompts against fixture inputs | Harness executes and captures output |
| 4.7.2 | Build assertion library: `output_matches_schema`, `contains_reasoning`, `no_hallucinated_files` | Assertions work correctly |
| 4.7.3 | Create regression test suite for decomposition prompts | Tests catch regressions |
| 4.7.4 | Create regression test suite for implementation prompts | Tests catch regressions |
| 4.7.5 | Add prompt diff tooling (compare outputs across prompt versions) | Can see behavioral changes |
| 4.7.6 | Implement "confusion detection" - identify when LLM output indicates uncertainty | Detects hedging language, contradictions |

**Test Example**:
```rust
#[test]
fn decomposition_produces_vertical_slices() {
    let ticket = "Add user authentication to the API";
    let output = run_decomposition_prompt(ticket);

    // Schema validation
    assert!(output.matches_schema::<DecompositionOutput>());

    // Quality checks
    assert!(output.slices.len() >= 2, "Should produce multiple slices");
    assert!(output.slices.iter().all(|s| !s.acceptance_criteria.is_empty()),
            "Each slice needs acceptance criteria");

    // Vertical slice check: no slice should be "write all tests" or "write all DB code"
    for slice in &output.slices {
        assert!(!is_horizontal_slice(&slice.title),
                "Slice '{}' appears to be horizontal, not vertical", slice.title);
    }
}
```

---

### Ticket 4.8: Context Management Strategy

Agents need the right context - not too much, not too little.

| Slice | Description | Test |
|-------|-------------|------|
| 4.8.1 | Design context budget system (max tokens per context type) | Budgets enforced |
| 4.8.2 | Implement smart file selection (relevance scoring for context files) | Relevant files ranked higher |
| 4.8.3 | Implement context summarization for large files | Large files summarized appropriately |
| 4.8.4 | Create "context request" protocol (agent asks for specific files) | Requests fulfilled efficiently |
| 4.8.5 | Implement conversation history truncation (keep recent + important) | History stays within budget |

**Context Priority Rules**:
```
1. Task description and requirements (always include, full)
2. Files agent will modify (always include, full if small, summarized if large)
3. Files agent references (include on request, summarize if large)
4. Related test files (include if task involves testing)
5. Recent conversation history (last 3-5 turns)
6. Project conventions (CLAUDE.md, style guides - always include, small)
7. Previous slice outputs (if this task depends on earlier work)

Budget allocation:
- 40% for files being modified
- 25% for reference files
- 20% for task + history
- 15% for project conventions
```

---

### Ticket 4.9: Self-Correction & Recovery Prompts

What happens when things go wrong?

| Slice | Description | Test |
|-------|-------------|------|
| 4.9.1 | Design "output didn't parse" recovery prompt | Agent reformats output correctly |
| 4.9.2 | Design "tests failed" analysis prompt | Agent diagnoses failure cause |
| 4.9.3 | Design "review rejected" revision prompt | Agent addresses feedback specifically |
| 4.9.4 | Design "stuck in loop" detection and breakout | Detects repetition, changes approach |
| 4.9.5 | Design "conflicting requirements" clarification prompt | Asks specific clarifying questions |

### Ticket 4.10: Tool Definition & Selection

Agents need to know what tools exist and how to use them.

| Slice | Description | Test |
|-------|-------------|------|
| 4.10.1 | Define tool schema format (name, description, parameters, returns, side_effects) | Schema documented |
| 4.10.2 | Create tool definitions for file ops: `read_file`, `write_file`, `list_dir` | Definitions valid |
| 4.10.3 | Create tool definitions for git ops: `git_status`, `git_diff`, `git_commit`, `git_branch` | Definitions valid |
| 4.10.4 | Create tool definitions for test ops: `run_tests`, `run_single_test` | Definitions valid |
| 4.10.5 | Design tool selection prompt (given task, which tools needed?) | Agent selects appropriate tools |
| 4.10.6 | Design tool invocation format (how agent requests tool use) | Format parseable, unambiguous |

**Tool Definition Schema**:
```json
{
  "name": "write_file",
  "description": "Write content to a file, creating it if it doesn't exist",
  "parameters": {
    "path": { "type": "string", "description": "Relative path from project root" },
    "content": { "type": "string", "description": "Full file content to write" }
  },
  "returns": { "type": "object", "properties": { "success": "bool", "bytes_written": "int" } },
  "side_effects": ["modifies_filesystem"],
  "requires_approval": false
}
```

### Ticket 4.11: Context Window Validation

Ensure prompts fit within model limits.

| Slice | Description | Test |
|-------|-------------|------|
| 4.11.1 | Create token counter (tiktoken or similar) | Counts tokens accurately for target models |
| 4.11.2 | Define context budgets per model (claude-sonnet: 200k, gpt-4: 128k, etc.) | Budgets documented |
| 4.11.3 | Implement pre-flight check before LLM calls | Rejects prompts that exceed budget |
| 4.11.4 | Implement automatic truncation strategy (what to cut first) | Truncates least-important context first |
| 4.11.5 | Add "context pressure" warning when approaching limits | Warning shown in feed |

**Recovery Prompt Template**:
```
## Situation
Your previous output couldn't be processed. Here's what happened:

**Your output**: [truncated output]
**Error**: [specific parse error or validation failure]

## What went wrong
[Specific explanation: "The JSON was missing a closing brace" or "The 'tier' field had value 'medium' but must be 'worker' or 'utility'"]

## Try again
Please regenerate your response, being careful to:
1. [Specific fix needed]
2. Use the exact schema format shown below

[schema reminder]

## Your corrected response:
```

---

## Milestone 5: Orchestration Core

**Goal**: Specialized bots for planning (Planner Bot in `/plan`) and execution (Orchestrator in `/main`), plus infrastructure to decompose tickets and schedule work.

**Checkpoint**: Create a PRD in `/plan` mode with the Planner Bot, then execute it via `/main` where the orchestrator decomposes milestones into slices.

### Ticket 5.0: Planner Bot (Interactive PRD Creation)

Specialized bot for `/plan` mode that helps users create PRDs through conversation.

| Slice | Description | Test |
|-------|-------------|------|
| 5.0.1 | Define `PRDDocument`, `MilestoneSpec`, `TechnicalDecision` types | Types compile, serialize |
| 5.0.2 | Create Planner Bot persona with phase-based system prompts | Persona works through Discovery → Scoping → Technical → Milestones → Review |
| 5.0.3 | Implement conversation loop with history and phase transitions | Multi-turn chat works |
| 5.0.4 | Implement PRD finalization and markdown export | PRD exports correctly |
| 5.0.5 | Persist PRDs and planning sessions to database | Sessions resumable |

**Planning Phases**:
- Discovery → Understanding the problem
- Scoping → Defining boundaries
- Technical → Making tech decisions
- Milestones → Breaking into milestones
- Review → Final approval

### Ticket 5.1: Planner (Ticket → Slices)

Decompose tickets into vertical slices.

| Slice | Description | Test |
|-------|-------------|------|
| 5.1.1 | Integrate decomposition prompt from M4 with LLM calls | Prompt executes, returns raw output |
| 5.1.2 | Implement `Planner::decompose(ticket)` using orchestrator LLM | Returns list of VerticalSlice |
| 5.1.3 | Parse LLM response into structured slice data | Slices have title, description, tasks |
| 5.1.4 | Implement retry with correction prompt on parse failure | Recovers from malformed output |
| 5.1.5 | Store slices in database | Slices persisted |

### Ticket 5.2: Task Queue

Priority-ordered work queue.

| Slice | Description | Test |
|-------|-------------|------|
| 5.2.1 | Implement `TaskQueue` with priority ordering | Higher priority dequeued first |
| 5.2.2 | Add `enqueue()`, `dequeue()`, `peek()` | Queue operations work correctly |
| 5.2.3 | Persist queue state to database | Queue survives restart |
| 5.2.4 | Implement `requeue()` for failed tasks | Failed tasks re-enter queue |

### Ticket 5.3: Router (Task → Tier)

Route tasks to appropriate agent tier.

| Slice | Description | Test |
|-------|-------------|------|
| 5.3.1 | Define routing rules (task type → tier) | Rules configured |
| 5.3.2 | Implement `Router::route(task)` returning target tier | Returns correct tier |
| 5.3.3 | Handle override hints in task metadata | Hints respected |

### Ticket 5.4: Dependency Tracking

Track task dependencies and blocking.

| Slice | Description | Test |
|-------|-------------|------|
| 5.4.1 | Add `depends_on: Vec<TaskId>` to Task | Field exists |
| 5.4.2 | Implement `is_blocked(task)` check | Returns true if deps incomplete |
| 5.4.3 | Filter blocked tasks from queue | Only unblocked tasks dequeued |

### Ticket 5.5: Scheduler

Coordinate task assignment to agents.

| Slice | Description | Test |
|-------|-------------|------|
| 5.5.1 | Implement scheduler loop: check queue → find agent → assign | Tasks assigned to idle agents |
| 5.5.2 | Handle "no available agent" by waiting | Scheduler waits, retries |
| 5.5.3 | Implement preemption for urgent tasks | Urgent task preempts lower priority |

---

## Milestone 6: TUI Basic

**Goal**: Functional terminal interface with feed, chat, and navigation.

**Checkpoint**: Can see agent activity in feed, chat with orchestrator, navigate with slash commands.

### Ticket 6.1: Terminal Setup

Initialize ratatui and crossterm.

| Slice | Description | Test |
|-------|-------------|------|
| 6.1.1 | Set up terminal initialization (raw mode, alternate screen) | Terminal enters TUI mode |
| 6.1.2 | Implement clean shutdown (restore terminal on exit/panic) | Terminal restored on Ctrl+C |
| 6.1.3 | Set up main event loop (input + tick) | App responds to input |

### Ticket 6.2: Layout System

Fixed panel arrangement.

| Slice | Description | Test |
|-------|-------------|------|
| 6.2.1 | Define layout constraints (header, main area, input bar) | Layout renders |
| 6.2.2 | Implement header bar with agent status (`w[0/6] o[0/2]`) | Shows agent counts |
| 6.2.3 | Implement input bar at bottom | Can type in input bar |

### Ticket 6.3: Home Screen

Startup/idle view with branding.

| Slice | Description | Test |
|-------|-------------|------|
| 6.3.1 | Render ASCII art logo centered | Logo displays |
| 6.3.2 | Show system status messages at bottom | Messages appear |
| 6.3.3 | Transition to chat when user types | Typing triggers view change |

### Ticket 6.4: Feed View (/feed)

Real-time agent activity.

| Slice | Description | Test |
|-------|-------------|------|
| 6.4.1 | Create scrollable feed widget | Feed scrolls |
| 6.4.2 | Subscribe to feed item channel | New items appear |
| 6.4.3 | Render different item types (report, milestone, error) | Types styled differently |
| 6.4.4 | Auto-scroll to bottom on new items | New items visible |

### Ticket 6.5: Chat View (/main)

Orchestrator conversation.

| Slice | Description | Test |
|-------|-------------|------|
| 6.5.1 | Create chat message list widget | Messages display |
| 6.5.2 | Implement message input and send | Can send message |
| 6.5.3 | Connect to orchestrator agent | Messages reach orchestrator |
| 6.5.4 | Display orchestrator responses | Responses appear in chat |
| 6.5.5 | Show streaming responses in real-time | Tokens appear as received |

### Ticket 6.6: Slash Command Router

Parse and route commands.

| Slice | Description | Test |
|-------|-------------|------|
| 6.6.1 | Detect `/` prefix in input | Commands identified |
| 6.6.2 | Parse command name and arguments | Parsed correctly |
| 6.6.3 | Route to appropriate view handler | View switches |
| 6.6.4 | Show error for unknown commands | Error displayed |

### Ticket 6.7: Logs View (/logs)

Technical log viewer.

| Slice | Description | Test |
|-------|-------------|------|
| 6.7.1 | Create log viewer widget | Logs display |
| 6.7.2 | Stream logs from tracing subscriber | New logs appear |
| 6.7.3 | Add log level filtering | Can filter by level |

### Ticket 6.8: Plan View (/plan)

Interactive PRD creation with Planner Bot.

| Slice | Description | Test |
|-------|-------------|------|
| 6.8.1 | Create two-panel layout (chat + PRD preview) with phase indicator | Layout renders |
| 6.8.2 | Implement input handling (typing, submit, scroll) | Can chat |
| 6.8.3 | Connect to Planner Bot, handle async responses | Bot responds |
| 6.8.4 | Add `/plan` commands to router (`/plan`, `/plan new`, `/plan approve`) | Commands work |

---

## Milestone 7: Execution Layer

**Goal**: Agents can read/write files, run git commands, execute tests.

**Checkpoint**: Agent can modify a file, commit it, run tests.

### Ticket 7.1: File Operations

Scoped file read/write.

| Slice | Description | Test |
|-------|-------------|------|
| 7.1.1 | Implement `read_file(path)` with path validation | Can read file in project |
| 7.1.2 | Implement `write_file(path, content)` with path validation | Can write file in project |
| 7.1.3 | Implement path scoping (prevent escape from project dir) | Paths outside project rejected |
| 7.1.4 | Add file operation audit logging | Operations logged |

### Ticket 7.2: Git Operations

Branch, commit, diff, push.

| Slice | Description | Test |
|-------|-------------|------|
| 7.2.1 | Implement `git_status()` | Returns current status |
| 7.2.2 | Implement `git_branch(name)` | Creates branch |
| 7.2.3 | Implement `git_commit(message)` | Creates commit |
| 7.2.4 | Implement `git_diff()` | Returns diff output |
| 7.2.5 | Implement `git_push()` | Pushes to remote |
| 7.2.6 | Add git operation audit logging | Operations logged |

### Ticket 7.3: Test Runner

Run project tests and capture output.

| Slice | Description | Test |
|-------|-------------|------|
| 7.3.1 | Detect test framework (cargo test, npm test, pytest, etc.) | Detects correctly |
| 7.3.2 | Implement `run_tests()` that executes test command | Tests run, output captured |
| 7.3.3 | Parse test results (pass/fail count) | Results parsed |
| 7.3.4 | Stream test output to feed | Output appears in feed |

### Ticket 7.4: Docker Sandbox

Isolated execution environment.

| Slice | Description | Test |
|-------|-------------|------|
| 7.4.1 | Create Dockerfile for sandbox environment | Image builds |
| 7.4.2 | Implement `sandbox_exec(command)` that runs in container | Command runs in container |
| 7.4.3 | Mount project directory read-write | Files accessible in container |
| 7.4.4 | Implement resource limits (CPU, memory, time) | Limits enforced |

### Ticket 7.5: Approval Gates

User confirmation for dangerous operations.

| Slice | Description | Test |
|-------|-------------|------|
| 7.5.1 | Define "dangerous operation" categories | Categories defined |
| 7.5.2 | Check approval config before executing | Config respected |
| 7.5.3 | Implement approval prompt in TUI | Prompt appears |
| 7.5.4 | Block execution until approval received | Waits for user input |

---

## Milestone 8: GitHub Integration

**Goal**: Can pull issues from GitHub, create PRs.

**Checkpoint**: Fetch a GitHub issue, have agents work it, create a PR.

### Ticket 8.1: GitHub API Client

REST API with authentication.

| Slice | Description | Test |
|-------|-------------|------|
| 8.1.1 | Implement authenticated HTTP client | Auth header included |
| 8.1.2 | Implement `get_issue(owner, repo, number)` | Returns issue data |
| 8.1.3 | Implement `list_issues(owner, repo, filters)` | Returns issue list |
| 8.1.4 | Handle rate limiting | Respects rate limits |

### Ticket 8.2: Issue Sync

Pull issues as tickets.

| Slice | Description | Test |
|-------|-------------|------|
| 8.2.1 | Convert GitHub issue to internal Ticket type | Ticket created with correct data |
| 8.2.2 | Implement `sync_issue(url)` command | Issue pulled and stored |
| 8.2.3 | Detect already-synced issues | No duplicates |

### Ticket 8.3: PR Creation

Create PRs from completed slices.

| Slice | Description | Test |
|-------|-------------|------|
| 8.3.1 | Implement `create_pr(title, body, branch, base)` | PR created |
| 8.3.2 | Generate PR description from slice info | Description includes slice details |
| 8.3.3 | Link PR to original issue | Issue referenced |

### Ticket 8.4: Progress Updates

Update issues with progress.

| Slice | Description | Test |
|-------|-------------|------|
| 8.4.1 | Implement `add_comment(issue, body)` | Comment added |
| 8.4.2 | Generate progress summary from task states | Summary accurate |
| 8.4.3 | Auto-comment on milestone completion | Comment posted automatically |

---

## Milestone 9: Polish & Production

**Goal**: Production-ready, fully-featured.

**Checkpoint**: All views work, headless mode works, documentation complete.

### Ticket 9.1: Remaining TUI Views

Complete all slash command views.

| Slice | Description | Test |
|-------|-------------|------|
| 9.1.1 | Implement `/tasks` view (task list with status) | Tasks display with status |
| 9.1.2 | Implement `/agents` view (agent pool status) | Agents display with status |
| 9.1.3 | Implement `/costs` view (cost breakdown) | Costs display by tier/task |

### Ticket 9.2: Headless Mode

Non-interactive operation.

| Slice | Description | Test |
|-------|-------------|------|
| 9.2.1 | Add `--headless` CLI flag | Flag parsed |
| 9.2.2 | Skip TUI initialization in headless mode | No terminal manipulation |
| 9.2.3 | Output to stdout/file instead of TUI | Output goes to file |
| 9.2.4 | Accept task input from stdin/file | Can process tasks without TUI |

### Ticket 9.3: Error Handling Polish

Graceful failures and recovery.

| Slice | Description | Test |
|-------|-------------|------|
| 9.3.1 | Add error boundaries around all async tasks | Errors don't crash app |
| 9.3.2 | Implement error display section in TUI | Errors visible |
| 9.3.3 | Add recovery suggestions for common errors | Suggestions helpful |

### Ticket 9.4: Docker Packaging

Containerized deployment.

| Slice | Description | Test |
|-------|-------------|------|
| 9.4.1 | Create production Dockerfile | Image builds |
| 9.4.2 | Add docker-compose for easy deployment | Compose works |
| 9.4.3 | Document Docker usage | Docs accurate |

### Ticket 9.5: Documentation

README and user guide.

| Slice | Description | Test |
|-------|-------------|------|
| 9.5.1 | Write installation instructions | User can install |
| 9.5.2 | Write configuration guide | User can configure |
| 9.5.3 | Write usage guide with examples | Examples work |
| 9.5.4 | Document all slash commands | Commands documented |

### Ticket 9.6: Observability & Replay

Debug why agents made specific decisions.

| Slice | Description | Test |
|-------|-------------|------|
| 9.6.1 | Log full prompt + response for every LLM call | Logs stored with task_id reference |
| 9.6.2 | Implement `/replay <task_id>` to view agent's thinking | Shows prompt, response, tool calls |
| 9.6.3 | Add decision tracing (why did orchestrator choose this decomposition?) | Reasoning captured and viewable |
| 9.6.4 | Create export format for debugging sessions | Can export full session for analysis |
| 9.6.5 | Add cost attribution per decision | Can see "this decomposition cost $0.12" |

---

## Milestone 10: In-TUI File Editor

**Goal**: Users can view and edit files directly within the TUI, including files agents are working on.

**Checkpoint**: Can open a file from agent's task, edit it in-app with syntax highlighting, save and commit changes.

### Ticket 10.1: File Viewer Widget

Read-only file viewing with syntax highlighting.

| Slice | Description | Test |
|-------|-------------|------|
| 10.1.1 | Create `FileViewer` widget with scrollable content | Can display file, scroll with arrow keys |
| 10.1.2 | Add line numbers and cursor position display | Line numbers visible, status bar shows position |
| 10.1.3 | Integrate `syntect` for syntax highlighting | Rust/JS/Python files highlighted correctly |
| 10.1.4 | Add search functionality (Ctrl+W) | Can search, highlights matches |

### Ticket 10.2: File Editor Widget

Full text editing using tui-textarea.

| Slice | Description | Test |
|-------|-------------|------|
| 10.2.1 | Integrate `tui-textarea` as editor core | Basic text input works |
| 10.2.2 | Add nano-style keybindings (Ctrl+X exit, Ctrl+O save, etc.) | Keybindings work as documented |
| 10.2.3 | Track modified state, show in status bar | "Modified" indicator appears on changes |
| 10.2.4 | Add undo/redo support | Ctrl+Z/Ctrl+Y work |
| 10.2.5 | Add go-to-line (Ctrl+G) and search (Ctrl+W) | Navigation features work |

### Ticket 10.3: File Browser Widget

Tree view for navigating project files.

| Slice | Description | Test |
|-------|-------------|------|
| 10.3.1 | Create `FileBrowser` widget with directory tree | Shows project structure |
| 10.3.2 | Add expand/collapse for directories | Can navigate tree |
| 10.3.3 | Filter by file type (show only .rs, .ts, etc.) | Filter works |
| 10.3.4 | Open file in viewer/editor on Enter | Selection opens file |

### Ticket 10.4: Diff Viewer

Show changes made by agents.

| Slice | Description | Test |
|-------|-------------|------|
| 10.4.1 | Create `DiffViewer` widget with side-by-side or unified view | Diff displays correctly |
| 10.4.2 | Integrate with git for file diffs | Shows uncommitted changes |
| 10.4.3 | Highlight additions (green) and deletions (red) | Colors applied |
| 10.4.4 | Navigate between diff hunks | Can jump between changes |

### Ticket 10.5: Save & Commit Flow

Save changes and optionally commit.

| Slice | Description | Test |
|-------|-------------|------|
| 10.5.1 | Implement save file functionality | File saved to disk |
| 10.5.2 | Add "unsaved changes" prompt on exit | Prompt appears, respects choice |
| 10.5.3 | Implement save & commit dialog | Can enter commit message |
| 10.5.4 | Create commit on current branch via git2 | Commit created successfully |
| 10.5.5 | Show success/error notification | User informed of result |

### Ticket 10.6: Slash Commands Integration

Wire up /view, /edit, /diff, /files commands.

| Slice | Description | Test |
|-------|-------------|------|
| 10.6.1 | Add `/view <path>` command to open FileViewer | Command opens viewer |
| 10.6.2 | Add `/edit <path>` command to open FileEditor | Command opens editor |
| 10.6.3 | Add `/diff <path>` command to open DiffViewer | Command opens diff |
| 10.6.4 | Add `/files` command to open FileBrowser | Command opens browser |
| 10.6.5 | Add file quick-open from agent task context | Can open files agent is working on |

### Ticket 10.7: Agent Integration

Connect editor to agent workflow.

| Slice | Description | Test |
|-------|-------------|------|
| 10.7.1 | Show "View/Edit" actions on files in task context | Actions visible in task view |
| 10.7.2 | Highlight files modified by agent in file browser | Modified files marked |
| 10.7.3 | Refresh file content when agent modifies it | User sees agent's changes |
| 10.7.4 | Handle conflicts when user and agent edit same file | Warning shown, user can resolve |

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
- Milestone 4 (Prompts) can start early - prompt design doesn't need working code
- Milestone 6 (TUI) can start after M1, parallel with M2-M5
- Milestone 7 (Execution) can start after M1

**Agent tier assignments:**

| Tier | Best suited for |
|------|-----------------|
| **Orchestrator** | Planning, ticket decomposition, code review, prompt design |
| **Worker** | Feature implementation, bug fixes, complex slices |
| **Utility** | Boilerplate, formatting, docs, simple migrations |

**Prompt work is special:**
- Tickets 4.1-4.4 (thinking patterns) are **design work** - orchestrator does this
- Tickets 4.5-4.6 (schemas, examples) can be done by **workers**
- Tickets 4.7-4.9 (testing, context, recovery) need **workers with LLM access**

---

## Status

- [x] Milestone 1: Foundation
- [x] Milestone 2: LLM Layer
- [x] Milestone 3: Agent Runtime
- [x] Milestone 4: Prompt Engineering & Agent Intelligence
- [x] Milestone 5: Orchestration Core
- [ ] Milestone 6: TUI Basic
- [ ] Milestone 7: Execution Layer
- [ ] Milestone 8: GitHub Integration
- [ ] Milestone 9: Polish & Production
- [ ] Milestone 10: In-TUI File Editor ← **NEW**

---

## Prompt Engineering Philosophy

> "The prompts are not configuration. The prompts ARE the product."

### Key Principles

1. **Show, don't tell** - Examples beat instructions. An agent learns more from seeing "good decomposition" than reading "decompose well."

2. **Make thinking visible** - Every agent should explain reasoning before conclusions. This helps debugging and builds user trust.

3. **Fail loudly, recover gracefully** - When confused, agents should say "I'm confused about X" not silently guess.

4. **Structured output is non-negotiable** - Every LLM output must be parseable. Natural language for humans, JSON for machines.

5. **Context is precious** - Every token of context should earn its place. Irrelevant context = worse outputs.

6. **Test prompts like code** - Prompts have regressions. Version them, test them, diff them.

### Anti-Patterns to Avoid

- ❌ "Be helpful and do your best" - Too vague, leads to inconsistent behavior
- ❌ Massive system prompts that cover every edge case - Dilutes focus
- ❌ Assuming the LLM "knows" your codebase conventions - Be explicit
- ❌ "Please" and "Thank you" consuming tokens - Be direct
- ❌ Hoping the LLM figures out output format - Specify exactly

---

## Future Considerations (Post v1.0)

Features to explore after core functionality is stable.

### Collaborative Planning

**Vision**: Connect with other people and share plans with AI.

| Feature | Description |
|---------|-------------|
| **Shared ROADMAP** | Multiple users can view/edit the same ROADMAP.md in real-time |
| **Plan export/import** | Export a decomposition as shareable JSON, import into another instance |
| **Team orchestration** | Multiple orchestrators coordinate across a team's agents |
| **Review handoff** | Human reviewer approves slices, AI picks up approved work |
| **Async collaboration** | Leave notes for other humans/agents, pick up where they left off |

**Technical considerations**:
- WebSocket or CRDT for real-time sync
- Authentication layer (GitHub OAuth?)
- Conflict resolution when two users edit same slice
- Permission model (who can approve, who can only suggest)

### In-App File Viewer/Editor

> **Now implemented as Milestone 10** - See M10 for full decomposition.

Full in-TUI file editing with syntax highlighting, nano-style keybindings, git integration, and agent-aware conflict handling.

### Other Ideas

- **Learning system** - Improve prompts based on success/failure patterns
- **Multi-repo support** - Orchestrate across multiple repositories
- **Plugin architecture** - Extensible integrations beyond GitHub
- **Pause/resume agents** - Save and restore agent state mid-task
- **Voice interface** - Speak to orchestrator, hear progress updates
- **Mobile companion** - Monitor agent progress from phone

---

*Last updated: Added gap-filling tickets (3.7, 4.10, 4.11, 9.6) and Future Considerations*
