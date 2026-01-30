# Nexor Rust Source Guide

A walkthrough of the Rust backend for experienced developers who are new to Rust.

---

## Quick Rust Concepts You'll See Everywhere

| Concept | What it means |
|---|---|
| `mod` | A module — Rust's way of organizing code into namespaces. A folder with `mod.rs` or a single `.rs` file. |
| `pub` | Public visibility. Without it, items are private to their module. |
| `struct` | A data type with named fields (like a class without inheritance). |
| `enum` | A tagged union — each variant can hold different data. Rust enums are far more powerful than C/Java enums. |
| `impl` | An implementation block — where you attach methods to a struct or enum. |
| `trait` | An interface. Defines a contract that types can implement. |
| `async fn` | An async function. Returns a `Future` that must be `.await`ed. Rust async is zero-cost — no runtime baked into the language. |
| `Result<T, E>` | Rust has no exceptions. Functions return `Result::Ok(value)` or `Result::Err(error)`. The `?` operator propagates errors up the call stack. |
| `Arc<T>` | Atomic reference-counted pointer. Lets multiple threads share ownership of data. |
| `RwLock<T>` | Reader-writer lock. Multiple readers OR one writer at a time. |
| `#[derive(...)]` | Auto-generates trait implementations. `#[derive(Clone, Debug, Serialize)]` is like auto-generating `.clone()`, debug printing, and JSON serialization. |
| Newtype pattern | Wrapping a generic type for type safety: `struct TaskId(Uuid)`. Prevents mixing up two `Uuid` values that mean different things. |

---

## Entry Points

### `src/main.rs` — Where it all starts

The binary entry point. Uses `#[tokio::main]` to boot the async runtime (Tokio is Rust's most popular async executor — think Node's event loop, but multi-threaded).

Two modes:
- **Server mode** — starts an HTTP + WebSocket server
- **Headless mode** — runs tasks from CLI input, outputs results, exits (for CI/CD)

### `src/lib.rs` — Library root

Re-exports every module with `pub mod`. This is a Rust convention: `main.rs` is the binary, `lib.rs` is the library. This separation lets tests and other crates import your code without going through `main`.

---

## Core Modules

### `src/types/` — Domain types

The heart of the type system. Defines the nouns of the application:

| File | Key Types | Notes |
|---|---|---|
| `task.rs` | `TaskId(Uuid)`, `Task`, `TaskStatus`, `Priority` | Newtype pattern for `TaskId` — the compiler won't let you accidentally pass an `AgentId` where a `TaskId` is expected |
| `agent.rs` | `AgentId(Uuid)`, `AgentTier`, `AgentStatus` | Agents are tiered: Orchestrator, Worker, Utility |
| `message.rs` | Message types for agent communication | |
| `config.rs` | `AppConfig` and nested config structs | |
| `cost.rs` | Token counting and cost tracking | |
| `ticket.rs` | GitHub issue/ticket representation | |
| `prd.rs` | Product requirement document types | |
| `refactor.rs` | Refactor session and change types | |

**Rust lesson:** Enums like `TaskStatus` with variants `Pending`, `InProgress`, `Review`, `Completed`, `Failed` must be exhaustively matched. The compiler forces you to handle every case — no forgotten status transitions.

---

### `src/error.rs` — Error handling

Defines `NexorError` using the `thiserror` crate. Each variant carries context:

```rust
enum NexorError {
    Config { message: String, suggestion: Option<String> },
    Database { message: String, suggestion: Option<String> },
    LlmApi { message: String, suggestion: Option<String> },
    // ...
}
```

Key methods:
- `is_recoverable()` — LLM and GitHub errors are retryable; database errors are not
- `enrich_error()` — converts generic `anyhow::Error` into a domain-specific `NexorError` by inspecting error messages

**Rust lesson:** Rust has two error crate conventions. `thiserror` is for libraries — you define structured error types. `anyhow` is for applications — you just propagate errors with context. Nexor uses both: `thiserror` for the core error enum, `anyhow` in application-level glue code.

