-- Add execution_mode to workflow_executions for workshop vs full runs.
ALTER TABLE workflow_executions ADD COLUMN execution_mode TEXT NOT NULL DEFAULT 'full';

-- Enforce one workshop per workflow (unique partial index).
CREATE UNIQUE INDEX idx_one_workshop_per_workflow
ON workflow_executions (workflow_id)
WHERE execution_mode = 'workshop';
