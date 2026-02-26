-- Fix steps created before board executor defaulted to 'workforce'.
-- Steps with a child_workflow_id (pipeline attached) but execution_mode='single'
-- should be 'workforce' so the Designer phase and pipeline execution run.
UPDATE workflow_steps
SET execution_mode = 'workforce'
WHERE child_workflow_id IS NOT NULL
  AND execution_mode = 'single';
