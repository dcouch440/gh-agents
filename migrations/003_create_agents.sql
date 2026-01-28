-- Agents table
CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY NOT NULL,
    tier TEXT NOT NULL,
    persona_name TEXT NOT NULL,
    persona_prompt TEXT NOT NULL DEFAULT '',
    persona_style TEXT NOT NULL DEFAULT 'casual',
    model_provider TEXT NOT NULL DEFAULT 'anthropic',
    model_id TEXT NOT NULL,
    model_max_tokens INTEGER NOT NULL DEFAULT 4096,
    model_temperature REAL NOT NULL DEFAULT 0.7,
    current_task TEXT,
    status TEXT NOT NULL DEFAULT 'idle',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (current_task) REFERENCES tasks(id)
);

-- Index for finding agents by tier and status
CREATE INDEX IF NOT EXISTS idx_agents_tier ON agents(tier);
CREATE INDEX IF NOT EXISTS idx_agents_status ON agents(status);
