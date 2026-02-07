-- Add reasoning_trace toggle to workflow steps.
-- When enabled (and an output schema is set), the LLM is prompted to include
-- chain-of-thought reasoning before its answer. The reasoning is stripped
-- before passing output downstream.

ALTER TABLE public.workflow_steps
    ADD COLUMN IF NOT EXISTS reasoning_trace BOOLEAN NOT NULL DEFAULT false;
