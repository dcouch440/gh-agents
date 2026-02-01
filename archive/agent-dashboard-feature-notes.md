# Agent Dashboard Feature - Planning Notes

## Date: 2026-01-31

## Problem Statement

The pre-defined agents and chat views aren't working well. Too many hardcoded tools and calls. The user wants a **self-service dashboard** to design, configure, and manage agent teams from the UI — not from code.

## Current State Summary

### What Exists (Backend)

- **3-tier agent hierarchy**: Orchestrator → Worker → Utility (hardcoded in `AgentTier` enum)
- **Agents persisted in DB** (`migrations/003_create_agents.sql`): tier, persona, model config, status
- **11 execution tools hardcoded** in `src/agents/execution_tools.rs`: file ops, git ops, run tests, run command
- **Role system** (`src/agents/roles.rs`): RoleLibrary with predefined roles, categories, delegation rules, required reading
- **Dispatcher + Pool + Scheduler** manage agent lifecycle and task routing
- **Task dependencies** tracked in DB, dependency-aware queue
- **Channel-based async runtime**: each agent is a tokio task with command/response channels

### What Exists (Frontend)

- **AgentsPage** (`ui/src/pages/AgentsPage/`): read-only overview with tier gauges and status dots
- **TasksPage** (`ui/src/pages/TasksPage/`): Kanban board with priority and status filtering
- **ChatPage** (`ui/src/pages/ChatPage/`): mode-based chat with SSE streaming
- **SettingsPage** (`ui/src/pages/SettingsPage/`): model config per tier, pool limits, autonomy level
- **Zustand stores** for agents, tasks, config, sessions

### What's Hardcoded (Needs to Become Dynamic)

1. **Tool definitions** — 11 tools baked into Rust code, no DB registry
2. **Agent tiers** — Only 3 tiers, no custom hierarchies
3. **Role library** — Roles defined in code, not user-configurable
4. **Delegation rules** — `can_delegate_to` set in role definitions, not editable
5. **Routing rules** — Router has hardcoded priority rules in `src/orchestration/router.rs`
6. **Tool allowlists** — Set per-task via `TaskConstraints.allowed_tools`, but no UI for it

## Desired Features

### 1. Code Editor (with bells and whistles)
- Full code editor in the UI for viewing/editing project files
- Syntax highlighting, file tree, multi-tab
- Likely Monaco Editor (VS Code engine)

### 2. Agent Creation & Persistence via UI
- Create agents from the dashboard, not just from system startup
- Configure: tier, persona (name, system prompt, style), model, tools
- Agents saved to DB and available across sessions
- CRUD operations for agents

### 3. Custom Hierarchies / Team Designer
- Move beyond fixed Orchestrator → Worker → Utility
- Let users define their own team structures
- Custom delegation chains (who can assign to whom)
- Visual team builder (drag-and-drop or graph editor)

### 4. Tool Selection Dashboard
- Registry of available tools (persisted in DB)
- Per-agent tool assignment
- Possibly custom tool creation (name, description, schema, handler)
- Tool categories and search

### 5. Task Management Dashboard
- Start tasks from the UI and assign to specific agents/teams
- Monitor progress in real-time
- View agent activity, tool calls, costs
- All from one unified dashboard

## Architecture Gaps to Address

| Gap | Current | Needed |
|-----|---------|--------|
| Tool registry | Hardcoded in Rust | DB-backed, CRUD API |
| Agent creation | Pool spawns at startup | On-demand via API |
| Hierarchies | 3 fixed tiers | User-defined graphs |
| Role config | Code-defined RoleLibrary | DB-backed, UI-editable |
| Delegation | Static `can_delegate_to` | Dynamic, per-team rules |
| Routing | Hardcoded rules in Router | Configurable rule engine |
| Code editor | None (FilesPage is a stub) | Monaco-based editor |
| Team designer | None | New UI component |

## Decisions Made

