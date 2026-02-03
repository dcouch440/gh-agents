# CLAUDE.md

## What is nexor?

Rust backend + React frontend + Ink CLI that orchestrates AI agents for software engineering tasks on GitHub repos.

# DB Login

Email: user@example.com
Password: password123

## Architecture

```
Rust (Axum)        → REST API, WebSocket, LLM providers, agents, orchestration, execution
React (Vite)       → New frontend in frontend/ (ui/ is deprecated)
Ink (TypeScript)   → Terminal CLI in cli/
PostgreSQL         → nexor database
```

## Commands

```bash
~/.cargo/bin/cargo check                    # Type check
~/.cargo/bin/cargo build                    # Build
~/.cargo/bin/cargo test                     # All tests
~/.cargo/bin/cargo test <module>::          # Module tests
~/.cargo/bin/cargo fmt                      # Format
~/.cargo/bin/cargo clippy                   # Lint
~/.cargo/bin/cargo run                      # Run server
RUST_LOG=debug ~/.cargo/bin/cargo run       # Debug logging

# Frontend (run from frontend/)
npx tsc --noEmit                            # Type check
npx eslint .                                # Lint (must pass, zero warnings)
npx vite build                              # Production build
```

## Key Source Layout

```
src/
├── main.rs            # Entry point
├── lib.rs             # Library root
├── types/             # Core types (Task, Agent, Message, etc.)
├── config/            # Config loading
├── db/                # PostgreSQL operations
├── llm/               # LLM provider clients
├── agents/            # Agent runtime & execution
├── orchestration/     # Task planning, routing, scheduling
├── prompts/           # Prompt templates
├── execution/         # File/git/test operations
├── github/            # GitHub API integration
└── cli.rs             # CLI arg parsing
frontend/              # React frontend (Vite) — active development
ui/                    # Legacy frontend (deprecated)
cli/                   # Ink terminal CLI
migrations/            # PostgreSQL migrations
decomp/                # Ticket breakdowns by milestone
```

## Conventions

- `cargo fmt` and `cargo clippy` before committing Rust code
- `npx tsc --noEmit` and `npx eslint .` before committing frontend code — zero warnings required
- `thiserror` for library errors, `anyhow` for application code
- Tokio for async, always timeout external calls
- Newtypes for IDs: `TaskId(Uuid)`, `AgentId(Uuid)`
- Unit tests in `#[cfg(test)]` modules, integration tests in `tests/`
- Commit format: `type(scope): description` (feat, fix, docs, refactor, test, chore)

## API Module Extraction Pattern

When extracting domains from `src/server/api/mod.rs` into separate modules:

1. **Create module structure:**
   ```bash
   mkdir -p src/server/api/<domain>
   # Create mod.rs with handlers and types
   # Create tests.rs placeholder with `mod tests;` declaration in mod.rs
   ```

2. **Update `src/server/api/mod.rs`:**
   - Add module declaration: `pub mod <domain>;`
   - Add re-exports: `pub use <domain>::{handlers, types};`
   - Delete the extracted section

3. **Update `src/server/openapi.rs`:**
   - Change `super::api::handler` → `super::api::<domain>::handler`
   - Change `super::api::Type` → `super::api::<domain>::Type`

4. **Verify:**
   ```bash
   ~/.cargo/bin/cargo check
   ~/.cargo/bin/cargo test --lib  # Must have 1193 passed, 9 pre-existing failures
   ```

5. **Commit:**
   ```bash
   git add -A && git commit -m "refactor(api): extract <domain> domain"
   ```

**Extracted domains so far:** auth, tasks, agents, tools, config, agent_context, chat, documents, sessions, output_schemas, prompt_templates, agent_executions, costs, results, workflows

## Off-limits directories

Do NOT read, modify, or reference files in these directories:

- `decomp/` — Ticket breakdowns managed by the project owner. Read-only for humans.
- `ui/` — Deprecated legacy frontend. Do not modify.

## Database

PostgreSQL runs in Docker. Container: `gh-agents-postgres-1` (image: `postgres:16-alpine`).

```bash
docker exec gh-agents-postgres-1 psql -U nexor -d nexor -c "SELECT 1;"   # Quick query
docker exec -it gh-agents-postgres-1 psql -U nexor -d nexor              # Interactive shell
```

## Working with this repo

- Always write tests when completing a ticket
- Verify with `cargo check` and `cargo test` before committing
- Save notes to `doc/` when requested
- Stay out of /cli and /archive
- In frontend/ always prefer reusable components.

## Frontend Conventions (frontend/)

**Strict TypeScript is mandatory.** `"strict": true` in tsconfig. No `any`, no `as` casts (unless commented why), no `@ts-ignore`.

