# Backend Architecture

## The 5-Layer Stack

```
┌─ API Handlers ──────────────────────────────────────────────┐
│  Parse request → call service → return response             │
│  For background work: return immediately → tokio::spawn(..)  │
│  src/server/api/                                            │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌─ Services ───────────────┼──────────────────────────────────┐
│  Domain logic. CRUD, validation, ownership checks,           │
│  file<->DB projection for the design-plane agents.           │
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
│  hub/protocols/  ← protocol compiler + template resolution   │
└─────────────────────────────────────────────────────────────┘
```

## The Engine + Strategy Pattern

Every LLM call in the app flows through `ExecutionEngine::execute(&strategy)` (`src/server/hub/execution/engine/mod.rs`).

The **engine** owns the loop:
1. Send messages + system prompt to LLM
2. If LLM returns tool calls → execute them via `strategy.execute_tool()`
3. Append tool results → go to 1
4. If LLM returns end_turn (or a `requires_terminal_tool()` fires) → call `strategy.on_complete()` → done

The **strategy** (`ExecutionStrategy` trait, `src/server/hub/execution/strategy.rs`) tells the engine *what* to use: system prompt, available tools, how to execute each tool, model/temperature/max rounds, streaming on/off, and post-processing.

### The current strategies (`src/server/hub/execution/strategies/`)

| Strategy | File | Purpose |
|----------|------|---------|
| `ChatStrategy` | `strategies/chat/` | Interactive chat: the generic per-agent chat (`run_chat`) and step-scoped chat (`run_step_chat`, per-node or manager-mode assistant). Streams via SSE. |
| `DagStepStrategy` | `strategies/dag_step/` | A single non-workforce workflow step (`single`/`container` execution mode) run through the DAG executor. |
| `WorkforceAgentStrategy` | `strategies/workforce_agent/` | One roster agent inside a workforce step. 3-way tool dispatch (container → local execution context → context-free). Used by the shared runner in `hub/dag/pipeline/agent_executor.rs` regardless of whether the roster came from the system-node agent's files or (historically) a DB-driven designer. |
| `WorkflowAgentStrategy` | `strategies/workflow_agent/` | **Primary board-design agent.** Conversational; edits `topology.json` + `nodes/*.md` in a board repo via `run_command`, syncs to DB on every command and on completion. |
| `SystemNodeStrategy` | `strategies/system_node/` | **Primary per-node design agent.** Runs in a container; writes `config.json`, `topology.json`, `agents/*.json`; signals done via `complete_system`. Replaces the old builder+designer pair (see `src/server/executors/dispatch/system_node.rs:8`: "Old: DispatchStrategy → complete_task → run_designer_after_builder / New: SystemNodeStrategy → complete_system → sync_to_db"). |
| `ManagerDispatchStrategy` | `strategies/manager_dispatch/` | The old L2 "manager builder": tool-call topology editing (`create_pipeline`, `insert_node`, `wire_edge`, `dispatch_to_builders`, ...) for a step with `execution_mode == "manager"`. Still wired up end-to-end (`chat/tools.rs`, `executors/manager_dispatch/`, `services/dispatch/`), but nothing in the current board/step-creation code path (`services/board/executor.rs`) ever creates a step with that mode — new workflows go through the workflow agent instead. Kept for compatibility with pre-existing "manager" steps. |

`DispatchStrategy` and `AgentDesignerStrategy` (the old per-node builder / designer-prephase pair) no longer exist anywhere in the codebase.

## The Design Plane: Workflow Agent + System Node Agent

This file-based pair is the **primary** way structure and agents get authored today. It replaced the old tool-call dispatch pipeline (topology tools + `complete_task`/`complete_design` handoffs). Two instances of the same pattern, one level apart:

