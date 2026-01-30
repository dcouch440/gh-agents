CREATE TABLE IF NOT EXISTS pipelines (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS pipeline_stages (
    pipeline_id UUID NOT NULL REFERENCES pipelines(id) ON DELETE CASCADE,
    stage_number INTEGER NOT NULL,
    agent_id UUID NOT NULL,
    role TEXT,
    approval_required BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (pipeline_id, stage_number)
);

CREATE INDEX IF NOT EXISTS idx_pipelines_user_id ON pipelines(user_id);
