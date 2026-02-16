-- Generalize protocol_executions for all protocol types (task_force, documenter, etc.)
-- Previously limited to documenter phases ('strategy', 'research', 'write').

-- Drop the documenter-specific phase constraint so all protocols can use this table
ALTER TABLE protocol_executions DROP CONSTRAINT IF EXISTS protocol_executions_phase_check;

-- Add agent_name column for task_force/room agent tracking
ALTER TABLE protocol_executions ADD COLUMN IF NOT EXISTS agent_name TEXT;

-- Add archetype column to know which protocol type this phase belongs to
ALTER TABLE protocol_executions ADD COLUMN IF NOT EXISTS archetype TEXT;

-- Add designer_run_id to link agent phases back to the designer that created them
ALTER TABLE protocol_executions ADD COLUMN IF NOT EXISTS designer_run_id UUID REFERENCES agent_designer_runs(id);