1. **Custom tiers**: Undecided — keep flexible. Design the schema so tiers are just a label string rather than an enum. The 3 defaults (Orchestrator, Worker, Utility) remain as presets.
2. **Team graphs**: DAG — agents can receive work from multiple parents, but with cycle prevention. Keep it practical, don't over-engineer.
3. **Tool extensibility**: Select from existing registry for now. Schema should allow custom tools later.
4. **Code editor**: Read-write. Full Monaco editor that saves to disk.
5. **Teams scope**: TBD — revisit when we get to team designer.

## Completed Work

### Part 1: Agent CRUD — Server (2026-01-31)

**Files modified:**
- `src/db/mod.rs` — Expanded `AgentRow` with all DB fields (persona_prompt, persona_style, model_provider, model_max_tokens, model_temperature)
- `src/db/pg_repo.rs` — Expanded list/upsert queries, added `get_persisted_agent`
- `src/db/traits.rs` — Added `get_persisted_agent` to `ServerRepo` trait
- `src/server/api.rs` — Added `CreateAgentRequest`, `UpdateAgentRequest`, expanded `AgentResponse`, added create/get/update/delete handlers, updated list_agents to query DB, added 7 integration tests
- `src/server/mod.rs` — Added routes: POST /agents, GET/PATCH/DELETE /agents/:id
- `src/server/tools.rs` — Updated AgentRow construction to include new fields
- `src/server/orchestrator.rs` — Added `get_persisted_agent` to test mock

**Endpoints added:**
- `POST /api/agents` — Create agent with full config
- `GET /api/agents` — List agents from DB (was returning empty vec)
- `GET /api/agents/:id` — Get single agent
- `PATCH /api/agents/:id` — Partial update
- `DELETE /api/agents/:id` — Delete agent

**Tests:** All 1,970 tests pass.

### Part 2: Tool Registry — Server (2026-01-31)

**Files modified:**
- `migrations/024_create_tools.sql` — Created `tools` table (id, user_id, name, description, category, parameter_schema JSONB, output_schema JSONB, enabled) + `agent_tools` join table
- `src/db/mod.rs` — Added `ToolRow` struct
- `src/db/traits.rs` — Added 6 tool methods to `ServerRepo` trait (list_tools, get_tool, upsert_tool, delete_tool, get_agent_tools, set_agent_tools)
- `src/db/pg_repo.rs` — Added `PgToolRow` + implemented all 6 queries (set_agent_tools uses transaction)
- `src/server/api.rs` — Added ToolResponse, CreateToolRequest, UpdateToolRequest, SetAgentToolsRequest, AgentToolsResponse + 7 handlers + 8 tests
- `src/server/mod.rs` — Added routes for /tools, /tools/:id, /agents/:id/tools + updated InMemoryServerRepo mock
- `src/server/orchestrator.rs` — Added tool method stubs to TestRepo mock

**Endpoints added:**
- `POST /api/tools` — Create tool with schema definitions
- `GET /api/tools` — List tools for user
- `GET /api/tools/:id` — Get single tool
- `PATCH /api/tools/:id` — Partial update
- `DELETE /api/tools/:id` — Delete tool
- `GET /api/agents/:id/tools` — Get agent's assigned tools
- `PUT /api/agents/:id/tools` — Set agent's tool assignments

**Notes:** Data-only registry. Tool handlers remain hardcoded in Rust. Registry tracks metadata (name, schemas, enabled) and per-agent assignment. Runtime wiring (executor reads from DB) deferred to future pass.

**Tests:** All 1,978 tests pass.

### Part 3: Pipeline Stage Templates — Server (2026-01-31)

**Files modified:**
- `migrations/025_add_stage_templates.sql` — Added 4 columns to `pipeline_stages`: `stage_name`, `input_definitions`, `output_description`, `output_schema`
- `src/db/mod.rs` — Added 4 fields to `PipelineStageRow`
- `src/agents/pipeline.rs` — Added 4 fields to `PipelineStage`, updated `add_stage()` signature
- `src/db/pg_repo.rs` — Updated list/upsert queries for new columns
- `src/server/tools.rs` — Updated `add_pipeline_stage` tool schema and handler with new fields
- `src/server/state.rs` — Updated pipeline restoration to pass new fields
- `src/server/api.rs` — Added `resolve_template()`, `render_stage_prompt()`, `render_pipeline_stage` endpoint, 7 unit tests

