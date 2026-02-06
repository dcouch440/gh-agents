-- Step 3.3: Simplify stage_executions
-- Add stage_member_id (nullable FK) and pipeline_id (nullable convenience column)

ALTER TABLE stage_executions ADD COLUMN stage_member_id UUID REFERENCES pipeline_stage_members(id);
ALTER TABLE stage_executions ADD COLUMN pipeline_id UUID REFERENCES pipelines(id);

CREATE INDEX IF NOT EXISTS idx_stage_executions_member ON stage_executions(stage_member_id);
CREATE INDEX IF NOT EXISTS idx_stage_executions_pipeline ON stage_executions(pipeline_id);
