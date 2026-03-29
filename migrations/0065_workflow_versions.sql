-- Workflow version checkpoints with full snapshot storage.
-- Users and the system auto-save before destructive operations (Generate, Revert).
-- Snapshots are serialized WorkflowSnapshot structs (same as run_templates).

CREATE TABLE workflow_versions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_id     UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    version_number  INT NOT NULL,
    label           TEXT,
    source          TEXT NOT NULL,
    snapshot        JSONB NOT NULL,
    created_by      UUID NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (workflow_id, version_number)
);

CREATE INDEX idx_wv_workflow ON workflow_versions(workflow_id, version_number DESC);
