//! Workshop execution context helpers.
//!
//! Builds the `WorkflowExecutionContext` for workshop runs, handles
//! pinned step replay, and creates error envelope snapshots.

use std::collections::HashMap;

use serde_json::Value as JsonValue;
use tracing::warn;
use uuid::Uuid;

use crate::db::WorkflowStepRow;
use crate::server::hub::error::HubError;
use crate::server::state::AppState;
use crate::types::{ExecutionError, ExecutionMetadata, ExecutionStatus, StepExecutionEnvelope};

use super::super::dag_state::DagExecutionState;
use super::super::utils::WorkflowExecutionContext;
use super::super::versioning;
use super::types::WorkshopStepResult;

/// Build the `WorkflowExecutionContext` for a workshop run.
pub(crate) fn build_execution_context(
    run_id: Uuid,
    user_id: Uuid,
    dag_state: &DagExecutionState,
) -> WorkflowExecutionContext {
    let mut prior_outputs = HashMap::new();
    for (key, val) in &dag_state.var_outputs {
        prior_outputs.insert(key.clone(), val.clone());
    }

    WorkflowExecutionContext {
        stage_execution_id: run_id,
        run_id,
        user_id,
        initial_input: String::new(),
        prior_outputs,
        execution_context: None,
        container_config: None,
        wg_client: None,
        snapshot: None,
        parent_context: None,
    }
}

/// Replay a pinned step from its last envelope snapshot.
///
/// Returns `Some(result)` if a prior envelope exists and was replayed.
/// Returns `None` if no prior output exists (caller should fall through
/// to normal execution).
pub(crate) async fn replay_pinned_step(
    state: &AppState,
    run_id: Uuid,
    step: &WorkflowStepRow,
) -> Result<Option<(WorkshopStepResult, JsonValue)>, HubError> {
    let envelope_json = state
        .repos()
        .content_versions
        .get_latest_envelope_for_step(step.id)
        .await
        .map_err(|e| HubError::Internal(anyhow::anyhow!("Failed to load pinned envelope: {e}")))?;

    let Some(json_str) = envelope_json else {
        warn!(step_id = %step.id, "Pinned step has no prior output, executing normally");
        return Ok(None);
    };

    let envelope: StepExecutionEnvelope = serde_json::from_str(&json_str)
        .map_err(|e| HubError::Internal(anyhow::anyhow!("Bad pinned envelope: {e}")))?;

    // Re-snapshot the replayed envelope for this run
    let _ = versioning::snapshot_content(
        &*state.repos().content_versions,
        run_id,
        step.id,
        step.id,
        versioning::content_types::ENVELOPE,
        "output",
        &json_str,
    )
    .await;

    let output = envelope.data.clone();
    let result = WorkshopStepResult {
        step_id: step.id,
        status: "completed".to_string(),
        output: output.clone(),
        tokens_in: 0,
        tokens_out: 0,
        cost_usd: 0.0,
        duration_ms: 0,
    };

    // Return the output data separately for broadcast
    let output_for_broadcast = output.unwrap_or(JsonValue::Null);
    Ok(Some((result, output_for_broadcast)))
}

/// Snapshot an error envelope so failure state survives page reloads.
pub(crate) async fn snapshot_error_envelope(
    state: &AppState,
    run_id: Uuid,
    step_id: Uuid,
    error_msg: &str,
) {
    let error_envelope = StepExecutionEnvelope {
        status: ExecutionStatus::Error,
        data: None,
        metadata: ExecutionMetadata::new(step_id),
        error: Some(ExecutionError {
            message: error_msg.to_string(),
            error_type: "execution_failed".to_string(),
            retryable: true,
            details: None,
        }),
    };

    if let Ok(envelope_json) = serde_json::to_string(&error_envelope) {
        let _ = versioning::snapshot_content(
            &*state.repos().content_versions,
            run_id,
            step_id,
            step_id,
            versioning::content_types::ENVELOPE,
            "output",
            &envelope_json,
        )
        .await;
    }
}
