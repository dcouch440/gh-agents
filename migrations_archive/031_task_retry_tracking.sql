-- Task retry tracking: prevent infinite requeue loops
ALTER TABLE tasks ADD COLUMN retry_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE tasks ADD COLUMN max_retries INTEGER NOT NULL DEFAULT 3;
ALTER TABLE tasks ADD COLUMN last_error TEXT;
