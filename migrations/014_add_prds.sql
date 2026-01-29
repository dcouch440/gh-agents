-- PRD documents table
CREATE TABLE IF NOT EXISTS prds (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'draft',
    vision TEXT NOT NULL DEFAULT '',
    problem_statement TEXT NOT NULL DEFAULT '',
    target_users TEXT NOT NULL DEFAULT '',
    success_criteria TEXT NOT NULL DEFAULT '[]',
    technical_decisions TEXT NOT NULL DEFAULT '[]',
    data_models TEXT NOT NULL DEFAULT '[]',
    milestones TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_prds_status ON prds(status);

-- Planning sessions table for resumable conversations
CREATE TABLE IF NOT EXISTS planning_sessions (
    id TEXT PRIMARY KEY,
    prd_id TEXT NOT NULL REFERENCES prds(id),
    phase TEXT NOT NULL DEFAULT 'discovery',
    history TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_planning_sessions_prd_id ON planning_sessions(prd_id)
