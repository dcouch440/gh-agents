-- Context Intelligence: pinned nodes + run results summarizer
ALTER TABLE workflow_steps
  ADD COLUMN pinned BOOLEAN NOT NULL DEFAULT false,
  ADD COLUMN run_results_summary TEXT NOT NULL DEFAULT '';
