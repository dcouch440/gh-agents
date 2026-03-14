-- Scope runtime artifacts to a specific workflow run.
-- Design configs (designer phase) have NULL run_id and persist across runs.
-- Runtime artifacts (workforce agents) get the run_id of the execution.
ALTER TABLE system_files ADD COLUMN workflow_run_id UUID;
CREATE INDEX idx_system_files_workflow_run ON system_files(workflow_run_id);
