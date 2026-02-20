# Rust Production Best Practices (2024-2026)

> Comprehensive research document for assessing technical debt in a production Axum application.
> Compiled February 2026 from recent blog posts, RustConf talks, and community consensus.

---

## Table of Contents

1. [Module Organization](#1-module-organization)
2. [Error Handling](#2-error-handling)
3. [Generics & Traits](#3-generics--traits)
4. [Reducing Boilerplate](#4-reducing-boilerplate)
5. [Async Patterns](#5-async-patterns)
6. [Type-State Patterns](#6-type-state-patterns)
7. [Testing](#7-testing)
8. [Performance](#8-performance)
9. [Dependency Injection](#9-dependency-injection)
10. [Project Structure for Large Codebases](#10-project-structure-for-large-codebases)

---

## 1. Module Organization

### Recommended Pattern: Named Files Over `mod.rs`

Since Rust 2018+, the preferred convention is to use named files (`module.rs`) instead of `module/mod.rs` directories for simple modules. Reserve directories for modules that have sub-modules.

```
src/
├── lib.rs
├── config.rs              # Simple module: single file
├── db.rs                  # Simple module: single file
├── server/                # Complex module: has sub-modules
│   ├── mod.rs             # Re-exports, declares sub-modules
│   ├── api/
│   │   ├── mod.rs
│   │   ├── agents.rs
│   │   └── tasks.rs
│   └── services/
│       ├── mod.rs
│       ├── agents.rs
│       └── tasks.rs
└── main.rs                # Thin entry point
```

**The `mod.rs` rule:** Use `mod.rs` only when the module has children. A `mod.rs` that is the only file in its directory is a code smell — just use a named file at the parent level.

### Re-exports and the Facade Pattern

Use `pub use` in `mod.rs` to present a curated public API rather than exposing internal structure:

```rust
// src/server/services/mod.rs
mod agents;
mod tasks;

// Re-export only what consumers need
pub use agents::AgentService;
pub use tasks::TaskService;
```

This decouples your internal module structure from your public API. Consumers import from the facade, not deep into your tree.

### Preludes: Use Sparingly or Not At All

The community consensus has shifted **against** prelude modules for application crates. [corrode.dev argues](https://corrode.dev/blog/dont-use-preludes-and-globs/) against preludes with six concrete reasons:

1. **Obscured origins** — harder to trace where types come from
2. **Naming conflicts** — minor version bumps in dependencies can introduce collisions
3. **Security audit difficulty** — opaque imports complicate auditing
4. **Hidden structure** — masks the module hierarchy
5. **IDE reliability** — name resolution becomes fragile with conflicts
6. **Documentation clarity** — obscures imports in examples

**If you must use a prelude**, restrict it to trait re-exports with unique prefixes (e.g., `NexorExt`). Enable the clippy lint `wildcard_imports` to catch accidental glob use.

**Exception:** `use super::*;` in test modules is acceptable and idiomatic.

### Common Anti-Patterns

- **Deep nesting** (4+ levels) — creates import verbosity and cognitive overhead. Flatten where possible.
- **Barrel modules that re-export everything** — defeats the purpose of encapsulation. Only re-export the public API.
- **Mixing `mod.rs` and named files inconsistently** — pick one convention and stick to it.
- **Fat `main.rs`** — business logic belongs in `lib.rs`; `main.rs` should only wire things together.

### Why It Matters at Scale

In a codebase with 130+ API endpoints, sloppy module organization creates three problems: (1) new developers cannot navigate the code, (2) circular dependency risks increase, and (3) IDE autocompletion becomes unreliable. A consistent facade pattern means every module has a predictable public surface.

**Sources:**
- [Module/mod.rs or module.rs? — Rust Forum](https://users.rust-lang.org/t/module-mod-rs-or-module-rs/122653)
- [Don't Use Preludes And Globs — corrode.dev](https://corrode.dev/blog/dont-use-preludes-and-globs/)
- [Rust Module and Crate Organization Best Practices](https://softwarepatternslexicon.com/patterns-rust/5/11/)
- [Rust Modules and Visibility — buildwithrs.dev](https://www.buildwithrs.dev/docs/rust-2025/module)

---

## 2. Error Handling

### Recommended Pattern: `thiserror` for Domain Errors, `anyhow` for Application Glue

The distinction is not "library vs. binary" — it is about **intent**:

- If callers must **match on different failure modes**, define a typed error enum with `thiserror`.
- If callers will only **report/propagate** the error, use `anyhow::Result`.

```rust
// Domain error — callers need to pattern match
#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("step '{step_id}' not found in workflow '{workflow_id}'")]
    StepNotFound {
        step_id: Uuid,
        workflow_id: Uuid,
    },

    #[error("cycle detected in DAG: {0}")]
    CycleDetected(String),

    #[error("port resolution failed for step '{step_id}'")]
    PortResolution {
        step_id: Uuid,
        #[source]
        source: serde_json::Error,
    },

    #[error(transparent)]
    Database(#[from] sqlx::Error),
}
```

```rust
// Application glue — just propagate with context
use anyhow::{Context, Result};

async fn execute_workflow(id: Uuid, state: &AppState) -> Result<()> {
    let workflow = state.repos.workflows
        .find(id)
        .await
        .context("failed to load workflow")?;

    let steps = state.repos.steps
        .find_by_workflow(id)
        .await
        .context("failed to load steps")?;

    run_dag(&workflow, &steps).await
}
```

### Per-Module Error Types (Recommended for Large Codebases)

[GreptimeDB's architecture](https://greptime.com/blogs/2024-05-07-error-rust) demonstrates the gold standard: each sub-crate or major module defines its own error type. Errors chain via `#[source]`, and a virtual stack trace is built without the cost of `std::backtrace::Backtrace`.

```
src/server/
├── hub/error.rs        # HubError — DAG execution failures
├── services/error.rs   # ServiceError — business logic failures
├── api/error.rs        # ApiError — maps to HTTP status codes
└── llm/error.rs        # LlmError — provider failures
```

Each layer converts errors from the layer below:

```rust
// api/error.rs
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error(transparent)]
    Service(#[from] ServiceError),

    #[error("unauthorized")]
    Unauthorized,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match &self {
            ApiError::Service(ServiceError::NotFound { .. }) => {
                (StatusCode::NOT_FOUND, self.to_string()).into_response()
            }
            ApiError::Unauthorized => {
                (StatusCode::UNAUTHORIZED, self.to_string()).into_response()
            }
            _ => {
                tracing::error!(?self, "internal error");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response()
            }
        }
    }
}
```

### Error Message Conventions

From the Rust API Guidelines:
- **Lowercase** sentences without trailing punctuation.
- Each error **describes only itself** — do not recursively format the source.
- Use `#[source]` or `#[from]` to chain, not string concatenation.

```rust
// Good
#[error("failed to parse config file")]

// Bad — don't embed the source error in the message
#[error("failed to parse config file: {source}")]
```

### When to Use `Box<dyn Error>`

Almost never in application code. Use it only at the FFI boundary or when you genuinely need type erasure across crate boundaries and cannot use `anyhow`. In practice, `anyhow::Error` replaces `Box<dyn Error + Send + Sync>` with better ergonomics.

### Common Anti-Patterns

- **One mega error enum for the entire application** — impossible to pattern match meaningfully; every handler sees variants it can never produce.
- **Panicking on recoverable errors** — `unwrap()` in handlers is a crash waiting to happen. Always propagate with `?`.
- **Swallowing errors with `let _ = ...`** — at minimum, log them with `tracing::warn!`.
- **`#[error(transparent)]` on every variant** — loses the ability to add context at each layer.
- **Stringly-typed errors** — `anyhow::anyhow!("something went wrong")` without `.context()` chains is debugging hell.

### Why It Matters at Scale

Error types are your application's failure contract. With 50+ entity types and 130+ endpoints, a monolithic error enum becomes unmaintainable. Per-module error types let each module evolve independently, and the HTTP layer maps errors to status codes in exactly one place.

**Sources:**
- [Error Handling for Large Rust Projects — GreptimeDB](https://greptime.com/blogs/2024-05-07-error-rust)
- [Rust Error Handling: thiserror, anyhow, and When to Use Each](https://momori.dev/posts/rust-error-handling-thiserror-anyhow/)
- [On Error Handling in Rust — Felix Knorr (2025)](https://felix-knorr.net/posts/2025-06-29-rust-error-handling.html)
- [Error Handling In Rust — A Deep Dive — Luca Palmieri](https://lpalmieri.com/posts/error-handling-rust/)
- [A Guide to Error Handling that Just Works](https://bugenzhao.com/2024/04/24/error-handling-1/)

---

## 3. Generics & Traits

### Trait Objects vs. Generics: Decision Framework

| Factor | Use Generics | Use `dyn Trait` |
|--------|-------------|-----------------|
| Heterogeneous collections | No | Yes |
| Binary size sensitivity | Acceptable | Preferred |
| Compile time sensitivity | Acceptable | Preferred |
| Maximum runtime speed | Preferred | Slight overhead (2 dereferences) |
| Need conditional methods via multiple trait bounds | Required | Difficult |
| Dynamic plugin loading | Not possible | Required |

**Default choice: generics.** Switch to trait objects when you need heterogeneous collections, you are behind a network boundary where vtable overhead is negligible, or you need to erase types for a cleaner API.

### `impl Trait` in Argument vs. Return Position

```rust
// Argument position: syntactic sugar for generics (monomorphized)
fn process(items: impl Iterator<Item = Step>) { ... }
// Equivalent to:
fn process<I: Iterator<Item = Step>>(items: I) { ... }

// Return position: opaque type (single concrete type, zero-cost)
fn active_steps(&self) -> impl Iterator<Item = &Step> {
    self.steps.iter().filter(|s| s.is_active())
}
```

**Rust 2024 change:** Return-position `impl Trait` now captures all generic parameters in scope by default. Use `+ use<'a, T>` bounds to restrict which parameters are captured when needed. This is detailed in the [official blog post](https://blog.rust-lang.org/2024/09/05/impl-trait-capture-rules/).

### Associated Types vs. Generic Parameters

**Use associated types when there is exactly one natural implementation per type:**

```rust
// Good — an Iterator has exactly one Item type
trait Iterator {
    type Item;
    fn next(&mut self) -> Option<Self::Item>;
}
```

**Use generic parameters when a type can implement the trait multiple times:**

```rust
// Good — a type might convert From multiple sources
trait From<T> {
    fn from(value: T) -> Self;
}
```

**Rule of thumb:** If you would only ever `impl MyTrait for Foo` once, use associated types. If you would `impl MyTrait<A> for Foo` and `impl MyTrait<B> for Foo`, use generics.

### `where` Clauses for Readability

Move complex bounds to `where` clauses to keep function signatures readable:

```rust
// Hard to read
fn execute<S: ExecutionStrategy + Send + Sync + 'static, P: LLMProvider + Send + Sync>(
    strategy: S, provider: P
) -> Result<()> { ... }

// Preferred
fn execute<S, P>(strategy: S, provider: P) -> Result<()>
where
    S: ExecutionStrategy + Send + Sync + 'static,
    P: LLMProvider + Send + Sync,
{ ... }
```

### Common Anti-Patterns

- **Over-genericizing** — making everything generic when there is only one implementation creates complexity without benefit. If `LLMProvider` will always be `Arc<dyn LLMProvider>`, just accept that type directly.
- **Trait objects for perf-critical inner loops** — vtable dispatch in tight loops is measurable. Use generics or enums there.
- **`dyn Trait` without `Send + Sync`** — in async code, forgetting these bounds leads to confusing compile errors downstream.
- **Generic parameter soup** — if a function has 4+ generic parameters, it is likely doing too much. Refactor.

### Why It Matters at Scale

Every generic parameter multiplied by every concrete type used = monomorphized code in your binary. In an Axum app with many handler functions, unnecessary generics inflate compile times and binary size. Use trait objects at service boundaries (where the overhead is negligible) and generics in hot paths (where every nanosecond counts).

**Sources:**
- [Item 12: Understand the trade-offs between generics and trait objects — Effective Rust](https://www.lurklurk.org/effective-rust/generics.html)
- [Changes to impl Trait in Rust 2024 — Rust Blog](https://blog.rust-lang.org/2024/09/05/impl-trait-capture-rules/)
- [An introduction to advanced Rust traits and generics — Shuttle](https://www.shuttle.dev/blog/2024/04/18/using-traits-generics-rust)
- [On Generics and Associated Types](https://blog.thomasheartman.com/posts/on-generics-and-associated-types/)

---

## 4. Reducing Boilerplate

### Derive Macros: The First Line of Defense

Always derive the maximum useful set. The standard "production derive" block:

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StepConfig {
    pub step_id: Uuid,
    pub mode: ExecutionMode,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}
```

**Derive guidelines:**
- `Debug` — on everything. No exceptions. You will need it in logs.
- `Clone` — on data types that cross async boundaries (needed for `Arc`, channels, etc.).
- `PartialEq` — on types used in assertions or comparisons.
- `Serialize`/`Deserialize` — on all API request/response types and DB row types.
- `Default` — when sensible defaults exist; enables `..Default::default()` spread syntax.

### `#[from]` with thiserror

Combine `#[error]` and `#[from]` to eliminate manual `From` implementations:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("database error")]
    Db(#[from] sqlx::Error),

    #[error("serialization error")]
    Serde(#[from] serde_json::Error),

    #[error("workflow not found: {0}")]
    NotFound(Uuid),
}
```

This generates `impl From<sqlx::Error> for ServiceError` and `impl From<serde_json::Error> for ServiceError` automatically, enabling `?` propagation without explicit conversions.

### Builder Pattern: `bon` (Recommended) or `typed-builder`

For structs with many optional fields, derive a builder instead of writing massive constructor functions:

```rust
use bon::Builder;

#[derive(Builder)]
pub struct ExecutionMetadata {
    pub execution_id: Uuid,
    pub workflow_id: Uuid,
    #[builder(default)]
    pub parent_execution_id: Option<Uuid>,
    #[builder(default)]
    pub retry_count: u32,
    #[builder(default = "ExecutionMode::Single")]
    pub mode: ExecutionMode,
}

// Usage
let meta = ExecutionMetadata::builder()
    .execution_id(exec_id)
    .workflow_id(wf_id)
    .parent_execution_id(parent_id)
    .build();
```

**`bon` vs. `typed-builder` vs. `derive_builder`:**

| Crate | Compile-time checked | Builder state encoding | Function builders |
|-------|---------------------|----------------------|-------------------|
| `bon` | Yes | Typestate in generics | Yes (unique feature) |
| `typed-builder` | Yes | Typestate in generics | No |
| `derive_builder` | No (runtime `Result`) | None | No |

`bon` is the newest and most actively developed (used by crates.io backend, tantivy, apache-avro). It supports builders for both structs and functions, which is unique. `typed-builder` is the mature alternative. Avoid `derive_builder` for new code — runtime checking defeats the purpose.

### `From`/`Into` Conversions

**Always implement `From`, never `Into`** — the blanket implementation provides `Into` for free:

```rust
impl From<WorkflowStepRow> for StepConfig {
    fn from(row: WorkflowStepRow) -> Self {
        StepConfig {
            step_id: row.id,
            mode: row.execution_mode.parse().unwrap_or_default(),
            timeout_secs: row.timeout_secs,
        }
    }
}

// Both work:
let config: StepConfig = row.into();
let config = StepConfig::from(row);
```

**Use `TryFrom` when conversion can fail** — never implement `From` with a panic path:

```rust
impl TryFrom<&str> for ExecutionMode {
    type Error = ParseError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "single" => Ok(Self::Single),
            "workforce" => Ok(Self::Workforce),
            _ => Err(ParseError::InvalidMode(s.to_owned())),
        }
    }
}
```

### Common Anti-Patterns

- **Manual `Display` implementations** when `#[error("...")]` suffices — pure noise.
- **Constructing structs with 10+ positional arguments** instead of using a builder or `Default` + field override.
- **Implementing `Into` directly** — always implement `From` instead.
- **`From` implementations that panic** — use `TryFrom` for fallible conversions.
- **Duplicating struct definitions** for request/response types when `#[serde(skip)]` or a view type would work.

### Why It Matters at Scale

With 50+ entity types and 18+ construction sites for `WorkflowStepRow` alone, every field added to a struct creates a ripple of updates. Builders absorb this cost: new optional fields get defaults, and existing call sites remain unchanged.

**Sources:**
- [bon crate documentation](https://docs.rs/bon/latest/bon/)
- [typed-builder — lib.rs](https://lib.rs/crates/typed-builder)
- [Item 7: Use builders for complex types — Effective Rust](https://effective-rust.com/builders.html)
- [Item 5: Understand type conversions — Effective Rust](https://effective-rust.com/casts.html)
- [Rethinking Builders with Lazy Generics (2024)](https://geo-ant.github.io/blog/2024/rust-rethinking-builders-lazy-generics/)

---

## 5. Async Patterns

### Native `async fn` in Traits (Rust 1.75+)

As of Rust 1.75 (stable December 2023), `async fn` in traits is supported natively for **static dispatch**:

```rust
trait ExecutionStrategy {
    async fn execute(&self, ctx: &ExecutionContext) -> Result<StepOutput>;

    async fn on_complete(&self, output: &StepOutput) -> Result<()> {
        // Default implementation
        Ok(())
    }
}
```

**Critical limitation:** `async fn` in traits does NOT support dynamic dispatch (`dyn Trait`). Traits with `async fn` are not object-safe. If you need `Arc<dyn ExecutionStrategy>`, you still need one of these workarounds:

1. **`#[async_trait]` from dtolnay** — returns `Pin<Box<dyn Future>>`, small heap allocation per call:

```rust
#[async_trait::async_trait]
trait ExecutionStrategy: Send + Sync {
    async fn execute(&self, ctx: &ExecutionContext) -> Result<StepOutput>;
}

// Can now use: Arc<dyn ExecutionStrategy>
```

2. **`trait-variant` crate** — generates a separate `dyn`-compatible version automatically.

3. **Manual desugaring** — return `Pin<Box<dyn Future + Send + '_>>` explicitly.

The Rust team is working on native dyn async trait support but it is [not yet stabilized as of early 2026](https://rust-lang.github.io/async-fundamentals-initiative/explainer/async_fn_in_dyn_trait.html).

### Tokio Best Practices

**Spawn tasks for work that must complete regardless of the caller:**

```rust
// If the HTTP connection drops, the task still runs
let handle = tokio::spawn(async move {
    execute_workflow(workflow_id, state).await
});

// Don't await immediately if you want fire-and-forget
// But DO store the handle if you need the result
```

**Use `tokio::select!` carefully** (see Cancellation Safety below):

```rust
// CORRECT: reuse pinned futures across loop iterations
let mut shutdown = pin!(shutdown_signal());
let mut interval = tokio::time::interval(Duration::from_secs(30));

loop {
    tokio::select! {
        _ = interval.tick() => {
            run_consistency_scan().await;
        }
        _ = &mut shutdown => {
            tracing::info!("shutting down");
            break;
        }
    }
}
```

**Prefer channels over shared mutexes:**

```rust
// Actor pattern — each piece of mutable state has one owner
let (tx, mut rx) = tokio::sync::mpsc::channel::<Command>(100);

tokio::spawn(async move {
    let mut state = InternalState::new();
    while let Some(cmd) = rx.recv().await {
        match cmd {
            Command::Update(data) => state.apply(data),
            Command::Query(reply_tx) => { let _ = reply_tx.send(state.snapshot()); }
        }
    }
});
```

### Cancellation Safety: The Most Subtle Async Problem

[Rain Goswami's RustConf 2025 talk](https://sunshowers.io/posts/cancelling-async-rust/) introduces a critical distinction:

- **Cancel safety** — a local property: dropping a future has no side effects.
- **Cancel correctness** — a global property: system invariants hold despite cancellations.

**Key insight:** Cancel correctness bugs require three simultaneous conditions: (1) cancel-unsafe futures exist, (2) they are actually cancelled, and (3) cancellation violates a system property. Break any one condition to prevent the bug.

**Concrete `select!` pitfalls:**

```rust
// BUG: recv() is cancel-safe, but lock().await is NOT
loop {
    tokio::select! {
        msg = rx.recv() => handle(msg),
        _ = shutdown.recv() => break,
    }
}

// If shutdown fires while lock().await is pending inside handle(),
// the lock acquisition is cancelled and you lose your place in the queue
```

**Solutions:**

1. **Spawn tasks** for cancel-unsafe work — tasks are runtime-driven and survive parent cancellation.
2. **Use `reserve()` + `send()`** instead of direct `send()` for MPSC channels.
3. **Pin futures outside the loop** to preserve their state across iterations.
4. **Break operations into cancel-safe segments** — acquire resources (cancellable), then use them (spawned task).

### `Send + Sync` Bounds

In Tokio's multi-threaded runtime, futures must be `Send`. This propagates to everything captured by an async block:

```rust
// This fails if `state` is not Send
tokio::spawn(async move {
    state.do_something().await
});

// Fix: ensure your state types derive Send + Sync, or use Arc
```

**Common `Send` blockers:**
- `Rc<T>` — use `Arc<T>` instead.
- `RefCell<T>` — use `tokio::sync::Mutex<T>` or the actor pattern.
- `*mut T` / raw pointers — wrap in a `Send`-safe newtype.
- Non-`Send` futures from third-party crates — spawn on `spawn_local` or restructure.

### Common Anti-Patterns

- **Holding `MutexGuard` across `.await` points** — blocks the executor and can deadlock. Extract the value before awaiting.
- **`tokio::spawn` without JoinHandle management** — fire-and-forget tasks that panic silently. At minimum, log errors from spawned tasks.
- **Creating new futures inside `select!` loops** — state is lost on every iteration when the other branch wins.
- **Missing timeouts on external calls** — always wrap network/DB calls in `tokio::time::timeout`.
- **`block_in_place` or `spawn_blocking` for CPU-bound work without bounded concurrency** — can starve the runtime. Use a semaphore or dedicated thread pool.

### Why It Matters at Scale

An Axum server handling concurrent workflow executions, WebSocket connections, and LLM streaming responses is a cancellation minefield. Every `select!` in the DAG executor, every WebSocket disconnect, every HTTP timeout is a potential cancellation point. Understanding the futures-vs-tasks distinction prevents data loss in mid-execution workflows.

**Sources:**
- [Cancelling Async Rust — Rain Goswami (RustConf 2025)](https://sunshowers.io/posts/cancelling-async-rust/)
- [Rust async in practice: tokio::select!, actor pattern & cancel safety](https://developerlife.com/2024/07/10/rust-async-cancellation-safety-tokio/)
- [The Evolution of Async Rust — JetBrains (2026)](https://blog.jetbrains.com/rust/2026/02/17/the-evolution-of-async-rust-from-tokio-to-high-level-applications/)
- [Async Rust is about concurrency, not (just) performance — Kobzol (2025)](https://kobzol.github.io/rust/2025/01/15/async-rust-is-about-concurrency.html)
- [Announcing async fn in traits — Rust Blog](https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits.html)
- [Dyn Async Traits, Part 10 — baby steps (2025)](https://smallcultfollowing.com/babysteps/blog/2025/03/24/box-box-box/)

---

## 6. Type-State Patterns

### Recommended Pattern: Encode State Machines in the Type System

The typestate pattern uses generic type parameters (usually zero-sized marker types) to represent states at compile time. Invalid state transitions become compile errors.

```rust
use std::marker::PhantomData;

// State markers — zero-sized, no runtime cost
struct Draft;
struct Validated;
struct Executing;
struct Completed;

struct Workflow<State> {
    id: Uuid,
    name: String,
    steps: Vec<Step>,
    _state: PhantomData<State>,
}

impl Workflow<Draft> {
    fn new(name: String) -> Self {
        Workflow {
            id: Uuid::new_v4(),
            name,
            steps: Vec::new(),
            _state: PhantomData,
        }
    }

    fn add_step(mut self, step: Step) -> Self {
        self.steps.push(step);
        self
    }

    fn validate(self) -> Result<Workflow<Validated>, ValidationError> {
        // Check for cycles, missing ports, etc.
        validate_dag(&self.steps)?;
        Ok(Workflow {
            id: self.id,
            name: self.name,
            steps: self.steps,
            _state: PhantomData,
        })
    }
}

impl Workflow<Validated> {
    fn execute(self, engine: &ExecutionEngine) -> Workflow<Executing> {
        engine.start(self.id);
        Workflow {
            id: self.id,
            name: self.name,
            steps: self.steps,
            _state: PhantomData,
        }
    }
}

impl Workflow<Executing> {
    fn complete(self, result: ExecutionResult) -> Workflow<Completed> {
        Workflow {
            id: self.id,
            name: self.name,
            steps: self.steps,
            _state: PhantomData,
        }
    }
}

// Compile error: cannot call execute() on a Draft workflow
// let bad = Workflow::new("test".into()).execute(&engine);
```

### Real-World Examples in the Ecosystem

- **serde** — the `Deserializer` trait models a complex state machine via typestates for parsing.
- **hyper** — HTTP request/response builders use typestates to ensure headers are set before body.
- **Tower `ServiceBuilder`** — layers are composed via type-level accumulation.

### PhantomData Guidelines

```rust
// PhantomData tells the compiler about type relationships
// without storing data at runtime
struct TypedId<T> {
    raw: Uuid,
    _marker: PhantomData<T>,  // Zero bytes at runtime
}

// Different ID types that cannot be mixed
type AgentId = TypedId<Agent>;
type WorkflowId = TypedId<Workflow>;

// Compile error: expected WorkflowId, got AgentId
fn load_workflow(id: WorkflowId) -> Workflow { ... }
```

### When NOT to Use Typestates

- When state transitions are data-driven at runtime (e.g., user input determines next state).
- When the state set is large or dynamic — the combinatorial explosion of types becomes unmanageable.
- When you need to store objects of different states in the same collection — use an enum instead.

### Common Anti-Patterns

- **State enums with runtime `panic!` on invalid transitions** — if the set of valid transitions is known at compile time, use typestates to eliminate the runtime check.
- **`PhantomData` without clear documentation** — always comment why the phantom type parameter exists.
- **Overly complex typestate hierarchies** — if the state machine has 10+ states, the compile error messages become cryptic. Consider a hybrid approach: typestates for the major phases, runtime checks for sub-states.

### Why It Matters at Scale

In a DAG execution engine, workflows pass through well-defined phases (created, validated, executing, completed, failed). Encoding these as types means "execute an unvalidated workflow" is a compile error, not a runtime bug discovered in production at 3am.

**Sources:**
- [The Typestate Pattern in Rust — Cliffle](https://cliffle.com/blog/rust-typestate/)
- [Generic Finite State Machines with Rust's Type State Pattern](https://medium.com/@alfred.weirich/generic-finite-state-machines-with-rusts-type-state-pattern-04593bba34a8)
- [Typestate Pattern in Rust — Software Patterns Lexicon](https://softwarepatternslexicon.com/patterns-rust/5/17/)
- [Zero Cost Abstractions — Embedded Rust Book](https://doc.rust-lang.org/beta/embedded-book/static-guarantees/zero-cost-abstractions.html)
- [How to Use PhantomData in Rust](https://oneuptime.com/blog/post/2026-01-25-rust-phantomdata/view)

---

## 7. Testing

### Integration Test Organization

```
src/
├── server/
│   ├── hub/
│   │   ├── dag/
│   │   │   ├── mod.rs
│   │   │   └── tests.rs      # Unit tests: colocated
│   │   └── mod.rs
│   └── services/
│       ├── agents/
│       │   ├── mod.rs
│       │   └── tests.rs      # Unit tests: colocated
│       └── mod.rs
tests/
├── api/                        # Integration tests: separate
│   ├── workflow_execution.rs
│   ├── agent_crud.rs
│   └── common/
│       ├── mod.rs              # Shared test helpers
│       └── fixtures.rs         # Shared test data
└── integration_test.rs         # Or one file per feature
```

**Key principle:** Unit tests (testing internal logic) go in colocated `tests.rs` files. Integration tests (testing the public API, database, HTTP layer) go in the top-level `tests/` directory.

### Test Utilities and Fixtures

Create reusable test helpers rather than duplicating setup code:

```rust
// tests/common/mod.rs
pub struct TestApp {
    pub state: AppState,
    pub db: PgPool,
}

impl TestApp {
    pub async fn new() -> Self {
        let db = setup_test_database().await;
        let state = AppState::new(db.clone()).await;
        Self { state, db }
    }

    pub async fn create_workflow(&self) -> WorkflowRow {
        self.state.repos.workflows
            .create(&NewWorkflow {
                name: "test-workflow".into(),
                ..Default::default()
            })
            .await
            .unwrap()
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        // Cleanup test database
    }
}
```

```rust
// tests/common/fixtures.rs
pub fn make_step(name: &str) -> WorkflowStepRow {
    WorkflowStepRow {
        id: Uuid::new_v4(),
        name: name.to_string(),
        execution_mode: "single".to_string(),
        ..Default::default()
    }
}

pub fn make_edge(from: Uuid, to: Uuid) -> StepEdgeRow {
    StepEdgeRow {
        from_step_id: from,
        to_step_id: to,
        ..Default::default()
    }
}
```

### Mocking: `mockall` vs. Manual Mocks

**`mockall` — use for traits with many methods or complex expectations:**

```rust
#[cfg_attr(test, mockall::automock)]
trait LLMProvider: Send + Sync {
    async fn send_message(&self, req: LLMRequest) -> Result<LLMResponse>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execution_calls_provider() {
        let mut mock = MockLLMProvider::new();
        mock.expect_send_message()
            .times(1)
            .returning(|_| Ok(LLMResponse { content: "done".into(), ..Default::default() }));

        let engine = ExecutionEngine::new(Arc::new(mock));
        let result = engine.run(&context).await;
        assert!(result.is_ok());
    }
}
```

**Manual mocks — use for simple traits or when you need full control:**

```rust
struct StubProvider {
    responses: Vec<LLMResponse>,
}

impl LLMProvider for StubProvider {
    async fn send_message(&self, _req: LLMRequest) -> Result<LLMResponse> {
        Ok(self.responses[0].clone())
    }
}
```

**Decision criteria:**
- Fewer than 3 methods, simple expectations? Manual mock.
- Many methods, need to verify call counts/arguments? `mockall`.
- Need to test against real infrastructure? Use testcontainers for Postgres, etc.

### Property-Based Testing with `proptest`

Use proptest for functions with complex input domains where you cannot enumerate all edge cases:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn topological_sort_is_stable(
        edges in prop::collection::vec((0u32..20, 0u32..20), 0..50)
    ) {
        let graph = build_graph(&edges);
        if let Ok(sorted) = topological_sort(&graph) {
            // Property: every edge (u, v) has u before v in sorted output
            for (u, v) in &edges {
                if let (Some(pu), Some(pv)) = (sorted.iter().position(|n| n == u),
                                                 sorted.iter().position(|n| n == v)) {
                    prop_assert!(pu < pv, "edge ({}, {}) violated: {} >= {}", u, v, pu, pv);
                }
            }
        }
    }
}
```

**When to use proptest:**
- Serialization roundtrips (encode then decode = identity).
- Parsers (no input should panic).
- Graph algorithms (topological sort properties, cycle detection correctness).
- Numerical code (commutativity, associativity).

### Snapshot Testing with `insta`

For complex output that is tedious to assert field-by-field:

```rust
#[test]
fn test_error_display() {
    let err = WorkflowError::CycleDetected("A -> B -> A".into());
    insta::assert_snapshot!(err.to_string(), @"cycle detected in DAG: A -> B -> A");
}
```

### Common Anti-Patterns

- **Tests that depend on execution order** — each test must be independent. Use unique IDs and isolated database transactions.
- **Testing private implementation details** — test behavior through the public API. If you need to test internals, the abstraction boundary is wrong.
- **Mock-heavy tests** — if you mock everything, you are testing the mocking framework, not your code. Prefer integration tests with real databases.
- **Skipped or `#[ignore]`d tests** — every test must pass or be removed. Skipped tests rot.
- **Shared mutable state between tests** — use per-test setup or `#[serial]` from the `serial_test` crate.

### Why It Matters at Scale

With DAG orchestration, variable interpolation, port resolution, and topological sorting, the combinatorial space of possible inputs is vast. Unit tests catch known cases; property-based tests find the unknown unknowns. Integration tests against real Postgres catch SQL bugs that mock-based tests will never find.

**Sources:**
- [Rust Testing Mastery — Luis Soares](https://www.linkedin.com/pulse/rust-testing-mastery-from-basics-best-practices-luis-soares-m-sc-)
- [Property Testing Stateful Code in Rust (2024)](https://rtpg.co/2024/02/02/property-testing-with-imperative-rust/)
- [Rust Testing with Mocks and Stubs — Software Patterns Lexicon](https://softwarepatternslexicon.com/rust/testing-and-quality-assurance/mocks-and-stubs-with-mockall-and-double/)
- [Complete Guide to Rust Testing — Blackwell Systems](https://blog.blackwell-systems.com/posts/rust-testing-comprehensive-guide/)

---

## 8. Performance

### `Cow<str>`: Borrow When You Can, Own When You Must

`Cow` (clone-on-write) avoids allocations when data can be borrowed:

```rust
use std::borrow::Cow;

fn format_step_name(name: &str) -> Cow<'_, str> {
    if name.contains(' ') {
        // Must allocate to transform
        Cow::Owned(name.replace(' ', "_"))
    } else {
        // Zero-cost: just borrows the input
        Cow::Borrowed(name)
    }
}
```

**Best use cases:**
- Functions that sometimes need to transform their input and sometimes pass it through.
- Deserialization with `#[serde(borrow)]` for zero-copy parsing.
- Config values that are usually static strings but occasionally runtime-computed.

### Avoiding Unnecessary Allocations

**String concatenation:**

```rust
// Bad: multiple allocations
let msg = "step ".to_string() + &step_id.to_string() + " failed";

// Good: single allocation with format!
let msg = format!("step {} failed", step_id);

// Better for hot paths: write to a pre-allocated buffer
use std::fmt::Write;
let mut buf = String::with_capacity(64);
write!(buf, "step {} failed", step_id).unwrap();
```

**Vec operations:**

```rust
// Bad: unknown initial capacity, multiple reallocations
let mut results = Vec::new();
for step in steps {
    results.push(execute(step).await?);
}

// Good: pre-allocate
let mut results = Vec::with_capacity(steps.len());
for step in steps {
    results.push(execute(step).await?);
}

// Also good: collect with size hint
let results: Vec<_> = steps.iter().map(|s| process(s)).collect();
```

### `SmallVec` and `ArrayVec`

**`SmallVec<[T; N]>`** — stores up to N elements inline on the stack, spills to heap after:

```rust
use smallvec::SmallVec;

// Most steps have 1-3 input ports; avoid heap allocation for the common case
fn resolve_ports(step: &Step) -> SmallVec<[PortValue; 4]> {
    let mut ports = SmallVec::new();
    for port in &step.input_ports {
        ports.push(resolve_port(port));
    }
    ports
}
```

**When to use:**
- You have profiling data showing allocation overhead in hot paths.
- The common case is a small, known number of elements.
- `ArrayVec` is better when you have a hard upper bound and want zero heap usage.

**Caveat:** SmallVec has slightly higher per-operation overhead than Vec due to the inline/heap check. Always benchmark; do not assume it is faster.

### `bytes::Bytes` for Network Code

```rust
use bytes::Bytes;

// Bytes is a reference-counted, zero-copy byte buffer
// Slicing creates a new handle to the same backing storage
fn parse_llm_response(raw: Bytes) -> Result<LLMResponse> {
    let header = raw.slice(0..8);     // No copy
    let body = raw.slice(8..);        // No copy
    serde_json::from_slice(&body)
        .map_err(|e| anyhow::anyhow!("parse error: {e}"))
}
```

Use `Bytes` when:
- Passing data between Tokio tasks (it is `Send + Sync + Clone`).
- Slicing large buffers without copying (WebSocket frames, HTTP bodies).
- The same data is consumed by multiple readers.

### Zero-Copy Deserialization with Serde

```rust
#[derive(serde::Deserialize)]
struct LLMMessage<'a> {
    #[serde(borrow)]
    role: &'a str,          // Borrows from the input buffer
    #[serde(borrow)]
    content: Cow<'a, str>,  // Borrows when possible, owns when escape sequences require it
}
```

This avoids allocating a new `String` for each field when the JSON input is already in memory.

### Stack vs. Heap Decision Framework

| Condition | Stack | Heap |
|-----------|-------|------|
| Size known at compile time, small (<= ~1KB) | Preferred | |
| Size unknown or large | | Preferred |
| Needs to outlive current scope | | `Box<T>`, `Arc<T>` |
| Shared across threads | | `Arc<T>` |
| Recursive data structures | | `Box<T>` required |
| Performance-critical inner loop | Preferred | Measure first |

### Common Anti-Patterns

- **`.clone()` as a first resort** — audit clones in hot paths. Often a borrow or `Cow` suffices.
- **`String` where `&str` works** — function parameters that only read a string should take `&str`, not `String`.
- **`Vec<u8>` for network buffers** — use `bytes::Bytes` for zero-copy slicing.
- **Premature optimization** — do not use `SmallVec` everywhere. Profile first, optimize where measurements indicate.
- **`.to_string()` / `.to_owned()` in loops** — if you are allocating the same string 1000 times, hoist it outside the loop.

### Why It Matters at Scale

An LLM orchestration platform processes potentially large payloads (model responses, streaming chunks, workflow state) across many concurrent requests. Unnecessary allocations in the hot path (message building, port resolution, variable interpolation) directly impact latency and memory pressure under load.

**Sources:**
- [Heap Allocations — The Rust Performance Book](https://nnethercote.github.io/perf-book/heap-allocations.html)
- [How to Optimize Rust Memory Usage and Prevent Allocation Bottlenecks](https://oneuptime.com/blog/post/2026-01-07-rust-memory-optimization/view)
- [Zero-Copy in Rust: Challenges and Solutions](https://coinsbench.com/zero-copy-in-rust-challenges-and-solutions-c0d38a6468e9)
- [SmallVec Rust Guide (2025)](https://generalistprogrammer.com/tutorials/smallvec-rust-crate-guide)
- [Working with Bytes in Rust: Vec<u8>, Cow, and Zero-Copy APIs](https://medium.com/@adamszpilewicz/working-with-bytes-in-rust-vec-u8-cow-and-zero-copy-apis-efbbad0c3450)

---

## 9. Dependency Injection

### Recommended Pattern: Trait-Based DI with `Arc<dyn Trait>`

The idiomatic Rust approach to dependency injection uses traits for abstraction and `Arc<dyn Trait>` for runtime polymorphism:

```rust
// Define the contract
#[async_trait::async_trait]
pub trait WorkflowRepository: Send + Sync {
    async fn find(&self, id: Uuid) -> Result<WorkflowRow>;
    async fn create(&self, input: &NewWorkflow) -> Result<WorkflowRow>;
    async fn list_by_org(&self, org_id: Uuid) -> Result<Vec<WorkflowRow>>;
}

// Production implementation
pub struct PgWorkflowRepository {
    pool: PgPool,
}

#[async_trait::async_trait]
impl WorkflowRepository for PgWorkflowRepository {
    async fn find(&self, id: Uuid) -> Result<WorkflowRow> {
        sqlx::query_as!(WorkflowRow, "SELECT * FROM workflows WHERE id = $1", id)
            .fetch_one(&self.pool)
            .await
            .map_err(Into::into)
    }
    // ...
}

// Wire it together in AppState
pub struct Repos {
    pub workflows: Arc<dyn WorkflowRepository>,
    pub steps: Arc<dyn StepRepository>,
    pub agents: Arc<dyn AgentRepository>,
}

impl Repos {
    pub fn new(pool: PgPool) -> Self {
        Self {
            workflows: Arc::new(PgWorkflowRepository::new(pool.clone())),
            steps: Arc::new(PgStepRepository::new(pool.clone())),
            agents: Arc::new(PgAgentRepository::new(pool)),
        }
    }

    // Test constructor with mock implementations
    #[cfg(test)]
    pub fn with_mocks(
        workflows: impl WorkflowRepository + 'static,
        steps: impl StepRepository + 'static,
        agents: impl AgentRepository + 'static,
    ) -> Self {
        Self {
            workflows: Arc::new(workflows),
            steps: Arc::new(steps),
            agents: Arc::new(agents),
        }
    }
}
```

### Compile-Time vs. Runtime Polymorphism

| Approach | Syntax | Performance | Flexibility |
|----------|--------|-------------|-------------|
| Generics (`T: Trait`) | `struct Engine<P: Provider>` | Zero-cost dispatch | Type fixed at compile time |
| Trait objects (`Arc<dyn Trait>`) | `struct Engine { provider: Arc<dyn Provider> }` | Vtable overhead (~2ns) | Swappable at runtime |
| Enum dispatch | `enum Provider { Anthropic(..), Ollama(..) }` | No vtable, branch prediction | Closed set of variants |

**For web services:** `Arc<dyn Trait>` is almost always the right choice. The vtable overhead is dwarfed by network I/O, and the flexibility for testing and runtime configuration is invaluable.

**For hot inner loops:** Use generics or enum dispatch. If you know the full set of implementations, enum dispatch avoids both vtable overhead and monomorphization bloat:

```rust
enum LLMProvider {
    Anthropic(AnthropicProvider),
    Ollama(OllamaProvider),
    NoOp(NoOpProvider),
}

impl LLMProvider {
    async fn send_message(&self, req: LLMRequest) -> Result<LLMResponse> {
        match self {
            Self::Anthropic(p) => p.send_message(req).await,
            Self::Ollama(p) => p.send_message(req).await,
            Self::NoOp(p) => p.send_message(req).await,
        }
    }
}
```

### AppState in Axum: The Recommended Shape

```rust
// Inner state holds all dependencies
pub struct AppStateInner {
    pub db: PgPool,
    pub repos: Repos,
    pub events: EventBus,
    pub providers: ProviderRegistry,
    pub config: AppConfig,
    pub cancellation_tokens: DashMap<Uuid, CancellationToken>,
}

// Outer wrapper is cheap to clone
#[derive(Clone)]
pub struct AppState(pub Arc<AppStateInner>);

impl std::ops::Deref for AppState {
    type Target = AppStateInner;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Handler extraction
async fn list_workflows(
    State(state): State<AppState>,
) -> Result<Json<Vec<WorkflowRow>>, ApiError> {
    let workflows = state.repos.workflows.list_by_org(org_id).await?;
    Ok(Json(workflows))
}
```

Axum's `State` extractor requires `Clone`. Wrapping in `Arc` makes cloning O(1).

### Common Anti-Patterns

- **Concrete types everywhere, mocked with `cfg(test)` conditionals** — fragile, requires recompilation to switch implementations.
- **DI frameworks/containers** — Rust does not need Spring-style containers. Constructor injection with traits is simpler and more explicit.
- **`Arc<Mutex<dyn Trait>>`** — if the trait methods take `&self`, you do not need the mutex. `Arc<dyn Trait>` suffices for read-only access.
- **Passing `AppState` to every function** — extract the specific dependency you need. Functions should depend on `&dyn WorkflowRepository`, not the entire application state.
- **`Box<dyn Trait>` when `Arc<dyn Trait>` is needed** — in async code, you almost always need shared ownership across tasks.

### Why It Matters at Scale

With 130+ endpoints and multiple execution strategies, every service and handler needs access to repositories, providers, and configuration. Trait-based DI makes each component testable in isolation. Swapping the LLM provider (Anthropic, Ollama, NoOp) becomes a configuration choice, not a code change.

**Sources:**
- [Rust traits and dependency injection — jmmv.dev](https://jmmv.dev/2022/04/rust-traits-and-dependency-injection.html)
- [Dynamic Dispatch and Dependency Injection with Trait Objects — Leapcell](https://leapcell.io/blog/dynamic-dispatch-and-dependency-injection-with-trait-objects-in-rust-web-services)
- [Master Hexagonal Architecture in Rust — howtocodeit.com](https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust)
- [Building Production Web Services with Rust and Axum (2026)](https://dasroot.net/posts/2026/01/building-production-web-services-rust-axum/)
- [Rust runtime type selection for dependency injection (2025)](https://owengage.com/writing/2025-06-11-rust-runtime-type-selection-for-dependency-injection/)

---

## 10. Project Structure for Large Codebases

### Workspace vs. Single Crate: Decision Framework

| Factor | Single Crate | Workspace |
|--------|-------------|-----------|
| Codebase size | < 50k LOC | > 50k LOC or clear module boundaries |
| Build time | Fast enough | Needs parallel compilation |
| Team size | 1-3 developers | 4+ developers |
| Shared dependencies | All shared | Each crate declares its own |
| Refactoring | Easy (everything is visible) | Requires maintaining crate interfaces |

**Practical rule:** If you have more than two logical modules that share dependencies or need to be versioned together, a workspace is justified. However, splitting too aggressively can hurt build times due to cargo resolution overhead.

### Recommended Workspace Layout

```
nexor/
├── Cargo.toml              # [workspace] members
├── crates/
│   ├── nexor-core/         # Shared types: Uuid, row types, error types
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── nexor-db/           # Database: repos, queries, migrations
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── nexor-llm/          # LLM providers: Anthropic, Ollama, Grok
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── nexor-hub/          # Execution engine, strategies, DAG
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   └── nexor-server/       # Axum: API handlers, services, WS
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           └── lib.rs
├── frontend/
└── cli/
```

**Dependency graph flows one direction:**
```
nexor-server → nexor-hub → nexor-llm → nexor-core
                         → nexor-db  → nexor-core
```

### Feature Flags: Guidelines

```toml
[features]
default = ["anthropic", "ollama"]
anthropic = ["reqwest"]
ollama = ["reqwest"]
grok = ["reqwest"]           # Opt-in: not all deployments need Grok
container = ["bollard"]      # Opt-in: Docker container execution
full = ["anthropic", "ollama", "grok", "container"]
```

**Feature flag rules:**
1. **Default features cover 80% of use cases** without additional configuration.
2. **Name by capability**, not implementation (`container`, not `bollard-support`).
3. **Avoid feature explosion** — if you have more than 10 features, reconsider your architecture.
4. **Test with `--no-default-features`** to catch feature-gating bugs.
5. **Additive only** — features should only add functionality, never remove it.

### Build Time Optimization

Concrete techniques with measured impact:

1. **Use a faster linker:**

```toml
# .cargo/config.toml
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]

[target.x86_64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

[target.aarch64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
```

2. **`cargo check` over `cargo build`** for development — 2-3x faster, skips code generation.

3. **Minimize dependency features:**

```toml
# Bad: pulls in everything
tokio = { version = "1", features = ["full"] }

# Good: only what you use
tokio = { version = "1", features = ["rt-multi-thread", "macros", "net", "time", "signal"] }
```

4. **Use `cargo-features-manager`** to audit unused dependency features. Disabling `bindgen`'s `clap` feature alone saved [~13s per build in one project](https://corrode.dev/blog/tips-for-faster-rust-compile-times/).

5. **Parallel frontend for nightly:** `-Z threads=8` can reduce compilation by up to 50%.

6. **sccache** — effective on shared CI build servers where multiple projects share dependency versions. Minimal benefit for single-project local development.

### Conditional Compilation

```rust
// Feature-gated modules
#[cfg(feature = "anthropic")]
pub mod anthropic;

#[cfg(feature = "container")]
pub mod container;

// Runtime feature detection in provider registry
pub fn build_registry(config: &AppConfig) -> ProviderRegistry {
    let mut registry = ProviderRegistry::new();

    #[cfg(feature = "anthropic")]
    if let Some(key) = &config.anthropic_api_key {
        registry.register("anthropic", AnthropicProvider::new(key));
    }

    #[cfg(feature = "ollama")]
    if let Some(url) = &config.ollama_url {
        registry.register("ollama", OllamaProvider::new(url));
    }

    registry
}
```

### Crate Splitting: Caution on Over-Splitting

A [real-world experience upgrading a 400-crate workspace](https://codeandbitters.com/rust-2024-upgrade/) highlights the coordination overhead. The [Bevy project measured](https://corrode.dev/blog/tips-for-faster-rust-compile-times/) 28% overhead from cargo resolution alone when only one crate in a large graph changed.

**Signs you have split too much:**
- Single-line changes trigger recompilation of 5+ crates.
- You spend more time on `pub use` re-exports than on business logic.
- Most crates have exactly one consumer.

**Signs you have not split enough:**
- `cargo check` takes 30+ seconds after a one-line change.
- Two developers frequently conflict on the same `mod.rs` files.
- Test compilation pulls in the entire application.

### Common Anti-Patterns

- **Monolith with 100+ modules in a single crate** — every change recompiles everything. Split along natural domain boundaries.
- **One crate per file** ("nano-crates") — excessive cargo resolution overhead and import verbosity.
- **Circular dependencies between workspace crates** — indicates poor boundary design. Extract the shared types into a `core` crate.
- **`pub` on everything** — default to `pub(crate)`. Only expose what the consuming crate needs.
- **Feature flags that change behavior** instead of adding it — leads to subtle bugs when features interact.

### Why It Matters at Scale

In a production Axum application with Rust backend, React frontend, and CLI, build times directly impact developer velocity. A 10-second reduction in `cargo check` time across a team of 5 developers saves hours per week. Proper workspace structure makes the difference between "I can iterate quickly" and "I go get coffee every time I save a file."

**Sources:**
- [Tips For Faster Rust Compile Times — corrode.dev](https://corrode.dev/blog/tips-for-faster-rust-compile-times/)
- [How to organize large Rust codebases — kerkour.com](https://kerkour.com/rust-how-to-organize-large-workspaces)
- [Organize Rust projects for faster compilation with Cargo workspaces — InfoWorld](https://www.infoworld.com/article/4050654/organize-rust-projects-for-faster-compilation-with-cargo-workspaces.html)
- [Updating a large codebase to Rust 2024 — Code and Bitters](https://codeandbitters.com/rust-2024-upgrade/)
- [Fast Rust Builds — matklad](https://matklad.github.io/2021/09/04/fast-rust-builds.html)
- [Cargo Workspace Best Practices for Large Rust Projects](https://reintech.io/blog/cargo-workspace-best-practices-large-rust-projects)

---

## Quick Reference: Decision Matrix

| Situation | Recommended Approach |
|-----------|---------------------|
| Function that might fail, caller needs to handle modes | `thiserror` enum |
| Function that might fail, caller just propagates | `anyhow::Result` with `.context()` |
| Struct with 5+ fields, some optional | `bon::Builder` derive |
| Need heterogeneous collection of implementations | `Arc<dyn Trait>` |
| Need maximum speed in inner loop | Generics / enum dispatch |
| State machine with known phases | Typestate pattern |
| String parameter (read-only) | `&str` |
| String parameter (sometimes owned) | `Cow<'_, str>` |
| Network buffer | `bytes::Bytes` |
| Small collection, known max size | `SmallVec<[T; N]>` (measure first) |
| Async trait for `dyn Trait` usage | `#[async_trait]` (until native dyn support lands) |
| Async trait for generic usage only | Native `async fn` in trait |
| `select!` with stateful futures | Pin futures outside the loop |
| Work that must complete despite cancellation | `tokio::spawn` a task |
| Inter-task communication | Channels (mpsc, broadcast) over mutexes |
| Testing complex input domains | proptest |
| Testing trait implementations | mockall or manual stub |
| Build time > 30s for incremental check | Split into workspace crates |

---

*Document compiled February 2026. Patterns reflect Rust 2024 edition, Tokio 1.x, and Axum 0.7+/0.8+.*
