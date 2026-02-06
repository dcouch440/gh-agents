-- Add router mode flag to agents
ALTER TABLE agents ADD COLUMN router_mode BOOLEAN NOT NULL DEFAULT false;
