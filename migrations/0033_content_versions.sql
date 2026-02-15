-- Immutable content version snapshots with SHA-256 dedup.
-- Each row is a single immutable snapshot — never updated after creation.
CREATE TABLE content_versions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id       UUID NOT NULL,
    content_type    TEXT NOT NULL,
    content_hash    TEXT NOT NULL,
    content         TEXT NOT NULL,
    version_number  INT NOT NULL DEFAULT 1,
    byte_size       INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Dedup: same content for the same source entity is stored once.
CREATE UNIQUE INDEX idx_cv_dedup ON content_versions(source_id, content_type, content_hash);

-- Fast lookup by source, newest first.
CREATE INDEX idx_cv_source ON content_versions(source_id, content_type, version_number DESC);

-- Run snapshots: links (run_id, step_id, content_type, role) to a content version.
CREATE TABLE run_snapshots (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id              UUID NOT NULL,
    step_id             UUID NOT NULL,
    content_type        TEXT NOT NULL,
    role                TEXT NOT NULL,
    content_version_id  UUID NOT NULL REFERENCES content_versions(id),
    source_id           UUID NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- One snapshot per run per step per content type per role.
CREATE UNIQUE INDEX idx_rs_unique ON run_snapshots(run_id, step_id, content_type, role);

-- Fast lookup: "what versions did this run use?"
CREATE INDEX idx_rs_run ON run_snapshots(run_id);

-- Fast lookup: "what runs used this version?"
CREATE INDEX idx_rs_version ON run_snapshots(content_version_id);
