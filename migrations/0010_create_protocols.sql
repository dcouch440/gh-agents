-- Protocol Layer: reusable, user-configurable execution recipes
-- that expand into workflow primitives (steps, edges, ports, schemas).

-- protocols: top-level protocol definitions
CREATE TABLE public.protocols (
    id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     uuid NOT NULL REFERENCES public.users(id) ON DELETE CASCADE,
    name        text NOT NULL,
    description text NOT NULL DEFAULT '',
    protocol_type text NOT NULL,  -- 'decomp', 'transform', 'review', 'route'
    config      jsonb NOT NULL DEFAULT '{}'::jsonb,
    version     integer NOT NULL DEFAULT 1,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT protocols_type_check CHECK (
        protocol_type IN ('decomp', 'transform', 'review', 'route')
    )
);

CREATE INDEX idx_protocols_user_id ON public.protocols(user_id);
CREATE INDEX idx_protocols_type ON public.protocols(protocol_type);

-- protocol_ports: agent slots within a protocol
CREATE TABLE public.protocol_ports (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    protocol_id   uuid NOT NULL REFERENCES public.protocols(id) ON DELETE CASCADE,
    port_name     text NOT NULL,
    description   text NOT NULL DEFAULT '',
    agent_id      uuid NOT NULL REFERENCES public.agents(id) ON DELETE CASCADE,
    display_order integer NOT NULL DEFAULT 0,
    CONSTRAINT protocol_ports_unique_name UNIQUE (protocol_id, port_name)
);

CREATE INDEX idx_protocol_ports_protocol_id ON public.protocol_ports(protocol_id);

-- workflow_step_protocols: links a protocol to a workflow step (the anchor/orchestrator step)
CREATE TABLE public.workflow_step_protocols (
    id                  uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_step_id    uuid NOT NULL REFERENCES public.workflow_steps(id) ON DELETE CASCADE,
    protocol_id         uuid NOT NULL REFERENCES public.protocols(id) ON DELETE CASCADE,
    applied_expansion   jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at          timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT wsp_unique_step UNIQUE (workflow_step_id)
);

CREATE INDEX idx_wsp_protocol_id ON public.workflow_step_protocols(protocol_id);
