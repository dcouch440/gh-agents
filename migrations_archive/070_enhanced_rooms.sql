-- ============================================================================
-- Migration 070: Enhanced Rooms - Structured Agent Collaboration
-- ============================================================================
-- Purpose: Enable structured agent-to-agent data passing in rooms.
--          Support output schemas, gatekeeper-aware collaboration, and
--          room state accumulation.
--
-- New Tables:
--   - room_execution_outputs: Structured outputs from room speakers
--
-- Extensions:
--   - room_members: Add input/output schema ports
--   - room_sessions: Add structured state accumulation
--   - rooms: Add output configuration
-- ============================================================================

-- ============================================================================
-- 1. CREATE NEW TABLES
-- ============================================================================

-- Room execution outputs (structured data passed between speakers)
CREATE TABLE room_execution_outputs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_session_id UUID NOT NULL REFERENCES room_sessions(id) ON DELETE CASCADE,
    agent_execution_id UUID NOT NULL REFERENCES agent_executions(id) ON DELETE CASCADE,
    agent_id UUID NOT NULL REFERENCES agents(id),
    speaker_order INTEGER NOT NULL,
    turn_number INTEGER NOT NULL,
    output_name TEXT NOT NULL,  -- Semantic output name (e.g., "analysis", "implementation_plan", "code_review")
    structured_output JSONB NOT NULL,
    raw_output TEXT NOT NULL,
    schema_id UUID REFERENCES output_schemas(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(room_session_id, turn_number, output_name)
);

CREATE INDEX idx_room_outputs_session ON room_execution_outputs(room_session_id, turn_number);
CREATE INDEX idx_room_outputs_agent ON room_execution_outputs(agent_id);
CREATE INDEX idx_room_outputs_schema ON room_execution_outputs(schema_id)
    WHERE schema_id IS NOT NULL;

COMMENT ON TABLE room_execution_outputs IS
    'Structured outputs from room members for agent-to-agent data passing.
     Next speakers receive previous agents structured data, not just text transcripts.';

-- ============================================================================
-- 2. EXTEND ROOM_MEMBERS
-- ============================================================================

-- Add port configuration for input/output schemas
ALTER TABLE room_members
    ADD COLUMN input_schema_id UUID REFERENCES output_schemas(id),
    ADD COLUMN output_schema_id UUID REFERENCES output_schemas(id),
    ADD COLUMN output_name TEXT;  -- What this agent's output should be called

CREATE INDEX idx_room_members_input_schema ON room_members(input_schema_id)
    WHERE input_schema_id IS NOT NULL;
CREATE INDEX idx_room_members_output_schema ON room_members(output_schema_id)
    WHERE output_schema_id IS NOT NULL;

COMMENT ON COLUMN room_members.input_schema_id IS
    'Optional: Schema of structured inputs this agent can consume. Gatekeeper uses this for informed speaker selection.';
COMMENT ON COLUMN room_members.output_schema_id IS
    'Optional: Schema this agent produces. System validates output against this schema if present.';
COMMENT ON COLUMN room_members.output_name IS
    'Semantic name for this agent''s output (e.g., "requirements_analysis", "architecture_plan"). Other agents reference this.';

-- ============================================================================
-- 3. EXTEND ROOM_SESSIONS
-- ============================================================================

-- Add structured state accumulation
ALTER TABLE room_sessions
    ADD COLUMN structured_outputs JSONB,  -- Aggregated outputs: {output_name: {data, agent_id, turn}}
    ADD COLUMN final_decision JSONB;  -- Final synthesized room output

CREATE INDEX idx_room_sessions_outputs ON room_sessions
    USING gin(structured_outputs) WHERE structured_outputs IS NOT NULL;

COMMENT ON COLUMN room_sessions.structured_outputs IS
    'Accumulated structured outputs from all speakers. Format: {
       "requirements": {"data": {...}, "agent_id": "...", "turn": 1},
       "architecture": {"data": {...}, "agent_id": "...", "turn": 2}
     }';
COMMENT ON COLUMN room_sessions.final_decision IS
    'Final aggregated output from the room session, determined by room.aggregation_mode';

-- ============================================================================
-- 4. EXTEND ROOMS
-- ============================================================================

-- Add output configuration
ALTER TABLE rooms
    ADD COLUMN default_output_schema_id UUID REFERENCES output_schemas(id),
    ADD COLUMN aggregation_mode TEXT DEFAULT 'final_speaker';  -- "final_speaker", "consensus", "all_outputs"

CREATE INDEX idx_rooms_output_schema ON rooms(default_output_schema_id)
    WHERE default_output_schema_id IS NOT NULL;

COMMENT ON COLUMN rooms.aggregation_mode IS
    'How to aggregate room outputs into final result:
     - "final_speaker": Use last speaker''s output
     - "consensus": Synthesize consensus from all speakers
     - "all_outputs": Return array of all speaker outputs';
COMMENT ON COLUMN rooms.default_output_schema_id IS
    'Default output schema for room members (can be overridden per member)';
