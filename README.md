# nexor

Visual workflow design platform for AI agents. Draw workflows on an Excalidraw canvas, and the system builds the structure instantly, then designs the agents that run it asynchronously.

Rust/Axum backend, React/Vite frontend, PostgreSQL.

## How It Works

The walkthrough below follows one real run end-to-end: a request to research the current state of agentic AI and produce an executive brief.

### 1. Describe your goal in plain language

Type what you want the system to build — no orchestration code required. Here the prompt asks for two research angles in parallel (technical landscape and industry adoption), synthesized into a single brief. The workflow agent reads the intent and starts planning the agent graph, using tools like `run_command` to inspect the board before making changes.

![Step 1 — Describe your goal](docs/images/step-1-describe-goal.png)

### 2. The system generates a structured agent graph

A DAG appears on the canvas: two independent research nodes feed into a synthesis node. The chat panel confirms the topology in plain language — "two parallel research angles → one synthesis step" — and offers to tweak node text, add a verification gate, change dependencies, or adjust scope.

![Step 2 — Agent graph generated](docs/images/step-2-agent-graph.png)

### 3. Hit Run — dispatch tracked node by node

Hit **Run**. The Activity panel's Dispatch tab shows each node being handed off to its own workforce — *Agentic AI Industry Research → Researcher*, *Agentic AI Landscape Research → AI Research Analyst*, *Agentic AI Executive Brief → Brief Writer* — marking each `completed` as it finishes, along with how many tools it used.

![Step 3 — Dispatch tracked per node](docs/images/step-3-dispatch.png)

### 4. Agents execute in parallel — tracked in real time

Switch to the Tree tab to watch the same run at the agent level. Each node expands to show its underlying agent with a live status indicator, so independent branches (Researcher, AI Research Analyst) can be seen progressing simultaneously before the downstream Brief Writer starts.

![Step 4 — Parallel execution in progress](docs/images/step-4-parallel-execution.png)

### 5. Results stream back as each agent completes

As agents finish, their outputs stream directly into the tree: file paths written to the shared workspace, summaries of what each report contains, and word counts. The final agent (Brief Writer) receives only the outputs it depends on and synthesizes them into the finished executive brief.

![Step 5 — Results streaming in](docs/images/step-5-results.png)

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
