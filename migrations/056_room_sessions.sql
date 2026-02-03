-- Room sessions (runtime — one per active room conversation)
CREATE TABLE room_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_id UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    run_id UUID REFERENCES pipeline_runs(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'active',
    current_turn INTEGER NOT NULL DEFAULT 0,
    transcript_summary TEXT,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX idx_room_sessions_room ON room_sessions(room_id);
CREATE INDEX idx_room_sessions_run ON room_sessions(run_id) WHERE run_id IS NOT NULL;
CREATE INDEX idx_room_sessions_status ON room_sessions(status);

-- Link agent_executions to room sessions
ALTER TABLE agent_executions ADD COLUMN room_session_id UUID REFERENCES room_sessions(id);
ALTER TABLE agent_executions ADD COLUMN speaker_order INTEGER;

CREATE INDEX idx_agent_executions_room ON agent_executions(room_session_id)
    WHERE room_session_id IS NOT NULL;

-- Link workflow_steps to rooms for DAG integration
ALTER TABLE workflow_steps ADD COLUMN room_id UUID REFERENCES rooms(id);
