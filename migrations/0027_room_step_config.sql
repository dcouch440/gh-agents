-- Room step design-time configuration tables.
-- room_step_configs stores room-level settings (purpose, turns, interaction mode).
-- room_step_members stores member blueprints (name, role, perspective).
-- These are design-time only; runtime materializes them into real rooms + agents.

CREATE TABLE room_step_configs (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    step_id uuid NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    meeting_purpose text NOT NULL DEFAULT '',
    max_turns integer NOT NULL DEFAULT 20,
    interaction_mode text NOT NULL DEFAULT 'moderated',
    gatekeeper_enabled boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE(step_id)
);

CREATE TABLE room_step_members (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    step_id uuid NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    name text NOT NULL,
    role text NOT NULL,
    perspective text NOT NULL DEFAULT '',
    display_order integer NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_room_step_members_step ON room_step_members(step_id);
