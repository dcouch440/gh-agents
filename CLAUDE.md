# CLAUDE.md

## What is nexor?

Rust backend + React frontend + Ink CLI that orchestrates AI agents for software engineering tasks on GitHub repos.

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

## Conventions

- `cargo fmt` and `cargo clippy` before committing Rust code
- `npx tsc --noEmit` and `npx eslint .` before committing frontend code — zero warnings required
- `thiserror` for library errors, `anyhow` for application code
- Tokio for async, always timeout external calls
- Newtypes for IDs: `TaskId(Uuid)`, `AgentId(Uuid)`
- Commit format: `type(scope): description` (feat, fix, docs, refactor, test, chore)
- Always write tests when completing a ticket
- Verify with `cargo check` and `cargo test` before committing
- Axum handlers return `Result<Json<T>, AppError>` — never unwrap or panic in handlers
- In frontend/ always prefer reusable components
- Think enterprise quality always.
- No co authored on commits.

## Reusability Requirements

**Every feature touches two layers — both must prioritize reusable, composable building blocks.**

**Frontend:** Extract primitives first. Before building a feature component, identify the atomic UI elements it needs (buttons, inputs, cards, badges, layout containers) and implement them as standalone, prop-driven primitives in the shared component library. Feature components compose primitives — they never hard-code styles or markup that could be reused elsewhere.

**Backend:** Extract utility functions and shared abstractions. Common logic — validation, transformation, formatting, error mapping, query helpers — must live in dedicated utility modules, not be inlined in handlers or services. If two endpoints share similar logic, that logic gets extracted. Prefer small, well-tested pure functions that compose into larger operations.

**The rule:** If you're writing it for the second time, it should already be a shared primitive or utility. If you're writing it for the first time, ask whether it *will* be needed again — and if so, build it reusable from the start.

## Module Organization

**Always use folder-based modules with separate test files:**

```
src/server/api/
├── agents/
│   ├── mod.rs      # Implementation
│   └── tests.rs    # Tests
├── tasks/
│   ├── mod.rs
│   └── tests.rs
└── mod.rs          # Re-exports: pub mod agents; pub use agents::*;
```

**In `mod.rs`:**

```rust
// Implementation code here

mod tests;  // Links to tests.rs
```

**In `tests.rs`:**

```rust
#[cfg(test)]
mod tests {
    // Tests here
}
```

Never inline `#[cfg(test)]` blocks in `mod.rs`. Tests always go in a separate `tests.rs` file. This keeps test code separate from implementation while maintaining rust-analyzer integration.

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

**ALWAYS use the typed endpoints from `api.ts`.** Never call raw `api.get`/`api.post` unless the endpoint has no typed method. Read `api.ts` for available endpoints, error types (`ApiError`), and request options.

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
- **Mock the API** (`../api`), constants (`USE_MOCK_DATA: false`), and WS (`useWebSocket`) as needed
- **Split large test files** by unit — one test file per context/hook, not one mega test file per feature
- **No skipped tests, no `TODO` tests** — every test must pass or be removed
