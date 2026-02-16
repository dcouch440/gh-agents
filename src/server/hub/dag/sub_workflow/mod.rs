//! Sub-workflow step execution within the DAG.
//!
//! When the DAG encounters a step with `execution_mode = "sub_workflow"`, this
//! module loads the referenced template snapshot, maps parent port inputs to
//! child workflow context, creates a child workflow execution, and calls
//! `execute_workflow_via_engine()` recursively. The child's outputs are wrapped
//! in a `StepExecutionEnvelope` for the parent DAG.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::anyhow;
use serde_json::Value as JsonValue;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use uuid::Uuid;

use crate::db::pg_repo::PgRepo;
use crate::db::traits::WorkflowCollectionRepo;
use crate::db::{WorkflowStepEdgeRow, WorkflowStepRow};
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::error::HubError;
use crate::server::state::AppState;
use crate::server::ws::events::WorkflowEventKind;
use crate::types::{ExecutionError, ExecutionMetadata, ExecutionStatus, StepExecutionEnvelope};

use super::dag_state::DagExecutionState;
use super::templates::WorkflowSnapshot;
use super::utils::{StepOutput, WorkflowExecutionContext, WorkflowExecutionResult};
use super::{
    broadcast_workflow_event, execute_workflow_via_engine, resolve_output_key,
    resolve_step_port_inputs, step_display_name, PortMetadata,
};

mod tests;

