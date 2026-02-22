-- Add stable readable ref_id to workflow_steps (e.g. "workforce-1", "context-2")
-- Used by L2 manager builder to reference nodes without relying on mutable names.

ALTER TABLE public.workflow_steps ADD COLUMN IF NOT EXISTS ref_id TEXT;

-- Backfill existing steps: assign "{execution_mode}-N" based on creation order
UPDATE workflow_steps SET ref_id = sub.ref_id
FROM (
  SELECT id,
    execution_mode || '-' || ROW_NUMBER() OVER (
      PARTITION BY workflow_id, execution_mode ORDER BY display_order, id
    ) AS ref_id
  FROM workflow_steps
  WHERE ref_id IS NULL
) sub
WHERE workflow_steps.id = sub.id;
