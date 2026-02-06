-- Add tool-to-cluster routing and builtin flag
ALTER TABLE tools ADD COLUMN cluster_id UUID REFERENCES clusters(id) ON DELETE SET NULL;
ALTER TABLE tools ADD COLUMN is_builtin BOOLEAN NOT NULL DEFAULT false;
CREATE INDEX idx_tools_cluster ON tools(cluster_id);
