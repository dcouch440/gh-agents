-- Board context: per-node Haiku-distilled awareness of the workflow board.
-- board_context_cache holds the cached Haiku summary for this node's perspective.
-- goal_summary holds the distilled conversational intent for this node.
-- Timestamps track freshness; NULL board_context_updated_at = stale/never rendered.

ALTER TABLE workflow_steps
    ADD COLUMN board_context_cache text NOT NULL DEFAULT '',
    ADD COLUMN board_context_updated_at timestamptz,
    ADD COLUMN goal_summary text NOT NULL DEFAULT '',
    ADD COLUMN goal_summary_updated_at timestamptz;
