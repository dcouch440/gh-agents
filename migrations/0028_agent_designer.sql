-- Agent Designer pre-lifecycle runs.
-- One run per task force step execution, stores the LLM call metadata.
CREATE TABLE agent_designer_runs (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_execution_id uuid NOT NULL,
    stage_execution_id uuid NOT NULL,
    step_id uuid NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    mission_brief_id uuid NOT NULL REFERENCES task_mission_briefs(id) ON DELETE CASCADE,
    model_id text NOT NULL,
    input_tokens bigint NOT NULL DEFAULT 0,
    output_tokens bigint NOT NULL DEFAULT 0,
    cost_usd real NOT NULL DEFAULT 0.0,
    created_at timestamptz NOT NULL DEFAULT now()
);

-- Generated prompt pairs + tool assignments, one per agent in the roster.
CREATE TABLE agent_designer_outputs (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    designer_run_id uuid NOT NULL REFERENCES agent_designer_runs(id) ON DELETE CASCADE,
    agent_roster_entry_id uuid NOT NULL REFERENCES task_agent_roster(id) ON DELETE CASCADE,
    agent_name text NOT NULL,
    assigned_tools text[] NOT NULL DEFAULT '{}',
    generated_system_prompt text NOT NULL,
    generated_task_prompt text NOT NULL,
    design_reasoning text NOT NULL DEFAULT '',
    execution_order integer NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_designer_runs_step ON agent_designer_runs(step_id);
CREATE INDEX idx_designer_runs_execution ON agent_designer_runs(workflow_execution_id);
CREATE INDEX idx_designer_outputs_run ON agent_designer_outputs(designer_run_id);
