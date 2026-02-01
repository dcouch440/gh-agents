-- Routing events: observability and analytics for tool routing through clusters.
-- Every request_assistance call creates a row. Completed rows have cost/timing data.
CREATE TABLE IF NOT EXISTS routing_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    session_id UUID,
    task_id UUID REFERENCES tasks(id) ON DELETE SET NULL,
    router_agent_id UUID NOT NULL,
    cluster_agent_id UUID,
    cluster_id UUID REFERENCES clusters(id) ON DELETE SET NULL,
    cluster_name TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    request TEXT NOT NULL,
    parameters JSONB NOT NULL DEFAULT '{}'::JSONB,
    response TEXT,
    error TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    agent_tier TEXT,
    model_id TEXT,
    input_tokens BIGINT NOT NULL DEFAULT 0,
    output_tokens BIGINT NOT NULL DEFAULT 0,
    duration_ms BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

-- Query patterns: by user, session, status, time range, cluster, tool
CREATE INDEX IF NOT EXISTS idx_routing_events_user ON routing_events(user_id);
CREATE INDEX IF NOT EXISTS idx_routing_events_session ON routing_events(session_id);
CREATE INDEX IF NOT EXISTS idx_routing_events_task ON routing_events(task_id);
CREATE INDEX IF NOT EXISTS idx_routing_events_status ON routing_events(status);
CREATE INDEX IF NOT EXISTS idx_routing_events_created ON routing_events(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_routing_events_cluster ON routing_events(cluster_id);
CREATE INDEX IF NOT EXISTS idx_routing_events_tool ON routing_events(tool_name);
-- Composite for analytics dashboards: cost by user over time
CREATE INDEX IF NOT EXISTS idx_routing_events_user_created ON routing_events(user_id, created_at DESC);
