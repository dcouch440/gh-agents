# nexor

**An experiment: can agents design their own agents?**

Rather than hand-writing orchestration, could one agent read a plain-language goal, decide what roles the work needs, write their system prompts and tool assignments, and hand a structured plan to an executor that runs it?

nexor is what I built to find out. Draw a workflow on a canvas; the system builds the structure instantly, then designs and runs the agents behind it.

Rust/Axum backend, React/Vite frontend, PostgreSQL.

I also used it to practise three things directly: orchestrating parallel agent pipelines, agentic coding against a large codebase, and refactoring at a scale I wouldn't attempt by hand. What came out of that is in [What I Learned](#what-i-learned).

*A personal research project — built to answer the question above, documented rather than maintained as a product.*

[How it works](#how-it-works) · [Workforce model](#the-workforce-model) · [Architecture](#architecture-overview) · [What I learned](#what-i-learned) · [Setup](#setup)

## Stack

![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![React](https://img.shields.io/badge/React-20232A?style=flat&logo=react&logoColor=61DAFB)
![TypeScript](https://img.shields.io/badge/TypeScript-007ACC?style=flat&logo=typescript&logoColor=white)
![PostgreSQL](https://img.shields.io/badge/PostgreSQL-316192?style=flat&logo=postgresql&logoColor=white)

**Backend:** Rust · Axum · PostgreSQL  
**Frontend:** React · TypeScript · Vite  
**Execution:** Custom DAG executor · Multi-agent workforce engine · LLM orchestration · SSE streaming

## How It Works

You describe what you want in the chat panel. The workflow agent decides the shape of the work — how many steps it takes, what each one is responsible for, and which ones depend on which — and draws it onto the board as nodes and arrows. You can also draw and edit nodes by hand; either way, the shape on the canvas *is* the dependency graph. There is no orchestration code and no config to write.

It builds that structure as files, not database rows: a `topology.json` plus one markdown file per node, projected out to a repo, edited, and synced back. The chat panel streams the whole thing as it happens, including the failures — a bad shell command shows up in the log the same way a successful one does.

![The workflow agent builds the board](docs/images/run-1-draw-the-board.png)

Structure lands immediately; nothing is staffed yet. Hit **Generate** and each node is dispatched to its own **system agent**, which decides what agents that node needs, writes their system prompts and tool assignments, and records the dependency graph between them. That runs asynchronously across the whole board. The system agent is told *what* the node must accomplish and never *how* to staff it, so a single sentence on the board routinely becomes a team — a step that reads "write the brief" can come back as an analyst feeding a writer.

Then you hit **Run**. Steps with no dependencies between them dispatch together; each level waits on the one before it. Inside a step, the agents that node was given run the same way, in parallel wherever their own graph allows. Agents in a step share one container, so they hand work off through files rather than through prose, and every file change is captured as a diff.

## The Workforce Model

The core primitive is the **workforce step** — a single node in the canvas that internally runs a coordinated team of agents.

### Two-Phase Design

Every workforce is designed before it executes, and the two phases are separate artifacts.

**Dispatch** runs first, once per step, when you hit Generate. The system agent receives one sentence from the board and answers *how*: which agents exist, what each one knows, and what file each produces. It designs by writing config files to disk — `config.json`, `topology.json`, `agents/*.json` — then calls `complete_system`, attesting against a six-item checklist (`prompts_not_trivial`, `assignments_expanded` and `no_filenames_prescribed` among them) before the execution engine reads those files back. The design is therefore inspectable and editable: a structured artifact, not an implicit LLM call.

**Runtime** is what those config files produced. Every runtime agent's system prompt is two halves joined at a named seam: the role the system agent wrote for it, wrapped in an `<expertise>` tag and unique to that agent, followed by one operational file that is byte-identical for every agent in every step. Its input is an `<assignment>` block and a `<deliverable>` block, with upstream results arriving as `<previous_step>` blocks.

<details>
<summary>The five hard rules, in full — the same five reach every runtime agent</summary>

```
- Never put the deliverable in your reply when you have write access. Write it to a file.
  A reply is read once by the next agent and then it is gone.

- Never overwrite a file you were given. Upstream files are the only copy and other agents
  are reading them. Save what you make under a new name.

- Never name a file in your receipt that you did not confirm landed. write_file and
  edit_file report what they wrote, and run_command's `changes:` block names what moved.

- Never re-run a command the report called a no-op or a loop. It did nothing the first
  time and it will do nothing again. Read the file and find out why.

- Never invent what an upstream file contains. If you could not read it, say so in your
  receipt and say what you did instead.
```

</details>

`<previous_step>` carries the upstream agents' prose receipts, not their files — nothing about a file's contents is in it. That is what the last rule is guarding against.

The contract is not always honoured. Agents told to write files sometimes return their output inline instead, which is one of the reasons the handoff is file-based rather than conversational — a missing file is a visible failure.

### Dependency-Based Parallelism

Agents within a workforce declare which upstream agents they depend on. The executor resolves this into execution levels via topological sort. Agents in the same level run in parallel; each level waits for the previous to complete.

This means a workforce automatically exploits concurrency wherever the dependency graph allows it — without any manual configuration.

### Shared Workspace

When a run is containerized, all agents in a step share one container — files written by one are visible to the next, which is what makes file-based handoff possible. The container is created once at the start of the step and torn down after all agents complete. A filesystem diff is extracted before teardown and stored, so every file change across the entire workforce is tracked, including from a step that wrote files and then failed.

The handoff contract itself is carried in the operational half of every agent's system prompt, shown in part above.

## Architecture Overview

The system splits into two planes. A **design plane** of agents that edit files writes the agent configs onto a shared filesystem. A **run plane** reads those files back and executes them. The handoff between the two is `system_node/<step-id>/agents/*.json` on disk, not a function call.

The design plane is one pattern applied at two scales. The **workflow agent** — the chat panel you actually type into — projects the whole board out to a repo, edits `board.md`, `topology.json` and `nodes/*.md` as files, and syncs the result back to the database. The **system agent** does the identical thing one level down, for the agents inside a single node. Neither writes to the DB directly; both edit files and let the sync reconcile.

Two details in the run plane are easy to get wrong. `execution_mode` looks like it selects the executor, but it only decides whether a step is a passthrough — everything else routes on whether `child_workflow_id` is set, so `"workforce"` is never actually matched at dispatch. And there are two topological sorts stacked: one ordering steps across the board, a second ordering agents inside a single workforce step.

![Architecture overview](docs/images/architecture.svg)

## What Makes This Interesting

**A unified execution engine under all step types.** Whether a step is a single agent, a full workforce, or a conversational chat session, all LLM execution flows through the same engine. Strategies parameterize it — supplying the system prompt, tools, model, and completion logic — so cross-cutting behavior like streaming, cancellation, and token accounting works identically everywhere.

**Behaviour is composed from filters, not branched into the engine.** Filters hook the execution loop at three points — `on_start` to augment the system prompt, `on_response` to accept or force a retry, `on_output` to transform final content. Seven ship today, including a multi-agent critique panel for step outputs, few-shot injection of exemplary execution traces, chain-of-thought wrapping for structured outputs, schema-validation retry, and recovery of truncated JSON by auto-closing brackets. Adding a behaviour is adding a filter.

**Canvas changes are filtered before any LLM call.** Not every canvas edit is worth dispatching. The board serializer runs a six-stage pipeline on every diff: pan detection (all nodes moving by the same delta is a camera move, not a rearrangement), whitespace normalisation, oscillation detection against a baseline snapshot (you typed it and undid it — net zero), reorder detection (same lines, different order), token-level change scoring on whatever survives, and finally a topological sort so surviving changes reach the agent upstream-first. Only genuinely meaningful changes reach an agent, tiered by significance. Everything else is a direct database write.

**The backend re-renders your drawing in order to see it — built, currently unplugged.** The goal is for a sketch drawn on a node to reach the agent as an image rather than as prose about a shape. The rasteriser is the part that exists: `perfect-freehand`, the pressure-sensitive stroke algorithm the canvas draws with, ported to Rust and numerically verified against the TypeScript original, then filled anti-aliased and cropped to the stroke's bounding box — so what the backend renders is the outline you actually saw, not a re-guess from the input points. The path from there to a live agent isn't connected today.

**Parallel agents share a workspace, and the merge is verified.** Steps running in the same level write through OverlayFS; their diffs are merged before the next batch begins. Non-conflicting changes auto-merge, multi-step modifications go through a three-way diff3, and genuine conflict hunks are resolved by a one-shot LLM call with file-type-aware context extraction. That resolution is then checked programmatically — non-empty, plausible length, imports preserved, still syntactically valid — because a merge you can't verify is a merge you can't ship.

## What I Learned

It was an experiment, so the findings matter more than the feature list.

### Handoff fidelity matters more than handoff volume

The hard part wasn't execution. It was the protocol between agents.

When an upstream agent is uncertain and passes rich detail downstream, the receiving agent has no way to separate its guesses from its knowledge — everything arrives as established fact. The next agent builds on it, elaborates, and the invention hardens. By the third hop the fabrication *is* the spec, and every agent after that is faithfully implementing something nobody asked for.

Brevity fixes it. A short, bounded summary of the job as it stands forces the downstream agent to derive detail from the actual source of truth rather than inherit an invention. Detail isn't free — detail from an uncertain agent is worse than none.

That's why the board-level handoff to a node is constrained rather than free-form. The workflow architect writes **what** a node must accomplish, never **how** to staff it, across five fixed fields: the node's role, what it receives and from where, what it must produce, the constraints the user actually stated, and how it relates to its neighbours. Team composition is the system agent's decision, made one level down with the context to make it. See [`config/manager/builder/system.md`](config/manager/builder/system.md).

### Mappable context

The other thing I tried was making context *mappable* rather than readable. Instead of handing a new agent the conversation so far, hand it a set of discrete statements it can navigate — each tagged, each carrying a confidence score, so it knows not just what is believed but how firmly. Uncertainty that travels *with* a claim stays uncertainty; uncertainty flattened into fluent prose becomes fact at the next hop, which is the failure above.

That grew into a side experiment and a short paper: [**Belief-Oriented Conversation Architecture**](proto/paper.md). A gatekeeper agent reads the full source and authors *belief slices*; downstream agents never see the source at all and reason entirely from the slices they are given. Seven phases of results, including where it loses to simply passing the whole source along, are in the paper.

### Refactoring at scale is a different skill from writing at scale

The experiment code carries its own history. Phases 2 through 6 are standalone scripts that grew by copy-and-extend:

```
proto/phase2.py    483 lines
proto/phase3.py    782
proto/phase4.py   1207
proto/phase5.py   1093
proto/phase6.py   1452
```

Phase 7 came in at 635 lines while doing more, because the shared machinery was pulled into a package first — `schemas`, `prompts`, `claims`, `questions`, `sources`, `client`, `formatters`. The cost of the refactor was paid once; every phase after it was cheaper to write and easier to trust.

## Prerequisites

- Rust (via [rustup](https://rustup.rs))
- Node.js + npm
- Docker + Docker Compose (for Postgres, MinIO, and JuiceFS)
- A DeepInfra API key — the active provider, set by `ACTIVE_PROVIDER` in `src/constants.rs`. xAI, Anthropic and local Ollama are also wired up; switching is a one-line change there.

## Setup

```bash
cp .env.example .env
# fill in DEEPINFRA_API_KEY and JWT_SECRET at minimum
# BRAVE_SEARCH_API_KEY too, if you want the agents' web tools to work
```

```bash
# start Postgres, MinIO, JuiceFS
make server-up

# start backend + frontend dev servers (migrations run automatically on startup)
make dev
```

There's no signup page — create your first account by hitting the register endpoint directly:

```bash
./scripts/seed-user.sh
# or: EMAIL=me@example.com PASSWORD=***** ./scripts/seed-user.sh
```

Then log in at the frontend with that email/password.

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
- [`proto/paper.md`](proto/paper.md) — *Belief-Oriented Conversation Architecture*, the belief-slice experiment and its results

## License

[MIT](LICENSE). Third-party attributions are in [NOTICE](NOTICE).
