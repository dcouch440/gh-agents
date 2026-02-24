-- Canvas snapshot persistence for board submit diff pipeline.
-- One row per workflow, upserted on each board submit.
CREATE TABLE canvas_snapshots (
    workflow_id UUID PRIMARY KEY REFERENCES workflows(id) ON DELETE CASCADE,
    snapshot_json TEXT NOT NULL,
    elements_json TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
