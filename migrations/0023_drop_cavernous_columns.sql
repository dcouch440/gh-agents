-- Drop cavernous routing columns and indexes from agent_executions.
-- These supported the CavernousStepStrategy which has been removed.

-- Drop indexes
DROP INDEX IF EXISTS idx_agent_executions_routing_analysis;
DROP INDEX IF EXISTS idx_agent_executions_routing_doc;

-- Drop FK constraint
ALTER TABLE agent_executions DROP CONSTRAINT IF EXISTS agent_executions_selected_routing_document_id_fkey;

-- Drop columns
ALTER TABLE agent_executions DROP COLUMN IF EXISTS routing_analysis;
ALTER TABLE agent_executions DROP COLUMN IF EXISTS selected_routing_document_id;
