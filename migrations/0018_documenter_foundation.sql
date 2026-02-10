-- Documenter Foundation
-- Adds default_reasoning_trace to agents and creates protocol_document_defs table.

-- 1. Add default_reasoning_trace to agents table
ALTER TABLE agents ADD COLUMN IF NOT EXISTS default_reasoning_trace boolean DEFAULT false;

-- 2. Create protocol_document_defs table
-- Stores document definitions per documenter workflow step.
-- Each documenter step can have multiple document definitions (name, description, target_length).
CREATE TABLE IF NOT EXISTS protocol_document_defs (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    step_id         uuid NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    name            text NOT NULL,
    description     text NOT NULL DEFAULT '',
    target_length   integer NOT NULL DEFAULT 2000,
    display_order   integer DEFAULT 0,
    created_at      timestamptz DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_protocol_document_defs_step_id
    ON protocol_document_defs(step_id);

-- 3. Add 'documenter' to the protocols type check constraint
ALTER TABLE protocols DROP CONSTRAINT IF EXISTS protocols_type_check;
ALTER TABLE protocols ADD CONSTRAINT protocols_type_check
    CHECK (protocol_type = ANY (ARRAY['decomp', 'transform', 'review', 'route', 'default', 'documenter']));
