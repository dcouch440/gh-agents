-- Sessions point to agents instead of hardcoded modes
ALTER TABLE chat_sessions ADD COLUMN agent_id UUID REFERENCES agents(id);

-- Dynamic agent modes (opt-in per agent, LLM-classified per turn)
CREATE TABLE agent_modes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    system_prompt_suffix TEXT,
    temperature_override DOUBLE PRECISION,
    model_override TEXT,
    tool_overrides TEXT[],
    classifier_hint TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (agent_id, name)
);
