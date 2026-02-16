-- Sub-workflow execution: allow steps to execute child workflows from templates.

-- Reference to the template snapshot that defines the child workflow.
ALTER TABLE workflow_steps
ADD COLUMN IF NOT EXISTS sub_workflow_template_id UUID REFERENCES run_templates(id) ON DELETE SET NULL;

-- Track parent-child execution hierarchy for nested workflows.
ALTER TABLE workflow_executions
ADD COLUMN IF NOT EXISTS parent_execution_id UUID REFERENCES workflow_executions(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_workflow_executions_parent
ON workflow_executions (parent_execution_id)
WHERE parent_execution_id IS NOT NULL;
