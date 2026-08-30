-- Give a pipeline agent's execution its own name.
--
-- A pipeline agent is designed per run and has no `agents` row, so the
-- timeline had no name to read off the execution. The agents on a step run in
-- parallel and their `protocol_executions` phase rows cannot be told apart by
-- step or by time, so the name has to live on `agent_executions` itself.
ALTER TABLE agent_executions ADD COLUMN agent_name TEXT;

-- Backfill where the task prompt names its own agent: "You are <name>, one of
-- N agents on this step." Peers appear as "  - <name> — ...", so the trailing
-- comma keeps the match to the agent this execution belongs to.
UPDATE agent_executions ae
SET agent_name = pe.agent_name
FROM protocol_executions pe
WHERE ae.agent_name IS NULL
  AND ae.execution_type = 'pipeline_agent'
  AND pe.protocol_step_id = ae.workflow_step_id
  AND pe.workflow_run_id = ae.workflow_execution_id
  AND pe.agent_name IS NOT NULL
  AND ae.input LIKE '%You are ' || pe.agent_name || ',%';

-- Then where the step ran exactly one named agent phase, which leaves nothing
-- to choose between. Executions outside both passes keep a NULL name and are
-- labelled by id.
UPDATE agent_executions ae
SET agent_name = pe.agent_name
FROM protocol_executions pe
WHERE ae.agent_name IS NULL
  AND ae.execution_type = 'pipeline_agent'
  AND pe.protocol_step_id = ae.workflow_step_id
  AND pe.workflow_run_id = ae.workflow_execution_id
  AND pe.phase LIKE 'agent_%'
  AND pe.agent_name IS NOT NULL
  AND (
    SELECT count(*)
    FROM protocol_executions p2
    WHERE p2.protocol_step_id = ae.workflow_step_id
      AND p2.workflow_run_id = ae.workflow_execution_id
      AND p2.phase LIKE 'agent_%'
      AND p2.agent_name IS NOT NULL
  ) = 1;
