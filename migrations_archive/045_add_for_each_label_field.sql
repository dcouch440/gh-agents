-- Add for_each_label_field to workflow_steps
-- Lets the user pick which field from the for_each array element to display as a label in the execution tree.
ALTER TABLE workflow_steps ADD COLUMN IF NOT EXISTS for_each_label_field TEXT;
