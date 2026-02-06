-- Step 3.4: Simplify pipeline_runs
-- Make stage_outputs, total_input_tokens, total_output_tokens nullable

ALTER TABLE pipeline_runs ALTER COLUMN stage_outputs DROP NOT NULL;
ALTER TABLE pipeline_runs ALTER COLUMN total_input_tokens DROP NOT NULL;
ALTER TABLE pipeline_runs ALTER COLUMN total_output_tokens DROP NOT NULL;
