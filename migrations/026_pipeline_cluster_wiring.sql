-- Wire clusters into pipeline stages, add fan-out, add side tasks

-- Add cluster_id and fan_out to pipeline_stages
ALTER TABLE pipeline_stages
    ADD COLUMN IF NOT EXISTS cluster_id UUID REFERENCES clusters(id),
    ADD COLUMN IF NOT EXISTS fan_out BOOLEAN NOT NULL DEFAULT FALSE;

-- Make agent_id nullable (stages can use cluster_id instead)
ALTER TABLE pipeline_stages ALTER COLUMN agent_id DROP NOT NULL;

-- Add role and persona override to cluster_members
ALTER TABLE cluster_members
    ADD COLUMN IF NOT EXISTS role TEXT,
    ADD COLUMN IF NOT EXISTS persona_override TEXT NOT NULL DEFAULT '';

-- Side tasks table
CREATE TABLE IF NOT EXISTS stage_side_tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pipeline_id UUID NOT NULL,
    stage_number INTEGER NOT NULL,
    agent_id UUID NOT NULL REFERENCES agents(id),
    input_definitions JSONB NOT NULL DEFAULT '[]'::JSONB,
    output_name TEXT NOT NULL DEFAULT '',
    blocking BOOLEAN NOT NULL DEFAULT FALSE,
    output_schema JSONB NOT NULL DEFAULT '{"fields":[]}'::JSONB,
    FOREIGN KEY (pipeline_id, stage_number)
        REFERENCES pipeline_stages(pipeline_id, stage_number) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_stage_side_tasks_stage
    ON stage_side_tasks(pipeline_id, stage_number);
