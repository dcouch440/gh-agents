-- Make user_id optional on output_schemas and prompt_templates
-- to support system-level shared resources (e.g. protocol schemas)

ALTER TABLE output_schemas ALTER COLUMN user_id DROP NOT NULL;
ALTER TABLE prompt_templates ALTER COLUMN user_id DROP NOT NULL;

-- The existing UNIQUE(user_id, name) constraints remain valid for user-owned rows.
-- PostgreSQL treats NULLs as distinct in unique constraints, so multiple system rows
-- (user_id IS NULL) could share the same name. Add partial unique indexes to prevent that.
CREATE UNIQUE INDEX idx_output_schemas_system_name
  ON output_schemas (name) WHERE user_id IS NULL;

CREATE UNIQUE INDEX idx_prompt_templates_system_name
  ON prompt_templates (name) WHERE user_id IS NULL;
