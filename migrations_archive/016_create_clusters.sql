-- Clusters table for agent grouping
CREATE TABLE IF NOT EXISTS clusters (
    id UUID PRIMARY KEY NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    conventions TEXT NOT NULL DEFAULT '',
    shared_files JSONB NOT NULL DEFAULT '[]'::JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_clusters_user_id ON clusters(user_id);

-- Cluster membership junction table
CREATE TABLE IF NOT EXISTS cluster_members (
    cluster_id UUID NOT NULL REFERENCES clusters(id) ON DELETE CASCADE,
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (cluster_id, agent_id)
);
