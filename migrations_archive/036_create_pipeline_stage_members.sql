CREATE TABLE IF NOT EXISTS pipeline_stage_members (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pipeline_id UUID NOT NULL,
    stage_number INTEGER NOT NULL,
    workflow_id UUID NOT NULL REFERENCES workflows(id),
    display_order INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (pipeline_id, stage_number)
        REFERENCES pipeline_stages(pipeline_id, stage_number) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_pipeline_stage_members_stage ON pipeline_stage_members(pipeline_id, stage_number);
CREATE INDEX IF NOT EXISTS idx_pipeline_stage_members_workflow ON pipeline_stage_members(workflow_id);
