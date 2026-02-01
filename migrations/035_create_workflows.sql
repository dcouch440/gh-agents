-- Workflows: reusable execution DAGs of agent steps
CREATE TABLE IF NOT EXISTS workflows (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_workflows_user ON workflows(user_id);

-- Workflow steps: DAG nodes
CREATE TABLE IF NOT EXISTS workflow_steps (
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
    display_order INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_workflow_steps_workflow ON workflow_steps(workflow_id);
CREATE INDEX IF NOT EXISTS idx_workflow_steps_agent ON workflow_steps(agent_id);

-- Workflow step edges: DAG edges defining execution order
CREATE TABLE IF NOT EXISTS workflow_step_edges (
    from_step_id UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    to_step_id UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    PRIMARY KEY (from_step_id, to_step_id)
);

CREATE INDEX IF NOT EXISTS idx_workflow_step_edges_from ON workflow_step_edges(from_step_id);
CREATE INDEX IF NOT EXISTS idx_workflow_step_edges_to ON workflow_step_edges(to_step_id);

-- Step documents: attach context documents to workflow steps
CREATE TABLE IF NOT EXISTS step_documents (
    step_id UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    PRIMARY KEY (step_id, document_id)
);

CREATE INDEX IF NOT EXISTS idx_step_documents_step ON step_documents(step_id);
