-- Add trace JSONB column to agent_executions for dispatch trace persistence.
-- Dispatch executors serialize their streaming trace (tokens, tool calls, errors)
-- into this column on completion so the frontend can hydrate on refresh.
ALTER TABLE agent_executions ADD COLUMN IF NOT EXISTS trace JSONB;
