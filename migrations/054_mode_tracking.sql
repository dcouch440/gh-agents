ALTER TABLE agent_executions ADD COLUMN selected_mode_id UUID REFERENCES agent_modes(id);
