# Nexor Database Schema

PostgreSQL 16 · Multi-tenant (user_id on all major tables) · 32 tables

---

## 1. Users & Authentication

```sql
users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT,
    github_id BIGINT UNIQUE,
    github_login TEXT,
    github_token_encrypted TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
-- idx_users_email(email)
-- idx_users_github_id(github_id) WHERE github_id IS NOT NULL

auth_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)

sessions (
    id UUID PRIMARY KEY,
    token_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    last_active TIMESTAMPTZ NOT NULL
)
```

---

## 2. Tasks & Events

```sql
tasks (
    id UUID PRIMARY KEY,
    slice_id UUID,
    user_id UUID NOT NULL REFERENCES users(id),
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    assigned_tier TEXT NOT NULL DEFAULT 'worker',
    assigned_agent UUID,
    status TEXT NOT NULL DEFAULT 'pending',        -- 'pending' | 'in_progress' | 'completed' | 'failed'
    priority TEXT NOT NULL DEFAULT 'normal',
    context_files JSONB NOT NULL DEFAULT '[]',
    metadata JSONB,
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 3,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
-- idx_tasks_status(status)
-- idx_tasks_slice_id(slice_id)
-- idx_tasks_assigned_agent(assigned_agent)
-- idx_tasks_user_id(user_id)

task_events (
    id UUID PRIMARY KEY,
    task_id UUID NOT NULL REFERENCES tasks(id),
    event_type TEXT NOT NULL,
    agent_id UUID,
    details TEXT NOT NULL DEFAULT '',
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
-- idx_task_events_task_id(task_id)
-- idx_task_events_timestamp(timestamp)

task_dependencies (
    task_id UUID NOT NULL REFERENCES tasks(id),
    depends_on_id UUID NOT NULL REFERENCES tasks(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (task_id, depends_on_id)
)
-- idx_task_dependencies_task_id(task_id)
-- idx_task_dependencies_depends_on(depends_on_id)
```

---

## 3. Agents & Communication

```sql
agents (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    tier TEXT NOT NULL,
    persona_name TEXT NOT NULL,
    persona_prompt TEXT NOT NULL DEFAULT '',
    persona_style TEXT NOT NULL DEFAULT 'casual',
    model_provider TEXT NOT NULL DEFAULT 'anthropic',
    model_id TEXT NOT NULL,
    model_max_tokens INTEGER NOT NULL DEFAULT 4096,
    model_temperature REAL NOT NULL DEFAULT 0.7,
    current_task UUID REFERENCES tasks(id),
    status TEXT NOT NULL DEFAULT 'idle',            -- 'idle' | 'busy' | 'offline'
    router_mode BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
-- idx_agents_tier(tier)
-- idx_agents_status(status)
-- idx_agents_user_id(user_id)

messages (
    id UUID PRIMARY KEY,
    from_agent UUID NOT NULL REFERENCES agents(id),
    to_agent UUID NOT NULL REFERENCES agents(id),
    message_type TEXT NOT NULL,
    content TEXT NOT NULL,
    task_id UUID REFERENCES tasks(id),
    context TEXT,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
-- idx_messages_from(from_agent)
-- idx_messages_to(to_agent)
-- idx_messages_task(task_id)
-- idx_messages_timestamp(timestamp)
```

---

## 4. Clusters & Organization

```sql
clusters (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    conventions TEXT NOT NULL DEFAULT '',
    shared_files JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
-- idx_clusters_user_id(user_id)

cluster_members (
    cluster_id UUID NOT NULL REFERENCES clusters(id) ON DELETE CASCADE,
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    role TEXT,
    persona_override TEXT NOT NULL DEFAULT '',
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (cluster_id, agent_id)
)
```

---

## 5. Pipelines & Stages

