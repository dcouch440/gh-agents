-- Canvas element maps: bridge Excalidraw element IDs to workflow step/edge UUIDs.
-- One row per canvas element. Exactly one of step_id or edge_id is populated.
-- Used by the Phase 0 structural executor to resolve element references across submits.

CREATE TABLE canvas_element_maps (
    workflow_id  UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    element_id   TEXT NOT NULL,
    step_id      UUID REFERENCES workflow_steps(id) ON DELETE CASCADE,
    edge_id      UUID REFERENCES workflow_step_edges(id) ON DELETE CASCADE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (workflow_id, element_id),
    CONSTRAINT exactly_one_target CHECK (
        (step_id IS NOT NULL AND edge_id IS NULL) OR
        (step_id IS NULL AND edge_id IS NOT NULL)
    )
);

-- Partial indexes for reverse lookups (step → element, edge → element).
CREATE INDEX idx_canvas_element_maps_step ON canvas_element_maps(step_id) WHERE step_id IS NOT NULL;
CREATE INDEX idx_canvas_element_maps_edge ON canvas_element_maps(edge_id) WHERE edge_id IS NOT NULL;
