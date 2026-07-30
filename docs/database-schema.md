# Database Schema Documentation

**Database:** nexor (PostgreSQL 16)

**Last Updated:** 2026-02-02

This document describes the complete database schema for the nexor platform, which orchestrates AI agents for software engineering tasks on GitHub repos.

---

## Table of Contents

1. [Authentication & Users](#authentication--users)
2. [Agents & Configuration](#agents--configuration)
3. [Tasks & Execution](#tasks--execution)
4. [Pipelines & Workflows](#pipelines--workflows)
5. [Rooms & Collaboration](#rooms--collaboration)
6. [Tools & Routing](#tools--routing)
7. [Chat & Sessions](#chat--sessions)
8. [Documents & Context](#documents--context)
9. [Observability & Costs](#observability--costs)
10. [Version History](#version-history)
11. [Miscellaneous](#miscellaneous)

---

## Authentication & Users

### users
User accounts with email and optional GitHub OAuth integration.

```sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT,
    github_id BIGINT UNIQUE,
    github_login TEXT,
    github_token_encrypted TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_github_id ON users(github_id) WHERE github_id IS NOT NULL;
```

### auth_config
Global authentication configuration (legacy, single-row table).

```sql
CREATE TABLE auth_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### sessions
Active user session tokens.

```sql
CREATE TABLE sessions (
    id UUID PRIMARY KEY,
    token_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    last_active TIMESTAMPTZ NOT NULL
);
```

---

## Agents & Configuration

### agents
Core agent definitions with model configuration and system prompts.

**Note:** The tier system was removed in migration 057. Agents are now configured individually.

```sql
CREATE TABLE agents (
    id UUID PRIMARY KEY NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    system_prompt TEXT NOT NULL DEFAULT '',
    persona_style TEXT DEFAULT 'casual',
    model_provider TEXT NOT NULL DEFAULT 'anthropic',
    model_id TEXT NOT NULL,
    model_max_tokens INTEGER NOT NULL DEFAULT 4096,
    model_temperature REAL NOT NULL DEFAULT 0.7,
    current_task UUID REFERENCES tasks(id),
    status TEXT DEFAULT 'idle',
    router_mode BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version INT NOT NULL DEFAULT 1
);

CREATE INDEX idx_agents_user_id ON agents(user_id);
CREATE INDEX idx_agents_status ON agents(status);
```

### agent_modes
Dynamic mode system: per-agent modes with LLM-based classification.

```sql
CREATE TABLE agent_modes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    system_prompt_suffix TEXT,
    temperature_override DOUBLE PRECISION,
    model_override TEXT,
    tool_overrides TEXT[],
    classifier_hint TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version INT NOT NULL DEFAULT 1,
    UNIQUE (agent_id, name)
);
```

---

## Tasks & Execution

### tasks
Task definitions assigned to agents.

```sql
CREATE TABLE tasks (
    id UUID PRIMARY KEY NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id),
    slice_id UUID,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    assigned_agent UUID,
    status TEXT NOT NULL DEFAULT 'pending',
    priority TEXT NOT NULL DEFAULT 'normal',
    context_files JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tasks_user_id ON tasks(user_id);
CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_slice_id ON tasks(slice_id);
CREATE INDEX idx_tasks_assigned_agent ON tasks(assigned_agent);
```

### task_events
Append-only event log for task lifecycle tracking.

```sql
CREATE TABLE task_events (
    id UUID PRIMARY KEY NOT NULL,
    task_id UUID NOT NULL REFERENCES tasks(id),
    event_type TEXT NOT NULL,
    agent_id UUID,
    details TEXT NOT NULL DEFAULT '',
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_task_events_task_id ON task_events(task_id);
CREATE INDEX idx_task_events_timestamp ON task_events(timestamp);
```

### agent_executions
Individual agent execution instances with token tracking.

```sql
CREATE TABLE agent_executions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    stage_execution_id UUID NOT NULL REFERENCES stage_executions(id) ON DELETE CASCADE,
    agent_id UUID NOT NULL REFERENCES agents(id),
    workflow_step_id UUID REFERENCES workflow_steps(id),
    is_interactive BOOLEAN NOT NULL DEFAULT FALSE,
    parent_agent_execution_id UUID REFERENCES agent_executions(id),
    system_prompt_rendered TEXT NOT NULL,
    input TEXT NOT NULL,
    output TEXT,
    structured_output JSONB,
    status TEXT NOT NULL DEFAULT 'running',
    input_tokens BIGINT NOT NULL DEFAULT 0,
    output_tokens BIGINT NOT NULL DEFAULT 0,
    cost_usd REAL NOT NULL DEFAULT 0.0,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    room_session_id UUID REFERENCES room_sessions(id),
    speaker_order INTEGER,
    selected_mode_id UUID REFERENCES agent_modes(id)
);

CREATE INDEX idx_agent_executions_stage ON agent_executions(stage_execution_id);
CREATE INDEX idx_agent_executions_agent ON agent_executions(agent_id);
CREATE INDEX idx_agent_executions_step ON agent_executions(workflow_step_id);
CREATE INDEX idx_agent_executions_status ON agent_executions(status);
CREATE INDEX idx_agent_executions_started ON agent_executions(started_at DESC);
CREATE INDEX idx_agent_executions_parent ON agent_executions(parent_agent_execution_id);
CREATE INDEX idx_agent_executions_room ON agent_executions(room_session_id)
    WHERE room_session_id IS NOT NULL;
```

### execution_messages
Message history for agent executions (conversation turns).

```sql
CREATE TABLE execution_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_execution_id UUID NOT NULL REFERENCES agent_executions(id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    tool_call_id TEXT,
    input_tokens BIGINT NOT NULL DEFAULT 0,
    output_tokens BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_execution_messages_execution ON execution_messages(agent_execution_id);
CREATE INDEX idx_execution_messages_role ON execution_messages(agent_execution_id, role);
CREATE INDEX idx_execution_messages_created ON execution_messages(created_at);
```

---

## Pipelines & Workflows

### pipelines
High-level pipeline definitions owned by users.

```sql
CREATE TABLE pipelines (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_pipelines_user_id ON pipelines(user_id);
```

### pipeline_stages
Sequential stages within a pipeline.

```sql
CREATE TABLE pipeline_stages (
    pipeline_id UUID NOT NULL REFERENCES pipelines(id) ON DELETE CASCADE,
    stage_number INTEGER NOT NULL,
    agent_id UUID NOT NULL,
    role TEXT,
    approval_required BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (pipeline_id, stage_number)
);
```

### pipeline_runs
Runtime instances of pipeline executions.

```sql
CREATE TABLE pipeline_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pipeline_id UUID NOT NULL REFERENCES pipelines(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id),
    status TEXT NOT NULL DEFAULT 'running',
    initial_task TEXT NOT NULL,
    stage_outputs JSONB NOT NULL DEFAULT '{}',
    current_stage INTEGER NOT NULL DEFAULT 0,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    total_input_tokens BIGINT NOT NULL DEFAULT 0,
    total_output_tokens BIGINT NOT NULL DEFAULT 0
);

CREATE INDEX idx_pipeline_runs_pipeline ON pipeline_runs(pipeline_id);
CREATE INDEX idx_pipeline_runs_user ON pipeline_runs(user_id);
CREATE INDEX idx_pipeline_runs_status ON pipeline_runs(status);
```

### stage_executions
Individual stage execution records within pipeline runs.

```sql
CREATE TABLE stage_executions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id UUID NOT NULL REFERENCES pipeline_runs(id) ON DELETE CASCADE,
    stage_number INTEGER NOT NULL,
    stage_name TEXT NOT NULL,
    agent_id UUID,
    status TEXT NOT NULL DEFAULT 'running',
    rendered_prompt TEXT,
    output TEXT,
    structured_output JSONB,
    user_input TEXT,
    input_tokens BIGINT NOT NULL DEFAULT 0,
    output_tokens BIGINT NOT NULL DEFAULT 0,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    duration_ms BIGINT NOT NULL DEFAULT 0
);

CREATE INDEX idx_stage_executions_run ON stage_executions(run_id);
CREATE UNIQUE INDEX idx_stage_executions_run_stage ON stage_executions(run_id, stage_number);
```

### workflows
Reusable execution DAGs composed of agent steps.

```sql
CREATE TABLE workflows (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version INT NOT NULL DEFAULT 1
);

CREATE INDEX idx_workflows_user ON workflows(user_id);
```

### workflow_steps
Individual steps (nodes) within workflow DAGs.

```sql
CREATE TABLE workflow_steps (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_id UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    agent_id UUID NOT NULL REFERENCES agents(id),
    execution_mode TEXT NOT NULL DEFAULT 'single',
    for_each_ref TEXT,
    prompt_template_id UUID REFERENCES prompt_templates(id),
    prompt_template TEXT NOT NULL DEFAULT '',
    output_schema_id UUID REFERENCES output_schemas(id),
    output_variable_name TEXT,
    interactive_agent_id UUID REFERENCES agents(id),
    display_order INTEGER NOT NULL DEFAULT 0,
    room_id UUID REFERENCES rooms(id),
    version INT NOT NULL DEFAULT 1
);

CREATE INDEX idx_workflow_steps_workflow ON workflow_steps(workflow_id);
CREATE INDEX idx_workflow_steps_agent ON workflow_steps(agent_id);
```

### workflow_step_edges
Edges defining execution order between workflow steps.

```sql
CREATE TABLE workflow_step_edges (
    from_step_id UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    to_step_id UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    PRIMARY KEY (from_step_id, to_step_id)
);

CREATE INDEX idx_workflow_step_edges_from ON workflow_step_edges(from_step_id);
CREATE INDEX idx_workflow_step_edges_to ON workflow_step_edges(to_step_id);
```

### step_documents
Context documents attached to workflow steps.

```sql
CREATE TABLE step_documents (
    step_id UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    PRIMARY KEY (step_id, document_id)
);

CREATE INDEX idx_step_documents_step ON step_documents(step_id);
```

---

## Rooms & Collaboration

### rooms
Multi-agent conversation rooms (pipeline-scoped).

```sql
CREATE TABLE rooms (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    pipeline_id UUID NOT NULL REFERENCES pipelines(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    gatekeeper_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    gatekeeper_model_id TEXT NOT NULL DEFAULT 'claude-haiku-4-20250414',
    max_speakers_per_turn INTEGER NOT NULL DEFAULT 4,
    max_turns INTEGER NOT NULL DEFAULT 20,
    tools_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_rooms_user ON rooms(user_id);
CREATE INDEX idx_rooms_pipeline ON rooms(pipeline_id);
```

### room_members
Agent membership in rooms.

```sql
CREATE TABLE room_members (
    room_id UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    display_name TEXT,
    role_description TEXT NOT NULL,
    display_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (room_id, agent_id)
);

CREATE INDEX idx_room_members_agent ON room_members(agent_id);
```

### room_sessions
Runtime instances of room conversations.

```sql
CREATE TABLE room_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_id UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    run_id UUID REFERENCES pipeline_runs(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'active',
    current_turn INTEGER NOT NULL DEFAULT 0,
    transcript_summary TEXT,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX idx_room_sessions_room ON room_sessions(room_id);
CREATE INDEX idx_room_sessions_run ON room_sessions(run_id) WHERE run_id IS NOT NULL;
CREATE INDEX idx_room_sessions_status ON room_sessions(status);
```

---

## Tools & Routing

### tools
Tool definitions with parameter schemas (metadata for hardcoded implementations).

```sql
CREATE TABLE tools (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    display_name TEXT NOT NULL,
    description TEXT NOT NULL,
    parameters JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version INT NOT NULL DEFAULT 1,
    UNIQUE (user_id, name)
);

CREATE INDEX idx_tools_user ON tools(user_id);
```

### agent_tools
Join table: which tools each agent can use.

```sql
CREATE TABLE agent_tools (
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
    PRIMARY KEY (agent_id, tool_id)
);

CREATE INDEX idx_agent_tools_tool ON agent_tools(tool_id);
```

### tool_routers
LLM-based routers that own subsets of tools.

```sql
CREATE TABLE tool_routers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    description TEXT,
    system_prompt TEXT NOT NULL,
    model_id TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tool_routers_user ON tool_routers(user_id);
```

### tool_router_tools
Join table: tools belonging to each router.

```sql
CREATE TABLE tool_router_tools (
    router_id UUID NOT NULL REFERENCES tool_routers(id) ON DELETE CASCADE,
    tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
    PRIMARY KEY (router_id, tool_id)
);

CREATE INDEX idx_tool_router_tools_tool ON tool_router_tools(tool_id);
```

---

## Chat & Sessions

### chat_sessions
User chat sessions with associated agents/modes.

```sql
CREATE TABLE chat_sessions (
    id UUID PRIMARY KEY NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id),
    mode_id TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    agent_id UUID REFERENCES agents(id)
);

CREATE INDEX idx_chat_sessions_user ON chat_sessions(user_id, updated_at DESC);
```

### chat_messages
Individual messages in chat sessions.

```sql
CREATE TABLE chat_messages (
    id UUID PRIMARY KEY NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id),
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
    content TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    session_id UUID
);

CREATE INDEX idx_chat_messages_user_id ON chat_messages(user_id);
CREATE INDEX idx_chat_messages_timestamp ON chat_messages(timestamp);
CREATE INDEX idx_chat_messages_session ON chat_messages(session_id, timestamp);
```

### messages
Inter-agent communication messages.

```sql
CREATE TABLE messages (
    id UUID PRIMARY KEY NOT NULL,
    from_agent UUID NOT NULL REFERENCES agents(id),
    to_agent UUID NOT NULL REFERENCES agents(id),
    message_type TEXT NOT NULL,
    content TEXT NOT NULL,
    task_id UUID REFERENCES tasks(id),
    context TEXT,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_messages_from ON messages(from_agent);
CREATE INDEX idx_messages_to ON messages(to_agent);
CREATE INDEX idx_messages_task ON messages(task_id);
CREATE INDEX idx_messages_timestamp ON messages(timestamp);
```

---

## Documents & Context

### documents
Knowledge documents with tagging and full-text search.

```sql
CREATE TABLE documents (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    session_id UUID REFERENCES chat_sessions(id),
    title TEXT NOT NULL,
    content TEXT NOT NULL DEFAULT '',
    summary TEXT NOT NULL DEFAULT '',
    doc_type TEXT NOT NULL DEFAULT 'architecture',
    ref_tag TEXT NOT NULL DEFAULT '',
    tags TEXT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_documents_user ON documents(user_id);
CREATE INDEX idx_documents_session ON documents(session_id);
CREATE INDEX idx_documents_tags ON documents USING GIN(tags);
CREATE INDEX idx_documents_ref_tag ON documents(ref_tag);
CREATE INDEX idx_documents_search ON documents USING GIN(to_tsvector('english', title || ' ' || content));
```

### context_store
Per-session context assembled before LLM calls.

```sql
CREATE TABLE context_store (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES chat_sessions(id) ON DELETE CASCADE,
    source TEXT NOT NULL,
    priority REAL NOT NULL DEFAULT 0.5,
    content TEXT NOT NULL,
    metadata JSONB,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ
);

CREATE INDEX idx_context_store_session ON context_store(session_id, status);
CREATE INDEX idx_context_store_priority ON context_store(session_id, priority DESC);
```

### output_schemas
JSON schemas for structured agent outputs.

```sql
CREATE TABLE output_schemas (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    schema JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version INT NOT NULL DEFAULT 1,
    UNIQUE (user_id, name)
);

CREATE INDEX idx_output_schemas_user ON output_schemas(user_id);
```

### prompt_templates
Reusable prompt templates for workflow steps.

```sql
CREATE TABLE prompt_templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version INT NOT NULL DEFAULT 1,
    UNIQUE (user_id, name)
);

CREATE INDEX idx_prompt_templates_user ON prompt_templates(user_id);
```

---

## Observability & Costs

### token_ledger
Centralized token usage and cost tracking.

```sql
CREATE TABLE token_ledger (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    agent_execution_id UUID NOT NULL REFERENCES agent_executions(id),
    model_id TEXT NOT NULL,
    input_tokens BIGINT NOT NULL,
    output_tokens BIGINT NOT NULL,
    cost_usd REAL NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_token_ledger_user ON token_ledger(user_id);
CREATE INDEX idx_token_ledger_agent_exec ON token_ledger(agent_execution_id);
CREATE INDEX idx_token_ledger_model ON token_ledger(model_id);
CREATE INDEX idx_token_ledger_created ON token_ledger(created_at DESC);
CREATE INDEX idx_token_ledger_user_created ON token_ledger(user_id, created_at DESC);
```

### cost_records
Legacy cost tracking (pre-token-ledger).

```sql
CREATE TABLE cost_records (
    id UUID PRIMARY KEY NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id),
    task_id UUID REFERENCES tasks(id),
    agent_id UUID NOT NULL REFERENCES agents(id),
    agent_tier TEXT NOT NULL,
    model_id TEXT NOT NULL,
    input_tokens INTEGER NOT NULL,
    output_tokens INTEGER NOT NULL,
    cost_usd REAL NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_cost_records_user_id ON cost_records(user_id);
CREATE INDEX idx_cost_records_task ON cost_records(task_id);
CREATE INDEX idx_cost_records_agent ON cost_records(agent_id);
CREATE INDEX idx_cost_records_tier ON cost_records(agent_tier);
CREATE INDEX idx_cost_records_timestamp ON cost_records(timestamp);
```

### llm_calls
All LLM API calls for replay and debugging.

```sql
CREATE TABLE llm_calls (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    task_id UUID,
    agent_id UUID,
    model TEXT NOT NULL,
    prompt TEXT NOT NULL,
    response TEXT NOT NULL,
    input_tokens INTEGER NOT NULL,
    output_tokens INTEGER NOT NULL,
    latency_ms INTEGER NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    cost_usd REAL NOT NULL
);

CREATE INDEX idx_llm_calls_user_id ON llm_calls(user_id);
CREATE INDEX idx_llm_calls_task ON llm_calls(task_id);
CREATE INDEX idx_llm_calls_timestamp ON llm_calls(timestamp);
CREATE INDEX idx_llm_calls_model ON llm_calls(model);
```

### decisions
Orchestrator decision tracking with reasoning.

```sql
CREATE TABLE decisions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    task_id UUID NOT NULL,
    decision_type TEXT NOT NULL,
    reasoning TEXT NOT NULL,
    outcome TEXT NOT NULL,
    llm_call_id UUID,
    cost_usd REAL NOT NULL DEFAULT 0.0,
    timestamp TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_decisions_user_id ON decisions(user_id);
CREATE INDEX idx_decisions_task ON decisions(task_id);
CREATE INDEX idx_decisions_type ON decisions(decision_type);
CREATE INDEX idx_decisions_timestamp ON decisions(timestamp);
```

---

## Version History

The following tables store version history for user-editable entities:

### agents_versions
```sql
CREATE TABLE agents_versions (
    id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    version INT NOT NULL,
    name TEXT NOT NULL,
    system_prompt TEXT NOT NULL,
    persona_style TEXT,
    model_provider TEXT NOT NULL,
    model_id TEXT NOT NULL,
    model_max_tokens INT NOT NULL,
    model_temperature REAL NOT NULL,
    status TEXT,
    router_mode BOOLEAN,
    changed_by UUID,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, version)
);
```

### agent_modes_versions
```sql
CREATE TABLE agent_modes_versions (
    id UUID NOT NULL REFERENCES agent_modes(id) ON DELETE CASCADE,
    version INT NOT NULL,
    agent_id UUID NOT NULL,
    name TEXT NOT NULL,
    system_prompt_suffix TEXT,
    temperature_override DOUBLE PRECISION,
    model_override TEXT,
    tool_overrides TEXT[],
    classifier_hint TEXT NOT NULL,
    changed_by UUID,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, version)
);
```

### tools_versions
```sql
CREATE TABLE tools_versions (
    id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
    version INT NOT NULL,
    name TEXT NOT NULL,
    display_name TEXT NOT NULL,
    description TEXT NOT NULL,
    parameters JSONB NOT NULL,
    changed_by UUID,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, version)
);
```

### workflows_versions
```sql
CREATE TABLE workflows_versions (
    id UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    version INT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    changed_by UUID,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, version)
);
```

### workflow_steps_versions
```sql
CREATE TABLE workflow_steps_versions (
    id UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    version INT NOT NULL,
    workflow_id UUID NOT NULL,
    agent_id UUID NOT NULL,
    execution_mode TEXT NOT NULL,
    for_each_ref TEXT,
    prompt_template_id UUID,
    prompt_template TEXT NOT NULL,
    output_schema_id UUID,
    output_variable_name TEXT,
    interactive_agent_id UUID,
    display_order INT NOT NULL,
    changed_by UUID,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, version)
);
```

### output_schemas_versions
```sql
CREATE TABLE output_schemas_versions (
    id UUID NOT NULL REFERENCES output_schemas(id) ON DELETE CASCADE,
    version INT NOT NULL,
    name TEXT NOT NULL,
    schema JSONB NOT NULL,
    changed_by UUID,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, version)
);
```

### prompt_templates_versions
```sql
CREATE TABLE prompt_templates_versions (
    id UUID NOT NULL REFERENCES prompt_templates(id) ON DELETE CASCADE,
    version INT NOT NULL,
    name TEXT NOT NULL,
    content TEXT NOT NULL,
    changed_by UUID,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, version)
);
```

---

## Miscellaneous

### tickets
GitHub issue tracking.

```sql
CREATE TABLE tickets (
    id UUID PRIMARY KEY NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id),
    source_type TEXT NOT NULL DEFAULT 'manual',
    source_owner TEXT,
    source_repo TEXT,
    source_issue_number INTEGER,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    labels JSONB NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'new',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tickets_user_id ON tickets(user_id);
CREATE INDEX idx_tickets_status ON tickets(status);
```

### vertical_slices
Task decomposition for tickets.

```sql
CREATE TABLE vertical_slices (
    id UUID PRIMARY KEY NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id),
    ticket_id UUID NOT NULL REFERENCES tickets(id),
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_vertical_slices_user_id ON vertical_slices(user_id);
CREATE INDEX idx_slices_ticket ON vertical_slices(ticket_id);
CREATE INDEX idx_slices_status ON vertical_slices(status);
```

### prds
Product requirement documents.

```sql
CREATE TABLE prds (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'draft',
    vision TEXT NOT NULL DEFAULT '',
    problem_statement TEXT NOT NULL DEFAULT '',
    target_users TEXT NOT NULL DEFAULT '',
    success_criteria JSONB NOT NULL DEFAULT '[]',
    technical_decisions JSONB NOT NULL DEFAULT '[]',
    data_models JSONB NOT NULL DEFAULT '[]',
    milestones JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_prds_user_id ON prds(user_id);
CREATE INDEX idx_prds_status ON prds(status);
```

### planning_sessions
Resumable PRD planning conversations.

```sql
CREATE TABLE planning_sessions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    prd_id UUID NOT NULL REFERENCES prds(id),
    phase TEXT NOT NULL DEFAULT 'discovery',
    history JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_planning_sessions_user_id ON planning_sessions(user_id);
CREATE INDEX idx_planning_sessions_prd_id ON planning_sessions(prd_id);
```

### clusters
Agent grouping with shared conventions.

```sql
CREATE TABLE clusters (
    id UUID PRIMARY KEY NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    conventions TEXT NOT NULL DEFAULT '',
    shared_files JSONB NOT NULL DEFAULT '[]'::JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_clusters_user_id ON clusters(user_id);
```

### cluster_members
Agent membership in clusters.

```sql
CREATE TABLE cluster_members (
    cluster_id UUID NOT NULL REFERENCES clusters(id) ON DELETE CASCADE,
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (cluster_id, agent_id)
);
```

### schedules
Periodic task scheduling.

```sql
CREATE TABLE schedules (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    agent_id UUID NOT NULL,
    interval_seconds INTEGER NOT NULL,
    task_title TEXT NOT NULL,
    task_description TEXT NOT NULL,
    role TEXT,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    last_run_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_schedules_user_id ON schedules(user_id);
```

### triggers
Event-based task triggers.

```sql
CREATE TABLE triggers (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    event_type TEXT NOT NULL,
    agent_id UUID NOT NULL,
    task_title TEXT NOT NULL,
    task_description TEXT NOT NULL,
    role TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_triggers_user_id ON triggers(user_id);
```

### pr_merge_queue
Ordered PR merge queue for conflict management.

```sql
CREATE TABLE pr_merge_queue (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    repo_owner TEXT NOT NULL,
    repo_name TEXT NOT NULL,
    pr_number INTEGER NOT NULL,
    queue_position INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    conflict_info TEXT,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    UNIQUE(repo_owner, repo_name, pr_number)
);

CREATE INDEX idx_pr_merge_queue_user_id ON pr_merge_queue(user_id);
CREATE INDEX idx_pr_queue_position ON pr_merge_queue(repo_owner, repo_name, queue_position);
CREATE INDEX idx_pr_queue_status ON pr_merge_queue(status);
```

### system_state
Global system state (key-value store).

```sql
CREATE TABLE system_state (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### refactor_sessions
Refactoring session tracking.

```sql
CREATE TABLE refactor_sessions (
    id UUID PRIMARY KEY NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id),
    started_at TIMESTAMPTZ NOT NULL,
    ended_at TIMESTAMPTZ,
    production_halted BOOLEAN NOT NULL DEFAULT FALSE,
    changes_applied INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_refactor_sessions_user_id ON refactor_sessions(user_id);
CREATE INDEX idx_refactor_sessions_active ON refactor_sessions(ended_at) WHERE ended_at IS NULL;
```

### refactor_changes
Proposed changes within refactoring sessions.

```sql
CREATE TABLE refactor_changes (
    id UUID PRIMARY KEY NOT NULL,
    session_id UUID NOT NULL REFERENCES refactor_sessions(id),
    file_path TEXT NOT NULL,
    change_type TEXT NOT NULL,
    before_content TEXT,
    after_content TEXT,
    reason TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'proposed',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_refactor_changes_session ON refactor_changes(session_id);
CREATE INDEX idx_refactor_changes_status ON refactor_changes(status);
```

---

## Key Design Patterns

### Multi-Tenancy
All major tables include `user_id` with corresponding indexes for efficient filtering.

### Soft Deletes
Most tables use CASCADE on foreign keys, relying on PostgreSQL referential integrity.

### Audit Trail
Version tables track all changes to user-editable entities with `changed_by` and `changed_at` fields.

### Token Tracking
Token usage flows through three layers:
1. `execution_messages` (per-message tokens)
2. `agent_executions` (aggregated per execution)
3. `token_ledger` (centralized accounting per user)

### Execution Hierarchy
```
pipeline_runs
  └─ stage_executions
      └─ agent_executions
          ├─ execution_messages
          └─ token_ledger entries
```

### Workflow Composition
Workflows are DAGs where:
- `workflow_steps` = nodes (agents + prompts + schemas)
- `workflow_step_edges` = edges (execution order)
- Steps can reference rooms for multi-agent collaboration

---

## Migration History

Total migrations: **57**

Notable changes:
- **040-044**: Simplified core tables (agents, pipelines, stages, runs, documents)
- **046-049**: Rebuilt tool system with routers and context store
- **051**: Added dynamic agent modes with LLM classification
- **053**: Added comprehensive versioning system
- **055-056**: Added rooms and room sessions for multi-agent collaboration
- **057**: Removed tier system entirely (agents now individually configured)

---

## Database Connection

**Container:** `gh-agents-postgres-1`
**Image:** `postgres:16-alpine`
**Database:** `nexor`
**User:** `nexor`

```bash
# Quick query
docker exec gh-agents-postgres-1 psql -U nexor -d nexor -c "SELECT 1;"

# Interactive shell
docker exec -it gh-agents-postgres-1 psql -U nexor -d nexor
```

---

**Generated:** 2026-02-02 by Claude Code
