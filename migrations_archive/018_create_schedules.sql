CREATE TABLE IF NOT EXISTS schedules (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    agent_id UUID NOT NULL,
    interval_seconds INTEGER NOT NULL,
    task_title TEXT NOT NULL,
    task_description TEXT NOT NULL,
    role TEXT,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    last_run_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS triggers (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    event_type TEXT NOT NULL,
    agent_id UUID NOT NULL,
    task_title TEXT NOT NULL,
    task_description TEXT NOT NULL,
    role TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_schedules_user_id ON schedules(user_id);
CREATE INDEX IF NOT EXISTS idx_triggers_user_id ON triggers(user_id);
