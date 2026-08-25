# Database Schema Documentation

**Database:** nexor (PostgreSQL 16)

**Last Updated:** 2026-08-25

This document describes the complete database schema for the nexor platform, a visual workflow design tool for AI agents. Users draw workflows on a canvas (an Excalidraw-compatible element schema), the system builds DAG structure instantly, then designs agents asynchronously. Schema reflects migrations `0001`–`0067` (verified against a live database at that migration version).

**Source of truth:** `migrations/*.sql`, read in order and cross-checked against `\d` output from a running `nexor` database. `migrations/0001_initial_schema.sql` is not a from-scratch initial schema — it is a consolidated `pg_dump` squash of 71 prior incremental migrations (dated 2026-02-05), so tables that look "original" (e.g. `workflow_collections`, `tool_capabilities`) may actually predate every migration in this directory. Anything from an even earlier era (`pipelines`, `tasks`, `agent_modes`, standalone `sessions`) is gone and does not appear below.

---

## Table of Contents

1. [Execution Hierarchy](#execution-hierarchy)
2. [Authentication & Users](#authentication--users)
3. [Core Workflow DAG Engine](#core-workflow-dag-engine)
4. [Agents & Configuration](#agents--configuration)
5. [Workforce Archetype Support](#workforce-archetype-support)
6. [Protocol System (Documenter)](#protocol-system-documenter)
7. [Room Archetype](#room-archetype)
8. [Collections](#collections)
9. [Canvas Persistence & Snapshots](#canvas-persistence--snapshots)
10. [Tool & Capability Routing](#tool--capability-routing)
11. [Chat & Sessions](#chat--sessions)
12. [Documents & Context](#documents--context)
13. [Output Schemas & Prompt Templates](#output-schemas--prompt-templates)
14. [System Config & Files](#system-config--files)
15. [Cost & Token Tracking](#cost--token-tracking)
16. [Version History (Audit Trail)](#version-history-audit-trail)
17. [Key Design Patterns](#key-design-patterns)
18. [Migration History](#migration-history)
19. [Database Connection](#database-connection)

---

## Execution Hierarchy

Nothing named "pipeline" exists in the current schema. The top-level grouping concept is a **collection**, and the run tree is a generic parent/child DAG-execution tree rather than a fixed pipeline→stage ladder:

```
workflow_collections                     (a named group of workflows; execution_mode: sequential | parallel)
  └─ collection_runs                     (one run of a collection)
      └─ workflow_executions             (one per workflow in the collection; depth 0)
          ├─ agent_executions            (execution_type: dag_step | dispatch | agent_designer |
          │                               interactive_review | debate_verification)
          │   ├─ execution_messages      (per-turn conversation history)
          │   ├─ results                 (structured output rows)
          │   └─ token_ledger entries    (cost/usage accounting)
          └─ workflow_executions         (nested child workflow run — a "workforce" step spawns
              └─ agent_executions         one of these; root_execution_id + depth track the tree)
```

A `workflow_execution` can also stand alone with `collection_run_id = NULL` (a workflow run outside any collection). `workflow_executions.root_execution_id`/`depth` give O(1) tree-traversal without recursive CTEs (added in migration `0039`, modeled on Temporal's execution-history pattern). `workflow_executions.execution_mode` is `'full'` for a normal run or `'workshop'` for the single persistent "live editing" execution each workflow keeps (enforced by a partial unique index on `workflow_id WHERE execution_mode = 'workshop'`); it transiently reads `'workshop_rebased'` during a conversation rebase.

`workforce` is **not** a table family — it is `execution_mode = 'workforce'` on a row in `workflow_steps`, the shared DAG engine used by every archetype. A workforce step has a `child_workflow_id` pointing at a nested `workflows` row (a generated roster of agents, editable at design time, snapshotted at execution). `room` is the other live archetype, similarly just `workflow_steps.execution_mode = 'room'` plus a `room_id`. The older `documenter`/`task_force` archetypes were unified into `workforce` in migration `0043` and have near-zero live code references today; their supporting tables (`task_mission_briefs`, `task_agent_roster`, `agent_designer_*`) still back workforce's roster-generation step, and the `protocols` framework still exists but is now scoped to a single `protocol_type = 'documenter'`.

---

## Authentication & Users

### users
User accounts with email/password and optional GitHub OAuth linkage.

```sql
CREATE TABLE users (
    id                        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email                     TEXT UNIQUE NOT NULL,
    password_hash             TEXT,
    github_id                 BIGINT UNIQUE,
    github_login              TEXT,
    github_token_encrypted    TEXT,
    created_at                TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at                TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_admin                  BOOLEAN NOT NULL DEFAULT false
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_github_id ON users(github_id) WHERE github_id IS NOT NULL;
```

### auth_config
Legacy single-row table (`id = 1`) holding a global admin password hash.

```sql
CREATE TABLE auth_config (
    id               INTEGER PRIMARY KEY CHECK (id = 1),
    password_hash    TEXT NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**Note:** There is no standalone `sessions` table. Authentication is stateless JWT (`src/server/auth/mod.rs`, `jsonwebtoken` crate) — a bearer token is verified per request against `state.jwt_secret()`, not looked up in the database. The `sessions` table described in older versions of this document no longer exists.

---

## Core Workflow DAG Engine

Workflows are reusable DAGs. `workflow_steps` are nodes, `workflow_step_edges` are edges. Every archetype (single agent, workforce, room, context/input/container utility steps) is a value of `workflow_steps.execution_mode` — there is one shared execution engine, not per-archetype tables.

### workflows
Top-level DAG definition, optionally bound to a target GitHub repo and container/VPN execution config.

```sql
CREATE TABLE workflows (
    id                        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id                   UUID NOT NULL REFERENCES users(id),
    name                      TEXT NOT NULL,
    description               TEXT NOT NULL DEFAULT '',
    created_at                TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version                   INTEGER NOT NULL DEFAULT 1,
    execution_mode            TEXT NOT NULL DEFAULT 'parallel',
    container_enabled         BOOLEAN NOT NULL DEFAULT false,
    target_repo_url           TEXT,
    target_branch             TEXT,
    vpn_enabled               BOOLEAN NOT NULL DEFAULT false,
    board_overview_summary    TEXT NOT NULL DEFAULT ''
);

CREATE INDEX idx_workflows_user ON workflows(user_id);
```

- `container_enabled`: steps execute inside persistent Docker containers with a clone of `target_repo_url`/`target_branch`.
- `vpn_enabled` (requires `container_enabled`): each agent container gets a WireGuard peer behind a VPN sidecar.
- `board_overview_summary`: a one-paragraph Haiku-generated summary of every step's assistant notes on this board, injected into each step assistant's system prompt.

### workflow_steps
A node in the DAG — an agent invocation, a workforce/room archetype anchor, or a utility node (context/input/container), plus canvas position and a large amount of design-time assistant state.

```sql
CREATE TABLE workflow_steps (
    id                              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_id                     UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    agent_id                        UUID REFERENCES agents(id),
    execution_mode                  TEXT NOT NULL DEFAULT 'single', -- single | workforce | room | context | input | container
    agent_execution_mode            TEXT,                            -- sequential | parallel; NULL = inherit from workflow
    for_each_ref                    TEXT,
    for_each_label_field            TEXT,
    prompt_template_id              UUID REFERENCES prompt_templates(id),
    prompt_template                 TEXT NOT NULL DEFAULT '',
    system_prompt_suffix            TEXT,
    output_schema_id                UUID REFERENCES output_schemas(id),
    output_variable_name            TEXT,
    interactive_agent_id            UUID REFERENCES agents(id),
    display_order                   INTEGER NOT NULL DEFAULT 0,
    room_id                         UUID REFERENCES rooms(id),
    version                         INTEGER NOT NULL DEFAULT 1,
    position_x                      DOUBLE PRECISION,
    position_y                      DOUBLE PRECISION,
    width                           DOUBLE PRECISION DEFAULT 200,
    height                          DOUBLE PRECISION DEFAULT 100,
    routing_mode                    TEXT,
    routing_field                   TEXT,
    cavernous_config_document_id    UUID REFERENCES documents(id), -- legacy, vestigial (CavernousStepStrategy removed)
    reasoning_trace                 BOOLEAN NOT NULL DEFAULT false,
    verification_agent_ids          JSONB, -- array of agent UUIDs that critique this step's output
    name                            TEXT,
    visible                         BOOLEAN DEFAULT true, -- false = executes but hidden from canvas
    description                     TEXT NOT NULL DEFAULT '',
    board_context_cache             TEXT NOT NULL DEFAULT '', -- Haiku-distilled per-node board awareness
    board_context_updated_at        TIMESTAMPTZ,
    goal_summary                    TEXT NOT NULL DEFAULT '', -- distilled conversational intent for this node
    goal_summary_updated_at         TIMESTAMPTZ,
    child_workflow_id               UUID REFERENCES workflows(id) ON DELETE SET NULL, -- workforce: live nested workflow
    ref_id                          TEXT, -- stable readable id, e.g. "workforce-1" (LLM-facing references)
    pinned                          BOOLEAN NOT NULL DEFAULT false,
    run_results_summary             TEXT NOT NULL DEFAULT '',
    designer_handoff                TEXT NOT NULL DEFAULT ''
);

CREATE INDEX idx_workflow_steps_workflow ON workflow_steps(workflow_id);
CREATE INDEX idx_workflow_steps_agent ON workflow_steps(agent_id);
CREATE INDEX idx_workflow_steps_routing ON workflow_steps(routing_mode) WHERE routing_mode IS NOT NULL;
CREATE INDEX idx_ws_child_workflow ON workflow_steps(child_workflow_id) WHERE child_workflow_id IS NOT NULL;
```

`agent_id` is nullable (utility steps and hidden protocol steps have no agent). `routing_mode`/`routing_field` pair with `step_routing_rules` below for label-based branching.

### workflow_step_edges
Edges between steps, with optional typed port wiring and conditional branching.

```sql
CREATE TABLE workflow_step_edges (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_id           UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    from_step_id          UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    to_step_id            UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    from_output_port      TEXT,
    to_input_port         TEXT,
    transform_jsonpath    TEXT,
    condition_type        TEXT,
    condition_value       JSONB,
    edge_label            TEXT,
    UNIQUE (workflow_id, from_step_id, to_step_id)
);

CREATE INDEX idx_workflow_step_edges_from ON workflow_step_edges(from_step_id);
CREATE INDEX idx_workflow_step_edges_to ON workflow_step_edges(to_step_id);
CREATE INDEX idx_workflow_step_edges_ports ON workflow_step_edges(from_output_port, to_input_port);
CREATE INDEX idx_workflow_step_edges_workflow ON workflow_step_edges(workflow_id);
```

### workflow_step_agents
Multiple agents assigned to one step (debate/verification/majority-vote fan-out), distinct from a single-agent step or a workforce roster.

```sql
CREATE TABLE workflow_step_agents (
    step_id               UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    agent_id              UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    execution_strategy    TEXT NOT NULL,
    agent_order           INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (step_id, agent_id)
);

CREATE INDEX idx_workflow_step_agents_step_id ON workflow_step_agents(step_id);
CREATE INDEX idx_workflow_step_agents_agent_id ON workflow_step_agents(agent_id);
```

### step_inputs / step_outputs
Named, typed ports on a step for the port-based wiring system (independent of the free-text `prompt_template`).

```sql
CREATE TABLE step_inputs (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_step_id    UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    port_name           TEXT NOT NULL,
    port_type           TEXT NOT NULL,
    required            BOOLEAN NOT NULL DEFAULT false,
    default_value       JSONB,
    description         TEXT,
    json_schema         JSONB,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (workflow_step_id, port_name)
);

CREATE INDEX idx_step_inputs_step ON step_inputs(workflow_step_id);

CREATE TABLE step_outputs (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_step_id    UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    port_name           TEXT NOT NULL,
    port_type           TEXT NOT NULL,
    json_path           TEXT NOT NULL,
    description         TEXT,
    json_schema         JSONB,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (workflow_step_id, port_name)
);

CREATE INDEX idx_step_outputs_step ON step_outputs(workflow_step_id);
```

### step_routing_rules
Maps a label value (extracted via `workflow_steps.routing_field`) to the agent that should handle it, for label-based conditional routing.

```sql
CREATE TABLE step_routing_rules (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_step_id    UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    label_value         TEXT NOT NULL,
    agent_id            UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    description         TEXT,
    display_order       INTEGER NOT NULL DEFAULT 0,
    UNIQUE (workflow_step_id, label_value)
);

CREATE INDEX idx_step_routing_rules_step ON step_routing_rules(workflow_step_id);
CREATE INDEX idx_step_routing_rules_agent ON step_routing_rules(agent_id);
```

### workflow_executions
One run of a workflow. Optionally belongs to a `collection_run`; optionally nested under a parent execution via `root_execution_id`/`depth` (a workforce step spawning its child workflow).

```sql
CREATE TABLE workflow_executions (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    collection_run_id    UUID REFERENCES collection_runs(id) ON DELETE CASCADE,
    workflow_id          UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    user_id              UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status               TEXT NOT NULL,
    started_at           TIMESTAMPTZ,
    completed_at         TIMESTAMPTZ,
    outputs              JSONB,
    error                TEXT,
    execution_mode       TEXT NOT NULL DEFAULT 'full', -- full | workshop | workshop_rebased
    template_id          UUID REFERENCES run_templates(id),
    root_execution_id    UUID REFERENCES workflow_executions(id),
    depth                INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_workflow_executions_workflow_id ON workflow_executions(workflow_id);
CREATE INDEX idx_workflow_executions_user_id ON workflow_executions(user_id);
CREATE INDEX idx_workflow_executions_collection_run_id ON workflow_executions(collection_run_id);
CREATE INDEX idx_workflow_executions_status ON workflow_executions(status);
CREATE INDEX idx_workflow_executions_root ON workflow_executions(root_execution_id) WHERE root_execution_id IS NOT NULL;
CREATE INDEX idx_workflow_executions_depth ON workflow_executions(root_execution_id, depth);
CREATE INDEX idx_workflow_executions_active ON workflow_executions(status, started_at DESC)
    WHERE status IN ('pending', 'running');
CREATE INDEX idx_workflow_executions_latest ON workflow_executions(workflow_id, user_id, started_at DESC)
    WHERE execution_mode <> 'workshop';
CREATE UNIQUE INDEX idx_workflow_executions_workshop_unique ON workflow_executions(workflow_id)
    WHERE execution_mode = 'workshop';
```

### agent_executions
One agent's turn within a workflow execution — the core token/cost/trace record. `agent_id` and `workflow_execution_id` are both nullable so workforce roster members (not real rows in `agents`) and non-DAG executions (chat dispatch) can still get a row.

```sql
CREATE TABLE agent_executions (
    id                           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id                     UUID REFERENCES agents(id),
    workflow_step_id             UUID REFERENCES workflow_steps(id) ON DELETE SET NULL,
    workflow_execution_id        UUID REFERENCES workflow_executions(id) ON DELETE CASCADE,
    is_interactive               BOOLEAN NOT NULL DEFAULT false,
    parent_agent_execution_id    UUID REFERENCES agent_executions(id),
    system_prompt_rendered       TEXT NOT NULL,
    input                        TEXT NOT NULL,
    output                       TEXT,
    structured_output            JSONB,
    status                       TEXT NOT NULL DEFAULT 'running',
    started_at                   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at                 TIMESTAMPTZ,
    room_session_id              UUID REFERENCES room_sessions(id),
    speaker_order                INTEGER,
    selected_router_mode_id      UUID REFERENCES tool_router_modes(id) ON DELETE SET NULL,
    is_exemplary                 BOOLEAN NOT NULL DEFAULT false, -- marks this run as a few-shot demonstration
    trace                        JSONB, -- serialized streaming trace (tokens, tool calls, errors)
    execution_type               TEXT NOT NULL DEFAULT 'dag_step'
        -- dag_step | dispatch | agent_designer | interactive_review | debate_verification
);

CREATE INDEX idx_agent_executions_agent ON agent_executions(agent_id);
CREATE INDEX idx_agent_executions_step ON agent_executions(workflow_step_id);
CREATE INDEX idx_agent_executions_workflow_execution_id ON agent_executions(workflow_execution_id);
CREATE INDEX idx_agent_executions_status ON agent_executions(status);
CREATE INDEX idx_agent_executions_started ON agent_executions(started_at DESC);
CREATE INDEX idx_agent_executions_parent ON agent_executions(parent_agent_execution_id);
CREATE INDEX idx_agent_executions_room ON agent_executions(room_session_id) WHERE room_session_id IS NOT NULL;
CREATE INDEX idx_agent_executions_router_mode ON agent_executions(selected_router_mode_id);
CREATE INDEX idx_agent_executions_type ON agent_executions(execution_type);
CREATE INDEX idx_agent_executions_exemplary ON agent_executions(agent_id, workflow_step_id) WHERE is_exemplary = true;
CREATE INDEX idx_agent_executions_active ON agent_executions(status, started_at DESC)
    WHERE status IN ('pending', 'running');
CREATE INDEX idx_agent_executions_dag_resume ON agent_executions(workflow_step_id, workflow_execution_id)
    WHERE status = 'completed' AND is_interactive = false;
```

There is no `stage_execution_id` column — that table (`stage_executions`) no longer exists. `agent_executions` links directly to `workflow_execution_id` and `workflow_step_id`.

### execution_messages
Per-turn conversation history for an agent execution (tool calls included via `tool_call_id`).

```sql
CREATE TABLE execution_messages (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_execution_id    UUID NOT NULL REFERENCES agent_executions(id) ON DELETE CASCADE,
    role                  TEXT NOT NULL,
    content               TEXT NOT NULL,
    tool_call_id          TEXT,
    input_tokens          BIGINT NOT NULL DEFAULT 0,
    output_tokens         BIGINT NOT NULL DEFAULT 0,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_execution_messages_execution ON execution_messages(agent_execution_id);
CREATE INDEX idx_execution_messages_role ON execution_messages(agent_execution_id, role);
CREATE INDEX idx_execution_messages_created ON execution_messages(created_at);
```

### results
A named structured-output row produced by an agent execution, validated against an `output_schemas` entry.

```sql
CREATE TABLE results (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id               UUID NOT NULL REFERENCES users(id),
    agent_execution_id    UUID NOT NULL REFERENCES agent_executions(id),
    output_schema_id      UUID REFERENCES output_schemas(id),
    name                  TEXT NOT NULL,
    data                  JSONB NOT NULL,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_results_user ON results(user_id);
CREATE INDEX idx_results_execution ON results(agent_execution_id);
CREATE INDEX idx_results_schema ON results(output_schema_id);
```

---

## Agents & Configuration

### agents
Core agent definition: model config, system prompt, optional output schema/tool router.

```sql
CREATE TABLE agents (
    id                         UUID PRIMARY KEY,
    user_id                    UUID REFERENCES users(id), -- NULL = system-owned agent
    name                       TEXT NOT NULL,
    system_prompt              TEXT NOT NULL DEFAULT '',
    persona_style              TEXT DEFAULT 'casual',
    model_provider             TEXT NOT NULL DEFAULT 'anthropic',
    model_id                   TEXT NOT NULL,
    model_max_tokens           INTEGER NOT NULL DEFAULT 4096,
    model_temperature          REAL NOT NULL DEFAULT 0.7,
    status                     TEXT DEFAULT 'idle',
    router_mode                BOOLEAN DEFAULT false,
    created_at                 TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version                    INTEGER NOT NULL DEFAULT 1,
    output_schema_id           UUID REFERENCES output_schemas(id) ON DELETE SET NULL,
    router_id                  UUID REFERENCES tool_routers(id) ON DELETE SET NULL,
    default_reasoning_trace    BOOLEAN DEFAULT false,
    is_system                  BOOLEAN NOT NULL DEFAULT false
);

CREATE INDEX idx_agents_user_id ON agents(user_id);
CREATE INDEX idx_agents_status ON agents(status);
CREATE INDEX idx_agents_output_schema ON agents(output_schema_id);
CREATE INDEX idx_agents_router ON agents(router_id);
```

**Note:** The old `current_task` FK to `tasks(id)` is gone — `tasks` no longer exists (dropped in migration `0044`, "unused standalone task entity"). `agent_modes`/`agent_modes_versions` (a dynamic-mode/LLM-classifier system) were also dropped (migration `0038`) in favor of `tool_routers`/`tool_router_modes` below.

### agent_tools
Join table: which tools an agent can call directly (independent of any tool router).

```sql
CREATE TABLE agent_tools (
    agent_id    UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    tool_id     UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
    PRIMARY KEY (agent_id, tool_id)
);

CREATE INDEX idx_agent_tools_tool ON agent_tools(tool_id);
```

### agent_context
Join table: which documents an agent has standing access to.

```sql
CREATE TABLE agent_context (
    agent_id       UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    document_id    UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    PRIMARY KEY (agent_id, document_id)
);

CREATE INDEX idx_agent_context_agent ON agent_context(agent_id);
```

### agent_guidances
Distilled feedback/suggestions for an agent that persist across restarts, optionally scoped to one workflow step. Used for few-shot-style behavioral correction rather than raw exemplar replay.

```sql
CREATE TABLE agent_guidances (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id            UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    workflow_step_id    UUID REFERENCES workflow_steps(id) ON DELETE SET NULL,
    suggestions         JSONB NOT NULL DEFAULT '[]'::JSONB,
    source              TEXT NOT NULL DEFAULT 'manual',
    version             INTEGER NOT NULL DEFAULT 1,
    is_active           BOOLEAN NOT NULL DEFAULT true,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_agent_guidances_agent_id ON agent_guidances(agent_id);
CREATE INDEX idx_agent_guidances_lookup ON agent_guidances(agent_id, workflow_step_id, is_active);
```

---

## Workforce Archetype Support

A `workflow_steps` row with `execution_mode = 'workforce'` needs a task brief and an agent roster before its `child_workflow_id` sub-workflow can be generated. These four tables back that "Agent Designer" pre-lifecycle step. Column defaults still say `'task_force'` in places — that was the pre-`0043` name for this same mechanism before it was unified with `documenter` into `workforce`; the tables were never renamed.

### task_mission_briefs
One brief per workforce step (`UNIQUE(step_id)`): what the roster is being asked to do.

```sql
CREATE TABLE task_mission_briefs (
    id                        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    step_id                   UUID NOT NULL UNIQUE REFERENCES workflow_steps(id) ON DELETE CASCADE,
    task_description          TEXT NOT NULL DEFAULT '',
    available_capabilities    TEXT[] NOT NULL DEFAULT '{}',
    failure_mode              TEXT NOT NULL DEFAULT 'fail_fast',
    downstream_context        TEXT,
    created_at                TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at                TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### task_agent_roster
The generated (or user-edited) list of agents for a mission brief. `child_step_id` links a roster entry to its materialized visual step inside the workforce step's child workflow.

```sql
CREATE TABLE task_agent_roster (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    mission_brief_id    UUID NOT NULL REFERENCES task_mission_briefs(id) ON DELETE CASCADE,
    name                TEXT NOT NULL,
    role_description    TEXT NOT NULL DEFAULT '',
    capabilities        TEXT[] NOT NULL DEFAULT '{}',
    execution_order     INTEGER NOT NULL DEFAULT 0,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    child_step_id       UUID REFERENCES workflow_steps(id) ON DELETE SET NULL
);

CREATE INDEX idx_tar_child_step ON task_agent_roster(child_step_id) WHERE child_step_id IS NOT NULL;
```

### agent_designer_runs
One LLM call that generated (or regenerated) a roster's prompts/tools, with token/cost metadata. Generalized in migration `0029` to serve `task_force`, `documenter`, and `room` archetypes via `archetype`/`phase`, not just workforce.

```sql
CREATE TABLE agent_designer_runs (
    id                       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_execution_id    UUID NOT NULL,
    stage_execution_id       UUID NOT NULL, -- legacy name; no FK (stage_executions table no longer exists)
    step_id                  UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    mission_brief_id         UUID REFERENCES task_mission_briefs(id) ON DELETE CASCADE,
    model_id                 TEXT NOT NULL,
    input_tokens             BIGINT NOT NULL DEFAULT 0,
    output_tokens            BIGINT NOT NULL DEFAULT 0,
    cost_usd                 REAL NOT NULL DEFAULT 0.0,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    archetype                TEXT NOT NULL DEFAULT 'task_force',
    phase                    TEXT NOT NULL DEFAULT ''
);

CREATE INDEX idx_designer_runs_step ON agent_designer_runs(step_id);
CREATE INDEX idx_designer_runs_execution ON agent_designer_runs(workflow_execution_id);
```

### agent_designer_outputs
One generated system-prompt/task-prompt/tool-assignment triple per roster entry.

```sql
CREATE TABLE agent_designer_outputs (
    id                         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    designer_run_id            UUID NOT NULL REFERENCES agent_designer_runs(id) ON DELETE CASCADE,
    agent_roster_entry_id      UUID REFERENCES task_agent_roster(id) ON DELETE CASCADE,
    agent_name                 TEXT NOT NULL,
    assigned_tools             TEXT[] NOT NULL DEFAULT '{}',
    generated_system_prompt    TEXT NOT NULL,
    generated_task_prompt      TEXT NOT NULL,
    design_reasoning           TEXT NOT NULL DEFAULT '',
    execution_order            INTEGER NOT NULL DEFAULT 0,
    created_at                 TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    source_entity_id           TEXT NOT NULL DEFAULT '',
    source_archetype           TEXT NOT NULL DEFAULT 'task_force',
    protocol_execution_id      UUID REFERENCES protocol_executions(id)
);

CREATE INDEX idx_designer_outputs_run ON agent_designer_outputs(designer_run_id);
CREATE INDEX idx_designer_outputs_protocol_exec ON agent_designer_outputs(protocol_execution_id)
    WHERE protocol_execution_id IS NOT NULL;
```

---

## Protocol System (Documenter)

A "protocol" is a reusable, system-owned recipe that expands into workflow primitives. The layer originally supported several protocol types (`decomp`, `transform`, `review`, `route`, `default`); all of those were deleted and the `protocol_type` CHECK constraint now only allows `'documenter'` (migration `0021`). `protocol_executions` and `protocol_document_defs` remain the live audit/config trail for the documenter feature and for any step that generates named documents (`workflow_steps.execution_mode` utility steps, workforce deliverables via `agent_roster_entry_id`).

### protocols
System-owned protocol definitions (no `user_id` — seeded, not user-created).

```sql
CREATE TABLE protocols (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name                  TEXT NOT NULL UNIQUE,
    description           TEXT NOT NULL DEFAULT '',
    protocol_type         TEXT NOT NULL CHECK (protocol_type = ANY (ARRAY['documenter'])),
    config                JSONB NOT NULL DEFAULT '{}'::JSONB,
    version               INTEGER NOT NULL DEFAULT 1,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    agent_id              UUID REFERENCES agents(id) ON DELETE SET NULL,
    output_schema_id      UUID REFERENCES output_schemas(id) ON DELETE SET NULL,
    prompt_template_id    UUID REFERENCES prompt_templates(id) ON DELETE SET NULL
);

CREATE INDEX idx_protocols_type ON protocols(protocol_type);
CREATE INDEX idx_protocols_agent_id ON protocols(agent_id);
CREATE INDEX idx_protocols_output_schema_id ON protocols(output_schema_id);
CREATE INDEX idx_protocols_prompt_template_id ON protocols(prompt_template_id);
```

### protocol_ports
Named agent slots within a protocol definition.

```sql
CREATE TABLE protocol_ports (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    protocol_id      UUID NOT NULL REFERENCES protocols(id) ON DELETE CASCADE,
    port_name        TEXT NOT NULL,
    description      TEXT NOT NULL DEFAULT '',
    agent_id         UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    display_order    INTEGER NOT NULL DEFAULT 0,
    UNIQUE (protocol_id, port_name)
);

CREATE INDEX idx_protocol_ports_protocol_id ON protocol_ports(protocol_id);
```

### workflow_step_protocols
Links a protocol to the workflow step that anchors its expansion (one protocol per step).

```sql
CREATE TABLE workflow_step_protocols (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_step_id     UUID NOT NULL UNIQUE REFERENCES workflow_steps(id) ON DELETE CASCADE,
    protocol_id          UUID NOT NULL REFERENCES protocols(id) ON DELETE CASCADE,
    applied_expansion    JSONB NOT NULL DEFAULT '{}'::JSONB,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_wsp_protocol_id ON workflow_step_protocols(protocol_id);
```

### protocol_document_defs
A document target: name, description, and target length, scoped either to a template protocol (`protocol_id`) or an applied step (`step_id`) — exactly one is set.

```sql
CREATE TABLE protocol_document_defs (
    id                       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    step_id                  UUID REFERENCES workflow_steps(id) ON DELETE CASCADE,
    name                     TEXT NOT NULL,
    description              TEXT NOT NULL DEFAULT '',
    target_length            INTEGER NOT NULL DEFAULT 2000,
    display_order            INTEGER DEFAULT 0,
    created_at               TIMESTAMPTZ DEFAULT NOW(),
    protocol_id              UUID REFERENCES protocols(id) ON DELETE CASCADE,
    document_id              UUID REFERENCES documents(id),
    agent_roster_entry_id    UUID REFERENCES task_agent_roster(id) ON DELETE SET NULL,
    CONSTRAINT check_document_def_scope CHECK (
        (step_id IS NOT NULL AND protocol_id IS NULL) OR (step_id IS NULL AND protocol_id IS NOT NULL)
    )
);

CREATE INDEX idx_protocol_document_defs_step_id ON protocol_document_defs(step_id);
CREATE INDEX idx_protocol_document_defs_protocol_id ON protocol_document_defs(protocol_id);
CREATE INDEX idx_pdd_agent_roster ON protocol_document_defs(agent_roster_entry_id) WHERE agent_roster_entry_id IS NOT NULL;
```

### protocol_executions
Hidden execution audit trail — one row per phase of a protocol run (e.g. a documenter's `strategy`/`research`/`write` phases, or a workforce/room agent's turn). The `phase` CHECK was dropped in migration `0037` to generalize this table beyond documenter.

```sql
CREATE TABLE protocol_executions (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    protocol_step_id     UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    workflow_run_id      UUID,
    phase                TEXT NOT NULL,
    document_def_id      UUID REFERENCES protocol_document_defs(id),
    agent_id             UUID REFERENCES agents(id),
    input_prompt         TEXT,
    output_content       TEXT,
    status               TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','running','complete','failed')),
    error_message        TEXT,
    tokens_in            INTEGER,
    tokens_out           INTEGER,
    cost_usd             DOUBLE PRECISION,
    model                TEXT,
    capabilities_used    TEXT[],
    created_at           TIMESTAMPTZ DEFAULT NOW(),
    completed_at         TIMESTAMPTZ,
    agent_name           TEXT,
    archetype            TEXT,
    designer_run_id      UUID REFERENCES agent_designer_runs(id)
);

CREATE INDEX idx_protocol_executions_step_id ON protocol_executions(protocol_step_id);
CREATE INDEX idx_protocol_executions_run_id ON protocol_executions(workflow_run_id);
CREATE INDEX idx_protocol_executions_run_step ON protocol_executions(workflow_run_id, protocol_step_id);
```

### belief_extraction_plans / beliefs
Design-time config (`belief_extraction_plans`, one per step) plus runtime-extracted structured facts (`beliefs`) pulled out of generated documents/execution output — feeds downstream steps' context without re-reading full documents.

```sql
CREATE TABLE belief_extraction_plans (
    id                        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    step_id                   UUID NOT NULL UNIQUE REFERENCES workflow_steps(id) ON DELETE CASCADE,
    extraction_focus          TEXT NOT NULL DEFAULT '',
    tag_vocabulary            TEXT[] NOT NULL DEFAULT '{}',
    contradiction_handling    TEXT NOT NULL DEFAULT 'flag',
    confidence_threshold      TEXT NOT NULL DEFAULT 'low',
    created_at                TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at                TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE beliefs (
    id                          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_id                 UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    workflow_execution_id       UUID, -- nullable: chat-sourced beliefs have no run
    source_step_id              UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    source_document_title       TEXT,
    source_document_def_id      UUID REFERENCES protocol_document_defs(id) ON DELETE SET NULL,
    source_phase                TEXT NOT NULL DEFAULT 'execution',
    content                     TEXT NOT NULL,
    reasoning                   TEXT NOT NULL,
    belief_type                 TEXT NOT NULL DEFAULT 'fact',
    confidence                  TEXT NOT NULL DEFAULT 'medium',
    confidence_justification    TEXT,
    semantic_tags               TEXT[] NOT NULL DEFAULT '{}',
    emotional_tone              TEXT,
    cross_source_tension        TEXT,
    source_step_name            TEXT NOT NULL,
    extraction_model            TEXT NOT NULL,
    extraction_tokens_in        INTEGER NOT NULL DEFAULT 0,
    extraction_tokens_out       INTEGER NOT NULL DEFAULT 0,
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_beliefs_workflow ON beliefs(workflow_id);
CREATE INDEX idx_beliefs_workflow_execution ON beliefs(workflow_execution_id);
CREATE INDEX idx_beliefs_source_step ON beliefs(source_step_id);
CREATE INDEX idx_beliefs_semantic_tags ON beliefs USING GIN(semantic_tags);
CREATE INDEX idx_beliefs_type ON beliefs(belief_type);
CREATE INDEX idx_beliefs_source_doc ON beliefs(source_document_title);
CREATE INDEX idx_beliefs_source_phase ON beliefs(source_phase);
```

---

## Room Archetype

A `workflow_steps` row with `execution_mode = 'room'` points at a `rooms` row — a multi-agent conversation with turn-taking and an optional gatekeeper. `room_step_configs`/`room_step_members` hold the design-time blueprint before it's materialized into a real room + agents at run time.

### rooms
A multi-agent conversation configuration, optionally scoped to a `workflow_collections` entry.

```sql
CREATE TABLE rooms (
    id                          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id                     UUID NOT NULL REFERENCES users(id),
    name                        TEXT NOT NULL,
    gatekeeper_enabled          BOOLEAN NOT NULL DEFAULT false,
    gatekeeper_model_id         TEXT NOT NULL DEFAULT 'claude-haiku-4-20250414',
    max_speakers_per_turn       INTEGER NOT NULL DEFAULT 4,
    max_turns                   INTEGER NOT NULL DEFAULT 20,
    tools_enabled               BOOLEAN NOT NULL DEFAULT false,
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    collection_id               UUID REFERENCES workflow_collections(id) ON DELETE SET NULL,
    default_output_schema_id    UUID REFERENCES output_schemas(id),
    aggregation_mode            TEXT DEFAULT 'final_speaker'
);

CREATE INDEX idx_rooms_user ON rooms(user_id);
CREATE INDEX idx_rooms_output_schema ON rooms(default_output_schema_id) WHERE default_output_schema_id IS NOT NULL;
```

`pipeline_id` (a hard `NOT NULL` FK to the old `pipelines` table) is gone; `collection_id` is its nullable replacement.

### room_members
Agent membership in a room, with optional per-member input/output schemas.

```sql
CREATE TABLE room_members (
    room_id             UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    agent_id            UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    display_name        TEXT,
    role_description    TEXT NOT NULL,
    display_order       INTEGER NOT NULL DEFAULT 0,
    input_schema_id     UUID REFERENCES output_schemas(id),
    output_schema_id    UUID REFERENCES output_schemas(id),
    output_name         TEXT,
    PRIMARY KEY (room_id, agent_id)
);

CREATE INDEX idx_room_members_agent ON room_members(agent_id);
CREATE INDEX idx_room_members_input_schema ON room_members(input_schema_id) WHERE input_schema_id IS NOT NULL;
CREATE INDEX idx_room_members_output_schema ON room_members(output_schema_id) WHERE output_schema_id IS NOT NULL;
```

### room_sessions
A runtime instance of a room conversation.

```sql
CREATE TABLE room_sessions (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_id               UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    status                TEXT NOT NULL DEFAULT 'active',
    current_turn          INTEGER NOT NULL DEFAULT 0,
    transcript_summary    TEXT,
    started_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at          TIMESTAMPTZ,
    structured_outputs    JSONB,
    final_decision        JSONB
);

CREATE INDEX idx_room_sessions_room ON room_sessions(room_id);
CREATE INDEX idx_room_sessions_status ON room_sessions(status);
CREATE INDEX idx_room_sessions_outputs ON room_sessions USING GIN(structured_outputs) WHERE structured_outputs IS NOT NULL;
```

The old `run_id` FK to `pipeline_runs` is gone — a room session's parent execution is reached through the `agent_executions.room_session_id` back-reference instead.

### room_step_configs / room_step_members
Design-time blueprint for a `room`-mode workflow step, materialized into a real `rooms`/`room_members` pair at execution.

```sql
CREATE TABLE room_step_configs (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    step_id               UUID NOT NULL UNIQUE REFERENCES workflow_steps(id) ON DELETE CASCADE,
    meeting_purpose       TEXT NOT NULL DEFAULT '',
    max_turns             INTEGER NOT NULL DEFAULT 20,
    interaction_mode      TEXT NOT NULL DEFAULT 'moderated',
    gatekeeper_enabled    BOOLEAN NOT NULL DEFAULT true,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE room_step_members (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    step_id          UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    name             TEXT NOT NULL,
    role             TEXT NOT NULL,
    perspective      TEXT NOT NULL DEFAULT '',
    display_order    INTEGER NOT NULL DEFAULT 0,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_room_step_members_step ON room_step_members(step_id);
```

### room_execution_outputs
One structured output per speaker turn in a room session, deduped by `(session, turn, output_name)`.

```sql
CREATE TABLE room_execution_outputs (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_session_id       UUID NOT NULL REFERENCES room_sessions(id) ON DELETE CASCADE,
    agent_execution_id    UUID NOT NULL REFERENCES agent_executions(id) ON DELETE CASCADE,
    agent_id              UUID NOT NULL REFERENCES agents(id),
    speaker_order         INTEGER NOT NULL,
    turn_number           INTEGER NOT NULL,
    output_name           TEXT NOT NULL,
    structured_output     JSONB NOT NULL,
    raw_output            TEXT NOT NULL,
    schema_id             UUID REFERENCES output_schemas(id),
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (room_session_id, turn_number, output_name)
);

CREATE INDEX idx_room_outputs_session ON room_execution_outputs(room_session_id, turn_number);
CREATE INDEX idx_room_outputs_agent ON room_execution_outputs(agent_id);
CREATE INDEX idx_room_outputs_schema ON room_execution_outputs(schema_id) WHERE schema_id IS NOT NULL;
```

---

## Collections

The top-level grouping concept that replaced `pipelines`. A collection is a named group of workflows executed together, sequentially or in parallel.

### workflow_collections
```sql
CREATE TABLE workflow_collections (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id           UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name              TEXT NOT NULL,
    description       TEXT,
    execution_mode    TEXT NOT NULL DEFAULT 'parallel', -- 'sequential' | 'parallel'
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_workflow_collections_user_id ON workflow_collections(user_id);
```

### collection_runs
One run of a collection; `workflow_executions` hang off this via `collection_run_id`.

```sql
CREATE TABLE collection_runs (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    collection_id    UUID NOT NULL REFERENCES workflow_collections(id) ON DELETE CASCADE,
    user_id          UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status           TEXT NOT NULL,
    started_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at     TIMESTAMPTZ,
    error            TEXT
);

CREATE INDEX idx_collection_runs_collection_id ON collection_runs(collection_id);
CREATE INDEX idx_collection_runs_user_id ON collection_runs(user_id);
CREATE INDEX idx_collection_runs_status ON collection_runs(status);
```

### collection_workflows
Membership + display order of workflows within a collection; `execution_mode` optionally overrides the collection's default per workflow.

```sql
CREATE TABLE collection_workflows (
    collection_id     UUID NOT NULL REFERENCES workflow_collections(id) ON DELETE CASCADE,
    workflow_id       UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    display_order     INTEGER NOT NULL DEFAULT 0,
    execution_mode    TEXT,
    PRIMARY KEY (collection_id, workflow_id)
);

CREATE INDEX idx_collection_workflows_collection_id ON collection_workflows(collection_id);
CREATE INDEX idx_collection_workflows_workflow_id ON collection_workflows(workflow_id);
```

### collection_workflow_edges
Ordering dependencies between workflows within a collection (for sequential mode).

```sql
CREATE TABLE collection_workflow_edges (
    from_workflow_id    UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    to_workflow_id      UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    collection_id       UUID NOT NULL REFERENCES workflow_collections(id) ON DELETE CASCADE,
    PRIMARY KEY (from_workflow_id, to_workflow_id, collection_id)
);

CREATE INDEX idx_collection_workflow_edges_collection_id ON collection_workflow_edges(collection_id);
CREATE INDEX idx_collection_workflow_edges_from_workflow_id ON collection_workflow_edges(from_workflow_id);
CREATE INDEX idx_collection_workflow_edges_to_workflow_id ON collection_workflow_edges(to_workflow_id);
```

---

## Canvas Persistence & Snapshots

Backs the hand-rolled `<canvas>` renderer's "submit a diff, resolve to DAG structure" flow (Phase 0), plus workflow version checkpoints and content-addressed snapshotting for reproducible runs.

### canvas_snapshots
One row per workflow (upserted on every board submit) holding the raw Excalidraw-style element JSON and the last structural-diff response.

```sql
CREATE TABLE canvas_snapshots (
    workflow_id           UUID PRIMARY KEY REFERENCES workflows(id) ON DELETE CASCADE,
    snapshot_json         TEXT NOT NULL,
    elements_json         TEXT NOT NULL,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_response_json    TEXT
);
```

### canvas_element_maps
Bridges an Excalidraw element id to the `workflow_steps`/`workflow_step_edges` row it represents (exactly one target per row). Used by the Phase 0 structural executor to resolve element references across submits.

```sql
CREATE TABLE canvas_element_maps (
    workflow_id    UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    element_id     TEXT NOT NULL,
    step_id        UUID REFERENCES workflow_steps(id) ON DELETE CASCADE,
    edge_id        UUID REFERENCES workflow_step_edges(id) ON DELETE CASCADE,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (workflow_id, element_id),
    CONSTRAINT exactly_one_target CHECK (
        (step_id IS NOT NULL AND edge_id IS NULL) OR (step_id IS NULL AND edge_id IS NOT NULL)
    )
);

CREATE INDEX idx_canvas_element_maps_step ON canvas_element_maps(step_id) WHERE step_id IS NOT NULL;
CREATE INDEX idx_canvas_element_maps_edge ON canvas_element_maps(edge_id) WHERE edge_id IS NOT NULL;
```

### step_images
Cached, pre-rendered stroke PNG (base64) per step, computed at board-submit time so execution reads a pixel image instead of re-rasterizing lossy stored coordinates.

```sql
CREATE TABLE step_images (
    step_id                UUID PRIMARY KEY REFERENCES workflow_steps(id) ON DELETE CASCADE,
    stroke_image_base64    TEXT NOT NULL DEFAULT '',
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### step_plan
Persistent per-step scratchpad maintained by the node assistant (one row per step). Named `assistant_notes` until migration `0052` renamed the table (the primary key and index still carry the old name).

```sql
CREATE TABLE step_plan (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(), -- constraint name: assistant_notes_pkey
    step_id       UUID NOT NULL UNIQUE REFERENCES workflow_steps(id) ON DELETE CASCADE,
    content       TEXT NOT NULL DEFAULT '',
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### step_question_state
Compressed status + one pending clarifying question per step, generationally overwritten (not appended) each time the node assistant re-evaluates.

```sql
CREATE TABLE step_question_state (
    step_id          UUID PRIMARY KEY REFERENCES workflow_steps(id) ON DELETE CASCADE,
    status_text      TEXT NOT NULL,
    question_text    TEXT,
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### workflow_versions
Named, full-snapshot checkpoints of a workflow (auto-saved before destructive operations like Generate/Revert, or manually labeled by the user).

```sql
CREATE TABLE workflow_versions (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_id       UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    version_number    INTEGER NOT NULL,
    label             TEXT,
    source            TEXT NOT NULL,
    snapshot          JSONB NOT NULL, -- serialized WorkflowSnapshot (same shape as run_templates.snapshot)
    created_by        UUID NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (workflow_id, version_number)
);

CREATE INDEX idx_wv_workflow ON workflow_versions(workflow_id, version_number DESC);
```

### run_templates
A frozen workflow snapshot used to reproduce a specific run configuration; `workflow_executions.template_id` records which template (if any) a run used.

```sql
CREATE TABLE run_templates (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_id    UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    user_id        UUID NOT NULL,
    name           TEXT NOT NULL,
    description    TEXT,
    snapshot       JSONB NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_run_templates_workflow ON run_templates(workflow_id, created_at DESC);
```

### content_versions / run_snapshots
Immutable, SHA-256-deduplicated content snapshots (`content_versions`) plus a join table (`run_snapshots`) recording which version a given run used for a given `(step, content_type, role)`.

```sql
CREATE TABLE content_versions (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id         UUID NOT NULL,
    content_type      TEXT NOT NULL,
    content_hash      TEXT NOT NULL,
    content           TEXT NOT NULL,
    version_number    INTEGER NOT NULL DEFAULT 1,
    byte_size         INTEGER NOT NULL DEFAULT 0,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (source_id, content_type, content_hash)
);

CREATE INDEX idx_cv_source ON content_versions(source_id, content_type, version_number DESC);

CREATE TABLE run_snapshots (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id                UUID NOT NULL,
    step_id               UUID NOT NULL,
    content_type          TEXT NOT NULL,
    role                  TEXT NOT NULL,
    content_version_id    UUID NOT NULL REFERENCES content_versions(id),
    source_id             UUID NOT NULL,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (run_id, step_id, content_type, role)
);

CREATE INDEX idx_rs_run ON run_snapshots(run_id);
CREATE INDEX idx_rs_version ON run_snapshots(content_version_id);
```

---

## Tool & Capability Routing

Tools are system-wide (no owner). Agents can hold tools directly (`agent_tools`) or delegate tool selection to an LLM-based router with per-mode tool subsets and prerequisite capabilities.

### tools
```sql
CREATE TABLE tools (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            TEXT NOT NULL UNIQUE,
    display_name    TEXT NOT NULL,
    description     TEXT NOT NULL,
    parameters      JSONB NOT NULL DEFAULT '{}'::JSONB,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version         INTEGER NOT NULL DEFAULT 1
);
```

`user_id` and its `UNIQUE(user_id, name)` constraint are gone (migration `0002`) — tools require backend code to execute and are shared across all users; `name` alone is unique.

### tool_routers
An LLM-based router that owns a subset of tools, with an optional parent for hierarchical routing (`level` 1–3).

```sql
CREATE TABLE tool_routers (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id             UUID NOT NULL REFERENCES users(id),
    name                TEXT NOT NULL,
    description         TEXT,
    system_prompt       TEXT NOT NULL,
    model_id            TEXT NOT NULL,
    is_active           BOOLEAN NOT NULL DEFAULT true,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    parent_router_id    UUID REFERENCES tool_routers(id) ON DELETE CASCADE,
    level               INTEGER NOT NULL DEFAULT 1 CHECK (level IN (1, 2, 3))
);

CREATE INDEX idx_tool_routers_user ON tool_routers(user_id);
CREATE INDEX idx_tool_routers_parent ON tool_routers(parent_router_id);
CREATE INDEX idx_tool_routers_level ON tool_routers(level);
```

### tool_router_modes
A router can classify into named modes, each with its own system-prompt suffix, temperature, and token budget.

```sql
CREATE TABLE tool_router_modes (
    id                               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    router_id                        UUID NOT NULL REFERENCES tool_routers(id) ON DELETE CASCADE,
    mode_key                         TEXT NOT NULL CHECK (mode_key ~ '^[a-z][a-z0-9_]*$'),
    display_name                     TEXT NOT NULL,
    description                      TEXT NOT NULL,
    system_prompt                    TEXT NOT NULL,
    temperature                      REAL NOT NULL DEFAULT 0.7 CHECK (temperature >= 0.0 AND temperature <= 2.0),
    max_tokens                       INTEGER NOT NULL DEFAULT 4096 CHECK (max_tokens > 0),
    append_to_agent_system_prompt    BOOLEAN NOT NULL DEFAULT false,
    append_to_agent_tools            BOOLEAN NOT NULL DEFAULT true,
    display_order                    INTEGER NOT NULL DEFAULT 0,
    created_at                       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at                       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (router_id, mode_key)
);

CREATE INDEX idx_tool_router_modes_router ON tool_router_modes(router_id);
CREATE INDEX idx_tool_router_modes_order ON tool_router_modes(router_id, display_order);
```

`agent_executions.selected_router_mode_id` records which mode was chosen for a given execution.

### tool_router_tools / tool_router_mode_tools
Join tables: tools belonging to a router as a whole, vs. tools scoped to one specific mode.

```sql
CREATE TABLE tool_router_tools (
    router_id    UUID NOT NULL REFERENCES tool_routers(id) ON DELETE CASCADE,
    tool_id      UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
    PRIMARY KEY (router_id, tool_id)
);

CREATE TABLE tool_router_mode_tools (
    mode_id    UUID NOT NULL REFERENCES tool_router_modes(id) ON DELETE CASCADE,
    tool_id    UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
    PRIMARY KEY (mode_id, tool_id)
);
```

### tool_capabilities / tool_capability_assignments / mode_required_capabilities
Capability-gating layer: a tool declares one or more capabilities (e.g. `filesystem_write`), and a router mode can require a capability be present before it is offered.

```sql
CREATE TABLE tool_capabilities (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    capability_key    TEXT NOT NULL UNIQUE CHECK (capability_key ~ '^[a-z][a-z0-9_]*$'),
    display_name      TEXT NOT NULL,
    category          TEXT NOT NULL,
    safety_level      TEXT NOT NULL DEFAULT 'safe',
    description       TEXT NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tool_capabilities_category ON tool_capabilities(category);
CREATE INDEX idx_tool_capabilities_safety ON tool_capabilities(safety_level);

CREATE TABLE tool_capability_assignments (
    tool_id          UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
    capability_id    UUID NOT NULL REFERENCES tool_capabilities(id) ON DELETE CASCADE,
    PRIMARY KEY (tool_id, capability_id)
);

CREATE TABLE mode_required_capabilities (
    mode_id          UUID NOT NULL REFERENCES tool_router_modes(id) ON DELETE CASCADE,
    capability_id    UUID NOT NULL REFERENCES tool_capabilities(id) ON DELETE CASCADE,
    is_required      BOOLEAN NOT NULL DEFAULT true,
    PRIMARY KEY (mode_id, capability_id)
);

CREATE INDEX idx_mode_required_capabilities_mode ON mode_required_capabilities(mode_id);
```

### router_requests
A single routed-tool request within a chat session — the intent, the tool/args the router chose, and (for async/chained requests) the result.

```sql
CREATE TABLE router_requests (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id            UUID NOT NULL REFERENCES chat_sessions(id) ON DELETE CASCADE,
    agent_execution_id    UUID REFERENCES agent_executions(id),
    intent                TEXT NOT NULL,
    priority              TEXT NOT NULL DEFAULT 'normal',
    callback_hint         TEXT,
    routed_tool           TEXT,
    routed_args           JSONB,
    is_async              BOOLEAN NOT NULL DEFAULT false,
    passdown              TEXT,
    chain                 JSONB,
    status                TEXT NOT NULL DEFAULT 'pending',
    result                TEXT,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at          TIMESTAMPTZ
);

CREATE INDEX idx_router_requests_session ON router_requests(session_id, status);
```

---

## Chat & Sessions

### chat_sessions
```sql
CREATE TABLE chat_sessions (
    id              UUID PRIMARY KEY,
    user_id         UUID NOT NULL REFERENCES users(id),
    mode_id         TEXT NOT NULL,
    title           TEXT NOT NULL DEFAULT '',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    summary         TEXT NOT NULL DEFAULT '',
    agent_id        UUID REFERENCES agents(id),
    draft_config    JSONB -- e.g. {"step_id": "..."} for a step-scoped design chat
);

CREATE INDEX idx_chat_sessions_user ON chat_sessions(user_id, updated_at DESC);
CREATE INDEX idx_chat_sessions_has_draft_config ON chat_sessions((draft_config IS NOT NULL));
CREATE INDEX idx_chat_sessions_step_id ON chat_sessions((draft_config ->> 'step_id'))
    WHERE (draft_config ->> 'step_id') IS NOT NULL;
```

### chat_messages
```sql
CREATE TABLE chat_messages (
    id             UUID PRIMARY KEY,
    role           TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
    content        TEXT NOT NULL,
    timestamp      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    user_id        UUID NOT NULL REFERENCES users(id),
    session_id     UUID,
    source_type    TEXT CHECK (source_type IS NULL OR source_type IN ('human', 'agent', 'system')),
    hidden_at      TIMESTAMPTZ, -- soft-delete marker for conversation rebase
    error          TEXT       -- durable record of a failed chat turn, attached to the failed user message
);

CREATE INDEX idx_chat_messages_user_id ON chat_messages(user_id);
CREATE INDEX idx_chat_messages_timestamp ON chat_messages(timestamp);
CREATE INDEX idx_chat_messages_session ON chat_messages(session_id, timestamp);
CREATE INDEX idx_cm_hidden ON chat_messages(session_id, hidden_at) WHERE hidden_at IS NOT NULL;
```

The old inter-agent `messages` table (`from_agent`/`to_agent`/`task_id`) no longer exists; agent-to-agent messages are now injected into `chat_messages` with `source_type = 'agent'`.

### context_store
Per-session context assembled before an LLM call, with a priority ordering and optional expiry.

```sql
CREATE TABLE context_store (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id    UUID NOT NULL REFERENCES chat_sessions(id) ON DELETE CASCADE,
    source        TEXT NOT NULL,
    priority      REAL NOT NULL DEFAULT 0.5,
    content       TEXT NOT NULL,
    metadata      JSONB,
    status        TEXT NOT NULL DEFAULT 'active',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at    TIMESTAMPTZ
);

CREATE INDEX idx_context_store_session ON context_store(session_id, status);
CREATE INDEX idx_context_store_priority ON context_store(session_id, priority DESC);
```

---

## Documents & Context

### documents
Knowledge documents with tagging and full-text search; optionally generated by/for a workflow.

```sql
CREATE TABLE documents (
    id                         UUID PRIMARY KEY,
    user_id                    UUID NOT NULL REFERENCES users(id),
    session_id                 UUID REFERENCES chat_sessions(id),
    title                      TEXT NOT NULL,
    content                    TEXT NOT NULL DEFAULT '',
    summary                    TEXT DEFAULT '',
    doc_type                   TEXT DEFAULT 'architecture',
    ref_tag                    TEXT DEFAULT '',
    tags                       TEXT[] DEFAULT '{}',
    created_at                 TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at                 TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    workflow_id                UUID REFERENCES workflows(id),
    target_length              INTEGER,
    is_static                  BOOLEAN DEFAULT false,
    source_protocol_step_id    UUID REFERENCES workflow_steps(id)
);

CREATE INDEX idx_documents_user ON documents(user_id);
CREATE INDEX idx_documents_session ON documents(session_id);
CREATE INDEX idx_documents_workflow_id ON documents(workflow_id);
CREATE INDEX idx_documents_source_protocol_step_id ON documents(source_protocol_step_id);
CREATE INDEX idx_documents_tags ON documents USING GIN(tags);
CREATE INDEX idx_documents_ref_tag ON documents(ref_tag);
CREATE INDEX idx_documents_search ON documents USING GIN(to_tsvector('english', title || ' ' || content));
```

### step_documents
Join table: context documents attached to a workflow step.

```sql
CREATE TABLE step_documents (
    step_id        UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    document_id    UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    PRIMARY KEY (step_id, document_id)
);

CREATE INDEX idx_step_documents_step ON step_documents(step_id);
```

---

## Output Schemas & Prompt Templates

Both tables allow `user_id IS NULL` for system-owned/shared rows (e.g. protocol schemas), with a partial unique index enforcing name-uniqueness within the system-owned set separately from each user's own set.

### output_schemas
```sql
CREATE TABLE output_schemas (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       UUID REFERENCES users(id),
    name          TEXT NOT NULL,
    schema        JSONB NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version       INTEGER NOT NULL DEFAULT 1,
    UNIQUE (user_id, name)
);

CREATE INDEX idx_output_schemas_user ON output_schemas(user_id);
CREATE UNIQUE INDEX idx_output_schemas_system_name ON output_schemas(name) WHERE user_id IS NULL;
```

### prompt_templates
```sql
CREATE TABLE prompt_templates (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       UUID REFERENCES users(id),
    name          TEXT NOT NULL,
    content       TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version       INTEGER NOT NULL DEFAULT 1,
    UNIQUE (user_id, name)
);

CREATE INDEX idx_prompt_templates_user ON prompt_templates(user_id);
CREATE UNIQUE INDEX idx_prompt_templates_system_name ON prompt_templates(name) WHERE user_id IS NULL;
```

---

## System Config & Files

### system_config
Global key/value config store, typed/grouped by `config_type`.

```sql
CREATE TABLE system_config (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    config_type     TEXT NOT NULL,
    config_key      TEXT NOT NULL UNIQUE,
    config_value    JSONB NOT NULL,
    description     TEXT,
    created_by      UUID REFERENCES users(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_system_config_type ON system_config(config_type);
CREATE INDEX idx_system_config_key ON system_config(config_key);
```

### system_files
Metadata sidecar for an S3-backed per-workflow `.system/` filesystem namespace (the actual bytes live in object storage; this table tracks path, size, producer, and sealing).

```sql
CREATE TABLE system_files (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_id          UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    path                 TEXT NOT NULL,
    media_type           TEXT NOT NULL DEFAULT 'application/octet-stream',
    description          TEXT NOT NULL DEFAULT '',
    tags                 TEXT[] NOT NULL DEFAULT '{}',
    produced_by          UUID REFERENCES workflow_steps(id) ON DELETE SET NULL,
    produced_by_agent    TEXT,
    version              INTEGER NOT NULL DEFAULT 1,
    size_bytes           BIGINT NOT NULL DEFAULT 0,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    workflow_run_id      UUID, -- NULL = design-time config, persists across runs
    sealed               BOOLEAN NOT NULL DEFAULT false, -- pinned steps mark artifacts immutable
    UNIQUE (workflow_id, path)
);

CREATE INDEX idx_system_files_workflow ON system_files(workflow_id);
CREATE INDEX idx_system_files_produced_by ON system_files(produced_by);
CREATE INDEX idx_system_files_workflow_run ON system_files(workflow_run_id);
```

---

## Cost & Token Tracking

The old `cost_records` and `llm_calls` tables are gone. `token_ledger` is the single current accounting table; per-message token counts live in `execution_messages` (see [Core Workflow DAG Engine](#core-workflow-dag-engine)).

### token_ledger
```sql
CREATE TABLE token_ledger (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id               UUID NOT NULL REFERENCES users(id),
    agent_execution_id    UUID REFERENCES agent_executions(id),
    model_id              TEXT NOT NULL,
    input_tokens          BIGINT NOT NULL,
    output_tokens         BIGINT NOT NULL,
    cost_usd              REAL NOT NULL,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_token_ledger_user ON token_ledger(user_id);
CREATE INDEX idx_token_ledger_agent_exec ON token_ledger(agent_execution_id);
CREATE INDEX idx_token_ledger_model ON token_ledger(model_id);
CREATE INDEX idx_token_ledger_created ON token_ledger(created_at DESC);
CREATE INDEX idx_token_ledger_user_created ON token_ledger(user_id, created_at DESC);
```

`decisions` (orchestrator decision tracking) also no longer exists.

---

## Version History (Audit Trail)

Six user-editable entities keep an app-level (not trigger-based) shadow `_versions` table: every update inserts a new `(id, version)` row rather than overwriting history. This is a separate mechanism from `workflow_versions` above, which stores full point-in-time snapshots rather than per-field diffs.

### agents_versions
```sql
CREATE TABLE agents_versions (
    id                   UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    version              INTEGER NOT NULL,
    tier                 TEXT, -- legacy column, tier system removed
    name                 TEXT NOT NULL,
    system_prompt        TEXT NOT NULL,
    persona_style        TEXT,
    model_provider       TEXT NOT NULL,
    model_id             TEXT NOT NULL,
    model_max_tokens     INTEGER NOT NULL,
    model_temperature    REAL NOT NULL,
    status               TEXT,
    router_mode          BOOLEAN,
    changed_by           UUID,
    changed_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, version)
);
```

### tools_versions
```sql
CREATE TABLE tools_versions (
    id              UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
    version         INTEGER NOT NULL,
    name            TEXT NOT NULL,
    display_name    TEXT NOT NULL,
    description     TEXT NOT NULL,
    parameters      JSONB NOT NULL,
    changed_by      UUID,
    changed_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, version)
);
```

### workflows_versions
```sql
CREATE TABLE workflows_versions (
    id             UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    version        INTEGER NOT NULL,
    name           TEXT NOT NULL,
    description    TEXT NOT NULL,
    changed_by     UUID,
    changed_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, version)
);
```

### workflow_steps_versions
```sql
CREATE TABLE workflow_steps_versions (
    id                      UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    version                 INTEGER NOT NULL,
    workflow_id             UUID NOT NULL,
    agent_id                UUID NOT NULL,
    execution_mode          TEXT NOT NULL,
    for_each_ref            TEXT,
    prompt_template_id      UUID,
    prompt_template         TEXT NOT NULL,
    output_schema_id        UUID,
    output_variable_name    TEXT,
    interactive_agent_id    UUID,
    for_each_label_field    TEXT,
    display_order           INTEGER NOT NULL,
    changed_by              UUID,
    changed_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, version)
);
```

Note this shadow table has not tracked the ~20 columns added to `workflow_steps` since (canvas position, `child_workflow_id`, `ref_id`, `pinned`, etc.) — it still reflects the original column set.

### output_schemas_versions
```sql
CREATE TABLE output_schemas_versions (
    id            UUID NOT NULL REFERENCES output_schemas(id) ON DELETE CASCADE,
    version       INTEGER NOT NULL,
    name          TEXT NOT NULL,
    schema        JSONB NOT NULL,
    changed_by    UUID,
    changed_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, version)
);
```

### prompt_templates_versions
```sql
CREATE TABLE prompt_templates_versions (
    id            UUID NOT NULL REFERENCES prompt_templates(id) ON DELETE CASCADE,
    version       INTEGER NOT NULL,
    name          TEXT NOT NULL,
    content       TEXT NOT NULL,
    changed_by    UUID,
    changed_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, version)
);
```

`agent_modes_versions` was dropped along with `agent_modes` in migration `0038`.

---

## Key Design Patterns

### Multi-Tenancy
Most user-facing tables carry `user_id` with an index for per-user filtering. A growing share of tables are deliberately **not** user-scoped: `tools`, `protocols`, `tool_capabilities`, and system agents (`agents.is_system`) are shared, system-owned resources by design, not owner-less by omission.

### One shared DAG engine, not per-archetype tables
`workflows`/`workflow_steps`/`workflow_step_edges`/`workflow_executions`/`agent_executions` are the entire execution engine. Archetypes (`single`, `workforce`, `room`, `context`, `input`, `container`) are just a `workflow_steps.execution_mode` string plus archetype-specific side tables (workforce → `task_mission_briefs`/`task_agent_roster`/`agent_designer_*`; room → `room_step_configs`/`room_step_members`/`rooms`). Adding an archetype does not mean adding a parallel execution hierarchy.

### Nested execution via root/depth, not a fixed ladder
`workflow_executions.root_execution_id`/`depth` (migration `0039`) let a workforce step's child workflow run be queried as part of the same tree as its parent in O(1), without a hardcoded `pipeline → stage` depth limit. `collection_run_id` is optional — a workflow can run standalone.

### Design-time vs. runtime split
Several features keep a design-time config table and a materialized runtime table: `room_step_configs`/`room_step_members` → `rooms`/`room_members`; `task_mission_briefs`/`task_agent_roster` → the generated `child_workflow_id` sub-workflow; `belief_extraction_plans` → `beliefs`.

### Content-addressed snapshotting
`content_versions` deduplicates identical content by `(source_id, content_type, content_hash)`; `run_snapshots` and `workflow_versions`/`run_templates` point at specific versions/snapshots rather than embedding full content repeatedly.

### Token & Cost Tracking
Token usage flows through two layers now (the old three-layer `cost_records`/`llm_calls` split is gone):
1. `execution_messages` — per-message tokens within one agent execution.
2. `token_ledger` — centralized per-user accounting, linked back to `agent_executions`.

### Audit Trail
Six `_versions` shadow tables record field-level history for user-editable entities (`changed_by`/`changed_at`). `workflow_versions` is a separate, newer full-snapshot checkpoint system layered on top for the canvas editor's revert/rebase flows.

---

## Migration History

67 migrations currently apply on top of the `0001` consolidated baseline (itself a squash of 71 prior migrations — see the note at the top of this document). Full detail lives in `migrations/*.sql`; the table below summarizes the eras rather than reproducing every file:

| Range | Theme |
|---|---|
| `0001` | Consolidated baseline (squash of a prior 71-migration history) |
| `0002`–`0009` | Early hardening: system-wide tools, agent guidances, container/VPN config, verification agents, admin role |
| `0010`–`0022` | Protocol layer introduced (`protocols`/`protocol_ports`/`workflow_step_protocols`), then narrowed toward the documenter feature; canvas node refactor; documenter foundation/assistant agent |
| `0023`–`0029` | Dead-code cleanup (`cavernous` routing columns); task-force mission briefs/roster; Agent Designer tables, generalized across archetypes |
| `0030`–`0038` | Board context caching; content-version snapshotting; run templates; sub-workflow execution; execution-hierarchy prep; drop of `agent_modes`/`*_backup` tables |
| `0039`–`0044` | `root_execution_id`/`depth` tree traversal; workforce archetype unifies documenter + task_force; drop of `tasks` and `pr_merge_queue` |
| `0045`–`0054` | Nullable `agent_id` on executions (workforce roster turns); dispatch trace persistence; `execution_type` discriminator on `agent_executions` |
| `0055`–`0060` | Canvas snapshot + element-map persistence for the Phase 0 structural executor; drop of sub-workflow-only columns |
| `0061`–`0065` | S3-backed system file store (`system_files`); designer→workforce handoff text; `workflow_versions` snapshot checkpoints |
| `0066`–`0067` | Chat message soft-delete (conversation rebase) and durable per-message error records |

For exact column-level history of any table, `grep -l '<table_name>' migrations/*.sql` and read forward from the first hit.

---

## Database Connection

**Container:** `nexor-postgres-1`
**Image:** `postgres:16-alpine`
**Database:** `nexor`
**User:** `nexor`

```bash
# Quick query
docker exec nexor-postgres-1 psql -U nexor -d nexor -c "SELECT 1;"

# Interactive shell
docker exec -it nexor-postgres-1 psql -U nexor -d nexor

# List tables / describe one table
docker exec nexor-postgres-1 psql -U nexor -d nexor -c "\dt"
docker exec nexor-postgres-1 psql -U nexor -d nexor -c "\d workflow_steps"
```

`sqlx` tracks applied migrations in `public._sqlx_migrations` (not an application table, not documented above).

---

**Generated:** 2026-08-25 by Claude Code (verified against `migrations/0001`–`0067` and a live database at that version)
