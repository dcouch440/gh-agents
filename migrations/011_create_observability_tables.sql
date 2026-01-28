-- Observability tables for LLM call logging and decision tracking

-- LLM calls table: stores all LLM API calls for replay and debugging
CREATE TABLE IF NOT EXISTS llm_calls (
    id TEXT PRIMARY KEY,
    task_id TEXT,
    agent_id TEXT,
    model TEXT NOT NULL,
    prompt TEXT NOT NULL,
    response TEXT NOT NULL,
    input_tokens INTEGER NOT NULL,
    output_tokens INTEGER NOT NULL,
    latency_ms INTEGER NOT NULL,
    timestamp TEXT NOT NULL,
    cost_usd REAL NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_llm_calls_task ON llm_calls(task_id);
CREATE INDEX IF NOT EXISTS idx_llm_calls_timestamp ON llm_calls(timestamp);
CREATE INDEX IF NOT EXISTS idx_llm_calls_model ON llm_calls(model);

-- Decisions table: stores orchestrator decisions with reasoning
CREATE TABLE IF NOT EXISTS decisions (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    decision_type TEXT NOT NULL,
    reasoning TEXT NOT NULL,
    outcome TEXT NOT NULL,
    llm_call_id TEXT,
    cost_usd REAL NOT NULL DEFAULT 0.0,
    timestamp TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_decisions_task ON decisions(task_id);
CREATE INDEX IF NOT EXISTS idx_decisions_type ON decisions(decision_type);
CREATE INDEX IF NOT EXISTS idx_decisions_timestamp ON decisions(timestamp);
