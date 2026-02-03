-- Drop tier system columns
-- The agent tier hierarchy (Orchestrator/Worker/Utility) has been removed.
-- Agents are now configured individually with their own model configs.

-- Remove tier column from agents table
ALTER TABLE agents DROP COLUMN IF EXISTS tier;

-- Remove assigned_tier column from tasks table
ALTER TABLE tasks DROP COLUMN IF EXISTS assigned_tier;

-- Drop tier index if exists
DROP INDEX IF EXISTS idx_agents_tier;
