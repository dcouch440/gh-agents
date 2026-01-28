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

---

## React & TypeScript

### Component Structure

**Production Pattern:** Reusable, type-safe, composable components.

```tsx
// components/Button/Button.tsx
import { ButtonHTMLAttributes, forwardRef } from 'react';
import styles from './Button.module.css';

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'danger';
  size?: 'sm' | 'md' | 'lg';
  isLoading?: boolean;
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ variant = 'primary', size = 'md', isLoading, children, ...props }, ref) => {
    return (
      <button
        ref={ref}
        className={`${styles.button} ${styles[variant]} ${styles[size]}`}
        disabled={isLoading || props.disabled}
        {...props}
      >
        {isLoading ? <Spinner size={size} /> : children}
      </button>
    );
  }
);

Button.displayName = 'Button';
```

### File Organization

```
src/
├── components/
│   ├── Button/
│   │   ├── Button.tsx
│   │   ├── Button.module.css
│   │   ├── Button.test.tsx
│   │   └── index.ts          # export { Button } from './Button'
│   ├── Card/
│   │   ├── Card.tsx
│   │   ├── CardHeader.tsx
│   │   ├── CardBody.tsx
│   │   ├── Card.module.css
│   │   └── index.ts
│   └── index.ts              # Barrel exports
├── hooks/
│   ├── useAsync.ts
│   ├── useDebounce.ts
│   └── index.ts
├── types/
│   ├── api.ts
│   ├── models.ts
│   └── index.ts
└── utils/
    ├── cn.ts                 # className utilities
    └── format.ts
```

### Component Composition Pattern

```tsx
// components/Card/Card.tsx
interface CardProps {
  children: React.ReactNode;
  className?: string;
}

export function Card({ children, className }: CardProps) {
  return <div className={cn(styles.card, className)}>{children}</div>;
}

// Subcomponents
interface CardHeaderProps {
  title: string;
  action?: React.ReactNode;
}

export function CardHeader({ title, action }: CardHeaderProps) {
  return (
    <div className={styles.header}>
      <h3>{title}</h3>
      {action}
    </div>
  );
}

export function CardBody({ children }: { children: React.ReactNode }) {
  return <div className={styles.body}>{children}</div>;
}

// Usage
<Card>
  <CardHeader title="Task Details" action={<Button>Edit</Button>} />
  <CardBody>
    <p>Content here</p>
  </CardBody>
</Card>
```

### Custom Hooks Pattern

```tsx
// hooks/useAsync.ts
interface UseAsyncState<T> {
  data: T | null;
  error: Error | null;
  isLoading: boolean;
}

export function useAsync<T>(
  asyncFn: () => Promise<T>,
  deps: React.DependencyList = []
): UseAsyncState<T> {
  const [state, setState] = React.useState<UseAsyncState<T>>({
    data: null,
    error: null,
    isLoading: true,
  });

  React.useEffect(() => {
    let mounted = true;

    setState({ data: null, error: null, isLoading: true });

    asyncFn()
      .then((data) => {
        if (mounted) {
          setState({ data, error: null, isLoading: false });
        }
      })
      .catch((error) => {
        if (mounted) {
          setState({ data: null, error, isLoading: false });
        }
      });

    return () => {
      mounted = false;
    };
  }, deps);

  return state;
}
```

### Props Patterns

```tsx
// Discriminated unions for variant props
type ButtonProps =
  | {
      variant: 'link';
      href: string;
      onClick?: never;
    }
  | {
      variant: 'button';
      href?: never;
      onClick: () => void;
    };

// Render props for flexibility
interface DataTableProps<T> {
  data: T[];
  renderRow: (item: T, index: number) => React.ReactNode;
  emptyState?: React.ReactNode;
}

// Children as render function
interface CollapsibleProps {
  children: (isOpen: boolean, toggle: () => void) => React.ReactNode;
}
```

### Type Safety

```tsx
// types/api.ts
export interface Task {
  id: string;
  title: string;
  status: 'pending' | 'in_progress' | 'completed';
  createdAt: string;
}

export interface APIResponse<T> {
  data: T;
  error?: string;
}

// Type guards
export function isTask(value: unknown): value is Task {
  return (
    typeof value === 'object' &&
    value !== null &&
    'id' in value &&
    'title' in value &&
    'status' in value
  );
}
```

### Naming Conventions

```tsx
// Components: PascalCase
export function TaskList() {}
export const Button = () => {};

// Hooks: camelCase with 'use' prefix
export function useAuth() {}
export function useLocalStorage() {}

// Utilities: camelCase
export function formatDate() {}
export function cn(...classes) {}

// Types/Interfaces: PascalCase
interface UserProfile {}
type TaskStatus = 'pending' | 'in_progress';

// Props: ComponentNameProps
interface ButtonProps {}
interface TaskListProps {}
```

