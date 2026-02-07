-- Add container execution configuration to workflows.
-- When container_enabled = true, workflow steps execute inside persistent
-- Docker containers with a cloned copy of the target repo.

ALTER TABLE public.workflows
    ADD COLUMN container_enabled BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN target_repo_url TEXT,
    ADD COLUMN target_branch TEXT;
