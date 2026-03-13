//! Service function for running a workflow (POST /workflows/:id/run).
//!
//! Owns the business logic: create execution row, load snapshot/template,
//! build engine, construct context, spawn background DAG execution.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::anyhow;
use uuid::Uuid;

use crate::server::hub::dag::templates::WorkflowSnapshot;
use crate::server::hub::dag::{
    broadcast_workflow_event, execute_workflow_via_engine, WorkflowExecutionContext,
};
use crate::server::hub::ExecutionEngine;
use crate::server::state::AppState;
use crate::server::ws::events::WorkflowEventKind;

use super::super::ServiceError;

// ── Input / Output types ────────────────────────────────────────────────────

/// Everything needed to run a workflow.
pub struct RunWorkflowInput {
    pub workflow_id: Uuid,
    pub user_id: Uuid,
    pub initial_input: Option<String>,
    pub template_id: Option<Uuid>,
}

/// Returned on successful dispatch (execution is running in the background).
pub struct RunWorkflowOutput {
    pub execution_id: Uuid,
    pub workflow_id: Uuid,
}

// ── Service function ────────────────────────────────────────────────────────

/// Create an execution row, resolve template/snapshot, build the engine,
/// and spawn background DAG execution. Returns immediately with the
/// execution ID (the caller returns 202 Accepted).
pub async fn run_workflow(
    state: &AppState,
    input: RunWorkflowInput,
) -> Result<RunWorkflowOutput, ServiceError> {
    let workflow_id = input.workflow_id;
    let user_id = input.user_id;
    let workflow_repo = &state.repos().workflows;
    let collection_repo = state.repos().collections.clone();

    // Create standalone workflow execution row
    let execution = collection_repo
        .create_standalone_workflow_execution(workflow_id, user_id)
        .await
        .map_err(|e| ServiceError::Internal(anyhow!(e.to_string())))?;

    let execution_id = execution.id;

    // If template_id is provided, load and deserialize the frozen snapshot
    let snapshot: Option<Arc<WorkflowSnapshot>> = match input.template_id {
        Some(tid) => {
            let template = workflow_repo
                .get_template(tid)
                .await?
                .ok_or_else(|| ServiceError::not_found("Template"))?;
            if template.workflow_id != workflow_id {
                return Err(ServiceError::not_found("Template"));
            }
            let ws: WorkflowSnapshot = serde_json::from_value(template.snapshot)
                .map_err(|e| {
                    ServiceError::Internal(anyhow!("Invalid template snapshot: {e}"))
                })?;
            Some(Arc::new(ws))
        }
        None => None,
    };

    // Load steps + edges from snapshot or live DB
    let (steps, edges) = match &snapshot {
        Some(snap) => (snap.steps.clone(), snap.edges.clone()),
        None => {
            let s = workflow_repo.list_steps(workflow_id).await?;
            let e = workflow_repo.list_edges(workflow_id).await?;
            (s, e)
        }
    };

    // Filter out hidden steps (e.g. manager dispatch anchor)
    let steps: Vec<_> = steps.into_iter().filter(|s| s.visible).collect();

    if steps.is_empty() {
        return Err(ServiceError::validation("Workflow has no steps"));
    }

    // Build execution engine
    let provider = state
        .provider()
        .ok_or_else(|| ServiceError::Internal(anyhow!("LLM provider not configured")))?
        .clone();
    let engine = ExecutionEngine::new(provider, state.env().debug_stream);

    // Resolve initial_input: prefer caller-supplied value, fall back to
    // first context/input step's prompt_template
    let initial_input = input.initial_input.unwrap_or_else(|| {
        steps
            .iter()
            .find(|s| s.execution_mode == "context" || s.execution_mode == "input")
            .map(|s| s.prompt_template.clone())
            .unwrap_or_default()
    });

    let mut prior_outputs = HashMap::new();
    if !initial_input.is_empty() {
        prior_outputs.insert(
            "input".to_string(),
            serde_json::Value::String(initial_input.clone()),
        );
    }

    let ctx = WorkflowExecutionContext {
        stage_execution_id: execution_id,
        run_id: execution_id,
        user_id,
        initial_input,
        prior_outputs,
        execution_context: None,
        container_config: None,
        wg_client: None,
        snapshot,
    };

    // Spawn execution in background (non-blocking)
    let bg_state = state.clone();
    let bg_collection_repo = collection_repo.clone();
    tokio::spawn(async move {
        // Mark as running
        let _ = bg_collection_repo
            .update_workflow_execution_status(execution_id, "running", None, None)
            .await;

        match execute_workflow_via_engine(&engine, &bg_state, &ctx, &steps, &edges, None).await {
            Ok(result) => {
                // Aggregate outputs
                let mut aggregated = serde_json::Map::new();
                for output in result.outputs.values() {
                    if let Some(structured) = &output.structured_output {
                        aggregated.insert(output.variable_name.clone(), structured.clone());
                    } else if !output.raw_output.is_empty() {
                        aggregated.insert(
                            output.variable_name.clone(),
                            serde_json::Value::String(output.raw_output.clone()),
                        );
                    }
                }
                let outputs_json = serde_json::Value::Object(aggregated);

                let _ = bg_collection_repo
                    .update_workflow_execution_status(
                        execution_id,
                        "completed",
                        Some(outputs_json),
                        None,
                    )
                    .await;
                broadcast_workflow_event(
                    &bg_state,
                    &ctx,
                    workflow_id,
                    WorkflowEventKind::Completed {
                        duration_ms: Some(result.duration_ms),
                    },
                );
            }
            Err(e) => {
                let error_msg = e.to_string();
                let _ = bg_collection_repo
                    .update_workflow_execution_status(
                        execution_id,
                        "failed",
                        None,
                        Some(error_msg.clone()),
                    )
                    .await;
                broadcast_workflow_event(
                    &bg_state,
                    &ctx,
                    workflow_id,
                    WorkflowEventKind::Failed { error: error_msg },
                );
            }
        }
    });

    Ok(RunWorkflowOutput {
        execution_id,
        workflow_id,
    })
}
