-- 0042: Bridge designer outputs to protocol executions.
--
-- Adds a direct FK from agent_designer_outputs to protocol_executions,
-- removing the need to join through agent_designer_runs for lookups.
-- This makes the protocol execution audit trail self-contained.

ALTER TABLE agent_designer_outputs
    ADD COLUMN protocol_execution_id UUID REFERENCES protocol_executions(id);

-- Backfill via the designer_run_id bridge:
-- protocol_executions.designer_run_id → agent_designer_runs.id = agent_designer_outputs.designer_run_id
UPDATE agent_designer_outputs ado
SET protocol_execution_id = pe.id
FROM protocol_executions pe
WHERE pe.designer_run_id = ado.designer_run_id;

CREATE INDEX idx_designer_outputs_protocol_exec
    ON agent_designer_outputs(protocol_execution_id)
    WHERE protocol_execution_id IS NOT NULL;
