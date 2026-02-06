-- ============================================================================
-- Migration 067: Port-Based Workflow System
-- ============================================================================
-- Purpose: Replace variable system with explicit port-based data flow.
--          Add visual positioning for canvas UI.
--          Support label-based routing for for-each steps.
--
-- New Tables:
--   - step_outputs: What each step produces
--   - step_inputs: What each step expects
--   - step_routing_rules: Label → Agent mappings for routing
--
-- Extensions:
--   - workflow_steps: Add positioning (x, y, width, height), routing config
--   - workflow_step_edges: Add UUID PK, port connections, transforms
--
-- Drops:
--   - execution_variables: Replaced by port system
-- ============================================================================

-- ============================================================================
-- 1. CREATE NEW TABLES
-- ============================================================================

-- Output ports (what each step produces)
CREATE TABLE step_outputs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_step_id UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    port_name TEXT NOT NULL,
    port_type TEXT NOT NULL,  -- "string", "array", "object", "number", "boolean"
    json_path TEXT NOT NULL,  -- Path in envelope.data (e.g., "sections", "items[*].name")
    description TEXT,
    json_schema JSONB,  -- Optional validation schema
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(workflow_step_id, port_name)
);

CREATE INDEX idx_step_outputs_step ON step_outputs(workflow_step_id);

COMMENT ON TABLE step_outputs IS
    'Output port definitions for workflow steps. Defines what data each step produces and where to find it in the output envelope.';

-- Input ports (what each step expects)
CREATE TABLE step_inputs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_step_id UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    port_name TEXT NOT NULL,
    port_type TEXT NOT NULL,  -- "string", "array", "object", "number", "boolean"
    required BOOLEAN NOT NULL DEFAULT false,
    default_value JSONB,
    description TEXT,
    json_schema JSONB,  -- Optional validation schema
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(workflow_step_id, port_name)
);

CREATE INDEX idx_step_inputs_step ON step_inputs(workflow_step_id);

COMMENT ON TABLE step_inputs IS
    'Input port definitions for workflow steps. Defines what data each step requires to execute.';

-- Routing rules for label-based agent assignment
CREATE TABLE step_routing_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_step_id UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    label_value TEXT NOT NULL,  -- Category/label value (e.g., "frontend", "backend", "database", "testing")
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    display_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(workflow_step_id, label_value)
);

CREATE INDEX idx_step_routing_rules_step ON step_routing_rules(workflow_step_id);
CREATE INDEX idx_step_routing_rules_agent ON step_routing_rules(agent_id);

COMMENT ON TABLE step_routing_rules IS
    'Label-based routing configuration for for-each steps. Maps label/category values to specialist agents.';

-- ============================================================================
-- 2. EXTEND WORKFLOW_STEPS
-- ============================================================================

-- Add visual positioning and routing columns
ALTER TABLE workflow_steps
    ADD COLUMN position_x FLOAT,
    ADD COLUMN position_y FLOAT,
    ADD COLUMN width FLOAT DEFAULT 200,
    ADD COLUMN height FLOAT DEFAULT 100,
    ADD COLUMN routing_mode TEXT,  -- NULL (no routing), "label" (route by field), "cavernous" (document-based)
    ADD COLUMN routing_field TEXT,  -- For routing_mode="label": which field to read (e.g., "category")
    ADD COLUMN cavernous_config_document_id UUID REFERENCES documents(id);  -- For routing_mode="cavernous"

CREATE INDEX idx_workflow_steps_routing ON workflow_steps(routing_mode)
    WHERE routing_mode IS NOT NULL;

COMMENT ON COLUMN workflow_steps.position_x IS
    'X coordinate for visual canvas positioning (future UI)';
COMMENT ON COLUMN workflow_steps.position_y IS
    'Y coordinate for visual canvas positioning (future UI)';
COMMENT ON COLUMN workflow_steps.routing_mode IS
    'Routing strategy:
     - NULL: Use step agent_id directly (static agent)
     - "label": Route array items by label/category field to specialist agents (TIER 2)
     - "cavernous": Document-based dynamic routing with agent collaboration (TIER 3)';
