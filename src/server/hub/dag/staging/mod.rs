//! Staged step execution — node-by-node workflow tuning.
//!
//! Enables executing individual workflow steps on demand, reconstructing
//! prior state from versioned content snapshots. Users can fine-tune
//! notes and agent settings between step executions.

use std::collections::HashMap;
use std::time::Instant;

use serde_json::Value as JsonValue;
use tracing::{info, warn};
use uuid::Uuid;

use crate::db::traits::ContentVersionRepo;
use crate::db::{WorkflowStepEdgeRow, WorkflowStepRow};
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::error::HubError;
use crate::server::state::AppState;
use crate::types::StepExecutionEnvelope;

use super::belief_capture::execute_belief_capture_step;
use super::dag_state::{
    resolve_output_key, wrap_in_agentless_envelope, DagExecutionState, PortMetadata,
};
use super::for_each::execute_for_each_step;
use super::room_step::execute_room_step;
use super::single::execute_single_step;
use super::utils::{check_step_readiness, StepOutput, StepReadiness, WorkflowExecutionContext};
use super::versioning;
use super::workforce::execute_workforce_step;

mod tests;

// ── Result Type ────────────────────────────────────────────────────────────

/// Result from executing a single staged step.
pub struct StagedStepResult {
    pub step_id: Uuid,
    pub status: String,
    pub output: Option<JsonValue>,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub cost_usd: f32,
    pub duration_ms: u64,
}

// ── DagState Reconstruction ────────────────────────────────────────────────

/// Reconstruct `DagExecutionState` from versioned snapshots for a run.
///
/// Queries all envelope output snapshots for the given `run_id` and
/// rebuilds the in-memory state needed for port resolution and variable
/// interpolation.
pub(crate) async fn reconstruct_dag_state_from_snapshots(
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
    ))
}

// ── Next-Step Computation ──────────────────────────────────────────────────

/// Compute which steps are ready to execute given current completion state.
///
/// Iterates all steps not yet completed and returns those whose upstream
/// dependencies are fully satisfied.
pub(crate) fn compute_next_executable_steps(
    steps: &[WorkflowStepRow],
    edges: &[WorkflowStepEdgeRow],
    dag_state: &DagExecutionState,
) -> Vec<Uuid> {
    steps
        .iter()
        .filter(|s| !dag_state.completed.contains_key(&s.id))
        .filter(|s| {
            check_step_readiness(
                s.id,
                edges,
                &dag_state.completed,
                &dag_state.completed_envelopes,
            ) == StepReadiness::Ready
        })
        .map(|s| s.id)
        .collect()
}

// ── Step Dispatch ──────────────────────────────────────────────────────────