### Avoid: Quick Prototypes

```tsx
// ❌ BAD: Not reusable, tightly coupled
function TaskCard({ task }) {
  const [isEditing, setEditing] = useState(false);

  return (
    <div style={{ padding: '20px', border: '1px solid gray' }}>
      <h3>{task.title}</h3>
      {isEditing && <input value={task.title} />}
      <button onClick={() => setEditing(!isEditing)}>Edit</button>
    </div>
  );
}

// ✅ GOOD: Reusable, composable, type-safe
interface TaskCardProps {
  task: Task;
  onEdit?: (task: Task) => void;
  className?: string;
}

export function TaskCard({ task, onEdit, className }: TaskCardProps) {
  return (
    <Card className={className}>
      <CardHeader
        title={task.title}
        action={onEdit && <Button onClick={() => onEdit(task)}>Edit</Button>}
      />
      <CardBody>
        <TaskStatus status={task.status} />
      </CardBody>
    </Card>
  );
}
```

### State Management

```tsx
// Local state: useState
const [count, setCount] = useState(0);

// Context for shared state
interface AppContextValue {
  user: User | null;
  setUser: (user: User | null) => void;
}

const AppContext = createContext<AppContextValue | null>(null);

export function useAppContext() {
  const context = useContext(AppContext);
  if (!context) {
    throw new Error('useAppContext must be used within AppProvider');
  }
  return context;
}
```

### useEffect Cleanup

**Always clean up side effects.** Prevent memory leaks, race conditions, and stale updates.

```tsx
// ✅ GOOD: Clean up subscriptions
useEffect(() => {
  const subscription = apiClient.subscribe((data) => {
    setData(data);
  });

  return () => {
    subscription.unsubscribe();
  };
}, []);

// ✅ GOOD: Clean up timers
useEffect(() => {
  const timer = setTimeout(() => {
    setShowMessage(true);
  }, 3000);

  return () => {
    clearTimeout(timer);
  };
}, []);

// ✅ GOOD: Clean up intervals
useEffect(() => {
  const interval = setInterval(() => {
    fetchLatestData();
  }, 5000);

  return () => {
    clearInterval(interval);
  };
}, []);

// ✅ GOOD: Prevent stale state updates after unmount
useEffect(() => {
  let mounted = true;

  fetchData().then((data) => {
    if (mounted) {
      setData(data);
    }
  });

  return () => {
    mounted = false;
  };
}, []);

// ✅ GOOD: Clean up event listeners
useEffect(() => {
  const handleResize = () => {
    setWindowWidth(window.innerWidth);
  };

  window.addEventListener('resize', handleResize);

  return () => {
    window.removeEventListener('resize', handleResize);
  };
}, []);

// ✅ GOOD: Clean up WebSocket connections
useEffect(() => {
  const ws = new WebSocket('ws://localhost:8080');

  ws.onmessage = (event) => {
    setMessages((prev) => [...prev, event.data]);
  };

  return () => {
    ws.close();
  };
}, []);

// ✅ GOOD: Abort fetch requests on unmount
useEffect(() => {
  const abortController = new AbortController();

  fetch('/api/data', { signal: abortController.signal })
    .then((res) => res.json())
    .then((data) => setData(data))
    .catch((err) => {
      if (err.name !== 'AbortError') {
        console.error(err);
      }
    });

  return () => {
    abortController.abort();
  };
}, []);

// ❌ BAD: No cleanup
useEffect(() => {
  const subscription = apiClient.subscribe((data) => {
    setData(data); // Memory leak if component unmounts
  });
  // Missing cleanup!
}, []);

// ❌ BAD: No cleanup for timer
useEffect(() => {
  setTimeout(() => {
    setShowMessage(true); // Error if component unmounts
  }, 3000);
  // Missing cleanup!
}, []);
```

### Testing

```tsx
// Button.test.tsx
import { render, screen, fireEvent } from '@testing-library/react';
import { Button } from './Button';

describe('Button', () => {
  it('renders children', () => {
    render(<Button>Click me</Button>);
    expect(screen.getByText('Click me')).toBeInTheDocument();
  });

  it('calls onClick when clicked', () => {
    const handleClick = vi.fn();
    render(<Button onClick={handleClick}>Click</Button>);

    fireEvent.click(screen.getByText('Click'));
    expect(handleClick).toHaveBeenCalledOnce();
  });

  it('shows loading state', () => {
    render(<Button isLoading>Submit</Button>);
    expect(screen.getByRole('button')).toBeDisabled();
  });
});
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
| React components | `PascalCase`, type-safe props |
| React hooks | `useCamelCase` |
| Component files | `ComponentName/ComponentName.tsx` |
