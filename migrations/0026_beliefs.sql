-- Belief capture: extraction plans (design-time) + beliefs (runtime)

CREATE TABLE belief_extraction_plans (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    step_id uuid NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    extraction_focus text NOT NULL DEFAULT '',
    tag_vocabulary text[] NOT NULL DEFAULT '{}',
    contradiction_handling text NOT NULL DEFAULT 'flag',
    confidence_threshold text NOT NULL DEFAULT 'low',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE(step_id)
);

CREATE TABLE beliefs (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_id uuid NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    workflow_execution_id uuid NOT NULL,
    source_step_id uuid NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    source_document_title text,
    source_document_def_id uuid REFERENCES protocol_document_defs(id) ON DELETE SET NULL,
    source_phase text NOT NULL DEFAULT 'execution',
    content text NOT NULL,
    reasoning text NOT NULL,
    belief_type text NOT NULL DEFAULT 'fact',
    confidence text NOT NULL DEFAULT 'medium',
    confidence_justification text,
    semantic_tags text[] NOT NULL DEFAULT '{}',
    emotional_tone text,
    cross_source_tension text,
    source_step_name text NOT NULL,
    extraction_model text NOT NULL,
    extraction_tokens_in integer NOT NULL DEFAULT 0,
    extraction_tokens_out integer NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_beliefs_workflow ON beliefs(workflow_id);
CREATE INDEX idx_beliefs_workflow_execution ON beliefs(workflow_execution_id);
CREATE INDEX idx_beliefs_source_step ON beliefs(source_step_id);
CREATE INDEX idx_beliefs_semantic_tags ON beliefs USING gin(semantic_tags);
CREATE INDEX idx_beliefs_type ON beliefs(belief_type);
CREATE INDEX idx_beliefs_source_doc ON beliefs(source_document_title);
CREATE INDEX idx_beliefs_source_phase ON beliefs(source_phase);