/// Execute a single staged step, dispatching to the appropriate executor.
///
/// Mirrors the dispatch logic in `run_dag_loop()` but for a single step.
/// All existing step executors are reused unchanged.
pub(crate) async fn execute_staged_step(
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    steps: &[WorkflowStepRow],
    edges: &[WorkflowStepEdgeRow],
    dag_state: &mut DagExecutionState,
    port_meta: &PortMetadata,
) -> Result<StagedStepResult, HubError> {
    let step_start = Instant::now();

    // Build default execution engine
    let provider = state
        .provider()
        .ok_or_else(|| HubError::Internal(anyhow::anyhow!("LLM provider not configured")))?
        .clone();
    let engine = ExecutionEngine::new(provider);

    info!(
        step_id = %step.id,
        mode = %step.execution_mode,
        "Staged step execution starting"
    );

    // Context/input pass-through — no LLM call needed
    if step.execution_mode == "context" || step.execution_mode == "input" {
        let data = if step.prompt_template.is_empty() {
            None
        } else {
            Some(JsonValue::String(step.prompt_template.clone()))
        };

        let envelope = wrap_in_agentless_envelope(step.id, data.clone(), 0, 0, 0, 0.0);

        // Snapshot the envelope
        let envelope_json = serde_json::to_string(&envelope).unwrap_or_default();
        let _ = versioning::snapshot_content(
            &*state.repos().content_versions,
            ctx.run_id,
            step.id,
            step.id,
            versioning::content_types::ENVELOPE,
            "output",
            &envelope_json,
        )
        .await;

        let output_key = resolve_output_key(step, &port_meta.step_outputs);
        let output = StepOutput {
            variable_name: output_key,
            structured_output: data.clone(),
            raw_output: step.prompt_template.clone(),
        };
        dag_state.record_step_output(step.id, output, envelope);

        return Ok(StagedStepResult {
            step_id: step.id,
            status: "completed".to_string(),
            output: data,
            tokens_in: 0,
            tokens_out: 0,
            cost_usd: 0.0,
            duration_ms: step_start.elapsed().as_millis() as u64,
        });
    }

    // Agentless modes — dispatch to existing executors
    if matches!(step.execution_mode.as_str(), "belief_capture" | "workforce") {
        let pre_tokens_in = dag_state.total_input_tokens;
        let pre_tokens_out = dag_state.total_output_tokens;
        let pre_cost = dag_state.total_cost_usd;

        match step.execution_mode.as_str() {
            "belief_capture" => {
                execute_belief_capture_step(
                    &engine, state, ctx, step, steps, edges, dag_state, port_meta, None,
                )
                .await?
            }
            "workforce" => {
                execute_workforce_step(
                    &engine, state, ctx, step, steps, edges, dag_state, port_meta, None,
                )
                .await?
            }
            _ => unreachable!(),
        };

        let output = dag_state
            .completed
            .get(&step.id)
            .and_then(|o| o.structured_output.clone());

        return Ok(StagedStepResult {
            step_id: step.id,
            status: "completed".to_string(),
            output,
            tokens_in: dag_state.total_input_tokens - pre_tokens_in,
            tokens_out: dag_state.total_output_tokens - pre_tokens_out,
            cost_usd: dag_state.total_cost_usd - pre_cost,
            duration_ms: step_start.elapsed().as_millis() as u64,
        });
    }

    // Agent-bearing modes — load agent + resolve provider
    let agent_id = step.agent_id.ok_or_else(|| {
        HubError::Internal(anyhow::anyhow!(
            "step {} has no agent_id for mode '{}'",
            step.id,
            step.execution_mode
        ))
    })?;
    let agent = state
        .repo()
        .get_persisted_agent(agent_id)
        .await
        .map_err(|e| anyhow::anyhow!("failed to load agent: {}", e))?
        .ok_or_else(|| HubError::AgentNotFound {
            step_id: step.id,
            agent_id,
        })?;

    // Resolve provider: use registry if agent targets non-default provider
    let step_engine = if agent.model_provider == "anthropic" || agent.model_provider.is_empty() {
        None
    } else {
        let provider = state.provider_for(&agent.model_provider).ok_or_else(|| {
            HubError::ProviderUnavailable {
                provider: agent.model_provider.clone(),
                step_id: step.id,
                agent_name: agent.name.clone(),
            }
        })?;
        Some(ExecutionEngine::new(provider))
    };
    let effective_engine = step_engine.as_ref().unwrap_or(&engine);

    let pre_tokens_in = dag_state.total_input_tokens;
    let pre_tokens_out = dag_state.total_output_tokens;
    let pre_cost = dag_state.total_cost_usd;

    if step.execution_mode == "room" {
        execute_room_step(
            effective_engine,
            state,
            ctx,
            step,
            steps,
            edges,
            dag_state,
            port_meta,
            None,
        )
        .await?;
    } else if step.execution_mode == "for_each" {
        execute_for_each_step(
            effective_engine,
            state,
            ctx,
            step,
            &agent,
            steps,
            edges,
            dag_state,
            port_meta,
            None,
        )
        .await?;
    } else {
        execute_single_step(
            effective_engine,
            state,
            ctx,
            step,
            &agent,
            steps,
            edges,
            dag_state,
            port_meta,
            None,
        )
        .await?;
    }

    let output = dag_state
        .completed
        .get(&step.id)
        .and_then(|o| o.structured_output.clone());

    Ok(StagedStepResult {
        step_id: step.id,
        status: "completed".to_string(),
        output,
        tokens_in: dag_state.total_input_tokens - pre_tokens_in,
        tokens_out: dag_state.total_output_tokens - pre_tokens_out,
        cost_usd: dag_state.total_cost_usd - pre_cost,
        duration_ms: step_start.elapsed().as_millis() as u64,
    })
}
