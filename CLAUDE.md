# CLAUDE.md

Visual workflow design platform for AI agents. Users draw workflows on an Excalidraw canvas, the system builds structure instantly (Phase 0), then designs agents asynchronously. Rust/Axum backend, React/Vite frontend, PostgreSQL.

See @visions/vision-visual-dispatch.md for the current product vision.

## Commands

```bash
# Backend (use full path — cargo is not on default PATH)
~/.cargo/bin/cargo check                    # Type check
~/.cargo/bin/cargo build                    # Build
~/.cargo/bin/cargo test                     # All tests
~/.cargo/bin/cargo test <module>::          # Module tests
~/.cargo/bin/cargo fmt                      # Format
~/.cargo/bin/cargo clippy                   # Lint

# ALWAYS run `cargo check` before `cargo test` — catch compile errors fast
# instead of waiting 3 min for a test run that can't compile.
#
# When running cargo test, use this grep to capture both compile errors AND
# test results in one pass (there are 3 test binaries — never use `tail`):
# ~/.cargo/bin/cargo test 2>&1 | grep -E "^(test result:|error)" | head -20

# Frontend (run from frontend/)
npx tsc --noEmit                            # Type check — zero errors
npx eslint .                                # Lint — zero warnings
npx vitest run                              # All tests

# Database
docker exec nexor-postgres-1 psql -U nexor -d nexor -c "SQL"
```

## Pre-commit Checklist

- Rust: `cargo fmt` + `cargo clippy` + `cargo test` must pass
- Frontend: `tsc --noEmit` + `eslint .` must pass with zero warnings
- Always write tests when completing a ticket
- Commit format: `type(scope): description` (feat, fix, docs, refactor, test, chore)
- No co-authored-by on commits

## Rust Conventions

- `thiserror` for library errors, `anyhow` for application code
- Handlers return `Result<Json<T>, AppError>` — never unwrap or panic in handlers
- Handlers are thin: parse request, call service, return response. Business logic lives in `src/server/services/`
- Always timeout external calls
- Folder-based modules with separate test files:

```
feature/
├── mod.rs      # Implementation + `mod tests;`
└── tests.rs    # #[cfg(test)] mod tests { ... }
```

Never inline `#[cfg(test)]` blocks in `mod.rs`.

## Frontend Conventions

- Strict TypeScript — no `any`, no `as` casts (unless commented why), no `@ts-ignore`
- Components use `function` declarations; everything else uses arrow functions
- `type` over `interface`. `null` over `undefined`. Named exports only. One component per file.
- No `React.FC`, no `forwardRef`, no external state libraries — vanilla React only
- Components are stateless and pure. Pages own data and state.
- Use `Collections` (`@/utils/collections`) for array operations — never raw `.filter().map()` chains or `.find()` inside loops
- Use typed endpoints from `frontend/src/api/api.ts` — never raw `api.get`/`api.post`
- Colocated tests: `Foo.test.tsx` next to `Foo.tsx`. Shared fixtures in `src/test/fixtures.ts`
- ESLint is strict (React 19): no eslint-disable, fix the code instead

## Reusability

If you're writing it for the second time, it should already be a shared primitive or utility. Frontend: extract atomic UI primitives before building feature components. Backend: extract common logic into utility modules, not inline in handlers.
