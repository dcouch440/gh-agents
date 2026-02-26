# Backend Architecture

## The 5-Layer Stack

```
┌─ API Handlers ──────────────────────────────────────────────┐
│  Parse request → call service → return response             │
│  For background work: return 202 → tokio::spawn(executor)   │
│  src/server/api/                                            │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌─ Services ───────────────┼──────────────────────────────────┐
│  Stateless domain logic. No LLM calls. No background work.  │
│  CRUD, validation, ownership checks.                         │
│  src/server/services/                                        │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌─ Executors ──────────────┼──────────────────────────────────┐
│  Background workers (tokio::spawn).                          │
│  Load config → call Hub entry point → persist results.       │
│  src/server/executors/                                       │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌─ Hub ────────────────────┼──────────────────────────────────┐
│  ALL LLM execution goes through here.                        │
│  src/server/hub/                                             │
│                                                              │
│  hub/execution/  ← engine, strategies, recorder, streaming   │
│  hub/board/      ← serializer, state, overview               │
│  hub/context/    ← beliefs, capabilities, dispatch_status    │
│  hub/dag/        ← DAG executor (walks the graph)            │
└─────────────────────────────────────────────────────────────┘
```

## The Engine + Strategy Pattern

Every LLM call in the app flows through `ExecutionEngine::execute(&strategy)`.

The **engine** owns the loop:
1. Send messages + system prompt to LLM
2. If LLM returns tool calls → execute them via `strategy.execute_tool()`
3. Append tool results → go to 1
4. If LLM returns end_turn → call `strategy.on_complete()` → done

The **strategy** tells the engine *what* to use:
- System prompt
- Available tools
- How to execute each tool
- Model, temperature, max rounds
- Post-processing (token logging, execution recording)

### The 6 Strategies

| Strategy | File | Purpose |
|----------|------|---------|
| `ChatStrategy` | `strategies/chat/` | Interactive user chat (streaming) |
| `DagStepStrategy` | `strategies/dag_step/` | Single-agent workflow step |
| `WorkforceAgentStrategy` | `strategies/workforce_agent/` | One roster agent in a workforce pipeline |
| `DispatchStrategy` | `strategies/dispatch/` | Per-Node Builder (L4) — configures a single node's workforce |
| `ManagerDispatchStrategy` | `strategies/manager_dispatch/` | Manager Builder (L2) — creates topology + dispatches to builders |
| `AgentDesignerStrategy` | `strategies/agent_designer/` | Designer pre-phase — generates agent system prompts |

## The DAG Executor

`hub/dag/` walks the workflow graph. For each step, it dispatches by execution mode:

| Mode | Handler | What happens |
|------|---------|-------------|
| `workforce` | `dag/pipeline/` | Designer phase → sequential agent roster execution |
| single-agent | `dag/single/` | One engine run with `DagStepStrategy` |
| `sub_workflow` | `dag/sub_workflow/` | Recursive DAG execution via `Box::pin` |
| `context` / `input` | passthrough | No LLM — data flows through unchanged |

### Workforce Pipeline (`dag/pipeline/`)

This is the runtime for workforce nodes. It runs during workflow execution:

1. Load mission brief + agent roster
2. Resolve port inputs from upstream
3. Run designer phase (generates per-agent system prompts)
4. Execute agents in level order (respecting dependencies)
5. Compose combined output

Submodules:
- `designer.rs` — Designer pre-phase (or static fallback)
- `agent_executor.rs` — Runs agents grouped by execution level
- `lifecycle.rs` — `PipelinePhase` trait for composable phases
- `output.rs` — Composes combined workforce output
- `types.rs` — `WorkforceStepEnv`, `DesignedAgentPrompt`, etc.

## Services vs Hub: The Pipeline Confusion

Two modules are both called "pipeline" but do completely different things:

| Module | What it does |
|--------|-------------|
| `services/pipeline/` | **CRUD** — create child workflow, add agents, add edges, destroy. Called by workforce tools during configuration. |
| `hub/dag/pipeline/` | **Execution** — run the designer + agent roster during a workflow run. Called by the DAG executor. |

The service builds the structure. The hub executes it.

## The Dispatch Pipeline

Two entry points converge at the Per-Node Builder:

```
Chat path:                          Board path:
  User types in chat                  User draws on canvas → submits
       │                                    │
       ▼                                    ▼
  Manager Assistant                   Phase 0 (agentless)
  (ChatStrategy)                      DB writes: create nodes,
       │ dispatch()                   delete removed, rewire edges
       ▼                                    │
  Manager Builder (L2)                Agentless fan-out
  (ManagerDispatchStrategy)           Iterates changeset,
  Creates topology + content          dispatches per node
  dispatch_to_builders                     │
       │                                    │
       └────────────┬───────────────────────┘
                    ▼
             Per-Node Builder (L4)
             (DispatchStrategy)
             Configures workforce:
             agents, prompts, tools
                    │
                    ▼
             Node ready for execution
```

## API Handlers (`src/server/api/`)

Key handler groups:

| Module | Endpoints |
|--------|-----------|
| `chat/` | Chat message send + SSE streaming |
| `workflows/` | Workflow CRUD |
| `steps/` | Step CRUD + configuration |
| `run_handlers` | Trigger workflow execution |
| `last_run_handlers` | Fetch execution results |
| `board_handlers` | Board submit (Phase 0 + dispatch) |
| `workshop_handlers` | Workshop step execution |
| `agent_executions/` | Execution history + SSE streaming |
| `protocols/` | Protocol CRUD + assignment |
| `rooms/` | Room session management |

## Services (`src/server/services/`)

| Module | Responsibility |
|--------|---------------|
| `dispatch/` | Orchestrates dispatch jobs (find/create session, spawn executor) |
| `pipeline/` | Child workflow CRUD for workforce nodes |
| `protocols/` | Protocol CRUD, resolution, application (`crud.rs`, `resolve.rs`, `apply.rs`) |
| `run_results/` | Build step run responses from execution history |
| `rooms/` | Room session lifecycle |
| `steps/` | Step CRUD + validation |
| `messaging/` | Message routing between agents |

## Tools (`src/server/tools/`)

| Module | Tools |
|--------|-------|
| `execution.rs` | Filesystem, git, sandbox, test runner + document tools (via `dispatch_tool_cascade`) |
| `documents/` | Document CRUD (read, create, update, search) |
| `workforce/` | Workforce configuration (add_agent, set_system_prompt, set_capabilities, etc.) |
| `manager/` | Topology tools (create_pipeline, insert_node, wire_edge, etc.) |
| `node_assistant/` | Node config tools (set_node_name, set_node_description, render_panel) |
| `haiku/` | Utility LLM calls (summarize, extract context) |
| `shared/` | Common tool helpers |

## Key Types

- `AppState` — wraps `Arc<AppStateInner>`, always Clone + Send
- `ExecutionEngine` — wraps `Arc<dyn LLMProvider>`, runs the LLM loop
- `ExecutionStrategy` — trait that parameterizes the engine
- `DagContext` — shared context for DAG execution (state, repos, cancel token)
- `DagExecutionState` — mutable state during DAG walk (completed outputs, variables, tokens)
- `StepExecutionEnvelope` — standard output wrapper per step
- `WorkflowExecutionContext` — top-level execution config (container, prior outputs)
