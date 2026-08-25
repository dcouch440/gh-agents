# nexor

**An experiment: can agents design their own agents?**

Rather than hand-writing orchestration, could a designer agent read a plain-language goal, decide what roles the work needs, write their system prompts and tool assignments, and hand a structured plan to an executor that runs it?

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

## The Workforce Model

The core primitive is the **workforce step** — a single node in the canvas that internally runs a coordinated team of agents.

### Two-Phase Design

Every workforce is designed before it executes. When you modify a node, a **designer agent** runs first: it receives your instruction, reasons about what agents are needed, writes their system prompts and tool assignments, and defines the dependency graph between them. The executor reads that design and resolves the topology before the first agent runs.

This separation means the design of a workforce is inspectable and editable — it exists as a structured artifact, not just an implicit LLM call.

```
 ┌───────────────────────────────────┐
 │         User instruction          │
 │                                   │
 │  "Research the latest trends in   │
 │   AI safety and write a report"   │
 └───────────────┬───────────────────┘
                 │  plain language goal
                 ▼
 ┌───────────────────────────────────┐
 │         Designer Agent            │
 │                                   │
 │  · Decides which agents are needed│
 │  · Writes system prompts per agent│
 │  · Defines dependency graph       │
 │  · Assigns tools per agent        │
 └───────────────┬───────────────────┘
                 │  structured design
                 ▼
 ┌───────────────────────────────────┐
 │         Workforce Executor        │
 │                                   │
 │  · Reads design                   │
 │  · Topological sort → levels      │
 │  · Agents in same level run in    │
 │    parallel (tokio JoinSet)       │
 │  · Cross-agent output routing     │
 └───────────────────────────────────┘
```

### Dependency-Based Parallelism

Agents within a workforce declare which upstream agents they depend on. The executor resolves this into execution levels via topological sort. Agents in the same level run in parallel; each level waits for the previous to complete.

```
 Level 0:  [Researcher]                 ← no dependencies, runs first
 Level 1:  [Analyst] [Summarizer]       ← both depend on Researcher, run in parallel
 Level 2:  [Writer]                     ← depends on both, runs last
```

This means a workforce automatically exploits concurrency wherever the dependency graph allows it — without any manual configuration.

### Shared Workspace

When a workforce runs in a containerized environment, all agents in the step share a single workspace. Files written by one agent are visible to the next. This enables file-based handoff — an agent can produce a document, code file, or structured artifact, and a downstream agent reads and builds on it directly.

The container is created once at the start of the step and torn down after all agents complete. A filesystem diff is captured and stored, so every file change across the entire workforce is tracked.

## Architecture Overview

```
 ┌─────────────────────────────────────────────────┐
 │                    Browser                      │
 │    Canvas  ·  Sidebar  ·  Activity panel        │
 │              ▲ SSE stream                       │
 └──────────────┼──────────────────────────────────┘
                │ HTTP / SSE
 ┌──────────────┼──────────────────────────────────┐
 │           Rust / Axum API                       │
 │                                                 │
 │  ┌──────────────────────────────────────────┐   │
 │  │             Execution Hub                │   │
 │  │                                          │   │
 │  │  DAG Orchestrator                        │   │
 │  │  · Topological sort                      │   │
 │  │  · Level-based parallelism               │   │
 │  │  · Port-based typed data flow            │   │
 │  │  · Conditional edge pruning              │   │
 │  │            │                             │   │
 │  │            ▼                             │   │
 │  │  Step Dispatch                           │   │
 │  │  · pass-through  (context/input nodes)   │   │
 │  │  · workforce     (multi-agent team)      │   │
 │  │  · single agent  (standard step)         │   │
 │  │            │                             │   │
 │  │            ▼                             │   │
 │  │  Execution Engine  (unified LLM loop)    │   │
 │  │  · Pluggable strategies per step type    │   │
 │  │  · Tool dispatch + multi-round loops     │   │
 │  │  · Composable response filter pipeline   │   │
 │  │  · Streaming via SSE sink                │   │
 │  └──────────────────────────────────────────┘   │
 │                                                 │
 │  Board Serializer  →  Noise Filter  →  Dispatch │
 │  (canvas diffs)       (multi-stage)   (LLM/DB)  │
 └─────────────────────────────────────────────────┘
          │                       │
 ┌────────┴────────┐    ┌─────────┴────────┐
 │   PostgreSQL    │    │   LLM Providers  │
 └─────────────────┘    └──────────────────┘
```

## What Makes This Interesting

