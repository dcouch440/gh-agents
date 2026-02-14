-- Assistant's Notes: persistent per-step scratchpad maintained by the node assistant.
-- One row per step (upsert pattern via unique index on step_id).
CREATE TABLE assistant_notes (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    step_id     UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    content     TEXT NOT NULL DEFAULT '',
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_assistant_notes_step_id ON assistant_notes(step_id);

-- Board Overview Summary: one-paragraph Haiku-generated summary of all assistant
-- notes across the workflow. Injected into every assistant's system prompt.
ALTER TABLE workflows ADD COLUMN board_overview_summary TEXT NOT NULL DEFAULT '';
