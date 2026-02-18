-- ============================================================================
-- Migration 0046: Make agent_executions.agent_id Nullable
-- ============================================================================
-- Purpose: Allow agent_execution rows without a real agent reference.
--          Workforce roster entries are not agents in the agents table, but
--          their executions need execution_messages for conversation history.
-- ============================================================================

ALTER TABLE agent_executions ALTER COLUMN agent_id DROP NOT NULL;
