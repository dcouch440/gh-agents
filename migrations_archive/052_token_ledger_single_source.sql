-- Make token_ledger the single source of truth for cost/token data.
-- Drop denormalized columns from agent_executions.
-- Allow NULL agent_execution_id on token_ledger for chat turns (no execution record).

ALTER TABLE token_ledger ALTER COLUMN agent_execution_id DROP NOT NULL;

ALTER TABLE agent_executions DROP COLUMN IF EXISTS input_tokens;
ALTER TABLE agent_executions DROP COLUMN IF EXISTS output_tokens;
ALTER TABLE agent_executions DROP COLUMN IF EXISTS cost_usd;
