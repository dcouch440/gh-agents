-- Multi-user support: users table, user_id on all data tables

-- 1. Users table
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT,
    github_id BIGINT UNIQUE,
    github_login TEXT,
    github_token_encrypted TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_github_id ON users(github_id) WHERE github_id IS NOT NULL;

-- 2. Create legacy user for existing data migration
-- If auth_config has a password, carry it over
INSERT INTO users (id, email, password_hash, created_at, updated_at)
SELECT
    '00000000-0000-0000-0000-000000000001'::UUID,
    'legacy@localhost',
    (SELECT password_hash FROM auth_config WHERE id = 1),
    NOW(),
    NOW()
WHERE EXISTS (SELECT 1 FROM auth_config WHERE id = 1);

-- If no auth_config row, still create the legacy user
INSERT INTO users (id, email, created_at, updated_at)
SELECT
    '00000000-0000-0000-0000-000000000001'::UUID,
    'legacy@localhost',
    NOW(),
    NOW()
WHERE NOT EXISTS (SELECT 1 FROM users WHERE id = '00000000-0000-0000-0000-000000000001'::UUID);

-- 3. Add user_id columns (nullable first for backfill)
ALTER TABLE tasks ADD COLUMN user_id UUID REFERENCES users(id);
ALTER TABLE agents ADD COLUMN user_id UUID REFERENCES users(id);
ALTER TABLE chat_messages ADD COLUMN user_id UUID REFERENCES users(id);
ALTER TABLE tickets ADD COLUMN user_id UUID REFERENCES users(id);
ALTER TABLE vertical_slices ADD COLUMN user_id UUID REFERENCES users(id);
ALTER TABLE prds ADD COLUMN user_id UUID REFERENCES users(id);
ALTER TABLE planning_sessions ADD COLUMN user_id UUID REFERENCES users(id);
ALTER TABLE cost_records ADD COLUMN user_id UUID REFERENCES users(id);
ALTER TABLE llm_calls ADD COLUMN user_id UUID REFERENCES users(id);
ALTER TABLE decisions ADD COLUMN user_id UUID REFERENCES users(id);
ALTER TABLE refactor_sessions ADD COLUMN user_id UUID REFERENCES users(id);
ALTER TABLE pr_merge_queue ADD COLUMN user_id UUID REFERENCES users(id);

-- 4. Backfill all existing rows to legacy user
UPDATE tasks SET user_id = '00000000-0000-0000-0000-000000000001'::UUID WHERE user_id IS NULL;
UPDATE agents SET user_id = '00000000-0000-0000-0000-000000000001'::UUID WHERE user_id IS NULL;
UPDATE chat_messages SET user_id = '00000000-0000-0000-0000-000000000001'::UUID WHERE user_id IS NULL;
UPDATE tickets SET user_id = '00000000-0000-0000-0000-000000000001'::UUID WHERE user_id IS NULL;
UPDATE vertical_slices SET user_id = '00000000-0000-0000-0000-000000000001'::UUID WHERE user_id IS NULL;
UPDATE prds SET user_id = '00000000-0000-0000-0000-000000000001'::UUID WHERE user_id IS NULL;
UPDATE planning_sessions SET user_id = '00000000-0000-0000-0000-000000000001'::UUID WHERE user_id IS NULL;
UPDATE cost_records SET user_id = '00000000-0000-0000-0000-000000000001'::UUID WHERE user_id IS NULL;
UPDATE llm_calls SET user_id = '00000000-0000-0000-0000-000000000001'::UUID WHERE user_id IS NULL;
UPDATE decisions SET user_id = '00000000-0000-0000-0000-000000000001'::UUID WHERE user_id IS NULL;
UPDATE refactor_sessions SET user_id = '00000000-0000-0000-0000-000000000001'::UUID WHERE user_id IS NULL;
UPDATE pr_merge_queue SET user_id = '00000000-0000-0000-0000-000000000001'::UUID WHERE user_id IS NULL;

-- 5. Set NOT NULL after backfill
ALTER TABLE tasks ALTER COLUMN user_id SET NOT NULL;
ALTER TABLE agents ALTER COLUMN user_id SET NOT NULL;
ALTER TABLE chat_messages ALTER COLUMN user_id SET NOT NULL;
ALTER TABLE tickets ALTER COLUMN user_id SET NOT NULL;
ALTER TABLE vertical_slices ALTER COLUMN user_id SET NOT NULL;
ALTER TABLE prds ALTER COLUMN user_id SET NOT NULL;
ALTER TABLE planning_sessions ALTER COLUMN user_id SET NOT NULL;
ALTER TABLE cost_records ALTER COLUMN user_id SET NOT NULL;
ALTER TABLE llm_calls ALTER COLUMN user_id SET NOT NULL;
ALTER TABLE decisions ALTER COLUMN user_id SET NOT NULL;
ALTER TABLE refactor_sessions ALTER COLUMN user_id SET NOT NULL;
ALTER TABLE pr_merge_queue ALTER COLUMN user_id SET NOT NULL;

-- 6. Indexes for user_id filtering
CREATE INDEX idx_tasks_user_id ON tasks(user_id);
CREATE INDEX idx_agents_user_id ON agents(user_id);
CREATE INDEX idx_chat_messages_user_id ON chat_messages(user_id);
CREATE INDEX idx_tickets_user_id ON tickets(user_id);
CREATE INDEX idx_vertical_slices_user_id ON vertical_slices(user_id);
CREATE INDEX idx_prds_user_id ON prds(user_id);
CREATE INDEX idx_planning_sessions_user_id ON planning_sessions(user_id);
CREATE INDEX idx_cost_records_user_id ON cost_records(user_id);
CREATE INDEX idx_llm_calls_user_id ON llm_calls(user_id);
CREATE INDEX idx_decisions_user_id ON decisions(user_id);
CREATE INDEX idx_refactor_sessions_user_id ON refactor_sessions(user_id);
CREATE INDEX idx_pr_merge_queue_user_id ON pr_merge_queue(user_id);
