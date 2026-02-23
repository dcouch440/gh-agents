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
mod tests;
mod types;

// Re-export used by the orchestrator and workshop dispatch
pub(crate) use output::compose_workforce_output;

// Re-exports for test access (tests.rs imports via crate path)
#[cfg(test)]
pub(crate) use output::{
    build_filtered_outputs_block, build_team_roster_string, compute_execution_levels,
    filter_outputs_for_agent,
};
#[cfg(test)]
pub(crate) use types::DesignedAgentPrompt;

use std::collections::{HashMap, HashSet};

use anyhow::anyhow;
use tracing::info;
use uuid::Uuid;

use crate::config::protocols::WORKFORCE;
use crate::db::WorkflowStepRow;
use crate::server::hub::error::HubError;
use crate::server::ws::events::WorkflowEventKind;
use crate::types::{ExecutionMetadata, ExecutionStatus, StepExecutionEnvelope};

use super::container::{create_optional_container, destroy_optional_container};
use super::dag_state::DagExecutionState;
use super::utils::collect_upstream_context_data;
use super::{
    broadcast_workflow_event, compose_prompt, resolve_output_key, resolve_step_port_inputs,
    step_display_name, DagContext, PromptRepos, StepOutput,
};

use agent_executor::execute_agent_levels;
pub(crate) use designer::DesignerPhase;
use designer::{build_static_fallback_prompts, build_user_notes_block};
use lifecycle::{PhaseOutput, PhaseTokenUsage, PipelineExecutionContext, PipelinePhase};
use types::WorkforceStepEnv;

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

        // 5. Collect upstream context from context nodes
        let incoming = dag
            .port_meta
            .incoming_edges
            .get(&step.id)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        let upstream_context =
            collect_upstream_context_data(incoming, dag.steps, &dag_state.completed_envelopes);

        // 6. Compose base prompt
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

        // 7. Build pipeline execution context
        // Only include envelopes from actual upstream steps (those with edges into this step),
        // not the full DAG state which may include this step's own prior output from workshop reruns.
        let upstream_step_ids: HashSet<Uuid> = incoming
            .iter()
            .map(|e| e.from_step_id)
            .collect();
        let upstream_envelopes: HashMap<Uuid, StepExecutionEnvelope> = dag_state
            .completed_envelopes
            .iter()
            .filter(|(id, _)| upstream_step_ids.contains(id))
            .map(|(id, env)| (*id, env.clone()))
            .collect();

        let pipeline_ctx = PipelineExecutionContext {
            step: step.clone(),
            brief: brief.clone(),
            roster: roster.clone(),
            base_prompt: prompt.clone(),
            upstream_context: upstream_context.clone(),
            completed_envelopes: upstream_envelopes,
        };

        // 8. Run before phases (or static fallback if none registered)
        let phase_output = if self.before_phases.is_empty() {
            let prompts = build_static_fallback_prompts(&brief, &roster, &prompt);
            let user_notes_block = build_user_notes_block(&upstream_context);
            PhaseOutput {
                designed_prompts: prompts,
                user_notes_block,
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

        // 9. Create optional container
        let managed_container = create_optional_container(
            dag.ctx.container_config.as_ref(),
            dag.ctx.wg_client.as_deref(),
            "pipeline",
        )
        .await?;

        // 10. Build env + execute agent levels
        let env = WorkforceStepEnv {
            state: dag.state.clone(),
            ctx: dag.ctx.clone(),
            user_notes_block: phase_output.user_notes_block,
            original_prompt: prompt.clone(),
            step_id: step.id,
            workflow_id: step.workflow_id,
            designer_run_id: phase_output.token_usage.run_id,
            total_agents: phase_output.designed_prompts.len(),
            container_handle: managed_container.as_ref().map(|mc| mc.agent_handle.clone()),
            cancel: dag.cancel.cloned(),
            task_description: brief.task_description.clone(),
        };

        let level_result = execute_agent_levels(
            &env,
            dag,
            &phase_output.designed_prompts,
            &brief.failure_mode,
            &managed_container,
        )
        .await?;

        // 11. Destroy optional container
        destroy_optional_container(&managed_container, dag.ctx.wg_client.as_deref()).await;

        // 12. Compose combined output + store results
        let step_in_tokens = phase_output.token_usage.input_tokens + level_result.input_tokens;
        let step_out_tokens = phase_output.token_usage.output_tokens + level_result.output_tokens;
        let step_cost = phase_output.token_usage.cost_usd + level_result.cost_usd;

        let combined_data = compose_workforce_output(&level_result.agent_outputs);
        let output_key = resolve_output_key(step, &dag.port_meta.step_outputs);

        dag_state.accumulate_tokens(step_in_tokens, step_out_tokens, step_cost);

        let output = StepOutput {
            variable_name: output_key,
            raw_output: serde_json::to_string(&combined_data).unwrap_or_default(),
            structured_output: Some(combined_data.clone()),
        };

        let envelope = StepExecutionEnvelope {
            status: ExecutionStatus::Success,
            data: Some(combined_data),
            metadata: ExecutionMetadata {
                execution_time_ms: step_start.elapsed().as_millis() as u64,
                tokens_in: Some(step_in_tokens as i32),
                tokens_out: Some(step_out_tokens as i32),
                cost_usd: Some(step_cost as f64),
                model: Some(WORKFORCE.agent("agent").model_id.clone()),
                ..ExecutionMetadata::new(step.id)
            },
            error: None,
        };

        super::utils::record_and_snapshot_output(dag, dag_state, step.id, output, envelope).await;

        // 13. Broadcast step completed
        broadcast_workflow_event(
            dag.state,
            dag.ctx,
            step.workflow_id,
            WorkflowEventKind::StepCompleted {
                step_id: step.id,
                step_name: step_display_name(step),
                agent_id: None,
                output: None,
                input_tokens: Some(step_in_tokens as u64),
                output_tokens: Some(step_out_tokens as u64),
                duration_ms: Some(step_start.elapsed().as_millis() as u64),
            },
        );

        info!(
            step_id = %step.id,
            agents = env.total_agents,
            tokens_in = step_in_tokens,
            tokens_out = step_out_tokens,
            duration_ms = step_start.elapsed().as_millis(),
            "Pipeline execution completed"
        );

        Ok(())
    }
}
