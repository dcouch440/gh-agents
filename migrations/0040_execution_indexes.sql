-- 0040: Add partial indexes for hot execution query paths.
--
-- These target the three most common query patterns identified from code analysis:
-- 1. DAG resume: finding completed step executions to skip re-running
-- 2. Latest run lookup: finding the most recent execution per workflow
-- 3. Active execution monitoring: dashboard/cancellation queries
-- 4. Protocol execution phase lookup: last_run_handlers hot path

-- DAG resume: find completed non-interactive executions per step.
-- Used by hub/dag/resume to skip already-completed steps.
CREATE INDEX IF NOT EXISTS idx_agent_executions_dag_resume
    ON agent_executions(workflow_step_id, workflow_execution_id)
    WHERE status = 'completed' AND is_interactive = false;

-- Latest run lookup: most recent execution per workflow+user.
-- Used by last_run_handlers and run_detail_handlers.
CREATE INDEX IF NOT EXISTS idx_workflow_executions_latest
    ON workflow_executions(workflow_id, user_id, started_at DESC)
    WHERE execution_mode != 'workshop';

-- Active agent executions: running/pending for dashboard and cancellation.
CREATE INDEX IF NOT EXISTS idx_agent_executions_active
    ON agent_executions(status, started_at DESC)
    WHERE status IN ('pending', 'running');

-- Protocol executions by run+step: last_run_handlers aggregates phases by run,
-- then filters by step_id. This composite index covers both.
CREATE INDEX IF NOT EXISTS idx_protocol_executions_run_step
    ON protocol_executions(workflow_run_id, protocol_step_id);
