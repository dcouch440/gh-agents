-- Cost records table
CREATE TABLE IF NOT EXISTS cost_records (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT,
    agent_id TEXT NOT NULL,
    agent_tier TEXT NOT NULL,
    model_id TEXT NOT NULL,
    input_tokens INTEGER NOT NULL,
    output_tokens INTEGER NOT NULL,
    cost_usd REAL NOT NULL,
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (task_id) REFERENCES tasks(id),
    FOREIGN KEY (agent_id) REFERENCES agents(id)
);

-- Index for cost aggregation queries
CREATE INDEX IF NOT EXISTS idx_cost_records_task ON cost_records(task_id);
CREATE INDEX IF NOT EXISTS idx_cost_records_agent ON cost_records(agent_id);
CREATE INDEX IF NOT EXISTS idx_cost_records_tier ON cost_records(agent_tier);
CREATE INDEX IF NOT EXISTS idx_cost_records_timestamp ON cost_records(timestamp);
