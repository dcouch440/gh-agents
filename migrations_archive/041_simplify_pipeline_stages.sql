-- Step 3.2: Simplify pipeline_stages
-- Make approval_required, fan_out, input_definitions, output_description, output_schema nullable

ALTER TABLE pipeline_stages ALTER COLUMN approval_required DROP NOT NULL;
ALTER TABLE pipeline_stages ALTER COLUMN fan_out DROP NOT NULL;
ALTER TABLE pipeline_stages ALTER COLUMN input_definitions DROP NOT NULL;
ALTER TABLE pipeline_stages ALTER COLUMN output_description DROP NOT NULL;
ALTER TABLE pipeline_stages ALTER COLUMN output_schema DROP NOT NULL;