- **Components** use `function` declarations. Everything else (hooks, helpers, callbacks) uses arrow functions.
- **`type` over `interface`** for all props and data shapes. Prefer discriminated unions with a `type` field for state actions and variant data.
- **No `React.FC`, `PropsWithChildren`, or `forwardRef` wrappers.** Destructure props directly with an inline or co-located `type`. If children are needed, add `children: ReactNode` explicitly.
- **No external state libraries.** Vanilla React only: `useState` for simple values, `useReducer` for complex state with multiple actions.
- **Every context gets a custom hook** that throws if used outside its provider, so consumers get non-nullable types without optional chaining.
- **`null` over `undefined`** for intentional absence. One bottom value.
- **Named exports only.** No `export default`.
- **One component per file.** File name matches component name.
- **Components are stateless and pure.** Props in, JSX out. No hooks, no context, no side effects. Pages own data and state.
- **Colocate** hooks, types, and helpers with the feature that uses them. Barrel `index.ts` only at feature/directory boundaries.
- **Early returns** for empty/loading/error states before the main render.
- **Constants file for app-wide values:** API base URL, WS URL, app name, route paths, WS channel names, polling intervals, localStorage keys. No magic strings or numbers scattered in components. All in `src/constants.ts`.

- **ESLint rules are strict (React 19):** No setState directly in effect bodies, no ref access during render, no mixing component and non-component exports in the same file (react-refresh). Fix the code, don't suppress with eslint-disable.

## Frontend API Client (frontend/src/api/)

**ALWAYS use the typed endpoints from `api.ts`.** Never call raw HTTP methods unless absolutely necessary.

```typescript
import {api} from "@/api";

// ✅ CORRECT: Use typed endpoints
const {agents} = await api.agents.list();
const agent = await api.agents.get(id);
await api.tasks.create({title, description});

// ❌ WRONG: Don't use raw HTTP methods
const agents = await api.get("/agents"); // No! Use api.agents.list()
```

**Features:**

- Typed endpoints for all resources (agents, tasks, tools, documents, sessions, etc.)
- Automatic retry with exponential backoff
- Request deduplication (prevents duplicate in-flight GET requests)
- Request/response/error interceptors
- AbortController support for cancellation
- Comprehensive error handling with typed errors

**Error handling:**

```typescript
import {api, ApiError} from "@/api";

try {
  const agent = await api.agents.get(id);
} catch (error) {
  if (error instanceof ApiError) {
    switch (error.type) {
      case "http_error": // 4xx/5xx errors
        if (error.status === 404) console.log("Not found");
        break;
      case "network_error": // Connection failed
      case "timeout_error": // Request timed out
      case "abort_error": // Request cancelled
        break;
    }
  }
}
```

**Request configuration:**

```typescript
// Custom timeout, retries, headers, cancellation
const agent = await api.agents.get(id, {
  timeout: 5000,
  retries: 3,
  signal: abortController.signal,
  headers: {"X-Custom": "value"},
});
```

**Only use raw HTTP methods** (`api.get`, `api.post`, etc.) **for truly custom endpoints** not covered by typed methods. This is rare.

## Frontend Testing (frontend/)

**Runner:** Vitest + @testing-library/react + jsdom. Config in `vite.config.ts` under `test`.

```bash
# Run from frontend/
npx vitest run                              # All tests
npx vitest run src/contexts/                # Context tests only
npx vitest run src/hooks/                   # Hook tests only
npx vitest                                  # Watch mode
```

**Strategy:** Unit + integration. Reducers tested through providers, hooks tested with `renderHook`.

**File structure:**

```
frontend/src/
├── test/
│   ├── setup.ts              # jest-dom matchers
│   └── fixtures.ts           # Shared mock data (mockAgent, mockTask, etc.)
├── contexts/
│   ├── AgentContext.tsx
│   ├── AgentContext.test.tsx  # Colocated test
│   └── ...
└── hooks/
    ├── useAgents.ts
    ├── useAgents.test.ts     # Colocated test
    └── ...
```

**Conventions:**

- **Colocated tests** — `Foo.test.tsx` next to `Foo.tsx`
- **Nested `describe`/`it`** blocks grouped by unit (reducer, provider, hook variant)
- **Shared fixtures** in `src/test/fixtures.ts` — reusable typed mock objects
- **`vi.hoisted()` + inline `vi.mock()`** — hoisted fn refs for mock setup, module mocking per file
- **`vi.clearAllMocks()`** in `beforeEach`, mock return values set per test or per describe
- Mock the API (`../api`), constants (`USE_MOCK_DATA: false`), and WS (`useWebSocket`) as needed
