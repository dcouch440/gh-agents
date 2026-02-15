-- Add execution_mode to workflow_executions for staging vs full runs.
ALTER TABLE workflow_executions ADD COLUMN execution_mode TEXT NOT NULL DEFAULT 'full';
