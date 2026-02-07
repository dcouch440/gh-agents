-- Agent Guidances: distilled feedback that persists across restarts.
-- Each row holds a JSON array of suggestion strings for a given agent
-- (and optionally a specific workflow step).

CREATE TABLE public.agent_guidances (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id UUID NOT NULL REFERENCES public.agents(id) ON DELETE CASCADE,
    workflow_step_id UUID REFERENCES public.workflow_steps(id) ON DELETE SET NULL,
    suggestions JSONB NOT NULL DEFAULT '[]'::jsonb,
    source TEXT NOT NULL DEFAULT 'manual',
    version INT NOT NULL DEFAULT 1,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_agent_guidances_agent_id ON public.agent_guidances(agent_id);
CREATE INDEX idx_agent_guidances_lookup ON public.agent_guidances(agent_id, workflow_step_id, is_active);