**Endpoints added:**
- `POST /api/pipelines/:id/stages/:stage_number/render` — Render a stage into a resolved markdown prompt given previous stage outputs

**Data model:**
- `stage_name` — unique name within pipeline, used in `{{stage_name.field}}` template refs
- `input_definitions` — JSON array of `{key, source: "static"|"stage", value?, ref?}`
- `output_description` — template text describing the goal, supports `{{}}` refs
- `output_schema` — `{fields: [{name, type, values?, description}]}` output contract

**Notes:** Data-only pass. The render endpoint is a pure function that resolves templates and produces a markdown prompt. No runtime execution wiring — the orchestrator does not yet call render automatically during pipeline runs.

**Tests:** All 1,985 tests pass.

### Part 4: Pipeline-Cluster Wiring + Fan-out + Side Tasks — Server (2026-01-31)

**Files modified:**
- `migrations/026_pipeline_cluster_wiring.sql` — Added cluster_id + fan_out to pipeline_stages, made agent_id nullable, added role + persona_override to cluster_members, created stage_side_tasks table
- `src/db/mod.rs` — Updated PipelineStageRow (agent_id → Option, added cluster_id, fan_out), added StageSideTaskRow
- `src/db/traits.rs` — Added 3 side task methods to ServerRepo (list_stage_side_tasks, upsert_stage_side_task, delete_stage_side_task)
- `src/db/pg_repo.rs` — Updated pipeline stage queries for new columns, added 3 side task query implementations
- `src/agents/pipeline.rs` — Updated PipelineStage (agent_id → Option, added cluster_id, fan_out), updated add_stage() signature, updated test macro
- `src/server/tools.rs` — Updated add_pipeline_stage handler (agent_id/cluster_id both optional, added fan_out), updated start_pipeline for optional agent_id
- `src/server/state.rs` — Updated pipeline restoration for new add_stage params
- `src/server/api.rs` — Added CreateSideTaskRequest, SideTaskResponse, 3 side task handlers, 3 integration tests
- `src/server/mod.rs` — Added side task routes, added `delete` import, updated mock
- `src/server/orchestrator.rs` — Updated pipeline advance for optional agent_id, updated mock

**Endpoints added:**
- `GET /api/pipelines/:id/stages/:n/side-tasks` — List side tasks for a stage
- `POST /api/pipelines/:id/stages/:n/side-tasks` — Create side task
- `DELETE /api/pipelines/:id/stages/:n/side-tasks/:sid` — Delete side task

**Schema changes:**
- `pipeline_stages.agent_id` now nullable (stages can use cluster_id instead)
- `pipeline_stages.cluster_id` — optional FK to clusters table
- `pipeline_stages.fan_out` — boolean, controls array output → N instances of next stage
- `cluster_members.role` — optional role override per cluster member
- `cluster_members.persona_override` — persona override per cluster member
- `stage_side_tasks` — new table for independent parallel agents (id, pipeline_id, stage_number, agent_id, input_definitions, output_name, blocking, output_schema)

**Notes:** Data-only pass. Fan-out execution, side task runtime, and cluster-based stage execution not yet wired — schema and API are ready for it.

**Tests:** All 1,988 tests pass.

### Part 5: Context Injection System — Server (2026-01-31)

**Files modified:**
- `migrations/027_agent_context.sql` — Created `agent_context` join table (agent_id, document_id) with CASCADE deletes
- `src/db/traits.rs` — Added `get_agent_context`, `set_agent_context` to `ServerRepo` trait
- `src/db/pg_repo.rs` — Implemented agent context queries (JOIN + transaction pattern)
- `src/server/api.rs` — Added `SetAgentContextRequest`, `AgentContextResponse`, get/set handlers, updated `resolve_template()` to support `{{context.ref_tag}}` patterns, updated `render_pipeline_stage` to fetch context docs from DB, added 3 unit tests
- `src/server/mod.rs` — Added route: GET/PUT /agents/:id/context, added mock stubs
- `src/server/orchestrator.rs` — Added mock stubs