**A unified execution engine under all step types.** Whether a step is a single agent, a full workforce, or a conversational chat session, all LLM execution flows through the same engine. Strategies parameterize it — supplying the system prompt, tools, model, and completion logic — so cross-cutting behavior like streaming, cancellation, and token accounting works identically everywhere.

**Behaviour is composed from filters, not branched into the engine.** Filters hook the execution loop at three points — `on_start` to augment the system prompt, `on_response` to accept or force a retry, `on_output` to transform final content. Seven ship today, including a multi-agent critique panel for step outputs, few-shot injection of exemplary execution traces, chain-of-thought wrapping for structured outputs, schema-validation retry, and recovery of truncated JSON by auto-closing brackets. Adding a behaviour is adding a filter.

**Canvas changes are filtered before any LLM call.** Not every canvas edit is worth dispatching. The board serializer runs a five-stage noise pipeline on every diff: pan detection (all nodes moving by the same delta is a camera move, not a rearrangement), whitespace normalisation, oscillation detection against a baseline snapshot (you typed it and undid it — net zero), reorder detection (same lines, different order), then token-level change scoring on whatever survives. Only genuinely meaningful changes reach an agent, tiered by significance. Everything else is a direct database write.

**The backend re-renders your drawing in order to see it.** Freehand strokes aren't handed to the model as raw coordinates. The server rasterises them — `perfect-freehand`, the pressure-sensitive stroke algorithm the canvas draws with, ported to Rust and numerically verified against the TypeScript original, so the outline the backend fills is the one you actually saw. Strokes then leave by one of two paths depending on the model: an ASCII grid for text-only models, or an anti-aliased PNG cropped to the stroke's bounding box for vision models.

**Parallel agents share a workspace, and the merge is verified.** Steps running in the same level write through OverlayFS; their diffs are merged before the next batch begins. Non-conflicting changes auto-merge, multi-step modifications go through a three-way diff3, and genuine conflict hunks are resolved by a one-shot LLM call with file-type-aware context extraction. That resolution is then checked programmatically — non-empty, plausible length, imports preserved, still syntactically valid — because a merge you can't verify is a merge you can't ship.

## What I Learned

It was an experiment, so the findings matter more than the feature list.

### Handoff fidelity matters more than handoff volume

The hard part wasn't execution. It was the protocol between agents.

When a designer agent is uncertain and passes rich detail downstream, the receiving agent has no way to separate its guesses from its knowledge — everything arrives as established fact. The next agent builds on it, elaborates, and the invention hardens. By the third hop the fabrication *is* the spec, and every agent after that is faithfully implementing something nobody asked for.

Brevity fixes it. A short, bounded summary of the job as it stands forces the downstream agent to derive detail from the actual source of truth rather than inherit an invention. Detail isn't free — detail from an uncertain agent is worse than none.

That's why the designer's handoff is constrained rather than free-form. It writes **what** a node must accomplish, never **how** to staff it, across five fixed fields: the node's role, what it receives and from where, what it must produce, the constraints the user actually stated, and how it relates to its neighbours. Team composition is the downstream builder's decision, made with the context to make it. See [`config/manager/builder/system.md`](config/manager/builder/system.md).

### Confidence-scored one-liners beat transcripts

When a new agent joins an ongoing conversation, it doesn't need the transcript. It needs a small set of factual statements, each carrying a confidence score — so it knows not just what is believed, but how firmly.

I tested this rather than assuming it, in a side experiment that grew into a short paper: [**Belief-Oriented Conversation Architecture**](proto/paper.md). A gatekeeper agent reads the full source and authors *belief slices* — tagged, confidence-weighted statements of understanding. Downstream agents never see the source at all; they reason entirely from the slices they are given.

Converged belief slices scored **26/30 against full context's 27/30**, at roughly a fifth of the token cost, compressing 70 raw beliefs into 22 contradiction-free ones. Against adversarially poisoned sources, the revision pass identified and killed every planted distortion from the structure of the belief store alone, with no access to ground truth.

The confidence score is what makes it work, and it is the fix for the failure above. Uncertainty that travels *with* a claim stays uncertainty. Uncertainty flattened into fluent prose becomes fact at the next hop.

The paper reports where the approach loses, too — full context still wins outright when the source fits in the window, and convergence dropped a claim in Phase 5 that only better prompting recovered.

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
- [`proto/paper.md`](proto/paper.md) — *Belief-Oriented Conversation Architecture*, the belief-slice experiment and its results

## License

[MIT](LICENSE). Third-party attributions are in [NOTICE](NOTICE).
