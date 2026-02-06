-- Version tracking for all user-editable entities.
-- Each entity gets a version counter + a _versions history table.

-- ── Agents ──────────────────────────────────────────────────────────────────
ALTER TABLE agents ADD COLUMN version INT NOT NULL DEFAULT 1;

CREATE TABLE agents_versions (
    id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    version INT NOT NULL,
    tier TEXT,
    name TEXT NOT NULL,
    system_prompt TEXT NOT NULL,
    persona_style TEXT,
    model_provider TEXT NOT NULL,
    model_id TEXT NOT NULL,
    model_max_tokens INT NOT NULL,
    model_temperature REAL NOT NULL,
    status TEXT,
    router_mode BOOLEAN,
    changed_by UUID,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (id, version)
);

-- ── Agent Modes ─────────────────────────────────────────────────────────────
ALTER TABLE agent_modes ADD COLUMN version INT NOT NULL DEFAULT 1;

CREATE TABLE agent_modes_versions (
    id UUID NOT NULL REFERENCES agent_modes(id) ON DELETE CASCADE,
    version INT NOT NULL,
    agent_id UUID NOT NULL,
    name TEXT NOT NULL,
    system_prompt_suffix TEXT,
    temperature_override DOUBLE PRECISION,
    model_override TEXT,
    tool_overrides TEXT[],
    classifier_hint TEXT NOT NULL,
    changed_by UUID,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (id, version)
);

-- ── Tools ───────────────────────────────────────────────────────────────────
ALTER TABLE tools ADD COLUMN version INT NOT NULL DEFAULT 1;

CREATE TABLE tools_versions (
    id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
    version INT NOT NULL,
    name TEXT NOT NULL,
    display_name TEXT NOT NULL,
    description TEXT NOT NULL,
    parameters JSONB NOT NULL,
    changed_by UUID,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (id, version)
);

-- ── Workflows ───────────────────────────────────────────────────────────────
ALTER TABLE workflows ADD COLUMN version INT NOT NULL DEFAULT 1;

CREATE TABLE workflows_versions (
    id UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    version INT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    changed_by UUID,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (id, version)
);

-- ── Workflow Steps ──────────────────────────────────────────────────────────
ALTER TABLE workflow_steps ADD COLUMN version INT NOT NULL DEFAULT 1;

CREATE TABLE workflow_steps_versions (
    id UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    version INT NOT NULL,
    workflow_id UUID NOT NULL,
    agent_id UUID NOT NULL,
    execution_mode TEXT NOT NULL,
    for_each_ref TEXT,
    prompt_template_id UUID,
    prompt_template TEXT NOT NULL,
    output_schema_id UUID,
    output_variable_name TEXT,
    interactive_agent_id UUID,
    for_each_label_field TEXT,
    display_order INT NOT NULL,
    changed_by UUID,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (id, version)
);

-- ── Output Schemas ──────────────────────────────────────────────────────────
ALTER TABLE output_schemas ADD COLUMN version INT NOT NULL DEFAULT 1;

CREATE TABLE output_schemas_versions (
    id UUID NOT NULL REFERENCES output_schemas(id) ON DELETE CASCADE,
    version INT NOT NULL,
    name TEXT NOT NULL,
    schema JSONB NOT NULL,
    changed_by UUID,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (id, version)
);

-- ── Prompt Templates ────────────────────────────────────────────────────────
ALTER TABLE prompt_templates ADD COLUMN version INT NOT NULL DEFAULT 1;

CREATE TABLE prompt_templates_versions (
    id UUID NOT NULL REFERENCES prompt_templates(id) ON DELETE CASCADE,
    version INT NOT NULL,
    name TEXT NOT NULL,
    content TEXT NOT NULL,
    changed_by UUID,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (id, version)
);
