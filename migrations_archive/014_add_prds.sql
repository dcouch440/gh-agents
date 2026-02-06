-- PRD documents table
CREATE TABLE IF NOT EXISTS prds (
    id UUID PRIMARY KEY,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'draft',
    vision TEXT NOT NULL DEFAULT '',
    problem_statement TEXT NOT NULL DEFAULT '',
    target_users TEXT NOT NULL DEFAULT '',
    success_criteria JSONB NOT NULL DEFAULT '[]',
    technical_decisions JSONB NOT NULL DEFAULT '[]',
    data_models JSONB NOT NULL DEFAULT '[]',
    milestones JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_prds_status ON prds(status);

-- Planning sessions table for resumable conversations
CREATE TABLE IF NOT EXISTS planning_sessions (
    id UUID PRIMARY KEY,
    prd_id UUID NOT NULL REFERENCES prds(id),
    phase TEXT NOT NULL DEFAULT 'discovery',
    history JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_planning_sessions_prd_id ON planning_sessions(prd_id);
