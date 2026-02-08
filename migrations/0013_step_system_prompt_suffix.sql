-- Add per-step system prompt suffix that appends to the agent's base system prompt
ALTER TABLE workflow_steps ADD COLUMN system_prompt_suffix TEXT;