**Endpoints added:**
- `GET /api/agents/:id/context` — Get agent's linked context documents
- `PUT /api/agents/:id/context` — Set agent's context documents (replaces existing)

**Template system update:**
- `resolve_template()` now accepts `context_docs: &HashMap<String, String>`
- `{{context.ref_tag}}` patterns are resolved by fetching documents from DB by ref_tag
- `render_pipeline_stage` endpoint scans templates for `{{context.*}}` refs and fetches docs before rendering

**Context model:**
- **Agent-level**: documents linked via `agent_context` table, intended for auto-injection into system prompt (set and forget)
- **Stage-level**: `{{context.ref_tag}}` in stage templates, resolved at render time by fetching from documents table
- **Dynamic**: agents can write documents (via existing create_doc tool), later stages reference them with `{{context.ref_tag}}`

**Tests:** All 1,991 tests pass.

## Design Notes

### Core Execution Model: Pipelines and Clusters

**Agents are stateless prompt-in / response-out units.** They don't "delegate." They don't know who came before or after them. The *pipeline* orchestrates everything — agents just receive a prompt and return a response.

**Pipeline** = the job. A sequence of stages that transforms input into output. The pipeline owns the execution flow: what runs, in what order, what fans out.

**Cluster** = a group of agents that handles one stage. A cluster defines *who* does the work at a given stage — which agents, how many, what role/persona they use. Multiple clusters participate in a single pipeline.

**Fan-out** = when a stage produces an array output, the pipeline can spawn one instance of the next stage per item. The next cluster handles each item independently.

**Example pipeline:**
```
Pipeline: "Feature Build"

Stage 1 → Cluster A (1 Orchestrator-type agent)
  Input: static ticket array from DB
  Output: array of individual tickets
  Fan-out: yes → each ticket goes to Stage 2 independently

Stage 2 → Cluster B (N Planner agents)
  Input: single ticket (from fan-out)
  Output: implementation plan
  Fan-out: yes → each plan goes to Stage 3

Stage 3 → Cluster C (N Implementer agents)
  Input: single plan (from fan-out)
  Output: code + report
```

**Key principles:**
1. **No tier system.** Agents are just agents with a persona, model config, and tools. A "smart" agent can hand off to other "smart" agents — the pipeline controls the flow, not a hierarchy.
2. **Agents are unaware of each other.** They receive a rendered prompt (from the stage template system) and return structured output. The pipeline stitches it together.
3. **Clusters are reusable.** The same cluster can appear in multiple pipelines.
4. **Fan-out is optional per stage.** If output is an array and next stage is configured for fan-out, the pipeline spawns N instances. Otherwise it passes the full output as-is.

**Side tasks:**
- Any stage can have zero or more **side tasks** — independent agents that run in parallel with the cluster's main work.
- A side task gets its own input (referencing any previous stage output), produces its own named output, and has a **blocking flag**.
- **Blocking = true**: pipeline waits for the side task before advancing to the next stage.
- **Blocking = false**: pipeline continues; the side task's output becomes available to later stages whenever it finishes.
- Use cases: generating docs in parallel, pre-processing data for a future stage, validation checks, anything that can run alongside the main work.
- Side task output feeds into later stages via the same `{{side_task_name.field}}` template system.

**Schema implications:**
- `clusters` table — id, user_id, name, description
- `cluster_agents` join table — cluster_id, agent_id, role/persona override
- `pipeline_stages` reworked — reference cluster_id instead of agent_id, add fan_out config
- `stage_side_tasks` table — stage reference, agent_id, input source, output_name, blocking flag
- Drop `AgentTier` enum from execution path (keep as optional label/tag if useful for UI)
- Delegation system (`DelegationContext`, `can_delegate_to`) becomes unnecessary — the pipeline handles all routing

### Required Reading Must Be Enforced

Previous iterations had agents with required_reading configured but they never actually read the files. Going forward:

