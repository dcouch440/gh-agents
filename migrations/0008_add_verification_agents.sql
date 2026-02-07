-- Multi-agent verification: critique agents review the primary agent's output.
ALTER TABLE public.workflow_steps
    ADD COLUMN IF NOT EXISTS verification_agent_ids JSONB;

COMMENT ON COLUMN public.workflow_steps.verification_agent_ids IS
    'JSON array of agent UUIDs that verify/critique this step''s output';
