# Code Conventions

> How we write code in nexor. Follow these for consistency.

---

## Rust Style

### Naming

```rust
// Types: PascalCase
struct TaskStatus;
enum AgentTier;
trait LLMProvider;

// Functions/methods: snake_case
fn get_available_agent() {}
fn spawn_worker() {}

// Constants: SCREAMING_SNAKE_CASE
const MAX_RETRIES: u32 = 3;
const DEFAULT_TIMEOUT_MS: u64 = 30_000;

// Modules: snake_case
mod task_queue;
mod llm_provider;

// Acronyms in names: treat as words
struct HttpClient;      // not HTTPClient
struct LlmProvider;     // not LLMProvider (exception: if it's THE type name)
fn parse_json() {}      // not parseJSON
```

### Formatting

Use `cargo fmt`. No exceptions.

```rust
// Line length: 100 chars max (rustfmt default)
// Indent: 4 spaces
// Braces: same line for functions, enums, structs

fn example() {
    // code here
}

struct Example {
    field: Type,
}
```

---

## Error Handling

### Use `thiserror` for Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("config file not found: {path}")]
    NotFound { path: PathBuf },

    #[error("invalid config: {reason}")]
    Invalid { reason: String },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
```

### Use `anyhow` in Application Code

```rust
use anyhow::{Context, Result};

fn load_config() -> Result<Config> {
    let content = std::fs::read_to_string(&path)
        .context("failed to read config file")?;

    let config: Config = toml::from_str(&content)
        .context("failed to parse config")?;

    Ok(config)
}
```

### Error Hierarchy

```
Library code (src/types/, src/db/)  → thiserror, specific error types
Application code (src/main.rs)      → anyhow, Result<T>
```

### Never Panic in Library Code

```rust
// Bad
fn get_agent(id: &str) -> Agent {
    self.agents.get(id).unwrap()  // panics!
}

// Good
fn get_agent(id: &str) -> Option<&Agent> {
    self.agents.get(id)
}

// Also good
fn get_agent(id: &str) -> Result<&Agent, AgentError> {
    self.agents.get(id).ok_or(AgentError::NotFound { id: id.to_string() })
}
```

---

## Module Organization

### File Structure

```
src/
├── lib.rs              ← Re-exports public API
├── main.rs             ← Entry point only
├── types/
│   ├── mod.rs          ← pub use all types
│   ├── task.rs         ← Task, TaskStatus, etc.
│   └── agent.rs        ← Agent, AgentTier, etc.
└── config/
    ├── mod.rs          ← pub use, module glue
    ├── global.rs       ← GlobalConfig loading
    └── project.rs      ← ProjectConfig loading
```

### mod.rs Pattern

```rust
// src/types/mod.rs
mod task;
mod agent;
mod message;

pub use task::*;
pub use agent::*;
pub use message::*;
```

### Keep Modules Focused

One module = one concept. If a file grows past ~300 lines, consider splitting.

---

## Async Patterns

### Use Tokio

```rust
use tokio::sync::{mpsc, oneshot};
use tokio::time::{timeout, Duration};

// Async functions
async fn fetch_data() -> Result<Data> {
    // ...
}

// Spawning tasks
tokio::spawn(async move {
    // background work
});
```

### Channel Patterns

```rust
// For commands: mpsc (many senders, one receiver)
let (tx, rx) = mpsc::channel::<Command>(32);

// For responses: oneshot (one sender, one receiver)
let (response_tx, response_rx) = oneshot::channel();

// Send command with response channel
tx.send(Command::GetStatus { response: response_tx }).await?;
let status = response_rx.await?;
```

### Timeouts

Always timeout external calls:

```rust
use tokio::time::{timeout, Duration};

let result = timeout(
    Duration::from_secs(30),
    client.send_request(request)
).await??;  // Note: double ? for timeout error + request error
```

---

## Database Patterns

### Use sqlx with Compile-Time Checking

```rust
use sqlx::{SqlitePool, FromRow};

#[derive(FromRow)]
struct Task {
    id: String,
    title: String,
    status: String,
}

async fn get_task(pool: &SqlitePool, id: &str) -> Result<Task> {
    sqlx::query_as!(
        Task,
        "SELECT id, title, status FROM tasks WHERE id = ?",
        id
    )
    .fetch_one(pool)
    .await
    .context("failed to fetch task")
}
```

### Migrations

```
migrations/
├── 001_create_tasks.sql
├── 002_create_agents.sql
└── 003_create_messages.sql
```

```sql
-- migrations/001_create_tasks.sql
CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

