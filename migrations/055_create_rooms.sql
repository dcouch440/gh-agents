-- Room definitions (pipeline-scoped)
CREATE TABLE rooms (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    pipeline_id UUID NOT NULL REFERENCES pipelines(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    gatekeeper_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    gatekeeper_model_id TEXT NOT NULL DEFAULT 'claude-haiku-4-20250414',
    max_speakers_per_turn INTEGER NOT NULL DEFAULT 4,
    max_turns INTEGER NOT NULL DEFAULT 20,
    tools_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_rooms_user ON rooms(user_id);
CREATE INDEX idx_rooms_pipeline ON rooms(pipeline_id);

-- Room membership (join table)
CREATE TABLE room_members (
    room_id UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    display_name TEXT,
    role_description TEXT NOT NULL,
    display_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (room_id, agent_id)
);

CREATE INDEX idx_room_members_agent ON room_members(agent_id);