- **Workflow agent** (whole board) — `services/workflow_agent/`, `strategies/workflow_agent/`. Projects the DB (steps + edges) out to a repo at `{workspace}/workflows/{workflow_id}/board/`, lets the LLM edit `topology.json` and one `nodes/{slug}.md` per node via shell commands, then syncs the repo back to DB (`sync::sync_to_db`). Entry point: `hub::run_workflow_agent_chat`, triggered by the chat consumer when a session's `draft_config.role == "workflow_agent"` (session created by `GET/POST /api/workflows/:id/agent-session`).
  - `topology.json`: `{"nodes": {"<slug>": {"depends_on": ["<slug>", ...]}}}` (validated in `services/workflow_agent/validate.rs`).
  - `nodes/{slug}.md`: free-form node instruction text; only checked for non-emptiness.
- **System node agent** (one node) — `services/system_node/`, `strategies/system_node/`. Same pattern, one level down: writes `config.json` (`name`, `description`), `topology.json` (`{"agents": {"<slug>": {"depends_on": [...]}}}`), and `agents/{slug}.json` (`name`, `system_prompt`, `assignment`, `expected_output`, `capabilities`) to `{workspace}/workflows/{workflow_id}/system_node/{step_id}/`. Runs in a container with `run_command` + `complete_system`; `complete_system`'s `verify` object makes the agent attest to checklist items (`prompts_not_trivial`, `assignments_expanded`, `no_filenames_prescribed`, ...) which the backend cross-checks against the files (`services/system_node/validate.rs`) before accepting completion.

Every write from either agent is validated immediately (`validate_written_files` in each strategy), including cross-reference checks (dangling deps, cycles, topology↔file mismatches) once `topology.json` exists.

Neither agent writes to the DB directly — both edit files, and a sync step (`sync.rs` in each service) reconciles files → DB. The execution engine later reads those files back to run the actual agents (see **Workforce execution** below). Per the README: *"The handoff between the two [planes] is `system_node/<step-id>/agents/*.json` on disk, not a function call."*

### How the system-node agent gets triggered per node

`POST /api/workflows/:id/board/submit` (`src/server/api/board/mod.rs`) still does the direct-manipulation path for canvas edits:
1. **Phase 0** (`services/board/executor.rs`, agentless) — turns the changeset into DB writes: create/update/delete/rewire/move steps and edges. New nodes are created with `execution_mode = "workforce"`. No LLM calls.
2. **Sequential design pipeline** (`services/dispatch/sequential.rs::run_sequential_design_pipeline`, spawned in the background) — walks the workflow in topological levels and, for each step with a dispatch instruction, calls `run_system_node_dispatch` (parallel within a level via `JoinSet`); downstream steps whose upstream `designer_handoff` changed get a propagation-only re-design. This is the "Dispatch designs the agents" phase described in the README.

The step-scoped chat session (`dispatch`/`cancel_dispatch` tools, `strategies/chat/dispatch.rs` → `services/dispatch::dispatch_to_builder`) is a second entry point into the same executors — it's how a user's freeform instruction in a step's chat panel reaches `executors::dispatch::system_node::run_system_node_task` (or, for `execution_mode == "manager"`, `executors::manager_dispatch`).

## The DAG Executor (`src/server/hub/dag/`)

| Submodule | Role |
|-----------|------|
| `orchestration/` | The inner loop (`run_dag_loop`): iterates topologically-sorted steps, applies guards (cancellation, pinned replay, dead-path elimination, conditional edges), routes each step via `dispatch::dispatch_step`. |
| `file_executor/` | Reads the system-node agent's `topology.json` + `agents/*.json` and executes the configured roster through the shared runner. The execution bridge between design plane and run plane. |
| `pipeline/` | The **shared agent-execution core**: level scheduling (`agent_executor.rs`), output composition (`output.rs`), and `runner.rs::run_agent_execution` — container lifecycle, per-level dispatch via `WorkforceAgentStrategy`, output recording. Used by `file_executor`. (Comments in this module and in `dispatch.rs` still describe it as also serving "the legacy Pipeline" with its own `DesignerPhase` — that DB-driven designer pre-phase and its `designer.rs`/`lifecycle.rs` no longer exist in the codebase; today `pipeline/` has no independent entry point of its own.) |
| `single/` | Executes one non-workforce step through `DagStepStrategy`. |
| `dag_state/`, `utils/`, `container/`, `merge/`, `resume/`, `templates/`, `versioning/`, `workshop/` | Shared execution state/types, pure DAG helpers, container+VPN sidecar lifecycle, parallel-overlay merge, pause/approval resume, frozen run snapshots, immutable content versioning, and the interactive node-by-node ("workshop") execution mode respectively. `workshop/dispatch.rs` also calls `file_executor::execute_from_files` directly. |

