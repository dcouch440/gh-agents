-- Few-shot from successful traces.
-- When a completed execution is marked as exemplary, the FewShotFilter
-- injects its input/output as a demonstration pair in future runs of
-- the same agent+step.

ALTER TABLE public.agent_executions
    ADD COLUMN IF NOT EXISTS is_exemplary BOOLEAN NOT NULL DEFAULT false;

-- Partial index: only exemplary rows are indexed, keeping the index tiny.
CREATE INDEX IF NOT EXISTS idx_agent_executions_exemplary
    ON public.agent_executions (agent_id, workflow_step_id)
    WHERE is_exemplary = true;
