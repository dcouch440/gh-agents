-- Workforce archetype: unifies documenter + task_force into a single model
-- where agents are sub-workflow steps in a child workflow.

-- 1a. Child workflow link (workforce step → live child workflow edited at design time)
ALTER TABLE workflow_steps
ADD COLUMN IF NOT EXISTS child_workflow_id UUID REFERENCES workflows(id) ON DELETE SET NULL;

-- 1b. Roster entry → visual step in child workflow
ALTER TABLE task_agent_roster
ADD COLUMN IF NOT EXISTS child_step_id UUID REFERENCES workflow_steps(id) ON DELETE SET NULL;

-- 1c. Deliverable → agent assignment
ALTER TABLE protocol_document_defs
ADD COLUMN IF NOT EXISTS agent_roster_entry_id UUID REFERENCES task_agent_roster(id) ON DELETE SET NULL;

-- 1d. Designer step marker
ALTER TABLE workflow_steps
ADD COLUMN IF NOT EXISTS is_designer_step BOOLEAN NOT NULL DEFAULT false;

-- 1e. Partial indexes for efficient lookups
CREATE INDEX IF NOT EXISTS idx_ws_child_workflow
ON workflow_steps (child_workflow_id)
WHERE child_workflow_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_tar_child_step
ON task_agent_roster (child_step_id)
WHERE child_step_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_pdd_agent_roster
ON protocol_document_defs (agent_roster_entry_id)
WHERE agent_roster_entry_id IS NOT NULL;
