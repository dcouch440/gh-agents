-- Step question state: compressed status + pending question from Tier 3 extraction.
-- 1:1 per step, always overwritten (generational model).
CREATE TABLE IF NOT EXISTS step_question_state (
    step_id       UUID PRIMARY KEY REFERENCES workflow_steps(id) ON DELETE CASCADE,
    status_text   TEXT NOT NULL,
    question_text TEXT,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
