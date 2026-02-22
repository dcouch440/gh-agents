-- Rename assistant_notes → step_plan
ALTER TABLE assistant_notes RENAME TO step_plan;
ALTER INDEX idx_assistant_notes_step_id RENAME TO idx_step_plan_step_id;
