-- Chat messages table (user-orchestrator conversation)
CREATE TABLE IF NOT EXISTS chat_messages (
    id UUID PRIMARY KEY NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
    content TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for querying chat history
CREATE INDEX IF NOT EXISTS idx_chat_messages_timestamp ON chat_messages(timestamp);