/// Execute a sub-workflow step within the DAG.
///
/// Loads the referenced template snapshot, maps parent port inputs to child
/// workflow initial context, creates a child execution record, and executes
/// the child workflow. Returns the child's outputs wrapped in an envelope.
#[allow(clippy::too_many_arguments)]
pub(super) async fn execute_sub_workflow_step(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    _steps: &[WorkflowStepRow],
    edges: &[WorkflowStepEdgeRow],
    dag_state: &mut DagExecutionState,
    port_meta: &PortMetadata,
    cancel: Option<&CancellationToken>,
) -> Result<(), HubError> {
    let step_start = std::time::Instant::now();

    // 1. Validate that step has sub_workflow_template_id
    let template_id = step.sub_workflow_template_id.ok_or_else(|| {
        HubError::Internal(anyhow!(
            "sub_workflow step {} has no sub_workflow_template_id",
            step.id
        ))
    })?;

    // 2. Broadcast step started (agentless)
    broadcast_workflow_event(
        state,
        ctx,
        step.workflow_id,
        WorkflowEventKind::StepStarted {
            step_id: step.id,
            step_name: step_display_name(step),
            agent_id: None,
            execution_id: None,
        },
    );

    // 3. Load template snapshot
    let template = state
        .repos()
        .workflows
        .get_template(template_id)
        .await
        .map_err(|e| HubError::Internal(anyhow!("failed to load template: {}", e)))?
        .ok_or_else(|| {
            HubError::Internal(anyhow!("sub_workflow template {} not found", template_id))
        })?;

    let snapshot: WorkflowSnapshot = serde_json::from_value(template.snapshot).map_err(|e| {
        HubError::Internal(anyhow!("failed to deserialize template snapshot: {}", e))
    })?;

    if snapshot.steps.is_empty() {
        return Err(HubError::Internal(anyhow!(
            "sub_workflow template {} has no steps",
            template_id
        )));
    }

    // 4. Resolve port inputs from parent step
    let port_inputs =
        resolve_step_port_inputs(step, edges, port_meta, &dag_state.completed_envelopes);

    // 5. Map port inputs to child workflow context
    let child_prior_outputs: HashMap<String, JsonValue> = port_inputs.unwrap_or_default();

    let child_initial_input = child_prior_outputs
        .values()
        .next()
        .map(|v| match v {
            JsonValue::String(s) => s.clone(),
            other => serde_json::to_string(other).unwrap_or_default(),
        })
        .unwrap_or_default();

    // 6. Create child workflow execution record
    let child_workflow_id = snapshot.steps[0].workflow_id;

    let db = state
        .db()
        .ok_or_else(|| HubError::Internal(anyhow!("database not available")))?
        .clone();
    let collection_repo: Arc<dyn WorkflowCollectionRepo> = Arc::new(PgRepo::new(db));

    let child_execution = collection_repo
        .create_child_workflow_execution(ctx.run_id, child_workflow_id, ctx.user_id, template_id)
        .await
        .map_err(|e| HubError::Internal(anyhow!("failed to create child execution: {}", e)))?;

    info!(
        parent_step_id = %step.id,
        child_execution_id = %child_execution.id,
        template_id = %template_id,
        child_steps = snapshot.steps.len(),
        "Starting sub-workflow execution"
    );

    // 7. Update child execution status to running
    let _ = collection_repo
        .update_workflow_execution_status(child_execution.id, "running", None, None)
        .await;

    // 8. Broadcast SubWorkflowStarted on parent's channel
    broadcast_workflow_event(
        state,
        ctx,
        step.workflow_id,
        WorkflowEventKind::SubWorkflowStarted {
            parent_step_id: step.id,
            child_execution_id: child_execution.id,
            total_steps: snapshot.steps.len(),
        },
    );

    // 9. Build child workflow execution context
    let child_ctx = WorkflowExecutionContext {
        stage_execution_id: child_execution.id,
        run_id: child_execution.id,
        user_id: ctx.user_id,
        initial_input: child_initial_input,
        prior_outputs: child_prior_outputs,
        execution_context: ctx.execution_context.clone(),
        container_config: ctx.container_config.clone(),
        wg_client: ctx.wg_client.clone(),
        snapshot: Some(Arc::new(snapshot.clone())),
    };

    // 10. Execute child workflow (recursive call, propagate cancellation token)
    // Box::pin is required because this creates async recursion:
    // execute_sub_workflow_step → execute_workflow_via_engine → run_dag_loop → execute_sub_workflow_step
    let child_result = Box::pin(execute_workflow_via_engine(
        engine,
        state,
        &child_ctx,
        &snapshot.steps,
        &snapshot.edges,
        cancel,
    ))
    .await;

    let step_duration = step_start.elapsed().as_millis() as u64;

    // 11. Handle result
    let (envelope, final_status) = build_result_envelope(
        state,
        &collection_repo,
        &child_execution.id,
        child_result,
        step_duration,
    )
    .await;

    // 12. Accumulate tokens from child into parent
    dag_state.accumulate_tokens(
        envelope.metadata.tokens_in.unwrap_or(0) as i64,
        envelope.metadata.tokens_out.unwrap_or(0) as i64,
        envelope.metadata.cost_usd.unwrap_or(0.0) as f32,
    );

    // 13. Broadcast SubWorkflowCompleted on parent's channel
    broadcast_workflow_event(
        state,
        ctx,
        step.workflow_id,
        WorkflowEventKind::SubWorkflowCompleted {
            parent_step_id: step.id,
            child_execution_id: child_execution.id,
            status: final_status.to_string(),
        },
    );

    // 14. Record output in parent's DAG state
    let output_key = resolve_output_key(step, &port_meta.step_outputs);
    let output = StepOutput {
        variable_name: output_key,
        structured_output: envelope.data.clone(),
        raw_output: envelope
            .data
            .as_ref()
            .map(|d| serde_json::to_string_pretty(d).unwrap_or_default())
            .unwrap_or_default(),
    };

    let envelope_json = serde_json::to_string(&envelope).unwrap_or_default();
    dag_state.record_step_output(step.id, output, envelope.clone());
    let _ = super::versioning::snapshot_content(
        &*state.repos().content_versions,
        ctx.run_id,
        step.id,
        step.id,
        super::versioning::content_types::ENVELOPE,
        "output",
        &envelope_json,
    )
    .await;

    // 15. Broadcast parent step completed or failed
    if envelope.status == ExecutionStatus::Success {
        broadcast_workflow_event(
            state,
            ctx,
            step.workflow_id,
            WorkflowEventKind::StepCompleted {
                step_id: step.id,
                step_name: step_display_name(step),
                agent_id: None,
                output: None,
                input_tokens: envelope.metadata.tokens_in.map(|t| t as u64),
                output_tokens: envelope.metadata.tokens_out.map(|t| t as u64),
                duration_ms: Some(step_duration),
            },
        );

        info!(
            parent_step_id = %step.id,
            child_execution_id = %child_execution.id,
            duration_ms = step_duration,
            "Sub-workflow step completed successfully"
        );

        Ok(())
    } else {
        let error_msg = envelope
            .error
            .as_ref()
            .map(|e| e.message.clone())
            .unwrap_or_else(|| "sub-workflow failed".to_string());

        Err(HubError::Internal(anyhow!(
            "Sub-workflow execution failed: {}",
            error_msg
        )))
    }
}

