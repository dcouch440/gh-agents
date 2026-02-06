-- Task events table (append-only log)
CREATE TABLE IF NOT EXISTS task_events (
    id UUID PRIMARY KEY NOT NULL,
    task_id UUID NOT NULL,
    event_type TEXT NOT NULL,
    agent_id UUID,
    details TEXT NOT NULL DEFAULT '',
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    FOREIGN KEY (task_id) REFERENCES tasks(id)
);

-- Index for querying events by task
CREATE INDEX IF NOT EXISTS idx_task_events_task_id ON task_events(task_id);
CREATE INDEX IF NOT EXISTS idx_task_events_timestamp ON task_events(timestamp);
