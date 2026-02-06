-- Cost records table
CREATE TABLE IF NOT EXISTS cost_records (
    id UUID PRIMARY KEY NOT NULL,
    task_id UUID,
    agent_id UUID NOT NULL,
    agent_tier TEXT NOT NULL,
    model_id TEXT NOT NULL,
    input_tokens INTEGER NOT NULL,
    output_tokens INTEGER NOT NULL,
    cost_usd REAL NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    FOREIGN KEY (task_id) REFERENCES tasks(id),
    FOREIGN KEY (agent_id) REFERENCES agents(id)
);

-- Index for cost aggregation queries
CREATE INDEX IF NOT EXISTS idx_cost_records_task ON cost_records(task_id);
CREATE INDEX IF NOT EXISTS idx_cost_records_agent ON cost_records(agent_id);
CREATE INDEX IF NOT EXISTS idx_cost_records_tier ON cost_records(agent_tier);
CREATE INDEX IF NOT EXISTS idx_cost_records_timestamp ON cost_records(timestamp);
