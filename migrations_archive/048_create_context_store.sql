-- Context store: per-session context assembled before every LLM call
CREATE TABLE context_store (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES chat_sessions(id) ON DELETE CASCADE,
    source TEXT NOT NULL,
    priority REAL NOT NULL DEFAULT 0.5,
    content TEXT NOT NULL,
    metadata JSONB,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ
);
CREATE INDEX idx_context_store_session ON context_store(session_id, status);
CREATE INDEX idx_context_store_priority ON context_store(session_id, priority DESC);
