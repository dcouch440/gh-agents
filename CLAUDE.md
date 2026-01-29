# CLAUDE.md

## What is nexor?

Rust backend + React frontend + Ink CLI that orchestrates AI agents for software engineering tasks on GitHub repos.

## Architecture

```
Rust (Axum)        → REST API, WebSocket, LLM providers, agents, orchestration, execution
React (Vite)       → Web UI in ui/
Ink (TypeScript)   → Terminal CLI in cli/
SQLite             → .nexor/state.db
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
```

## Key Source Layout

```
src/
├── main.rs            # Entry point
├── lib.rs             # Library root
├── types/             # Core types (Task, Agent, Message, etc.)
├── config/            # Config loading
├── db/                # SQLite operations
├── llm/               # LLM provider clients
├── agents/            # Agent runtime & execution
├── orchestration/     # Task planning, routing, scheduling
├── prompts/           # Prompt templates
├── execution/         # File/git/test operations
├── github/            # GitHub API integration
├── cli.rs             # CLI arg parsing
└── headless.rs        # Headless mode
ui/                    # React frontend (Vite + TailwindCSS + Zustand)
cli/                   # Ink terminal CLI
migrations/            # SQLite migrations
decomp/                # Ticket breakdowns by milestone
```

## Conventions

- `cargo fmt` and `cargo clippy` before committing
- `thiserror` for library errors, `anyhow` for application code
- Tokio for async, always timeout external calls
- Newtypes for IDs: `TaskId(Uuid)`, `AgentId(Uuid)`
- Unit tests in `#[cfg(test)]` modules, integration tests in `tests/`
- Commit format: `type(scope): description` (feat, fix, docs, refactor, test, chore)

## Status

See `PROGRESS.md` for detailed tracking. See `ROADMAP.md` for milestone plans.

M1-M5, M7-M11: Complete. M6: Deprecated (TUI). M12: In progress. M13-M16: Planned.

## Working with this repo

- Read `PROGRESS.md` before starting work to check dependencies
- Ticket specs live in `decomp/M{n}/` directories
- Verify with `cargo check` and `cargo test` before committing
- Save notes to `doc/` when requested