/// Build either a success or error envelope from the child workflow result.
async fn build_result_envelope(
    state: &AppState,
    collection_repo: &Arc<dyn WorkflowCollectionRepo>,
    child_execution_id: &Uuid,
    child_result: Result<WorkflowExecutionResult, HubError>,
    step_duration: u64,
) -> (StepExecutionEnvelope, &'static str) {
    match child_result {
        Ok(result) => {
            // Build a JSON object from the child's completed step outputs.
            // Each key is the step's output variable name, value is the structured output.
            let outputs_map: serde_json::Map<String, JsonValue> = result
                .outputs
                .iter()
                .filter_map(|(key, step_output)| {
                    step_output
                        .structured_output
                        .clone()
                        .map(|v| (key.clone(), v))
                })
                .collect();
            let outputs_json = JsonValue::Object(outputs_map);

            let _ = collection_repo
                .update_workflow_execution_status(
                    *child_execution_id,
                    "completed",
                    Some(outputs_json.clone()),
                    None,
                )
                .await;

            let envelope = StepExecutionEnvelope {
                status: ExecutionStatus::Success,
                data: Some(outputs_json),
                metadata: ExecutionMetadata {
                    execution_time_ms: step_duration,
                    tokens_in: Some(result.total_input_tokens as i32),
                    tokens_out: Some(result.total_output_tokens as i32),
                    cost_usd: Some(result.total_cost_usd as f64),
                    child_workflow_execution_id: Some(*child_execution_id),
                    ..ExecutionMetadata::new(*child_execution_id)
                },
                error: None,
            };

            (envelope, "completed")
        }
        Err(e) => {
            let error_msg = e.to_string();
            warn!(
                child_execution_id = %child_execution_id,
                error = %error_msg,
                "Sub-workflow execution failed"
            );

            let _ = collection_repo
                .update_workflow_execution_status(
                    *child_execution_id,
                    "failed",
                    None,
                    Some(error_msg.clone()),
                )
                .await;

            // Broadcast failure on child's channel
            state.broadcast_workflow(crate::server::ws::events::WorkflowEvent {
                run_id: Some(*child_execution_id),
                workflow_id: Uuid::nil(),
                user_id: None,
                kind: WorkflowEventKind::Failed {
                    error: error_msg.clone(),
                },
            });

            let envelope = StepExecutionEnvelope {
                status: ExecutionStatus::Error,
                data: None,
                metadata: ExecutionMetadata {
                    execution_time_ms: step_duration,
                    child_workflow_execution_id: Some(*child_execution_id),
                    ..ExecutionMetadata::new(*child_execution_id)
                },
                error: Some(ExecutionError {
                    message: error_msg,
                    error_type: "SubWorkflowFailed".into(),
                    retryable: false,
                    details: None,
                }),
            };

            (envelope, "failed")
        }
    }
}
