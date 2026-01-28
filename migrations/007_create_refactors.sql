-- System state table for production mode
CREATE TABLE IF NOT EXISTS system_state (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Initialize production mode to running
INSERT OR IGNORE INTO system_state (key, value)
VALUES ('production_mode', 'running');

-- Refactor sessions
CREATE TABLE IF NOT EXISTS refactor_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    production_halted INTEGER NOT NULL DEFAULT 0,
    changes_applied INTEGER NOT NULL DEFAULT 0
);

-- Proposed changes within a session
CREATE TABLE IF NOT EXISTS refactor_changes (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    change_type TEXT NOT NULL,
    before_content TEXT,
    after_content TEXT,
    reason TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'proposed',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (session_id) REFERENCES refactor_sessions(id)
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_refactor_sessions_active
    ON refactor_sessions(ended_at) WHERE ended_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_refactor_changes_session
    ON refactor_changes(session_id);
CREATE INDEX IF NOT EXISTS idx_refactor_changes_status
    ON refactor_changes(status);
