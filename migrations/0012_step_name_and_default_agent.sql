-- 0012: Add name column to workflow_steps, make agents.user_id nullable, seed default agent
--
-- The React Flow canvas needs a display label for each step node.
-- Also seeds a default agent so steps can be created without an explicit agent_id.
-- Making agents.user_id nullable allows system-level agents with no owner.

-- Add name column to workflow_steps (nullable for backward compatibility)
ALTER TABLE workflow_steps ADD COLUMN IF NOT EXISTS name text;

-- Make agents.user_id nullable (allows system agents with no owner)
ALTER TABLE agents ALTER COLUMN user_id DROP NOT NULL;

-- Seed default agent (well-known UUID, no owner, idempotent)
INSERT INTO agents (id, name, system_prompt, model_provider, model_id, model_max_tokens, model_temperature)
VALUES (
    '00000000-0000-0000-0000-000000000001',
    'Default Agent',
    '',
    'anthropic',
    'claude-sonnet-4-20250514',
    8192,
    0.7
)
ON CONFLICT (id) DO NOTHING;