1. **Required reading = cloud documents, not repo files.** These are user-authored docs (conventions, style guides, PRDs, etc.) stored in the DB and managed through the UI editor — not files in the git repo.
2. **Content must be injected into context**, not just referenced. The system prompt should include the actual document contents, not just a "please read X" instruction. The system fetches from DB and injects at task assignment time.
3. **CRUD via the dashboard** — users create, edit, and manage these documents in the UI editor. The role config links documents to roles as required reading.
4. **Validate at task assignment time** — if a required document has been deleted or is empty, surface a warning in the UI rather than silently skipping it.
5. **DB schema needed** — a `documents` table (id, user_id, title, content, created_at, updated_at) and a join to roles via required_reading references.

### Future: Human-in-the-Loop Stage Checkpoints

The pipeline should support pausing at any stage for human interaction before and after execution:

1. **Pre-execution chat** — when a stage is paused, the user can open a chat with the agent to discuss the task. The agent presents its understanding of the work (e.g. "here are the features I'll implement"), and the human refines, adds detail, or corrects before approving execution.
2. **Prompt intervention** — the rendered prompt is visible and editable. The human can modify it before the agent runs. Similar to the existing chat experience but scoped to a specific stage instance.
3. **Post-execution verification** — after a stage completes, the pipeline can loop through a checklist (feature list, requirements, acceptance criteria) and verify each item was addressed before advancing to the next stage.
4. **Stage execution states** — beyond pending/running/done, stages need: `paused_pre`, `awaiting_approval`, `paused_post`, `verified`. Schema should leave room for this.
5. **Chat session per stage instance** — tie a chat/conversation to a specific stage execution so the context is preserved and reviewable.

This is a future feature. The current schema should accommodate it by:
- Including a `status` field on stage executions with extensible states
- Supporting a link between stage instances and chat sessions
- Keeping the rendered prompt as a stored artifact (not just computed on the fly)

## Part 6: Runtime Wiring (Completed)

Wired the template system into actual pipeline execution:

