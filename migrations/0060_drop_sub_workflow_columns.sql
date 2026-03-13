-- Drop sub_workflow-only columns. root_execution_id and depth stay (used by collection DAG).
ALTER TABLE workflow_steps DROP COLUMN IF EXISTS sub_workflow_template_id;
ALTER TABLE workflow_executions DROP COLUMN IF EXISTS parent_execution_id;
DROP INDEX IF EXISTS idx_workflow_executions_parent;
