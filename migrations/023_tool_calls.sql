CREATE TABLE IF NOT EXISTS tool_calls (
    id UUID PRIMARY KEY,
    session_id UUID REFERENCES chat_sessions(id),
    message_id UUID NOT NULL,
    round INTEGER NOT NULL,
    tool_name TEXT NOT NULL,
    tool_use_id TEXT NOT NULL,
    input JSONB NOT NULL,
    output TEXT NOT NULL,
    latency_ms INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_tool_calls_session ON tool_calls(session_id);
CREATE INDEX IF NOT EXISTS idx_tool_calls_message ON tool_calls(message_id);
CREATE INDEX IF NOT EXISTS idx_tool_calls_created ON tool_calls(created_at);
CREATE INDEX IF NOT EXISTS idx_tool_calls_tool_name ON tool_calls(tool_name);
