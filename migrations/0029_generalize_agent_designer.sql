-- Generalize agent designer tables to support all archetypes (task_force, documenter, room).
-- Previously, these tables were task-force-specific with hard FK references.

-- Add archetype + phase columns to runs table, make mission_brief_id nullable.
ALTER TABLE agent_designer_runs
    ALTER COLUMN mission_brief_id DROP NOT NULL,
    ADD COLUMN archetype text NOT NULL DEFAULT 'task_force',
    ADD COLUMN phase text NOT NULL DEFAULT '';

-- Add generic source columns to outputs table, make agent_roster_entry_id nullable.
ALTER TABLE agent_designer_outputs
    ALTER COLUMN agent_roster_entry_id DROP NOT NULL,
    ADD COLUMN source_entity_id text NOT NULL DEFAULT '',
    ADD COLUMN source_archetype text NOT NULL DEFAULT 'task_force';
