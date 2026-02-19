//! Static pipeline execution — runs a child workflow through the DAG loop.
//!
//! Adapted from `sub_workflow` but uses a live child workflow (via
//! `child_workflow_id`) instead of a frozen template snapshot. Each child
//! step executes according to its own `execution_mode`, agent, and prompt
//! configuration.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::anyhow;
use serde_json::Value as JsonValue;
use tracing::{info, warn};
use uuid::Uuid;

use crate::db::pg_repo::PgRepo;
use crate::db::traits::WorkflowCollectionRepo;
use crate::db::WorkflowStepRow;
use crate::server::hub::error::HubError;
use crate::server::ws::events::WorkflowEventKind;
use crate::types::{ExecutionError, ExecutionMetadata, ExecutionStatus, StepExecutionEnvelope};

use super::super::dag_state::DagExecutionState;
use super::super::utils::{
    StepOutput, SubWorkflowParentContext, WorkflowExecutionContext, WorkflowExecutionResult,
};
use super::super::{
    broadcast_workflow_event, execute_workflow_via_engine, resolve_output_key,
    resolve_step_port_inputs, step_display_name, DagContext,
};

/// Execute a static pipeline (no Designer step) by running the child
/// workflow through the standard DAG loop.
///
/// 1. Resolves port inputs from the parent step
/// 2. Loads child workflow edges
/// 3. Creates a child execution record for tracking
/// 4. Builds a child execution context with parent port data
/// 5. Calls `execute_workflow_via_engine` on the child workflow
/// 6. Wraps child outputs in an envelope for the parent DAG
pub(super) async fn execute_static_pipeline(
    dag: &DagContext<'_>,
    step: &WorkflowStepRow,
    dag_state: &mut DagExecutionState,
    child_workflow_id: Uuid,
    child_steps: &[WorkflowStepRow],
) -> Result<(), HubError> {
    let step_start = std::time::Instant::now();

    if child_steps.is_empty() {
        return Err(HubError::Internal(anyhow!(
            "pipeline step {} has no child steps",
            step.id
        )));
    }

    // 1. Broadcast step started (agentless)
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

    // 2. Resolve port inputs from parent step
    let port_inputs = resolve_step_port_inputs(step, dag.port_meta, &dag_state.completed_envelopes);

    let child_prior_outputs: HashMap<String, JsonValue> = port_inputs.unwrap_or_default();
    let child_initial_input = child_prior_outputs
        .values()
        .next()
        .map(|v| match v {
            JsonValue::String(s) => s.clone(),
            other => serde_json::to_string(other).unwrap_or_default(),
        })
        .unwrap_or_default();

    // 3. Load child workflow edges
    let child_edges = dag
        .state
        .repos()
        .workflows
        .list_edges(child_workflow_id)
        .await
        .map_err(|e| HubError::Internal(anyhow!("failed to load pipeline edges: {}", e)))?;

    // 4. Create child execution record for tracking
    let db = dag
        .state
        .db()
        .ok_or_else(|| HubError::Internal(anyhow!("database not available")))?
        .clone();
    let collection_repo: Arc<dyn WorkflowCollectionRepo> = Arc::new(PgRepo::new(db));

    let child_execution = collection_repo
        .create_child_workflow_execution(
            dag.ctx.run_id,
            child_workflow_id,
            dag.ctx.user_id,
            Uuid::nil(), // no template for live pipelines
        )
        .await
        .map_err(|e| HubError::Internal(anyhow!("failed to create pipeline execution: {}", e)))?;

    info!(
        parent_step_id = %step.id,
        child_execution_id = %child_execution.id,
        child_workflow_id = %child_workflow_id,
        child_steps = child_steps.len(),
        "Starting static pipeline execution"
    );

    // 5. Update child execution status to running
    let _ = collection_repo
        .update_workflow_execution_status(child_execution.id, "running", None, None)
        .await;

    // 6. Broadcast pipeline started on parent's channel
    broadcast_workflow_event(
        dag.state,
        dag.ctx,
        step.workflow_id,
        WorkflowEventKind::SubWorkflowStarted {
            parent_step_id: step.id,
            child_execution_id: child_execution.id,
            total_steps: child_steps.len(),
        },
    );

    // 7. Build child workflow execution context
    let child_ctx = WorkflowExecutionContext {
        stage_execution_id: child_execution.id,
        run_id: child_execution.id,
        user_id: dag.ctx.user_id,
        initial_input: child_initial_input,
        prior_outputs: child_prior_outputs,
        execution_context: dag.ctx.execution_context.clone(),
        container_config: dag.ctx.container_config.clone(),
        wg_client: dag.ctx.wg_client.clone(),
        snapshot: None, // live pipeline — no frozen snapshot
        parent_context: Some(SubWorkflowParentContext {
            parent_step_id: step.id,
            parent_run_id: dag.ctx.run_id,
            parent_workflow_id: step.workflow_id,
        }),
    };

    // 8. Execute child workflow (recursive via DAG loop)
    // Box::pin is required because this creates async recursion:
    // execute_pipeline_step → execute_static_pipeline → execute_workflow_via_engine
    //   → run_dag_loop → execute_pipeline_step (if nested)
    let child_result = Box::pin(execute_workflow_via_engine(
        dag.engine,
        dag.state,
        &child_ctx,
        child_steps,
        &child_edges,
        dag.cancel,
    ))
    .await;

    let step_duration = step_start.elapsed().as_millis() as u64;

    // 9. Build result envelope
    let (envelope, final_status) = build_result_envelope(
        dag.state,
        &collection_repo,
        &child_execution.id,
        child_result,
        step_duration,
    )
    .await;

    // 10. Accumulate child tokens into parent
    dag_state.accumulate_tokens(
        envelope.metadata.tokens_in.unwrap_or(0) as i64,
        envelope.metadata.tokens_out.unwrap_or(0) as i64,
        envelope.metadata.cost_usd.unwrap_or(0.0) as f32,
    );

    // 11. Broadcast pipeline completed on parent's channel
    broadcast_workflow_event(
        dag.state,
        dag.ctx,
        step.workflow_id,
        WorkflowEventKind::SubWorkflowCompleted {
            parent_step_id: step.id,
            child_execution_id: child_execution.id,
            status: final_status.to_string(),
        },
    );

    // 12. Record output in parent's DAG state
    let output_key = resolve_output_key(step, &dag.port_meta.step_outputs);
    let output = StepOutput {
        variable_name: output_key,
        structured_output: envelope.data.clone(),
        raw_output: envelope
            .data
            .as_ref()
            .map(|d| serde_json::to_string_pretty(d).unwrap_or_default())
            .unwrap_or_default(),
    };

    super::super::utils::record_and_snapshot_output(dag, dag_state, step.id, output, envelope.clone())
        .await;

    // 13. Broadcast parent step completed or failed
    if envelope.status == ExecutionStatus::Success {
        broadcast_workflow_event(
            dag.state,
            dag.ctx,
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
            "Static pipeline step completed successfully"
        );

        Ok(())
    } else {
        let error_msg = envelope
            .error
            .as_ref()
            .map(|e| e.message.clone())
            .unwrap_or_else(|| "pipeline execution failed".to_string());

        Err(HubError::Internal(anyhow!(
            "Pipeline execution failed: {}",
            error_msg
        )))
    }
}

/// Build either a success or error envelope from the child workflow result.
async fn build_result_envelope(
    state: &crate::server::state::AppState,
    collection_repo: &Arc<dyn WorkflowCollectionRepo>,
    child_execution_id: &Uuid,
    child_result: Result<WorkflowExecutionResult, HubError>,
    step_duration: u64,
) -> (StepExecutionEnvelope, &'static str) {
    match child_result {
        Ok(result) => {
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
                "Pipeline execution failed"
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
                    error_type: "PipelineFailed".into(),
                    retryable: false,
                    details: None,
                }),
            };

            (envelope, "failed")
        }
    }
}
