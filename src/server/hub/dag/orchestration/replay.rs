//! Pinned replay — replays the last known output for pinned steps, skipping execution.

use tracing::{info, warn};

use crate::db::WorkflowStepRow;
use crate::server::hub::error::HubError;
use crate::server::ws::events::WorkflowEventKind;
use crate::types::StepExecutionEnvelope;

use super::super::broadcast::broadcast_workflow_event;
use super::super::utils;
use super::super::{
    resolve_output_key, step_display_name, wrap_in_agentless_envelope, DagContext,
    DagExecutionState, StepOutput,
};

/// Attempt to replay a pinned step's last output. Returns `true` if replayed.
pub(super) async fn try_replay_pinned(
    dag: &DagContext<'_>,
    dag_state: &mut DagExecutionState,
    step: &WorkflowStepRow,
) -> Result<bool, HubError> {
    let Some((output, envelope)) = load_pinned_output(dag, step).await? else {
        return Ok(false);
    };

    utils::record_and_snapshot_output(dag, dag_state, step.id, output, envelope).await;
    broadcast_workflow_event(
        dag.state,
        dag.ctx,
        step.workflow_id,
        WorkflowEventKind::StepCompleted {
            step_id: step.id,
            step_name: step_display_name(step),
            agent_id: None,
            output: None,
            input_tokens: Some(0),
            output_tokens: Some(0),
            duration_ms: Some(0),
        },
    );
    info!(step_id = %step.id, mode = %step.execution_mode, "Pinned step replayed");
    Ok(true)
}

/// Load output for a pinned step, replaying its last known result.
///
/// For `context`/`input` modes: always returns Some with the pass-through output.
/// For `single` and other modes: loads the last envelope from DB; returns None
/// if no prior execution exists (caller should fall through to normal execution).
async fn load_pinned_output(
    dag: &DagContext<'_>,
    step: &WorkflowStepRow,
) -> Result<Option<(StepOutput, StepExecutionEnvelope)>, HubError> {
    match step.execution_mode.as_str() {
        "context" | "input" => {
            let output_key = resolve_output_key(step, &dag.port_meta.step_outputs);
            let content = if step.prompt_template.is_empty() {
                dag.ctx.initial_input.clone()
            } else {
                step.prompt_template.clone()
            };
            let (output, value) = StepOutput::passthrough(output_key, content);
            let envelope = wrap_in_agentless_envelope(step.id, Some(value), 0, 0, 0, 0.0);
            Ok(Some((output, envelope)))
        }
        _ => {
            let envelope_json = dag
                .state
                .repos()
                .content_versions
                .get_latest_envelope_for_step(step.id)
                .await
                .map_err(|e| {
                    HubError::Internal(anyhow::anyhow!("Failed to load pinned envelope: {}", e))
                })?;

            match envelope_json {
                Some(json_str) => {
                    let envelope: StepExecutionEnvelope =
                        serde_json::from_str(&json_str).map_err(|e| {
                            HubError::Internal(anyhow::anyhow!(
                                "Failed to deserialize pinned envelope: {}",
                                e
                            ))
                        })?;
                    let output_key = resolve_output_key(step, &dag.port_meta.step_outputs);
                    let output = StepOutput {
                        variable_name: output_key,
                        structured_output: envelope.data.clone(),
                        raw_output: envelope
                            .data
                            .as_ref()
                            .map(|d| {
                                serde_json::to_string(d)
                                    .inspect_err(|e| {
                                        warn!(
                                            step_id = %step.id,
                                            "Failed to serialize pinned output: {e}"
                                        )
                                    })
                                    .unwrap_or_default()
                            })
                            .unwrap_or_default(),
                    };
                    Ok(Some((output, envelope)))
                }
                None => Ok(None),
            }
        }
    }
}
