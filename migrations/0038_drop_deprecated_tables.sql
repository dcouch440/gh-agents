-- 0038: Drop deprecated tables that have zero code references.
--
-- agent_modes was replaced by tool_router_modes (migration 064 archive era).
-- agent_executions.selected_mode_id FK must be dropped first since it references agent_modes.
--
-- All _backup tables are legacy leftovers with no code references.

-- Drop FK from agent_executions to the deprecated agent_modes table.
ALTER TABLE agent_executions DROP CONSTRAINT IF EXISTS agent_executions_selected_mode_id_fkey;

-- Drop deprecated agent mode tables (replaced by tool_router_modes).
DROP TABLE IF EXISTS agent_modes_versions;
DROP TABLE IF EXISTS agent_modes;

-- Drop legacy backup tables (no code references anywhere in src/).
DROP TABLE IF EXISTS pipelines_backup;
DROP TABLE IF EXISTS agent_executions_backup;
DROP TABLE IF EXISTS rooms_backup;
DROP TABLE IF EXISTS room_sessions_backup;
