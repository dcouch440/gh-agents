//! Shared agent execution core for workforce steps.
//!
//! Extracts the common lifecycle from `Pipeline::execute()`:
//! container creation → agent level dispatch → overlay extraction →
//! container teardown → output composition → result recording.
//!
//! Used by:
//! - `Pipeline` (DB-driven: designer phases produce prompts)
//! - `file_executor` (file-driven: system node agent files produce prompts)
//! - Future prompt sources that build `AgentExecutionInput`

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;

use tracing::info;
use uuid::Uuid;

use crate::config::protocols::WORKFORCE;
use crate::db::WorkflowStepRow;
use crate::server::hub::error::HubError;
use crate::server::ws::events::WorkflowEventKind;
use crate::types::{ExecutionMetadata, ExecutionStatus, StepExecutionEnvelope};

use super::super::container::{create_optional_container, destroy_optional_container};
use super::super::dag_state::DagExecutionState;
use super::super::{
    broadcast_workflow_event, resolve_output_key, step_display_name, DagContext, StepOutput,
};
use super::agent_executor::execute_agent_levels;
use super::output::compose_workforce_output;
use super::types::{DesignedAgentPrompt, WorkforceStepEnv};

#[cfg(test)]
#[path = "runner_tests.rs"]
mod tests;

/// Input for the shared agent execution core.
///
/// The seam between "how we got agent configs" and "how we execute them."
/// Pipeline builds this from designer phases. File executor builds it from
/// filesystem reads. Future sources build it their own way — the runner
/// doesn't care where the prompts came from.
pub(crate) struct AgentExecutionInput {
    /// Designed agent prompts (from designer, file reader, or static fallback).
    pub designed_prompts: Vec<DesignedAgentPrompt>,
    /// How to handle agent failures: `"fail_fast"` or `"skip_failed"`.
    pub failure_mode: String,
    /// Pre-formatted upstream DAG step output text for `<previous_step>` injection
    /// on the first agent in the workforce (cross-step handoff).
    pub upstream_step_output: String,
    /// For traceability recording only — the step's task description or
    /// config description. Stored in protocol execution rows but does not
    /// affect LLM behavior.
    pub original_prompt: String,
    /// Designer run ID for FK linkage in protocol execution records.
    /// `None` for file-based execution (no designer phase ran).
    pub designer_run_id: Option<Uuid>,
    /// Token usage from any pre-phase (designer LLM call). Zero for
    /// file-based or static fallback execution.
    pub phase_tokens_in: i64,
    pub phase_tokens_out: i64,
    pub phase_cost: f32,
}

/// Execute designed agents as a workforce step.
///
/// Handles the full lifecycle after prompts are ready:
/// 1. Create optional container (if workflow has container config)
/// 2. Build `WorkforceStepEnv` and dispatch agent levels
/// 3. Extract overlay diff (if containerized)
/// 4. Destroy container
/// 5. Compose workforce output and record results
/// 6. Broadcast `StepCompleted` event
///
/// Does NOT broadcast `StepStarted` — callers handle that because Pipeline
/// broadcasts it early (before DB loading) for responsive UX, while
/// file_executor broadcasts it before reading files.
pub(crate) async fn run_agent_execution(
    dag: &DagContext<'_>,
    step: &WorkflowStepRow,
    dag_state: &mut DagExecutionState,
    input: AgentExecutionInput,
    step_start: Instant,
) -> Result<(), HubError> {
    // 1. Create optional container
    let managed_container = create_optional_container(
        dag.ctx.container_config.as_ref(),
        dag.ctx.wg_client.as_deref(),
        "pipeline",
        dag.state.workspace(),
    )
    .await?;

    // 2. Build env + execute agent levels
    let env = WorkforceStepEnv {
        state: dag.state.clone(),
        ctx: dag.ctx.clone(),
        original_prompt: input.original_prompt,
        step_id: step.id,
        workflow_id: step.workflow_id,
        designer_run_id: input.designer_run_id,
        total_agents: input.designed_prompts.len(),
        container_handle: managed_container.as_ref().map(|mc| mc.agent_handle.clone()),
        cancel: dag.cancel.cloned(),
        stroke_image: dag
            .state
            .repos()
            .workflows
            .get_step_stroke_image(step.id)
            .await
            .unwrap_or(None),
        upstream_step_output: input.upstream_step_output,
    };

    let level_result = execute_agent_levels(
        &env,
        dag,
        &input.designed_prompts,
        &input.failure_mode,
        &managed_container,
    )
    .await?;

    // 3. Extract overlay diff before destroying container
    if let (Some(workspace), Some(cc)) = (dag.state.workspace(), dag.ctx.container_config.as_ref())
    {
        if cc.overlay_enabled {
            if let (Some(wf_id), Some(run_id)) = (cc.workflow_id, cc.run_id) {
                let ws = workspace.clone();
                let base_paths: HashSet<PathBuf> = tokio::task::spawn_blocking(move || {
                    ws.list_files(wf_id, run_id, None)
                        .unwrap_or_default()
                        .into_iter()
                        .collect()
                })
                .await
                .unwrap_or_default();
                dag_state.step_overlay = super::super::container::extract_step_overlay(
                    &managed_container,
                    step.id,
                    step_display_name(step),
                    step.prompt_template.clone(),
                    step.display_order,
                    &base_paths,
                    true,
                )
                .await;
            }
        }
    }

    // 4. Destroy optional container
    destroy_optional_container(&managed_container, dag.ctx.wg_client.as_deref()).await;

    // 5. Compose combined output + store results
    let step_in_tokens = input.phase_tokens_in + level_result.input_tokens;
    let step_out_tokens = input.phase_tokens_out + level_result.output_tokens;
    let step_cost = input.phase_cost + level_result.cost_usd;

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

    let output_text = output.raw_output.clone();
    super::super::utils::record_and_snapshot_output(dag, dag_state, step.id, output, envelope)
        .await;

    // 6. Broadcast step completed
    broadcast_workflow_event(
        dag.state,
        dag.ctx,
        step.workflow_id,
        WorkflowEventKind::StepCompleted {
            step_id: step.id,
            step_name: step_display_name(step),
            agent_id: None,
            output: Some(output_text),
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
        "Agent execution completed"
    );

    Ok(())
}
