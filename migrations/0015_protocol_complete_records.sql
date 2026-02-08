-- 0015: Make protocols "complete records" with associated agent, output schema,
-- and optional prompt template. These FKs reference system-owned rows
-- (user_id IS NULL) seeded during `make sync`.

-- Add FK columns to protocols
ALTER TABLE protocols ADD COLUMN IF NOT EXISTS agent_id uuid
    REFERENCES agents(id) ON DELETE SET NULL;

ALTER TABLE protocols ADD COLUMN IF NOT EXISTS output_schema_id uuid
    REFERENCES output_schemas(id) ON DELETE SET NULL;

ALTER TABLE protocols ADD COLUMN IF NOT EXISTS prompt_template_id uuid
    REFERENCES prompt_templates(id) ON DELETE SET NULL;

-- Indexes for FK lookups
CREATE INDEX IF NOT EXISTS idx_protocols_agent_id ON protocols(agent_id);
CREATE INDEX IF NOT EXISTS idx_protocols_output_schema_id ON protocols(output_schema_id);
CREATE INDEX IF NOT EXISTS idx_protocols_prompt_template_id ON protocols(prompt_template_id);
