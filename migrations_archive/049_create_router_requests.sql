-- Router requests: logs every routing decision
CREATE TABLE router_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES chat_sessions(id) ON DELETE CASCADE,
    agent_execution_id UUID REFERENCES agent_executions(id),
    intent TEXT NOT NULL,
    priority TEXT NOT NULL DEFAULT 'normal',
    callback_hint TEXT,
    routed_tool TEXT,
    routed_args JSONB,
    is_async BOOLEAN NOT NULL DEFAULT FALSE,
    passdown TEXT,
    chain JSONB,
    status TEXT NOT NULL DEFAULT 'pending',
    result TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);
CREATE INDEX idx_router_requests_session ON router_requests(session_id, status);
