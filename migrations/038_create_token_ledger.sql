-- Step 2.2: token_ledger

CREATE TABLE IF NOT EXISTS token_ledger (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    agent_execution_id UUID NOT NULL REFERENCES agent_executions(id),
    model_id TEXT NOT NULL,
    input_tokens BIGINT NOT NULL,
    output_tokens BIGINT NOT NULL,
    cost_usd REAL NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_token_ledger_user ON token_ledger(user_id);
CREATE INDEX IF NOT EXISTS idx_token_ledger_agent_exec ON token_ledger(agent_execution_id);
CREATE INDEX IF NOT EXISTS idx_token_ledger_model ON token_ledger(model_id);
CREATE INDEX IF NOT EXISTS idx_token_ledger_created ON token_ledger(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_token_ledger_user_created ON token_ledger(user_id, created_at DESC);
