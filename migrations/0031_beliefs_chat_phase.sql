-- Allow chat-sourced beliefs that have no workflow execution run.
ALTER TABLE beliefs ALTER COLUMN workflow_execution_id DROP NOT NULL;
