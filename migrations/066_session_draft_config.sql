-- Add draft_config JSONB column for workshop sessions
-- Allows sessions to store agent configuration without creating an agent
-- until the user explicitly saves

ALTER TABLE chat_sessions ADD COLUMN IF NOT EXISTS draft_config JSONB;

-- Index for efficient filtering of sessions with draft configs (workshop sessions)
CREATE INDEX IF NOT EXISTS idx_chat_sessions_has_draft_config ON chat_sessions ((draft_config IS NOT NULL));