---

### `src/config/` — Configuration

Layered config loading:

| File | Purpose |
|---|---|
| `global.rs` | User-level config (`~/.config/nexor/`) |
| `project.rs` | Project-level config (`nexor.toml`) |
| `credentials.rs` | API keys and tokens |
| `validation.rs` | Config validation rules |

`load_config()` merges global + project configs and validates. This is a common Rust pattern — parse, validate, then hand off an immutable config struct.

---

### `src/db/` — Database layer

Uses **SQLx** with PostgreSQL. SQLx is unique in Rust: it verifies your SQL queries *at compile time* against a real database schema.

| File | Purpose |
|---|---|
| `traits.rs` | Repository trait definitions (`MergeQueueRepo`, `ServerRepo`, etc.) |
| `pg_repo.rs` | PostgreSQL implementations of those traits |
| `queries.rs` | Generated SQLx queries |
| `test_utils.rs` | Test database helpers |
| `prd.rs` | PRD persistence |
| `refactor.rs` | Refactor state persistence |

**Rust lesson:** The repository trait pattern here is how Rust does dependency injection. Define a `trait` (interface), implement it for Postgres, and in tests implement it with mocks. No DI framework needed — traits + generics handle it.

---

### `src/llm/` — LLM provider clients

Abstraction over LLM APIs:

| File | Purpose |
|---|---|
| `provider.rs` | Provider trait — the interface all LLM backends implement |
| `anthropic.rs` | Claude API client |
| `types.rs` | Request/response types |
| `retry.rs` | Exponential backoff retry logic |
| `cost.rs` | Token counting and cost calculation |

**Rust lesson:** The provider trait lets you swap LLM backends without changing calling code. Rust traits are resolved at compile time (static dispatch) by default, or at runtime (`dyn Trait`, dynamic dispatch) when you need flexibility. This module likely uses `Box<dyn Provider>` for runtime provider selection.

---

### `src/agents/` — Agent runtime

The brain of the system. Agents are autonomous units that receive tasks and produce results.

| File | Purpose |
|---|---|
| `agent.rs` | Core Agent struct and lifecycle |
| `executor.rs` | Execution engine — runs agent logic |
| `pool.rs` | Agent pool management (pre-allocated agents by tier) |
| `dispatcher.rs` | Routes tasks to available agents |
| `channels.rs` | Tokio MPSC channels for agent-to-agent messaging |
| `protocol.rs` | Communication protocol definitions |
| `roles.rs` | Role templates that define agent personalities |
| `escalation.rs` | When to punt to a human |
| `planner_bot.rs` | Specialized planning agent |

**Rust lesson:** `channels.rs` uses Tokio's MPSC (multi-producer, single-consumer) channels. This is Rust's preferred concurrency primitive — instead of shared mutable state, you send messages between tasks. The ownership system guarantees no data races at compile time.

---

### `src/orchestration/` — Task planning and scheduling

Breaks work down and assigns it:

| File | Purpose |
|---|---|
| `planner.rs` | Decomposes tickets into vertical slices using LLM |
| `dependency.rs` | Tracks task dependencies as a DAG |
| `queue.rs` | Persistent task queue with dependency awareness |
| `router.rs` | Routes tasks to the right agent tier |
| `scheduler.rs` | Work scheduling with preemption support |

The flow: **Planner** breaks a ticket into tasks &rarr; **DependencyTracker** builds the DAG &rarr; **Queue** orders by dependency &rarr; **Router** picks the tier &rarr; **Scheduler** assigns to agents.

---

### `src/execution/` — File, git, and test operations

The hands of the system — how agents interact with the real world:

