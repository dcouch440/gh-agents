-- Index for efficient lookup of chat sessions linked to workflow steps.
-- Step sessions store their step_id in the existing draft_config JSONB column.
CREATE INDEX idx_chat_sessions_step_id
ON chat_sessions ((draft_config->>'step_id'))
WHERE draft_config->>'step_id' IS NOT NULL;
