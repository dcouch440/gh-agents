-- Documenter Protocol
-- Schema changes for the full documenter protocol system:
--   1A. Make workflow_steps.agent_id nullable (agent-less execution)
--   1B. Add visible column to workflow_steps (hidden step support)
--   1C. Extend documents table for protocol integration
--   1D. Extend protocol_document_defs for protocol-scoped defs
--   1E. Create protocol_executions table (hidden execution audit trail)

-- 1A. Make workflow_steps.agent_id nullable
-- Steps can now execute without an agent (e.g., documenter hidden steps
-- get their execution config from the protocol/workflow itself).
ALTER TABLE workflow_steps ALTER COLUMN agent_id DROP NOT NULL;

-- 1B. Add visible column to workflow_steps
-- Hidden steps (visible=false) execute normally in the DAG but are not
-- rendered on the canvas. Used by the DocumenterExecutor's hidden chain.
ALTER TABLE workflow_steps ADD COLUMN IF NOT EXISTS visible boolean DEFAULT true;

-- 1C. Extend documents table for protocol integration
ALTER TABLE documents
    ADD COLUMN IF NOT EXISTS workflow_id uuid REFERENCES workflows(id),
    ADD COLUMN IF NOT EXISTS target_length integer,
    ADD COLUMN IF NOT EXISTS is_static boolean DEFAULT false,
    ADD COLUMN IF NOT EXISTS source_protocol_step_id uuid REFERENCES workflow_steps(id);

CREATE INDEX IF NOT EXISTS idx_documents_workflow_id
    ON documents(workflow_id);
CREATE INDEX IF NOT EXISTS idx_documents_source_protocol_step_id
    ON documents(source_protocol_step_id);

-- 1D. Extend protocol_document_defs for protocol-scoped definitions
-- Defs can now be scoped to a protocol (template) or a step (applied instance).
-- Adding document_id to link def to the actual generated document.
ALTER TABLE protocol_document_defs ALTER COLUMN step_id DROP NOT NULL;
ALTER TABLE protocol_document_defs
    ADD COLUMN IF NOT EXISTS protocol_id uuid REFERENCES protocols(id) ON DELETE CASCADE,
    ADD COLUMN IF NOT EXISTS document_id uuid REFERENCES documents(id);

CREATE INDEX IF NOT EXISTS idx_protocol_document_defs_protocol_id
    ON protocol_document_defs(protocol_id);

-- Exactly one of step_id or protocol_id must be set
ALTER TABLE protocol_document_defs
    ADD CONSTRAINT check_document_def_scope CHECK (
        (step_id IS NOT NULL AND protocol_id IS NULL) OR
        (step_id IS NULL AND protocol_id IS NOT NULL)
    );

-- 1E. Create protocol_executions table
-- Persists hidden execution state for each phase of a documenter run.
-- Enables debugging, retry, and future UI inspection of what happened
-- inside a DocumenterExecutor run.
CREATE TABLE IF NOT EXISTS protocol_executions (
    id                uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    protocol_step_id  uuid NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    workflow_run_id   uuid,
    phase             text NOT NULL,
    document_def_id   uuid REFERENCES protocol_document_defs(id),
    agent_id          uuid REFERENCES agents(id),
    input_prompt      text,
    output_content    text,
    status            text NOT NULL DEFAULT 'pending',
    error_message     text,
    tokens_in         integer,
    tokens_out        integer,
    cost_usd          double precision,
    model             text,
    capabilities_used text[],
    created_at        timestamptz DEFAULT now(),
    completed_at      timestamptz
);

CREATE INDEX IF NOT EXISTS idx_protocol_executions_step_id
    ON protocol_executions(protocol_step_id);
CREATE INDEX IF NOT EXISTS idx_protocol_executions_run_id
    ON protocol_executions(workflow_run_id);

ALTER TABLE protocol_executions
    ADD CONSTRAINT protocol_executions_phase_check
    CHECK (phase IN ('strategy', 'research', 'write'));
ALTER TABLE protocol_executions
    ADD CONSTRAINT protocol_executions_status_check
    CHECK (status IN ('pending', 'running', 'complete', 'failed'));
