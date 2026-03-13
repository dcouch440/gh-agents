-- System store: metadata sidecar for S3-backed workflow filesystems.
-- Each workflow gets a .system/ namespace; this table tracks file metadata.
CREATE TABLE system_files (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_id       UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    path              TEXT NOT NULL,
    media_type        TEXT NOT NULL DEFAULT 'application/octet-stream',
    description       TEXT NOT NULL DEFAULT '',
    tags              TEXT[] NOT NULL DEFAULT '{}',
    produced_by       UUID REFERENCES workflow_steps(id) ON DELETE SET NULL,
    produced_by_agent TEXT,
    version           INT NOT NULL DEFAULT 1,
    size_bytes        BIGINT NOT NULL DEFAULT 0,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(workflow_id, path)
);

CREATE INDEX idx_system_files_workflow ON system_files(workflow_id);
CREATE INDEX idx_system_files_produced_by ON system_files(produced_by);