| File | Purpose |
|---|---|
| `files.rs` | File read/write/create behind a `FileOps` trait |
| `git.rs` | Git operations behind a `GitOps` trait |
| `test_runner.rs` | Runs tests and parses results |
| `sandbox.rs` | Path sandboxing to prevent directory traversal |
| `approval.rs` | Danger classification and approval gates |

**Rust lesson:** `sandbox.rs` demonstrates Rust's security-conscious culture. `is_path_allowed()` validates every file path against the project root before any I/O. The `DangerLevel` enum (`Benign`, `Caution`, `Dangerous`) with `AutonomyLevel`-based gating is a pattern you'll see in safety-critical Rust code.

---

### `src/github/` — GitHub API integration

Full GitHub integration:

| File | Purpose |
|---|---|
| `client.rs` | REST + GraphQL API client |
| `auth.rs` | OAuth / device code authentication |
| `pr.rs` | Pull request operations |
| `comments.rs` | PR/issue comment management |
| `issue_sync.rs` | Sync GitHub issues to internal tickets |
| `merge.rs` | PR merging with conflict detection |
| `merge_queue.rs` | Queue-based merge management |
| `types.rs` | GitHub API types (`GitHubIssue`, `GitHubPullRequest`, etc.) |

---

### `src/server/` — HTTP + WebSocket server

