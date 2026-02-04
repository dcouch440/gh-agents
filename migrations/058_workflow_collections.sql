-- Migration: Workflow Collections (Multi-Tier DAG Architecture)
-- Add a tier above workflows for creating DAGs of workflows

-- Collection definition (like workflows table)
CREATE TABLE workflow_collections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    execution_mode TEXT NOT NULL DEFAULT 'parallel', -- "sequential" or "parallel"
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_workflow_collections_user_id ON workflow_collections(user_id);

-- Members: which workflows belong to this collection
CREATE TABLE collection_workflows (
    collection_id UUID NOT NULL REFERENCES workflow_collections(id) ON DELETE CASCADE,
    workflow_id UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    display_order INT NOT NULL DEFAULT 0,
    execution_mode TEXT, -- NULL = inherit from collection, "sequential"/"parallel" = override
    PRIMARY KEY (collection_id, workflow_id)
);

CREATE INDEX idx_collection_workflows_collection_id ON collection_workflows(collection_id);
CREATE INDEX idx_collection_workflows_workflow_id ON collection_workflows(workflow_id);

-- DAG edges between workflows (like workflow_step_edges)
CREATE TABLE collection_workflow_edges (
    from_workflow_id UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    to_workflow_id UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    collection_id UUID NOT NULL REFERENCES workflow_collections(id) ON DELETE CASCADE,
    PRIMARY KEY (from_workflow_id, to_workflow_id, collection_id)
);

CREATE INDEX idx_collection_workflow_edges_collection_id ON collection_workflow_edges(collection_id);
CREATE INDEX idx_collection_workflow_edges_from_workflow_id ON collection_workflow_edges(from_workflow_id);
CREATE INDEX idx_collection_workflow_edges_to_workflow_id ON collection_workflow_edges(to_workflow_id);

-- Execution tracking for collection runs
CREATE TABLE collection_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    collection_id UUID NOT NULL REFERENCES workflow_collections(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status TEXT NOT NULL, -- "running", "completed", "failed", "cancelled"
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    error TEXT
);

CREATE INDEX idx_collection_runs_collection_id ON collection_runs(collection_id);
CREATE INDEX idx_collection_runs_user_id ON collection_runs(user_id);
CREATE INDEX idx_collection_runs_status ON collection_runs(status);

-- Workflow-level execution within a collection run
CREATE TABLE workflow_executions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    collection_run_id UUID NOT NULL REFERENCES collection_runs(id) ON DELETE CASCADE,
    workflow_id UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status TEXT NOT NULL, -- "pending", "running", "completed", "failed"
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    outputs JSONB, -- { "variable_name": {...} } for downstream workflows
    error TEXT
);

CREATE INDEX idx_workflow_executions_collection_run_id ON workflow_executions(collection_run_id);
CREATE INDEX idx_workflow_executions_workflow_id ON workflow_executions(workflow_id);
CREATE INDEX idx_workflow_executions_user_id ON workflow_executions(user_id);
CREATE INDEX idx_workflow_executions_status ON workflow_executions(status);

-- Link existing agent_executions to workflow_executions
ALTER TABLE agent_executions
ADD COLUMN workflow_execution_id UUID REFERENCES workflow_executions(id) ON DELETE CASCADE;

CREATE INDEX idx_agent_executions_workflow_execution_id ON agent_executions(workflow_execution_id);

-- Execution variables (for text editor variable capture)
CREATE TABLE execution_variables (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    collection_run_id UUID REFERENCES collection_runs(id) ON DELETE CASCADE,
    workflow_execution_id UUID REFERENCES workflow_executions(id) ON DELETE CASCADE,
    step_execution_id UUID REFERENCES agent_executions(id) ON DELETE CASCADE,
    variable_name TEXT NOT NULL,
    variable_path TEXT NOT NULL, -- "$workflow_a.step1.analysis"
    value JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_exec_vars_collection_run_id ON execution_variables(collection_run_id);
CREATE INDEX idx_exec_vars_lookup ON execution_variables(collection_run_id, workflow_execution_id, variable_name);
CREATE INDEX idx_exec_vars_path ON execution_variables(collection_run_id, variable_path);

-- Add execution_mode to workflows table (for controlling step execution)
ALTER TABLE workflows
ADD COLUMN execution_mode TEXT NOT NULL DEFAULT 'parallel'; -- "sequential" or "parallel"

-- Add agent_execution_mode to workflow_steps table (for controlling multi-agent execution)
-- Note: execution_mode already exists for iteration mode ("single", "for_each", "room")
ALTER TABLE workflow_steps
ADD COLUMN agent_execution_mode TEXT; -- NULL = inherit from workflow, "sequential"/"parallel" = override

-- Multi-agent step support: allow multiple agents per step
CREATE TABLE workflow_step_agents (
    step_id UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    execution_strategy TEXT NOT NULL, -- "sequential", "parallel", "vote"
    agent_order INT NOT NULL DEFAULT 0, -- for sequential execution
    PRIMARY KEY (step_id, agent_id)
);

CREATE INDEX idx_workflow_step_agents_step_id ON workflow_step_agents(step_id);
CREATE INDEX idx_workflow_step_agents_agent_id ON workflow_step_agents(agent_id);

-- Migrate existing workflow_steps.agent_id to workflow_step_agents
-- Only migrate rows where agent_id is NOT NULL
INSERT INTO workflow_step_agents (step_id, agent_id, execution_strategy, agent_order)
SELECT id, agent_id, 'sequential', 0
FROM workflow_steps
WHERE agent_id IS NOT NULL;

-- Note: We're NOT dropping workflow_steps.agent_id yet for backwards compatibility
-- This can be done in a future migration after confirming the new system works
