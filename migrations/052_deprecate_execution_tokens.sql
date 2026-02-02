-- Token/cost data now lives exclusively in token_ledger.
-- These columns on agent_executions are deprecated and no longer written to.
-- They remain for backward compatibility with existing rows.
COMMENT ON COLUMN agent_executions.input_tokens IS 'DEPRECATED: use token_ledger for token counts';
COMMENT ON COLUMN agent_executions.output_tokens IS 'DEPRECATED: use token_ledger for token counts';
COMMENT ON COLUMN agent_executions.cost_usd IS 'DEPRECATED: use token_ledger for cost data';