Built on **Axum** (Rust's most popular web framework, built on top of Tower and Hyper):

| File | Purpose |
|---|---|
| `api.rs` | REST endpoint handlers |
| `ws.rs` | WebSocket handler for real-time updates |
| `auth.rs` | Authentication middleware |
| `state.rs` | Shared application state (`Arc<AppState>`) |

Routes: `/health`, `/tasks`, `/agents`, `/config`, `/chat`, `/ws`, plus SPA static file serving.

**Rust lesson:** Axum handlers are just async functions. Axum uses Rust's type system for dependency injection — if your handler takes `State<AppState>`, Axum extracts it automatically. No decorators or magic strings. The middleware is composed using Tower's layer system, which is like stacking middleware in Express but type-checked.

---

### `src/prompts/` — Prompt engineering

Structured prompt construction:

| File | Purpose |
|---|---|
| `builder.rs` | Fluent `PromptBuilder` API |
| `context.rs` | Priority-based context injection |
| `version.rs` | Prompt version tracking for replay |
| `templates/` | Agent-specific prompt templates |
| `schemas/` | JSON schemas for structured LLM output |
| `tools/` | Tool definitions for function calling |
| `examples/` | Few-shot example library |
| `recovery.rs` | Self-correction prompts |

---

### `src/observability/` — Debugging and tracing

| File | Purpose |
|---|---|
| `logging.rs` | LLM call and decision tracing |
| `export.rs` | Session export for post-mortem analysis |
| `replay.rs` | Replay agent decisions step-by-step |

---

### `src/logging.rs` — Structured logging

Uses the `tracing` crate (Rust's structured logging standard). Sets up:
- Console output with filtering
- Daily rotating log files
- Named spans: `agent_span()`, `task_span()`, `llm_span()`, `db_span()`

**Rust lesson:** `tracing` is more than logging — it creates structured spans that propagate through async code. When an agent makes an LLM call inside a task, the log output automatically includes the agent ID, task ID, and LLM model in context. This is critical for debugging concurrent async systems.

---

### `src/headless.rs` — CI/CD mode

Runs tasks without a server. Reads input (JSON array, JSON object, or line-by-line text), executes tasks, writes structured output. Uses `Box<dyn Write + Send>` for output abstraction (stdout or file).

---

### `src/refactor/` — Mid-stream plan changes

Lets users modify active work plans through conversation. Detects intent from messages, proposes changes, and applies them.

---

### `src/cli.rs` — Argument parsing

Uses `clap` (Rust's standard CLI parser). The `#[derive(Parser)]` macro generates all argument parsing from struct field annotations — no manual parsing code.

---

## Data Flow Summary

```
CLI args / HTTP request
        |
        v
  [ Config + Auth ]
        |
        v
  [ Orchestration ]
  Planner --> Dependency Tracker --> Queue --> Router --> Scheduler
        |
        v
  [ Agents ]
  Pool --> Dispatcher --> Executor --> Agent (with Role + Prompts)
        |                                  |
        v                                  v
  [ LLM Providers ]                [ Execution ]
  Anthropic API                    Files, Git, Tests, Sandbox
        |                                  |
        v                                  v
  [ GitHub ]                       [ Database ]
  PRs, Issues, Merge Queue         Task state, Agent state
        |
        v
  [ Observability ]
  Tracing, Export, Replay
```

---

## Key Rust Patterns in This Codebase

1. **Newtype IDs** — `TaskId(Uuid)` prevents mixing up IDs at compile time
2. **Trait-based abstraction** — `FileOps`, `GitOps`, repository traits enable testing without mocks frameworks
3. **Enum state machines** — `TaskStatus`, `AgentStatus` with exhaustive matching
4. **Error enrichment** — structured errors with user-facing suggestions
5. **Channel-based concurrency** — MPSC channels instead of shared mutable state
6. **Builder pattern** — `PromptBuilder` for complex object construction
7. **Layered middleware** — Tower/Axum composition for HTTP concerns
8. **Compile-time guarantees** — SQLx query verification, exhaustive matches, ownership rules

---

## Rust for JavaScript Developers — Using Real Examples from This App

This section maps Rust concepts to their JavaScript equivalents using actual code from the nexor codebase. Written for someone who thinks in JS/TS but has never touched Rust.

---

### Ownership & Borrowing — There Is No Garbage Collector

In JavaScript, you never think about who "owns" a value. Everything is heap-allocated and garbage collected.

Rust has no GC. Every value has exactly **one owner**. When the owner goes out of scope, the value is dropped (freed). If you want to share, you either **borrow** it (a reference) or **clone** it (a copy).

```javascript
// JavaScript — this just works, GC handles it
const config = loadConfig();
startServer(config);
startScheduler(config); // fine, config is still alive
```

```rust
// Rust — config is MOVED into start_server, it's gone after that
let config = load_config()?;
start_server(config);       // config moved here
start_scheduler(config);    // COMPILE ERROR: config was already moved

// Fix 1: Clone it (like structuredClone in JS)
start_server(config.clone());
start_scheduler(config);

// Fix 2: Borrow it (pass a reference — zero cost)
start_server(&config);
start_scheduler(&config);
```

In this codebase, you'll see `Arc<AppConfig>` in `src/server/state.rs`:

```rust
pub struct AppState {
    pub config: Arc<AppConfig>,  // Arc = reference-counted pointer
}
```

`Arc` is like a shared pointer with a reference count (similar to how JS objects work under the hood). Multiple threads can hold an `Arc` to the same config. When the last `Arc` is dropped, the config is freed. This is the escape hatch for when you need shared ownership — but you have to opt into it explicitly.

---

### `Result<T, E>` vs try/catch — Errors Are Values, Not Exceptions

JavaScript throws exceptions. You wrap things in try/catch and hope you caught everything. Rust has no exceptions. Every function that can fail returns `Result<T, E>` — it's either `Ok(value)` or `Err(error)`.

```javascript
// JavaScript
try {
  const pool = await initDb();
} catch (err) {
  console.error("DB failed:", err);
}
```

```rust
// Rust — from src/db/mod.rs
pub async fn init_db() -> Result<PgPool> {
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL not set")?;  // ? = return early if Err

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .context("Failed to connect to database")?;

    sqlx::migrate!().run(&pool).await
        .context("Failed to run migrations")?;

    Ok(pool)
}
```

The `?` operator is the key. It means: "if this is `Err`, return it from this function immediately. If it's `Ok`, unwrap the value and continue." It's like a `throw` that the compiler *forces* you to handle somewhere up the chain. You literally cannot forget to handle an error — the code won't compile.

The `enrich_error()` function in `src/error.rs` pattern-matches on error messages to add user-facing suggestions:

```rust
pub fn enrich_error(error: anyhow::Error) -> NexorError {
    let msg = error.to_string().to_lowercase();

    if msg.contains("rate limit") || msg.contains("429") {
        return NexorError::rate_limited_simple();
    }

    if msg.contains("api key") || msg.contains("unauthorized") {
        return NexorError::api_key_missing("unknown");
    }

    NexorError::Internal { message: error.to_string() }
}
```

---

### `Option<T>` vs null/undefined — No More "Cannot read property of undefined"

JavaScript has `null`, `undefined`, and the billion-dollar mistake. Rust replaces all of that with `Option<T>`: either `Some(value)` or `None`. The compiler forces you to handle both cases.

```javascript
// JavaScript — runtime explosion waiting to happen
const task = tasks.find(t => t.id === id);
console.log(task.name);  // TypeError if not found
```

```rust
// Rust — from src/server/state.rs
pub async fn get_response_stream(&self, message_id: Uuid) -> broadcast::Receiver<StreamChunk> {
    let mut streams = self.response_streams.write().await;

    if let Some(tx) = streams.get(&message_id) {
        // Key exists — subscribe to existing channel
        tx.subscribe()
    } else {
        // Key doesn't exist — create a new one
        let (tx, rx) = broadcast::channel(100);
        streams.insert(message_id, tx);
        rx
    }
}
```

`if let Some(tx) = ...` is Rust's way of saying "if this has a value, bind it to `tx` and run this block." You can also use `match`, `.unwrap()` (crashes if None — like JS), or `.unwrap_or(default)`.

In `src/types/message.rs`, optional fields use `Option`:

```rust
pub struct AgentMessage {
    pub id: MessageId,
    pub from: AgentId,
    pub content: String,
    pub task_id: Option<TaskId>,     // might not be associated with a task
    pub context: Option<TaskContext>, // might not have context
}
```

TypeScript's `taskId?: TaskId` is the closest equivalent, but Rust's version is enforced at every level — serialization, pattern matching, function calls. There's no way to accidentally treat `None` as a value.

---

### Enums — Way More Than JavaScript's Object Constants

JS enums are just string or number constants. Rust enums are **tagged unions** — each variant can carry different data. Combined with `match`, they form state machines.

```javascript
// JavaScript — just labels
const TaskStatus = { PENDING: "pending", IN_PROGRESS: "in_progress", COMPLETED: "completed" };
```

```rust
// Rust — from src/types/task.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TaskStatus {
    #[default]
    Pending,
    InProgress,
    Review,
    Completed,
    Failed,
}
```

Simple enough. But enums can also carry data — from `src/error.rs`:

```rust
pub enum NexorError {
    Config { message: String, suggestion: Option<String> },
    LlmApi { message: String, suggestion: Option<String> },
    TaskFailed { task_id: String, message: String, recoverable: bool },
    Internal { message: String },
}
```

Each variant has different fields. The `match` expression forces you to handle every variant:

```rust
// From src/orchestration/scheduler.rs
match self.try_assign_one().await? {
    AssignResult::Assigned => assigned += 1,
    AssignResult::NoTasks => break,
    AssignResult::NoAgents(tier) => {
        tracing::debug!(tier = ?tier, "Waiting for available agent");
        break;
    }
}
```

If you add a new variant to `AssignResult` later, the compiler flags every `match` that doesn't handle it. This is how Rust prevents the "I added a new status but forgot to handle it in 3 places" class of bugs.

---

### Channels — Pub/Sub Without an Event Emitter

In JavaScript, you'd use EventEmitter, RxJS, or a message queue. Rust uses **channels** from Tokio. A channel has a sender (`tx`) and receiver (`rx`). Multiple senders can push messages; one receiver pulls them out.

From `src/agents/channels.rs`:

```rust
/// Create an agent channel pair
pub fn create_agent_channel(
    buffer_size: usize,
) -> (mpsc::Sender<AgentCommand>, mpsc::Receiver<AgentCommand>) {
    mpsc::channel(buffer_size)
}
```

This creates a **bounded** channel (like a queue with a max size). The sender side gets cloned and handed out:

```rust
#[derive(Clone)]  // Clone = can be duplicated, so multiple producers can hold a sender
pub struct AgentHandle {
    pub agent_id: AgentId,
    pub tier: AgentTier,
    command_tx: mpsc::Sender<AgentCommand>,
}

impl AgentHandle {
    pub async fn send(&self, command: AgentCommand) -> Result<(), mpsc::error::SendError<AgentCommand>> {
        self.command_tx.send(command).await
    }
}
```

The agent on the other end sits in a loop pulling commands from the receiver. The JavaScript equivalent would be:

```javascript
// Conceptual JS equivalent — not real API
const channel = new Channel(bufferSize);

// Producer side (dispatcher)
await channel.send({ type: "execute", task });

// Consumer side (agent) — blocks until a message arrives
while (true) {
    const command = await channel.receive();
    await handle(command);
}
```

The critical difference: Rust channels are **ownership-based**. When you send a value into a channel, it's *moved* — the sender no longer has it. This guarantees no two threads can mutate the same data simultaneously. In JS, you'd pass a reference and pray nobody mutates it.

The codebase also uses `broadcast` channels in `src/server/state.rs` for one-to-many streaming:

```rust
pub struct AppState {
    response_streams: Arc<RwLock<HashMap<Uuid, broadcast::Sender<StreamChunk>>>>,
    pub feed_tx: broadcast::Sender<FeedUpdate>,
}
```

`broadcast::channel` is like an EventEmitter — multiple subscribers each get a copy of every message. MPSC is many-to-one; broadcast is one-to-many.

---

### Traits vs Interfaces — Dependency Injection Without a Framework

TypeScript has interfaces. Rust has traits. They look similar but work differently — traits can have default implementations and are resolved at compile time.

From `src/db/traits.rs`:

```rust
#[async_trait]
pub trait MergeQueueRepo: Send + Sync {
    async fn insert_queue_entry(
        &self, id: Uuid, owner: String, repo: String,
        pr_number: u32, position: u32, now: DateTime<Utc>,
    ) -> Result<(), MergeQueueError>;

    async fn get_next_position(&self, owner: String, repo: String) -> Result<u32, MergeQueueError>;
}
```

Then `src/db/pg_repo.rs` implements it for Postgres. In tests, `#[cfg_attr(test, mockall::automock)]` auto-generates a `MockMergeQueueRepo`. No DI container, no runtime reflection — the compiler wires it all together.

The `Send + Sync` bounds are concurrency markers:
- `Send` = this type can be transferred to another thread
- `Sync` = this type can be referenced from multiple threads

JavaScript doesn't have these concepts because JS is single-threaded. Rust's compiler uses them to prevent data races at compile time.

---

### `async`/`await` — Same Keywords, Different Engine

Rust's `async`/`await` syntax looks identical to JavaScript's, but the runtime model is different.

JS has a built-in event loop. Rust has **no built-in async runtime** — you choose one. This codebase uses **Tokio**, the most common choice.

```rust
// src/main.rs
#[tokio::main]  // boots the Tokio runtime (like starting Node's event loop)
async fn main() -> Result<()> {
    let pool = init_db().await?;
    start_server(pool).await?;
    Ok(())
}
```

From `src/orchestration/scheduler.rs`, the scheduler loop:

```rust
pub async fn run(&self) -> Result<(), SchedulerError> {
    *self.running.write().await = true;

    loop {
        if !*self.running.read().await { break; }

        match self.tick().await {
            Ok(assigned) => {
                if assigned > 0 {
                    tracing::debug!(count = assigned, "Assigned tasks this tick");
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Scheduler tick error");
            }
        }
    }
    Ok(())
}
```

`tokio::select!` is Rust's equivalent of `Promise.race` — from the scheduler:

```rust
tokio::select! {
    _ = self.agent_available.notified() => {
        tracing::debug!("Agent became available, resuming");
    }
    _ = tokio::time::sleep(Duration::from_millis(self.config.agent_wait_timeout_ms)) => {
        // Timeout, will retry on next tick
    }
}
```

Whichever future completes first wins. The other is cancelled. Clean, no callback hell.

---

### `Arc<RwLock<T>>` vs Shared Mutable State

In JavaScript, any code can read or write any object at any time. Rust won't let you. If multiple threads need to read *and* write the same data, you wrap it in `Arc<RwLock<T>>`.

- `Arc` = shared ownership (multiple holders)
- `RwLock` = multiple readers OR one writer (enforced at runtime)

From `src/server/state.rs`:

```rust
pub struct AppState {
    pub scheduler: Option<Arc<RwLock<Scheduler>>>,
    response_streams: Arc<RwLock<HashMap<Uuid, broadcast::Sender<StreamChunk>>>>,
}
```

Reading:
```rust
let running = self.running.read().await;  // non-blocking if no writer holds it
```

Writing:
```rust
let mut streams = self.response_streams.write().await;  // blocks until all readers release
streams.insert(message_id, tx);
```

The mental model: `Arc` is `shared_ptr` from C++ (or just "a JS reference" with explicit refcounting). `RwLock` is "only one writer at a time, readers are free." Together they give you JS-like shared mutable state, but with explicit locking so you can't accidentally race.

---

### The Builder Pattern — Fluent APIs Without Method Chaining Footguns

From `src/prompts/builder.rs`:

```rust
let prompt = PromptBuilder::new()
    .role("senior engineer")
    .task("implement the login endpoint")
    .file_to_modify("src/auth.rs", existing_code)
    .output_json(schema)
    .build();
```

Each method takes `mut self` (ownership) and returns `Self`. This means the builder is consumed and returned at each step — you can't accidentally use a half-built builder. In TypeScript you'd do the same with method chaining, but nothing stops you from using the builder before calling `.build()`.

---

### `#[derive]` — Codegen Instead of Boilerplate

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub Uuid);
```

Each derive generates a trait implementation at compile time:

| Derive | JS Equivalent |
|---|---|
| `Debug` | `console.log` / `util.inspect` output |
| `Clone` | `structuredClone()` |
| `PartialEq`, `Eq` | Custom `===` that works on struct contents, not references |
| `Hash` | Allows use as a `HashMap` key (like `Map` key in JS) |
| `Serialize` / `Deserialize` | `JSON.stringify` / `JSON.parse`, but type-safe and works with any format |

No runtime reflection, no decorators, no `JSON.parse` surprise `undefined` fields. It's all generated at compile time.

---

### Summary — What Rust Gives You Over JavaScript

| Concern | JavaScript | Rust (in this codebase) |
|---|---|---|
| Null safety | `undefined`, `null`, optional chaining | `Option<T>` — compiler-enforced |
| Error handling | try/catch, unhandled rejections | `Result<T, E>` + `?` — every error is tracked |
| Concurrency | Single-threaded + event loop | Multi-threaded + ownership prevents races |
| Shared state | Just mutate anything, YOLO | `Arc<RwLock<T>>` — explicit, safe |
| Pub/sub | EventEmitter, RxJS | MPSC/broadcast channels with ownership transfer |
| Interfaces | TS interfaces (erased at runtime) | Traits (enforced at compile time, used for dispatch) |
| Enums | String constants | Tagged unions with data + exhaustive matching |
| Memory | Garbage collected | Ownership + borrowing, zero-cost |
| DI | Frameworks (InversifyJS, etc.) | Traits + generics, no runtime cost |
