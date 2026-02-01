-- Step 2.3: results

CREATE TABLE IF NOT EXISTS results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    agent_execution_id UUID NOT NULL REFERENCES agent_executions(id),
    output_schema_id UUID REFERENCES output_schemas(id),
    name TEXT NOT NULL,
    data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_results_user ON results(user_id);
CREATE INDEX IF NOT EXISTS idx_results_execution ON results(agent_execution_id);
CREATE INDEX IF NOT EXISTS idx_results_schema ON results(output_schema_id);
