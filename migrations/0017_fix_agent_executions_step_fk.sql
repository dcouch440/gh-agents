-- Fix: allow workflow step deletion when agent_executions reference it.
-- The column is already nullable, so SET NULL preserves execution history.
ALTER TABLE agent_executions
    DROP CONSTRAINT agent_executions_workflow_step_id_fkey,
    ADD CONSTRAINT agent_executions_workflow_step_id_fkey
        FOREIGN KEY (workflow_step_id) REFERENCES workflow_steps(id) ON DELETE SET NULL;
