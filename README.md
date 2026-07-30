# nexor

Visual workflow design platform for AI agents. Draw workflows on an Excalidraw canvas, and the system builds the structure instantly, then designs the agents that run it asynchronously.

Rust/Axum backend, React/Vite frontend, PostgreSQL.

## Prerequisites

- Rust (via [rustup](https://rustup.rs))
- Node.js + npm
- Docker + Docker Compose (for Postgres, MinIO, and JuiceFS)
- An xAI API key (the default LLM provider)

## Setup

```bash
cp .env.example .env
# fill in XAI_API_KEY and JWT_SECRET at minimum
```

```bash
# start Postgres, MinIO, JuiceFS
make server-up

# start backend + frontend dev servers (migrations run automatically on startup)
make dev
```

See `.env.example` for the full list of configuration options (LLM providers, S3/object storage, VPN, rate limiting, etc).

## Commands

```bash
# Backend
make build       # Build debug binary
make check       # Fast type checking
make test        # Run all tests
make fmt          # Format code
make lint         # Run clippy linter
make run          # Run the application

# Frontend (or from frontend/)
make ui-dev       # Start Vite dev server
make ui-build     # Build for production
make ui-lint      # Run eslint

# Docker
make server       # Build + start the full dockerized stack
make server-down  # Stop the dockerized stack
```

Run `make help` for the full target list.

## Documentation

- [`CLAUDE.md`](CLAUDE.md) — coding conventions and pre-commit checklist
- [`docs/backend-architecture.md`](docs/backend-architecture.md) — the 5-layer backend stack
- [`docs/database-schema.md`](docs/database-schema.md) — full database schema
- [`docs/database-model-guide.md`](docs/database-model-guide.md) — how the schema layers fit together
- [`docs/frontend-build-guide.md`](docs/frontend-build-guide.md) — frontend pages, API endpoints, and components

## License

[MIT](LICENSE)
