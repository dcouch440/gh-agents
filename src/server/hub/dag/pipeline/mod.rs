//! Pipeline step execution within the DAG.
//!
//! A pipeline step owns a child workflow (via `child_workflow_id`) whose
//! roster agents are executed as a unit. Lifecycle phases (before/after)
//! can be composed via the `Pipeline` builder:
//!
//! ```ignore
//! Pipeline::new()
//!     .before(DesignerPhase)
//!     .execute(dag, step, dag_state)
//!     .await
//! ```

mod agent_executor;
mod designer;
pub(crate) mod lifecycle;
mod output;
mod runner;
mod tests;
mod types;

// Re-exports for crate-wide access
pub(crate) use types::DesignedAgentPrompt;

// Re-exports for the file executor (file-based execution bridge)
pub(crate) use output::build_upstream_step_output;
pub(crate) use runner::{run_agent_execution, AgentExecutionInput};

// Re-exports for test access (tests.rs imports via crate path)
#[cfg(test)]
pub(crate) use output::{
    build_filtered_outputs_block, build_upstream_outputs_block, compose_workforce_output,
    compute_execution_levels, filter_outputs_for_agent,
};

use anyhow::anyhow;
use tracing::info;

use crate::db::WorkflowStepRow;
use crate::server::hub::error::HubError;
use crate::server::ws::events::WorkflowEventKind;

use super::dag_state::DagExecutionState;
use super::{
    broadcast_workflow_event, compose_prompt, resolve_step_port_inputs, step_display_name,
    DagContext, PromptRepos,
};

use designer::build_static_fallback_prompts;
pub(crate) use designer::DesignerPhase;
use lifecycle::{PhaseOutput, PhaseTokenUsage, PipelineExecutionContext, PipelinePhase};

/// Composable pipeline executor with lifecycle phases.
///
/// Phases registered via `.before()` run before agent execution.
/// If no before-phase produces designed prompts, static fallback
/// prompts are used automatically.
pub(crate) struct Pipeline {
    before_phases: Vec<Box<dyn PipelinePhase>>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            before_phases: vec![],
        }
    }

    pub fn before(mut self, phase: impl PipelinePhase + 'static) -> Self {
        self.before_phases.push(Box::new(phase));
        self
    }

    /// Execute the pipeline: run lifecycle phases, then agent levels.
    pub async fn execute(
        &self,
        dag: &DagContext<'_>,
        step: &WorkflowStepRow,
        dag_state: &mut DagExecutionState,
    ) -> Result<(), HubError> {
        let step_start = std::time::Instant::now();

        // 1. Broadcast step started
        broadcast_workflow_event(
            dag.state,
            dag.ctx,
            step.workflow_id,
            WorkflowEventKind::StepStarted {
                step_id: step.id,
                step_name: step_display_name(step),
                agent_id: None,
                execution_id: None,
            },
        );

        // 2. Load mission brief
        let brief = dag
            .state
            .repos()
            .workflows
            .get_mission_brief(step.id)
            .await
            .map_err(|e| HubError::Internal(anyhow!("failed to load mission brief: {}", e)))?
            .ok_or_else(|| {
                HubError::Internal(anyhow!("pipeline step {} has no mission brief", step.id))
            })?;

        // 3. Load agent roster (sorted by execution_order)
        let roster = dag
            .state
            .repos()
            .workflows
            .list_agent_roster(brief.id)
            .await
            .map_err(|e| HubError::Internal(anyhow!("failed to load agent roster: {}", e)))?;

        if roster.is_empty() {
            return Err(HubError::Internal(anyhow!(
                "pipeline step {} has empty agent roster",
                step.id
            )));
        }

        info!(
            step_id = %step.id,
            task = %brief.task_description,
            agents = roster.len(),
            failure_mode = %brief.failure_mode,
            "Starting pipeline execution"
        );

        // 4. Resolve port inputs
        let port_inputs =
            resolve_step_port_inputs(step, dag.port_meta, &dag_state.completed_envelopes);

        // 5. Compose base prompt
        let repos = PromptRepos {
            prompt_template_repo: Some(&*dag.state.repos().prompt_templates),
            doc_repo: Some(&*dag.state.repos().documents),
            workflow_repo: Some(&*dag.state.repos().workflows),
            agent_repo: &*dag.state.repos().agents,
        };
        let prompt = compose_prompt(
            step,
            &repos,
            &dag_state.var_outputs,
            &dag.ctx.prior_outputs,
            port_inputs.as_ref(),
        )
        .await;

        // 6. Build pipeline execution context
        let pipeline_ctx = PipelineExecutionContext {
            step: step.clone(),
            brief: brief.clone(),
            roster: roster.clone(),
            base_prompt: prompt.clone(),
        };

        // 7. Run before phases (or static fallback if none registered)
        let phase_output = if self.before_phases.is_empty() {
            let prompts = build_static_fallback_prompts(&brief, &roster, &prompt);
            PhaseOutput {
                designed_prompts: prompts,
                token_usage: PhaseTokenUsage {
                    input_tokens: 0,
                    output_tokens: 0,
                    cost_usd: 0.0,
                    run_id: None,
                },
            }
        } else {
            // Run each before-phase; last one's output is used for agent execution
            let mut output = None;
            for phase in &self.before_phases {
                info!(phase = phase.name(), step_id = %step.id, "Running pipeline phase");
                output = Some(phase.execute(dag, &pipeline_ctx).await?);
            }
            output.unwrap()
        };

        // 8–12. Delegate agent execution, output composition, and recording to the runner
        let upstream_step_output =
            output::build_upstream_step_output(dag, step, &dag_state.completed_envelopes);

        let input = runner::AgentExecutionInput {
            designed_prompts: phase_output.designed_prompts,
            failure_mode: brief.failure_mode.clone(),
            upstream_step_output,
            original_prompt: prompt,
            designer_run_id: phase_output.token_usage.run_id,
            phase_tokens_in: phase_output.token_usage.input_tokens,
            phase_tokens_out: phase_output.token_usage.output_tokens,
            phase_cost: phase_output.token_usage.cost_usd,
        };

        runner::run_agent_execution(dag, step, dag_state, input, step_start).await
    }
}
