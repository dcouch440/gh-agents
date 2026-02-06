-- ============================================================================
-- Migration 069: Cavernous Routing - Document-Based Dynamic Execution
-- ============================================================================
-- Purpose: Add database fields for document-based cavernous routing.
--          Store routing analysis and selected routing config references.
--
-- Extensions:
--   - agent_executions: Add routing analysis and selected routing document tracking
-- ============================================================================

-- ============================================================================
-- 1. EXTEND AGENT_EXECUTIONS
-- ============================================================================

-- Add routing analysis and selected document tracking
ALTER TABLE agent_executions
    ADD COLUMN routing_analysis JSONB,  -- Document search + selection reasoning
    ADD COLUMN selected_routing_document_id UUID REFERENCES documents(id);  -- Which routing config was selected

-- Create indexes
CREATE INDEX idx_agent_executions_routing_doc ON agent_executions(selected_routing_document_id)
    WHERE selected_routing_document_id IS NOT NULL;

CREATE INDEX idx_agent_executions_routing_analysis ON agent_executions
    USING gin(routing_analysis) WHERE routing_analysis IS NOT NULL;

-- ============================================================================
-- 2. ADD COMMENTS
-- ============================================================================

COMMENT ON COLUMN agent_executions.routing_analysis IS
    'For cavernous routing executions: Document search results and selection reasoning.
     Format: {
       "search_query": "...",
       "documents_found": [{"id": "uuid", "title": "routing:...", "score": 0.95}],
       "selected_document_id": "uuid",
       "reasoning": "Selected because...",
       "collaborative_selection": false
     }';

COMMENT ON COLUMN agent_executions.selected_routing_document_id IS
    'For cavernous routing: Reference to the routing config document that was selected and applied for this execution';

-- Update workflow_steps execution_mode comment with cavernous routing documentation
COMMENT ON COLUMN workflow_steps.execution_mode IS
    'Execution strategy:
     - "single": Execute once with step agent (TIER 1 - Static)
     - "for_each": Iterate over array, with optional label routing (TIER 2 - Label-based)
     - "cavernous": Document-based dynamic routing with agent collaboration (TIER 3 - Cavernous)
     - "room": Multi-agent room discussion';
