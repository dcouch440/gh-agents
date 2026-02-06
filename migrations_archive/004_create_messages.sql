-- Messages table (inter-agent communication)
CREATE TABLE IF NOT EXISTS messages (
    id UUID PRIMARY KEY NOT NULL,
    from_agent UUID NOT NULL,
    to_agent UUID NOT NULL,
    message_type TEXT NOT NULL,
    content TEXT NOT NULL,
    task_id UUID,
    context TEXT,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    FOREIGN KEY (from_agent) REFERENCES agents(id),
    FOREIGN KEY (to_agent) REFERENCES agents(id),
    FOREIGN KEY (task_id) REFERENCES tasks(id)
);

-- Index for querying messages
CREATE INDEX IF NOT EXISTS idx_messages_from ON messages(from_agent);
CREATE INDEX IF NOT EXISTS idx_messages_to ON messages(to_agent);
CREATE INDEX IF NOT EXISTS idx_messages_task ON messages(task_id);
CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp);
