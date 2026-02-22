-- 0054: Add execution_type discriminator to agent_executions.
--
-- Replaces implicit NULL-pattern detection with an explicit TEXT column.
-- Uses TEXT (not ENUM) per project convention.
--
-- Known limitation: existing dispatch vs manager_dispatch and
-- agent_designer vs pipeline_agent rows are indistinguishable.
-- All pre-existing rows are tagged with the older/default variant.

-- 1. Add column with default so existing rows get backfilled as 'dag_step'.
ALTER TABLE agent_executions
    ADD COLUMN IF NOT EXISTS execution_type TEXT NOT NULL DEFAULT 'dag_step';

-- 2. Backfill existing rows (most specific patterns first).

-- debate_verification: has parent execution AND has agent_id
UPDATE agent_executions
SET execution_type = 'debate_verification'
WHERE parent_agent_execution_id IS NOT NULL
  AND agent_id IS NOT NULL
  AND execution_type = 'dag_step';

-- interactive_review: is_interactive = true
UPDATE agent_executions
SET execution_type = 'interactive_review'
WHERE is_interactive = true
  AND execution_type = 'dag_step';

-- dispatch: agent_id IS NULL AND workflow_execution_id IS NULL
-- (cannot distinguish dispatch vs manager_dispatch retroactively)
UPDATE agent_executions
SET execution_type = 'dispatch'
WHERE agent_id IS NULL
  AND workflow_execution_id IS NULL
  AND execution_type = 'dag_step';

-- agent_designer / pipeline_agent: agent_id IS NULL, workflow_execution_id IS NOT NULL
-- (cannot distinguish retroactively; tag all as agent_designer)
UPDATE agent_executions
SET execution_type = 'agent_designer'
WHERE agent_id IS NULL
  AND workflow_execution_id IS NOT NULL
  AND execution_type = 'dag_step';

-- 3. Index on execution_type for filtered queries.
CREATE INDEX IF NOT EXISTS idx_agent_executions_type
    ON agent_executions(execution_type);