```sql
pipelines (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
-- idx_pipelines_user_id(user_id)

pipeline_stages (
    pipeline_id UUID NOT NULL REFERENCES pipelines(id) ON DELETE CASCADE,
    stage_number INTEGER NOT NULL,
    agent_id UUID,
    cluster_id UUID REFERENCES clusters(id),
    role TEXT,
    approval_required BOOLEAN NOT NULL DEFAULT FALSE,
    fan_out BOOLEAN NOT NULL DEFAULT FALSE,
    stage_name TEXT NOT NULL DEFAULT '',
    input_definitions JSONB NOT NULL DEFAULT '[]',
    output_description TEXT NOT NULL DEFAULT '',
    output_schema JSONB NOT NULL DEFAULT '{"fields":[]}',
    PRIMARY KEY (pipeline_id, stage_number)
)
-- idx_pipeline_stages_name(pipeline_id, stage_name) WHERE stage_name != ''

stage_side_tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pipeline_id UUID NOT NULL,
    stage_number INTEGER NOT NULL,
    agent_id UUID NOT NULL REFERENCES agents(id),
    input_definitions JSONB NOT NULL DEFAULT '[]',
    output_name TEXT NOT NULL DEFAULT '',
    blocking BOOLEAN NOT NULL DEFAULT FALSE,
    output_schema JSONB NOT NULL DEFAULT '{"fields":[]}',
    FOREIGN KEY (pipeline_id, stage_number) REFERENCES pipeline_stages(pipeline_id, stage_number) ON DELETE CASCADE
)
-- idx_stage_side_tasks_stage(pipeline_id, stage_number)

pipeline_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pipeline_id UUID NOT NULL REFERENCES pipelines(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id),
    status TEXT NOT NULL DEFAULT 'running',         -- 'running' | 'completed' | 'failed' | 'paused'
    initial_task TEXT NOT NULL,
    stage_outputs JSONB NOT NULL DEFAULT '{}',
    current_stage INTEGER NOT NULL DEFAULT 0,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    total_input_tokens BIGINT NOT NULL DEFAULT 0,
    total_output_tokens BIGINT NOT NULL DEFAULT 0
)
-- idx_pipeline_runs_pipeline(pipeline_id)
-- idx_pipeline_runs_user(user_id)
-- idx_pipeline_runs_status(status)

stage_executions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id UUID NOT NULL REFERENCES pipeline_runs(id) ON DELETE CASCADE,
    stage_number INTEGER NOT NULL,
    stage_name TEXT NOT NULL,
    agent_id UUID,
    status TEXT NOT NULL DEFAULT 'running',         -- 'running' | 'completed' | 'failed'
    rendered_prompt TEXT,
    output TEXT,
    structured_output JSONB,
    user_input TEXT,
    input_tokens BIGINT NOT NULL DEFAULT 0,
    output_tokens BIGINT NOT NULL DEFAULT 0,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    duration_ms BIGINT NOT NULL DEFAULT 0,
    UNIQUE (run_id, stage_number)
)
-- idx_stage_executions_run(run_id)
```

---

## 6. Tickets & Slices

```sql
tickets (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    source_type TEXT NOT NULL DEFAULT 'manual',
    source_owner TEXT,
    source_repo TEXT,
    source_issue_number INTEGER,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    labels JSONB NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'new',             -- 'new' | 'decomposed' | 'in_progress' | 'done'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
-- idx_tickets_status(status)
-- idx_tickets_user_id(user_id)

vertical_slices (
    id UUID PRIMARY KEY,
    ticket_id UUID NOT NULL REFERENCES tickets(id),
    user_id UUID NOT NULL REFERENCES users(id),
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'pending',         -- 'pending' | 'in_progress' | 'completed'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
-- idx_slices_ticket(ticket_id)
-- idx_slices_status(status)
-- idx_vertical_slices_user_id(user_id)
```

---

## 7. Costs & Observability

```sql
cost_records (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    task_id UUID REFERENCES tasks(id),
    agent_id UUID NOT NULL REFERENCES agents(id),
    agent_tier TEXT NOT NULL,
    model_id TEXT NOT NULL,
    input_tokens INTEGER NOT NULL,
    output_tokens INTEGER NOT NULL,
    cost_usd REAL NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
-- idx_cost_records_task(task_id)
-- idx_cost_records_agent(agent_id)
-- idx_cost_records_tier(agent_tier)
-- idx_cost_records_timestamp(timestamp)
-- idx_cost_records_user_id(user_id)

llm_calls (
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
)
-- idx_llm_calls_task(task_id)
-- idx_llm_calls_timestamp(timestamp)
-- idx_llm_calls_model(model)
-- idx_llm_calls_user_id(user_id)

decisions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    task_id UUID NOT NULL,
    decision_type TEXT NOT NULL,
    reasoning TEXT NOT NULL,
    outcome TEXT NOT NULL,
    llm_call_id UUID,
    cost_usd REAL NOT NULL DEFAULT 0.0,
    timestamp TIMESTAMPTZ NOT NULL
)
-- idx_decisions_task(task_id)
-- idx_decisions_type(decision_type)
-- idx_decisions_timestamp(timestamp)
-- idx_decisions_user_id(user_id)

token_usage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID,
    agent_id UUID,
    tier TEXT NOT NULL DEFAULT 'unknown',
    model_id TEXT NOT NULL,
    input_tokens BIGINT NOT NULL DEFAULT 0,
    output_tokens BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
-- idx_token_usage_session(session_id)
-- idx_token_usage_created(created_at)
```

---

## 8. Refactoring & Change Tracking

```sql
system_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)

refactor_sessions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    started_at TIMESTAMPTZ NOT NULL,
    ended_at TIMESTAMPTZ,
    production_halted BOOLEAN NOT NULL DEFAULT FALSE,
    changes_applied INTEGER NOT NULL DEFAULT 0
)
-- idx_refactor_sessions_active(ended_at) WHERE ended_at IS NULL
-- idx_refactor_sessions_user_id(user_id)

refactor_changes (
    id UUID PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES refactor_sessions(id),
    file_path TEXT NOT NULL,
    change_type TEXT NOT NULL,
    before_content TEXT,
    after_content TEXT,
    reason TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'proposed',        -- 'proposed' | 'applied' | 'reverted'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
-- idx_refactor_changes_session(session_id)
-- idx_refactor_changes_status(status)
```

---

## 9. PR Merge Queue

