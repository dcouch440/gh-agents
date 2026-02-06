-- Step 2.1: agent_executions + execution_messages

CREATE TABLE IF NOT EXISTS agent_executions (
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
    completed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_agent_executions_stage ON agent_executions(stage_execution_id);
CREATE INDEX IF NOT EXISTS idx_agent_executions_agent ON agent_executions(agent_id);
CREATE INDEX IF NOT EXISTS idx_agent_executions_step ON agent_executions(workflow_step_id);
CREATE INDEX IF NOT EXISTS idx_agent_executions_status ON agent_executions(status);
CREATE INDEX IF NOT EXISTS idx_agent_executions_started ON agent_executions(started_at DESC);
CREATE INDEX IF NOT EXISTS idx_agent_executions_parent ON agent_executions(parent_agent_execution_id);

CREATE TABLE IF NOT EXISTS execution_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_execution_id UUID NOT NULL REFERENCES agent_executions(id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    tool_call_id TEXT,
    input_tokens BIGINT NOT NULL DEFAULT 0,
    output_tokens BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_execution_messages_execution ON execution_messages(agent_execution_id);
CREATE INDEX IF NOT EXISTS idx_execution_messages_role ON execution_messages(agent_execution_id, role);
CREATE INDEX IF NOT EXISTS idx_execution_messages_created ON execution_messages(created_at);