- **Stage output storage**: `PipelineRun.stage_outputs` (HashMap<String, Value>) stores parsed structured output from each completed stage, keyed by stage_name
- **Output parser**: `parse_stage_output()` extracts JSON from LLM output (```json fences or bare objects), falls back to `{"output": "raw text"}`
- **Reusable render_stage()**: Extracted from HTTP endpoint into standalone async fn callable from orchestrator and tools
- **Template rendering in auto-advance**: Orchestrator now renders stage prompts via `render_stage()` with accumulated `stage_outputs` instead of raw text append
- **Template rendering in start_pipeline**: First stage also renders via template system
- **Cluster-based agent selection**: Stages with `cluster_id` pick first member from `cluster_members` table
- **Agent-level context injection**: Context documents loaded via `get_agent_context()` and passed as `required_reading` FileContent entries

Files modified: `src/agents/pipeline.rs`, `src/server/api.rs`, `src/server/orchestrator.rs`, `src/server/tools.rs`, `doc/agent-dashboard-feature-notes.md`

## Part 7: Pipeline Run Persistence + Gate Resume + Token Tracking (Completed)

Persist pipeline execution history to database. Every run, every stage, every token spent.

- **Migration 028**: Two new tables — `pipeline_runs` (run history with JSONB stage_outputs, token totals) and `stage_executions` (per-stage tracking with rendered_prompt, output, user_input, tokens, duration)
- **Row structs**: `PipelineRunRow` and `StageExecutionRow` in `db/mod.rs`
- **7 new trait methods on ServerRepo**: CRUD for pipeline runs and stage executions
- **Token tracking on TaskResult**: Added `input_tokens`, `output_tokens`, `duration_ms` to `channels::TaskResult`; populated from `StreamAccumulator` in `executor.rs` with cross-round accumulation
- **Gate resume endpoint**: `POST /api/pipeline-runs/:run_id/approve` with optional `user_input` body — validates waiting status, stores user_input on stage_execution, records in stage_outputs for downstream template access, resumes pipeline by advancing and dispatching next stage
- **Run history endpoints**: `GET /api/pipeline-runs?pipeline_id=X` and `GET /api/pipeline-runs/:run_id` (includes stage executions)
- **Orchestrator persistence**: Creates/updates stage_execution and pipeline_run rows at each lifecycle event — stage start, completion, failure, waiting_for_approval, pipeline completion
- **Pipeline start persistence**: `execute_start_pipeline` in tools.rs creates pipeline_run and first stage_execution rows

Files modified: `migrations/028_pipeline_runs.sql`, `src/db/mod.rs`, `src/db/traits.rs`, `src/db/pg_repo.rs`, `src/agents/channels.rs`, `src/agents/executor.rs`, `src/server/api.rs`, `src/server/orchestrator.rs`, `src/server/tools.rs`, `src/server/mod.rs`

## Part 8: Pipelines WebSocket Channel (Completed)

Added a 5th broadcast channel `pipelines` for real-time pipeline execution events.

- **PipelineUpdate struct**: run_id, pipeline_id, event, stage_number, stage_name, agent_id, output, tokens, duration, user_input, timestamp, user_id
- **8 lifecycle events**: run_started, stage_started, stage_completed, stage_failed, gate_waiting, gate_resumed, run_completed, run_failed
- **WS integration**: New `CHANNEL_PIPELINES` constant, `ServerMessage::PipelineUpdate` variant, select arm in handle_socket, channel validation
- **Broadcast at every lifecycle point**: orchestrator, tools (run_started), api (gate_resumed) all emit pipeline events alongside existing feed broadcasts

Files modified: `src/server/ws.rs`, `src/server/state.rs`, `src/server/orchestrator.rs`, `src/server/tools.rs`, `src/server/api.rs`

## Part 9: Tool Routing System (Completed)

Replaced hardcoded execution tools with a DB-driven system. Tools can map to clusters. Agents in router_mode get a single `request_assistance` meta-tool.

- **Migration 029**: Added `cluster_id` (FK to clusters, nullable) and `is_builtin` flag to tools table. NULL cluster_id = direct execution, non-NULL = route to cluster
- **Migration 030**: Added `router_mode` boolean to agents table
- **Seed function**: `builtin_tool_rows()` generates the 11 execution tools as `ToolRow` with deterministic UUIDs (v5). `seed_builtin_tools()` called on user registration. Idempotent via `ON CONFLICT DO NOTHING`
- **Dynamic tool loading**: `TaskContext.tool_rows` carries DB-loaded tools. Executor prefers these over hardcoded list. Falls back to hardcoded when empty (backward compat)
- **Cluster dispatch check**: Executor checks `cluster_id` on tool — direct execution for NULL, placeholder error for cluster-routed tools
- **tool_router.rs** (new): `request_assistance_tool()` meta-tool definition, `execute_request_assistance()` dispatcher, `route_to_cluster()` placeholder
- **Router mode**: Agents with `router_mode = true` receive only `request_assistance` as their tool. Executor detects this and routes calls through tool_router
- **Builtin protection**: `delete_tool` SQL filters `AND is_builtin = false`
- **uuid v5 feature**: Added to Cargo.toml for deterministic tool IDs

Files modified: `Cargo.toml`, `migrations/029_tool_routing.sql`, `migrations/030_agent_router_mode.sql`, `src/db/mod.rs`, `src/db/pg_repo.rs`, `src/db/traits.rs`, `src/agents/execution_tools.rs`, `src/agents/tool_router.rs` (new), `src/agents/mod.rs`, `src/agents/executor.rs`, `src/agents/channels.rs`, `src/server/api.rs`, `src/server/tools.rs`, `src/server/orchestrator.rs`, `src/server/mod.rs`

## Open Questions (remaining)

1. Should teams be scoped per-project or global?
2. Fan-in: when multiple fan-out instances complete, does the pipeline need a "reduce" stage that collects all results? Or is that a future concern?
3. Concurrency limits on fan-out — if a stage fans out to 50 items, do we cap parallel execution?
4. Error handling in fan-out — if 1 of N fails, does the whole pipeline fail or continue with partial results?
