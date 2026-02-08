-- Allow standalone workflow execution (without a collection run)
ALTER TABLE workflow_executions ALTER COLUMN collection_run_id DROP NOT NULL;