```sql
pr_merge_queue (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    repo_owner TEXT NOT NULL,
    repo_name TEXT NOT NULL,
    pr_number INTEGER NOT NULL,
    queue_position INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',         -- 'pending' | 'merging' | 'merged' | 'failed'
    conflict_info TEXT,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    UNIQUE (repo_owner, repo_name, pr_number)
)
-- idx_pr_queue_position(repo_owner, repo_name, queue_position)
-- idx_pr_queue_status(status)
-- idx_pr_merge_queue_user_id(user_id)
```

---

## 10. Chat & Sessions

```sql
chat_sessions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    mode_id TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    summary TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
-- idx_chat_sessions_user(user_id, updated_at DESC)

chat_messages (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    session_id UUID,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
    content TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
-- idx_chat_messages_timestamp(timestamp)
-- idx_chat_messages_session(session_id, timestamp)
-- idx_chat_messages_user_id(user_id)
```

---

## 11. Documents & Knowledge Base

```sql
documents (
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
)
-- idx_documents_user(user_id)
-- idx_documents_session(session_id)
-- idx_documents_tags(tags) USING GIN
-- idx_documents_ref_tag(ref_tag)
-- idx_documents_search USING GIN(to_tsvector('english', title || ' ' || content))

agent_context (
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    PRIMARY KEY (agent_id, document_id)
)
-- idx_agent_context_agent(agent_id)
```

---

## 12. Tools & Scheduling

```sql
tools (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL DEFAULT 'general',
    parameter_schema JSONB NOT NULL DEFAULT '{}',
    output_schema JSONB NOT NULL DEFAULT '{}',
    enabled BOOLEAN NOT NULL DEFAULT true,
    cluster_id UUID REFERENCES clusters(id) ON DELETE SET NULL,
    is_builtin BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, name)
)
-- idx_tools_user(user_id)
-- idx_tools_category(category)
-- idx_tools_cluster(cluster_id)

agent_tools (
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
    PRIMARY KEY (agent_id, tool_id)
)

tool_calls (
    id UUID PRIMARY KEY,
    session_id UUID REFERENCES chat_sessions(id),
    message_id UUID NOT NULL,
    round INTEGER NOT NULL,
    tool_name TEXT NOT NULL,
    tool_use_id TEXT NOT NULL,
    input JSONB NOT NULL,
    output TEXT NOT NULL,
    latency_ms INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
-- idx_tool_calls_session(session_id)
-- idx_tool_calls_message(message_id)
-- idx_tool_calls_created(created_at)
-- idx_tool_calls_tool_name(tool_name)

schedules (
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
)
-- idx_schedules_user_id(user_id)

triggers (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    event_type TEXT NOT NULL,
    agent_id UUID NOT NULL,
    task_title TEXT NOT NULL,
    task_description TEXT NOT NULL,
    role TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
-- idx_triggers_user_id(user_id)
```

---

## 13. PRDs & Planning

```sql
prds (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'draft',           -- 'draft' | 'active' | 'completed'
    vision TEXT NOT NULL DEFAULT '',
    problem_statement TEXT NOT NULL DEFAULT '',
    target_users TEXT NOT NULL DEFAULT '',
    success_criteria JSONB NOT NULL DEFAULT '[]',
    technical_decisions JSONB NOT NULL DEFAULT '[]',
    data_models JSONB NOT NULL DEFAULT '[]',
    milestones JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
)
-- idx_prds_status(status)
-- idx_prds_user_id(user_id)

planning_sessions (
    id UUID PRIMARY KEY,
    prd_id UUID NOT NULL REFERENCES prds(id),
    user_id UUID NOT NULL REFERENCES users(id),
    phase TEXT NOT NULL DEFAULT 'discovery',        -- 'discovery' | 'refinement' | 'complete'
    history JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
)
-- idx_planning_sessions_prd_id(prd_id)
-- idx_planning_sessions_user_id(user_id)
```

---

## 14. Routing & Analytics

```sql
routing_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    session_id UUID,
    task_id UUID REFERENCES tasks(id) ON DELETE SET NULL,
    router_agent_id UUID NOT NULL,
    cluster_agent_id UUID,
    cluster_id UUID REFERENCES clusters(id) ON DELETE SET NULL,
    cluster_name TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    request TEXT NOT NULL,
    parameters JSONB NOT NULL DEFAULT '{}',
    response TEXT,
    error TEXT,
    status TEXT NOT NULL DEFAULT 'pending',         -- 'pending' | 'completed' | 'failed'
    agent_tier TEXT,
    model_id TEXT,
    input_tokens BIGINT NOT NULL DEFAULT 0,
    output_tokens BIGINT NOT NULL DEFAULT 0,
    duration_ms BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
)
-- idx_routing_events_user(user_id)
-- idx_routing_events_session(session_id)
-- idx_routing_events_task(task_id)
-- idx_routing_events_status(status)
-- idx_routing_events_created(created_at DESC)
-- idx_routing_events_cluster(cluster_id)
-- idx_routing_events_tool(tool_name)
-- idx_routing_events_user_created(user_id, created_at DESC)
```