### Connection Pool

```rust
// Create once at startup
let pool = SqlitePool::connect("sqlite:.nexor/state.db").await?;

// Pass by reference everywhere
async fn do_work(pool: &SqlitePool) -> Result<()> {
    // use pool
}
```

---

## Type Definitions

### Derive Common Traits

```rust
#[derive(Debug, Clone, PartialEq)]  // minimum for most types
struct Task {
    // ...
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]  // for enums
enum TaskStatus {
    Pending,
    InProgress,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]  // for config/API types
struct Config {
    // ...
}
```

### Use Newtypes for IDs

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentId(pub Uuid);

// Now the compiler prevents mixing them up
fn assign_task(task: TaskId, agent: AgentId) { }
```

### Builder Pattern for Complex Types

```rust
#[derive(Default)]
pub struct TaskBuilder {
    title: Option<String>,
    priority: Priority,
}

impl TaskBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn build(self) -> Result<Task, BuildError> {
        Ok(Task {
            id: TaskId(Uuid::new_v4()),
            title: self.title.ok_or(BuildError::MissingTitle)?,
            priority: self.priority,
        })
    }
}

// Usage
let task = TaskBuilder::new()
    .title("Implement feature")
    .priority(Priority::High)
    .build()?;
```

---

## Testing

### Test Organization

```rust
// Unit tests: same file as code
// src/types/task.rs

pub struct Task { /* ... */ }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        // ...
    }
}
```

### Integration Tests

```
tests/
├── config_loading.rs
├── database_operations.rs
└── common/
    └── mod.rs          ← shared test utilities
```

### Async Tests

```rust
#[tokio::test]
async fn test_async_operation() {
    let result = async_function().await;
    assert!(result.is_ok());
}
```

### Test Naming

```rust
#[test]
fn task_status_transitions_from_pending_to_in_progress() { }

#[test]
fn config_returns_defaults_when_file_missing() { }

#[test]
fn agent_pool_respects_max_limit() { }
```

---

## Comments & Documentation

### When to Comment

```rust
// Comment WHY, not WHAT
// Bad:
let count = items.len();  // get the length

// Good:
// We check length first to avoid allocating when empty
let count = items.len();
if count == 0 {
    return Ok(vec![]);
}
```

### Doc Comments for Public API

```rust
/// Creates a new agent with the specified tier.
///
/// # Arguments
///
/// * `tier` - The agent tier (Orchestrator, Worker, Utility)
/// * `config` - Model configuration for this agent
///
/// # Returns
///
/// A new `Agent` instance in the `Idle` state.
///
/// # Example
///
/// ```
/// let agent = Agent::new(AgentTier::Worker, config);
/// ```
pub fn new(tier: AgentTier, config: ModelConfig) -> Self {
    // ...
}
```

### No Comments for Self-Evident Code

```rust
// Bad: comment states the obvious
/// Returns the task's status
pub fn status(&self) -> TaskStatus {
    self.status
}

// Good: no comment needed
pub fn status(&self) -> TaskStatus {
    self.status
}
```

---

## Git Commits

### Message Format

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

### Types

| Type | Use For |
|------|---------|
| `feat` | New feature |
| `fix` | Bug fix |
| `refactor` | Code change that neither fixes nor adds |
| `docs` | Documentation only |
| `test` | Adding or fixing tests |
| `chore` | Build, CI, dependencies |

### Examples

```
feat(types): add Task and TaskStatus types

Implements ticket 1.2 slice 1:
- TaskStatus enum with Pending, InProgress, etc.
- Task struct with all required fields
- Basic derives for Debug, Clone, PartialEq
```

```
fix(config): handle missing config file gracefully

Returns default config instead of panicking when
~/.config/nexor/config.toml doesn't exist.
```

---

## Quick Reference

| Thing | Convention |
|-------|------------|
| Type names | `PascalCase` |
| Functions | `snake_case` |
| Constants | `SCREAMING_SNAKE_CASE` |
| Modules | `snake_case` |
| Error types | `thiserror` in libs, `anyhow` in app |
| IDs | Newtypes: `TaskId(Uuid)` |
| Async | `tokio`, always timeout externals |
| Tests | Same file for unit, `tests/` for integration |
| Comments | WHY not WHAT, doc comments for public API |
| Commits | `type(scope): description` |
