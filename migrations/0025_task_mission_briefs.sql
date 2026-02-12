-- Task Force archetype: mission briefs and agent roster
-- One brief per step (UNIQUE step_id), cascading to roster on delete.

CREATE TABLE task_mission_briefs (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    step_id uuid NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    task_description text NOT NULL DEFAULT '',
    available_capabilities text[] NOT NULL DEFAULT '{}',
    failure_mode text NOT NULL DEFAULT 'fail_fast',
    downstream_context text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE(step_id)
);

CREATE TABLE task_agent_roster (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    mission_brief_id uuid NOT NULL REFERENCES task_mission_briefs(id) ON DELETE CASCADE,
    name text NOT NULL,
    role_description text NOT NULL DEFAULT '',
    capabilities text[] NOT NULL DEFAULT '{}',
    execution_order integer NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT now()
);
