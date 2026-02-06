-- Step 3.1: Simplify agents table
-- Rename persona_name→name, persona_prompt→system_prompt
-- Make tier, persona_style, current_task, status, router_mode nullable with defaults

ALTER TABLE agents RENAME COLUMN persona_name TO name;
ALTER TABLE agents RENAME COLUMN persona_prompt TO system_prompt;

ALTER TABLE agents ALTER COLUMN tier DROP NOT NULL;
ALTER TABLE agents ALTER COLUMN tier SET DEFAULT 'worker';
ALTER TABLE agents ALTER COLUMN persona_style DROP NOT NULL;
ALTER TABLE agents ALTER COLUMN persona_style SET DEFAULT 'casual';
ALTER TABLE agents ALTER COLUMN status DROP NOT NULL;
ALTER TABLE agents ALTER COLUMN status SET DEFAULT 'idle';
ALTER TABLE agents ALTER COLUMN router_mode DROP NOT NULL;
ALTER TABLE agents ALTER COLUMN router_mode SET DEFAULT false;