COMMENT ON COLUMN workflow_steps.routing_field IS
    'For routing_mode="label": which field in array items contains the routing label/category';
COMMENT ON COLUMN workflow_steps.cavernous_config_document_id IS
    'For routing_mode="cavernous": document containing routing configuration JSON';

-- ============================================================================
-- 3. RESTRUCTURE WORKFLOW_STEP_EDGES
-- ============================================================================
-- WARNING: This changes the primary key structure!
-- Current PK: (from_step_id, to_step_id) composite
-- New PK: id (UUID)
-- Adds: Port connections, transforms, conditions

-- Step 1: Add new columns
ALTER TABLE workflow_step_edges
    ADD COLUMN id UUID DEFAULT gen_random_uuid(),
    ADD COLUMN from_output_port TEXT,  -- Which output port from source step
    ADD COLUMN to_input_port TEXT,     -- Which input port on target step
    ADD COLUMN transform_jsonpath TEXT, -- Optional JSONPath transformation (e.g., "$.items[*].name")
    ADD COLUMN condition_type TEXT,    -- NULL, "if_true", "if_false", "if_equals" (for conditional edges)
    ADD COLUMN condition_value JSONB,  -- Value for condition evaluation
    ADD COLUMN edge_label TEXT,        -- Visual label for canvas UI
    ADD COLUMN workflow_id UUID;       -- Will be populated from from_step_id

-- Step 2: Populate workflow_id from from_step_id
UPDATE workflow_step_edges
SET workflow_id = (
    SELECT workflow_id
    FROM workflow_steps
    WHERE workflow_steps.id = workflow_step_edges.from_step_id
);

-- Step 3: Make workflow_id NOT NULL (after population)
ALTER TABLE workflow_step_edges
    ALTER COLUMN workflow_id SET NOT NULL;

-- Step 4: Add foreign key constraint
ALTER TABLE workflow_step_edges
    ADD CONSTRAINT workflow_step_edges_workflow_id_fkey
    FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE;

-- Step 5: Drop old primary key, add new one
ALTER TABLE workflow_step_edges DROP CONSTRAINT workflow_step_edges_pkey;
ALTER TABLE workflow_step_edges ADD CONSTRAINT workflow_step_edges_pkey PRIMARY KEY (id);

-- Step 6: Add unique constraint on (workflow_id, from_step_id, to_step_id)
-- This ensures one edge per step pair within a workflow
ALTER TABLE workflow_step_edges
    ADD CONSTRAINT workflow_step_edges_workflow_from_to_unique
    UNIQUE(workflow_id, from_step_id, to_step_id);

-- Step 7: Add indexes for port lookups
CREATE INDEX idx_workflow_step_edges_ports ON workflow_step_edges(from_output_port, to_input_port);
CREATE INDEX idx_workflow_step_edges_workflow ON workflow_step_edges(workflow_id);

COMMENT ON TABLE workflow_step_edges IS
    'DAG edges connecting workflow steps. Now supports port-based connections with optional transformations.';

COMMENT ON COLUMN workflow_step_edges.from_output_port IS
    'Source step output port name. System automatically reads from envelope.data.<from_output_port>';
COMMENT ON COLUMN workflow_step_edges.to_input_port IS
    'Target step input port name. Mapped data becomes available as input.<to_input_port>';
COMMENT ON COLUMN workflow_step_edges.transform_jsonpath IS
    'Optional JSONPath transformation applied to data flowing through edge (e.g., "$.items[*].name" to extract names)';

-- ============================================================================
-- 4. DROP EXECUTION_VARIABLES
-- ============================================================================
-- This table is being replaced by the port-based system
-- User confirmed: no production data to preserve

DROP TABLE IF EXISTS execution_variables CASCADE;

-- ============================================================================
-- 5. UPDATE TABLE COMMENTS
-- ============================================================================

COMMENT ON TABLE workflow_steps IS
    'Workflow DAG nodes. Note: output_variable_name column is deprecated - use step_outputs table for port definitions.';

COMMENT ON COLUMN agent_executions.structured_output IS
    'Standard output envelope: {status, data, metadata, error}.
     For execution_mode="for_each", data is an array of iteration envelopes.
     For single execution, data contains the actual output.';
