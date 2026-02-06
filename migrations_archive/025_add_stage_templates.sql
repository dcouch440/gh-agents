ALTER TABLE pipeline_stages
    ADD COLUMN IF NOT EXISTS stage_name TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS input_definitions JSONB NOT NULL DEFAULT '[]'::JSONB,
    ADD COLUMN IF NOT EXISTS output_description TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS output_schema JSONB NOT NULL DEFAULT '{"fields":[]}'::JSONB;

CREATE UNIQUE INDEX IF NOT EXISTS idx_pipeline_stages_name
    ON pipeline_stages(pipeline_id, stage_name) WHERE stage_name != '';
