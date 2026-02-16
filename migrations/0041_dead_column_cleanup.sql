-- 0041: Remove orphaned columns from agent_executions.
--
-- selected_mode_id was an FK to agent_modes (dropped in 0038).
-- Its replacement is selected_router_mode_id (FK to tool_router_modes).
-- The FK constraint was already dropped in 0038; this removes the column itself.

ALTER TABLE agent_executions DROP COLUMN IF EXISTS selected_mode_id;
