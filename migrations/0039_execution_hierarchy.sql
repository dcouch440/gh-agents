-- 0039: Add root+depth for O(1) execution tree traversal.
--
-- Follows Temporal's pattern: root_execution_id enables single-query tree lookups,
-- depth enables level-filtered queries. Together they eliminate the need for
-- recursive CTEs in common sub-workflow paper trail queries.
--
-- Hierarchy: Collection Run -> Workflow Execution (depth 0) -> Sub-Workflow (depth 1) -> ...

ALTER TABLE workflow_executions
    ADD COLUMN root_execution_id UUID REFERENCES workflow_executions(id),
    ADD COLUMN depth INT NOT NULL DEFAULT 0;

-- Backfill: root executions (no parent) point to themselves at depth 0.
UPDATE workflow_executions
SET root_execution_id = id, depth = 0
WHERE parent_execution_id IS NULL;

-- Backfill: depth-1 children (direct sub-workflows).
-- Their root is the parent's root (or the parent itself if parent is root).
UPDATE workflow_executions child
SET root_execution_id = COALESCE(parent.root_execution_id, parent.id),
    depth = parent.depth + 1
FROM workflow_executions parent
WHERE child.parent_execution_id = parent.id
  AND child.root_execution_id IS NULL;

-- Backfill: depth-2+ children (nested sub-workflows, if any exist).
-- Repeat until no more unlinked children remain.
UPDATE workflow_executions child
SET root_execution_id = COALESCE(parent.root_execution_id, parent.id),
    depth = parent.depth + 1
FROM workflow_executions parent
WHERE child.parent_execution_id = parent.id
  AND child.root_execution_id IS NULL;

-- Index for "get all executions in this tree" queries.
CREATE INDEX idx_workflow_executions_root
    ON workflow_executions(root_execution_id)
    WHERE root_execution_id IS NOT NULL;

-- Composite index for depth-filtered tree queries.
CREATE INDEX idx_workflow_executions_depth
    ON workflow_executions(root_execution_id, depth);

-- Partial index for active executions (dashboard/monitoring hot path).
CREATE INDEX idx_workflow_executions_active
    ON workflow_executions(status, started_at DESC)
    WHERE status IN ('pending', 'running');
