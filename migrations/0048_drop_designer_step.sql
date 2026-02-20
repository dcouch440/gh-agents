-- Remove Designer step rows from child workflows.
-- These phantom marker steps are replaced by the DesignerPhase
-- pipeline lifecycle middleware.

-- Clean edges touching designer steps first (FK safety)
DELETE FROM workflow_step_edges
WHERE from_step_id IN (SELECT id FROM workflow_steps WHERE is_designer_step = true)
   OR to_step_id IN (SELECT id FROM workflow_steps WHERE is_designer_step = true);

-- Remove the phantom designer step rows
DELETE FROM workflow_steps WHERE is_designer_step = true;

-- Drop the column
ALTER TABLE workflow_steps DROP COLUMN IF EXISTS is_designer_step;
