-- PR merge queue for ordered merging
CREATE TABLE IF NOT EXISTS pr_merge_queue (
    id TEXT PRIMARY KEY,
    repo_owner TEXT NOT NULL,
    repo_name TEXT NOT NULL,
    pr_number INTEGER NOT NULL,
    queue_position INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    conflict_info TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,

    UNIQUE(repo_owner, repo_name, pr_number)
);

CREATE INDEX IF NOT EXISTS idx_pr_queue_position
    ON pr_merge_queue(repo_owner, repo_name, queue_position);

CREATE INDEX IF NOT EXISTS idx_pr_queue_status
    ON pr_merge_queue(status);