### Dispatch routing (`orchestration/dispatch.rs::dispatch_step`)

```rust
match step.execution_mode.as_str() {
    "context" | "input" => execute_passthrough(...),        // no LLM call
    _ if step.child_workflow_id.is_some() => {               // file-based workforce execution
        file_executor::execute_from_files(...)
    }
    _ => execute_with_agent(...),                             // single-agent execution
}
```

Two details that are easy to get wrong (also called out in the README):

- **`execution_mode` only decides passthrough.** Everything else routes on whether `child_workflow_id` is set — so `"workforce"` is never actually matched by string at dispatch time. A workforce step just happens to be the only kind that has a `child_workflow_id`.
- **`"container"` has no distinct dispatch handling.** It's a recognized value in the `execution_mode` comment (`src/db/types/workflow.rs:27`: `"single", "workforce", "context", "input", "container"`) but nothing branches on it specifically — it falls into the same `execute_with_agent` catch-all as `"single"`, which requires the step to have an `agent_id`.
- There is no `sub_workflow` execution mode. Recursion into a child workflow (a distinct concept from the workforce roster's own child workflow) is driven the same way — by `child_workflow_id.is_some()` — via `Box::pin` recursion in the orchestration loop.

## Workforce Execution: Primary Path vs. Legacy Fallback

`workforce` is the flagship (and, as of today, only) node archetype — `src/server/api/archetypes/mod.rs`'s `ARCHETYPES` const has a single entry, `"workforce"`.

- **Primary path**: `file_executor::execute_from_files()` reads `topology.json` + `agents/*.json` written by the system-node agent, patches in real roster-entry UUIDs from the DB roster row, and delegates to `pipeline::run_agent_execution` (the shared runner). If no files exist yet, it errors rather than falling back — there is no live code path left that builds a roster from a DB-only designer phase (see the `pipeline/` note above).
- **Shared core**: `dag/pipeline/runner.rs::run_agent_execution` — container creation → agent level dispatch (`WorkforceAgentStrategy` per roster agent, in topologically-sorted levels so independent agents run in parallel) → overlay extraction → container teardown → output composition → result recording.

## Services vs Hub: The Pipeline Confusion

Two things are both called "pipeline" but do different jobs — worth keeping straight when grepping:

| Module | What it does |
|--------|-------------|
| `services/pipeline/` | **CRUD** for the child workflow attached to a workforce step (`create_pipeline`, `add_step`, `add_edge`, `destroy_pipeline`, ...). Used by the manager builder's topology tools and by protocol `apply`. |
| `hub/dag/pipeline/` | **Execution** — the shared runner that dispatches a workforce's agent roster during a workflow run (used by `file_executor`). Despite the name, it no longer contains a "pipeline" object or a designer phase of its own — just level-scheduling + output-composition + the runner. |

## API Handlers (`src/server/api/`)

Top-level modules: `agent_context/`, `agent_executions/`, `agent_roster/`, `agents/`, `archetypes/`, `auth/`, `board/`, `cancellation/`, `chat/`, `collections/`, `config/`, `costs/`, `dispatch/`, `documents/`, `error/`, `health/`, `output_schemas/`, `prompt_templates/`, `protocols/`, `results/`, `room_step_members/`, `rooms/`, `routing_rules/`, `sessions/`, `step_ports/`, `system_config/`, `timeline/`, `tools/`, `workflows/`.

`workflows/` is the largest and holds most step/run endpoints:

| File | Endpoints |
|------|-----------|
| `workflow_handlers.rs` | Workflow CRUD + `get_or_create_workflow_agent_session` (the workflow agent's persistent chat session) |
| `step_handlers.rs` | Step CRUD |
| `step_chat_handlers.rs` | Step-scoped chat session lifecycle (find-or-create, clear, debug prompt inspection) |
| `edge_handlers.rs` | Edge CRUD |
| `run_handlers.rs` | Trigger workflow execution |
| `last_run_handlers.rs` | Fetch most-recent-run results per step (also holds `build_step_run_response()`, shared with run-detail) |
| `run_detail_handlers.rs` | Fetch a specific historical run's results |
| `live_state_handlers.rs` | Live in-progress run state |
| `workshop_handlers.rs` | Node-by-node ("workshop") interactive execution — thin layer over `hub::dag::workshop` |
| `sub_dag_handlers.rs` | Internal execution sub-DAG (designer → agent → agent phases) for visualizing a step's protocol run |
| `execution_handlers.rs` | List a workflow's execution history |
| `document_handlers.rs`, `template_handlers.rs`, `version_handlers.rs` | Step document attachments; run template CRUD (promote/list/get/delete); workflow version checkpoints (list/save/restore) |

`board/mod.rs` handles `POST /api/workflows/:id/board/submit` (Phase 0 + sequential design dispatch, see above) and `GET /api/workflows/:id/board/elements` (rebuilds canvas elements from live step/edge state).

`dispatch/mod.rs` is the direct REST entry point into `services::dispatch::dispatch_to_builder` (the third caller alongside the chat `dispatch` tool and the manager's `dispatch_to_builders` tool).

`collections/mod.rs` is the API surface for the Collection DAG (see "Implemented, not yet frontend-exposed" below).

## Services (`src/server/services/`)

| Module | Responsibility |
|--------|---------------|
| `workflow_agent/` | Board repo projection, file reading, validation, sync-to-DB, version checkpoints — backs `WorkflowAgentStrategy` |
| `system_node/` | Same, one level down, for a single node's file repo — backs `SystemNodeStrategy` |
| `board/` | Board submit orchestration: classify → diff → filter → execute (Phase 0) pipeline |
| `dispatch/` | Shared dispatch orchestration (`dispatch_to_builder`, session find-or-create, task registry, sequential design pipeline) |
| `pipeline/` | Child-workflow CRUD for workforce nodes (see "Pipeline Confusion" above) |
| `steps/`, `edges/`, `step_ports/` | Step/edge/port CRUD |
| `agent_roster/`, `routing_rules/` | Workforce roster CRUD, label-based agent routing |
| `agents/`, `agent_context/`, `agent_executions/` | Agent CRUD, document-linkage context, execution history (list/get/approve/mark-exemplary) |
| `protocols/` | Protocol CRUD, port management, resolution/application to steps |
| `run_results/`, `workflow_state/` | Per-step run response building; shared node-status resolution |
| `workflows/`, `collections/` | Workflow CRUD; workflow-collection ("DAG of workflows") CRUD |
| `sessions/`, `chat/`, `messaging/` | Chat/session CRUD, message validation & history, cross-session message injection |
| `canvas_sync/` | Live canvas-change ingestion from the frontend |
| `documents/`, `output_schemas/`, `prompt_templates/`, `results/` | Document CRUD/search; output-schema CRUD; prompt-template CRUD; structured result storage |
| `rooms/` | Room + member + session lifecycle |
| `costs/`, `timeline/` | Spend tracking; unified execution debug stream |
| `system_config/`, `tools/` | System-wide config CRUD; tool + agent-tool-assignment CRUD |
| `system_store/` | Backing store for the implicit `store_read_file`/`store_write_file` workforce tools |
| `workspace/` | JuiceFS-backed workflow workspace paths (`board_path`, `system_node_path`, ...) |
| `ownership/` | Shared ownership-verification helpers |

## Tools (`src/server/tools/`)

| Module | Tools |
|--------|-------|
| `execution/` | `run_command` and friends — wraps FileOps/GitOps/TestRunner/Sandbox (container, local, and file-IO backends) behind one dispatcher, `dispatch_tool_cascade` |
| `system_node/` | `complete_system` — the system-node agent's only dedicated tool (verification/attestation schema) |
| `manager/` | Topology tools for the manager builder (L2): `create_pipeline`, `create_parallel`, `insert_node`, `remove_node`, `wire_edge`, `remove_edge`, plus `resolve/` (node/name resolution helpers) |
| `node_assistant/` | Universal per-node config tools: `set_node_name`, `set_node_description`, `render_panel` |
| `documents/` | Document CRUD tools (read, create, update, search) |
| `haiku/` | Utility-model calls: summarize, extract context, title generation |
| `shared/` | Helpers shared across tool modules (e.g. step loading, arg validation) |
| `system_store.rs` | `store_read_file` / `store_write_file` — implicit tools auto-available to every workforce agent (paths under `.system/artifacts/...`) |

There is no `workforce/` tools directory anymore — the old `add_agent`/`set_capabilities`/`set_system_prompt` tool-call configuration has been replaced by the system-node agent writing `agents/*.json` files directly.

A separate crate-level registry (`src/tools/registry/`, `get_tool_definition`) holds shared low-level tool defs (`run_command`, `think`, `render_panel`, ...) that strategies pull from by name; `src/server/tools/` holds the domain-specific handlers described above.

## Key Types

- `AppState` (`src/server/state/mod.rs`) — wraps `Arc<AppStateInner>`, always Clone + Send
- `ExecutionEngine` (`hub/execution/engine/mod.rs`) — wraps `Arc<dyn LLMProvider>`, runs the LLM loop
- `ExecutionStrategy` (`hub/execution/strategy.rs`) — trait that parameterizes the engine
- `DagContext` / `DagExecutionState` (`hub/dag/dag_state/`) — shared/mutable context for a DAG walk (state, repos, cancel token, completed outputs, variables, tokens)
- `StepExecutionEnvelope` (`src/types/execution.rs`) — standard output wrapper per step
- `WorkflowExecutionContext` (`hub/dag/utils/types.rs`) — top-level execution config (container, prior outputs)

## Implemented, Not Yet Frontend-Exposed

These are fully implemented and tested on the backend but have no (or effectively no) frontend surface today — not dead code, just ahead of the UI:

- **Collection DAG** — a workflow-of-workflows orchestrator. `src/server/executors/collection_dag/` (`CollectionDagExecutor`) walks a DAG of `WorkflowCollectionRow`/`CollectionWorkflowRow`/`CollectionWorkflowEdgeRow`/`CollectionRunRow` (`src/db/types/collection.rs`), sequentially or in parallel per dependency level, and delegates each node to `execute_workflow_via_engine`. It has a real API (`src/server/api/collections/mod.rs`) and a corresponding frontend type file + hand-rolled store (`frontend/src/types/collection.ts`, `frontend/src/stores/collectionStore.ts`), but the store is only re-exported from `frontend/src/stores/index.ts` — no page or component actually imports and renders it, so there's no way for a user to reach this feature today.
- **`execution_mode = "container"`** — a recognized DB value (`src/db/types/workflow.rs:27`) with no distinct dispatch handling; it currently behaves identically to `"single"` at `orchestration/dispatch.rs`'s catch-all.
- **Protocol compiler extension point** — `src/server/hub/protocols/compilers/mod.rs` is an explicit empty stub: `// Protocol compilers — currently empty. // Future protocol types will register their compilers here.` Only `workforce` is registered as a protocol/archetype today.
