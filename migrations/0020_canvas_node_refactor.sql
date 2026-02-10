-- Canvas node refactor: rename entry → context, remove document steps
--
-- 1. Rename execution_mode 'entry' to 'context'
-- 2. Remove edges connected to 'document' steps
-- 3. Remove 'document' steps (no longer connectable nodes on canvas)

-- Step 1: Rename entry → context
UPDATE workflow_steps SET execution_mode = 'context' WHERE execution_mode = 'entry';

-- Step 2: Remove edges connected to document-mode steps
DELETE FROM workflow_step_edges
WHERE from_step_id IN (SELECT id FROM workflow_steps WHERE execution_mode = 'document')
   OR to_step_id IN (SELECT id FROM workflow_steps WHERE execution_mode = 'document');

-- Step 3: Remove document-mode steps
DELETE FROM workflow_steps WHERE execution_mode = 'document';
