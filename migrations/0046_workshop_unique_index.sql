-- Workshop: one execution row per workflow.
-- Required by the ON CONFLICT clause in get_or_create_workshop().
CREATE UNIQUE INDEX IF NOT EXISTS idx_workflow_executions_workshop_unique
    ON workflow_executions (workflow_id)
    WHERE execution_mode = 'workshop';
