CREATE TABLE IF NOT EXISTS agent_context (
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    PRIMARY KEY (agent_id, document_id)
);

CREATE INDEX IF NOT EXISTS idx_agent_context_agent ON agent_context(agent_id);
