//! DAG state reconstruction from versioned snapshots.
//!
//! Rebuilds the in-memory `DagExecutionState` from envelope snapshots
//! stored in the content versions table. This lets workshop execution
//! resume from where it left off between step runs.

use std::collections::HashMap;

use tracing::warn;
use uuid::Uuid;

use crate::db::traits::ContentVersionRepo;
use crate::db::WorkflowStepRow;
use crate::server::hub::error::HubError;
use crate::types::{ExecutionStatus, StepExecutionEnvelope};

use super::super::dag_state::{resolve_output_key, DagExecutionState, PortMetadata};
use super::super::utils::StepOutput;

/// Reconstruct `DagExecutionState` from versioned snapshots for a run.
///
/// Queries all envelope output snapshots for the given `run_id` and
/// rebuilds the in-memory state needed for port resolution and variable
/// interpolation.
pub(crate) async fn reconstruct_state(
    cv_repo: &dyn ContentVersionRepo,
    steps: &[WorkflowStepRow],
    port_meta: &PortMetadata,
    run_id: Uuid,
) -> Result<DagExecutionState, HubError> {
    let snapshots = cv_repo
        .list_envelope_snapshots_for_run(run_id)
        .await
        .map_err(HubError::Internal)?;

    let step_map: HashMap<Uuid, &WorkflowStepRow> = steps.iter().map(|s| (s.id, s)).collect();

    let mut completed = HashMap::new();
    let mut completed_envelopes = HashMap::new();
    let mut var_outputs = HashMap::new();
    let mut failed: HashMap<Uuid, String> = HashMap::new();

    for snapshot in &snapshots {
        let envelope: StepExecutionEnvelope = match serde_json::from_str(&snapshot.content) {
            Ok(env) => env,
            Err(e) => {
                warn!(
                    step_id = %snapshot.step_id,
                    "Failed to deserialize envelope snapshot: {}",
                    e
                );
                continue;
            }
        };

        // Error envelopes go to `failed`; successful ones go to `completed`.
        // Snapshots are ordered by created_at ASC, so retries overwrite prior state.
        if envelope.status == ExecutionStatus::Error && envelope.data.is_none() {
            let error_msg = envelope
                .error
                .as_ref()
                .map(|e| e.message.clone())
                .unwrap_or_else(|| "Unknown error".to_string());
            // Remove from completed in case this is a re-run that failed
            completed.remove(&snapshot.step_id);
            completed_envelopes.remove(&snapshot.step_id);
            failed.insert(snapshot.step_id, error_msg);
            continue;
        }

        // Success path — remove from failed in case this is a retry after failure
        failed.remove(&snapshot.step_id);

        // Build StepOutput for the completed map
        let variable_name = step_map
            .get(&snapshot.step_id)
            .map(|s| resolve_output_key(s, &port_meta.step_outputs))
            .unwrap_or_default();

        let step_output = StepOutput {
            variable_name: variable_name.clone(),
            structured_output: envelope.data.clone(),
            raw_output: String::new(),
        };

        // Populate var_outputs for variable resolution
        if !variable_name.is_empty() {
            if let Some(ref data) = envelope.data {
                var_outputs.insert(variable_name, data.clone());
            }
        }

        completed_envelopes.insert(snapshot.step_id, envelope);
        completed.insert(snapshot.step_id, step_output);
    }

    Ok(DagExecutionState::from_snapshots(
        completed,
        var_outputs,
        completed_envelopes,
        failed,
    ))
}
