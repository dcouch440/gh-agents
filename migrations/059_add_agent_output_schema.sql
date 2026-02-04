-- Add output_schema_id column to agents table
ALTER TABLE agents
ADD COLUMN output_schema_id UUID REFERENCES output_schemas(id) ON DELETE SET NULL;

-- Add index for lookups
CREATE INDEX idx_agents_output_schema ON agents(output_schema_id);
