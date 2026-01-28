# CLAUDE.md

This file provides guidance for AI assistants working with the nexor repository.

## Project Overview

**nexor** is an AI Agent Orchestration TUI for GitHub Workflows - a Rust-based terminal application that orchestrates multiple AI agents to handle software engineering tasks.

## Repository Structure

```
nexor/
├── CLAUDE.md              # AI assistant guidelines (this file)
├── README.md              # Project overview and documentation flow
├── QUICKSTART.md          # 60-second overview
├── PHILOSOPHY.md          # Why this system works
├── PRD.md                 # Product Requirements Document
├── ROADMAP.md             # Technical roadmap with milestones
├── PROGRESS.md            # Work tracking and status
├── CONVENTIONS.md         # Code style, patterns, naming
├── ORCHESTRATOR.md        # Guide for decomposition agents
├── WORKER.md              # Guide for implementation agents
├── templates/             # Document templates
│   ├── worker.md          # Task assignment for workers
│   ├── orchestrator.md    # Task assignment for orchestrators
│   ├── ticket.md          # Detailed ticket breakdown
│   ├── report.md          # Work completion report
│   ├── handoff.md         # Work handoff context
│   └── ...
├── decomp/                # Detailed ticket breakdowns
│   ├── M1/                # Milestone 1 (Foundation) - Complete
│   ├── M2/                # Milestone 2 (LLM Layer)
│   └── M3-M9/             # Future milestones
├── migrations/            # SQLite database migrations
├── src/
│   ├── lib.rs             # Library root
│   ├── main.rs            # Entry point
│   ├── types/             # Core type definitions
│   ├── config/            # Configuration loading
│   ├── db/                # Database operations
│   ├── logging.rs         # Logging infrastructure
│   └── ...                # Future modules
└── .nexor/                # Runtime data
    ├── logs/              # Log files
    ├── work/              # Work tracking
    └── state.db           # SQLite database (created at runtime)
```

## Development Guidelines

### Getting Started

1. Read `PROGRESS.md` to understand current state
2. Check the current branch with `git status`
3. Follow `CONVENTIONS.md` for code style

### Agent Workflow

**As a Worker:**

1. Receive ticket assignment (e.g., "Ticket 2.1")
2. Read `WORKER.md` for process
3. Read `decomp/M{n}/{ticket}.md` for spec
4. Implement slice by slice, verify each
5. Update `PROGRESS.md` when done
6. Optionally create a report using `templates/report.md`

**As an Orchestrator:**

1. Receive milestone assignment (e.g., "Milestone 2")
2. Read `ORCHESTRATOR.md` for process
3. Create decomp files in `decomp/M{n}/`
4. Update `PROGRESS.md` with decomposition status

### Code Style

- Follow `CONVENTIONS.md` for all Rust code
- Use `cargo fmt` before committing
- Use `cargo clippy` to catch issues
- Keep functions focused and single-purpose

### Git Workflow

- Create feature branches from `main`
- Use descriptive commit messages (see `CONVENTIONS.md` for format)
- Keep commits atomic and focused

### Commit Message Format

```
<type>(<scope>): <description>

[optional body]
```

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`

Example:

```
feat(db): implement task CRUD operations

Implements ticket 1.4 slice 8:
- insert_task, get_task, update_task_status
- list_tasks_by_status query
- TaskRow mapping to Task struct
```

## Commands

```bash
# Build and check
~/.cargo/bin/cargo check          # Fast type checking
~/.cargo/bin/cargo build          # Full build
~/.cargo/bin/cargo build --release # Release build

# Testing
~/.cargo/bin/cargo test           # Run all tests
~/.cargo/bin/cargo test db::      # Run db module tests
~/.cargo/bin/cargo test -- --nocapture  # Show println output

# Code quality
~/.cargo/bin/cargo fmt            # Format code
~/.cargo/bin/cargo clippy         # Lint code

# Run
~/.cargo/bin/cargo run            # Run the application
RUST_LOG=debug cargo run          # Run with debug logging
```

## Testing

- Unit tests live alongside code in `#[cfg(test)]` modules
- Integration tests go in `tests/` directory
- Use `tempfile` crate for tests that need filesystem
- Use `tokio::test` for async tests

## Current Status

**Milestone 1: Foundation** - Complete (5/5 tickets)

- Project scaffolding, types, config, database, logging all done

**Next:** Milestone 2: LLM Layer

- See `PROGRESS.md` for details

## Key Conventions

1. **Keep it simple** - Avoid over-engineering
2. **Document as you go** - Update `PROGRESS.md` when completing work
3. **Security first** - Never commit secrets or credentials
4. **One slice at a time** - Verify before moving on
5. **Trust the spec** - Decomp files have what you need

## Notes for AI Assistants

- Read relevant files before making changes
- Check `PROGRESS.md` for dependencies before starting work
- Verify changes with `cargo check` and `cargo test`
- Update `PROGRESS.md` when completing tickets
- Use `templates/report.md` to document significant work
- Ask for clarification when requirements are ambiguous
- Please safe file in doc/ when I request for you to take notes.
