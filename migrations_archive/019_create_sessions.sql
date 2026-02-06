-- Add session support to chat messages
ALTER TABLE chat_messages ADD COLUMN session_id UUID;
CREATE INDEX idx_chat_messages_session ON chat_messages(session_id, timestamp);

-- Chat sessions table
CREATE TABLE chat_sessions (
    id UUID PRIMARY KEY NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id),
    mode_id TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_chat_sessions_user ON chat_sessions(user_id, updated_at DESC);
