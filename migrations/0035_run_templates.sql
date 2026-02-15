-- Run templates: frozen workflow snapshots for reproducible execution.
CREATE TABLE run_templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_id UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    snapshot JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_run_templates_workflow ON run_templates (workflow_id, created_at DESC);

-- Track which template a run used (NULL = ran against live DB).
ALTER TABLE workflow_executions ADD COLUMN template_id UUID REFERENCES run_templates(id);
