//! Core DAG dispatch loop — the inner engine of workflow execution.
//!
//! `run_dag_loop` iterates topologically-sorted steps, applying guard
//! clauses (cancellation, pinned replay, dead-path elimination, conditional
//! edges) before routing each step to its executor. Private helpers handle
//! passthrough forwarding, agent loading, provider resolution, and pinned
//! output replay.

use std::collections::{HashMap, HashSet};

use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::db::WorkflowStepRow;
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::error::HubError;
use crate::server::state::AppState;
use crate::server::ws::events::WorkflowEventKind;
use crate::types::StepExecutionEnvelope;

use super::broadcast::broadcast_workflow_event;
use super::pipeline::{DesignerPhase, Pipeline};
use super::single::execute_single_step;
use super::utils;
use super::{
    broadcast_step_failure_if_real, check_step_readiness, compute_dead_path_steps,
    resolve_output_key, step_display_name, topological_sort_levels, wrap_in_agentless_envelope,
    DagContext, DagExecutionState, StepOutput, StepReadiness,
};

// ── Main Loop ───────────────────────────────────────────────────────────────

/// Core step dispatch loop shared by both fresh execution and resume paths.
///
/// Iterates through topological levels, executing steps within each level in
/// parallel when possible. Single-step levels execute directly (no spawn
/// overhead). Multi-step levels use `JoinSet` for concurrent dispatch.
///
/// Handles cancellation, conditional edges, pinned replay, dead-path
/// elimination, and provider resolution for non-default LLM providers.
pub(crate) async fn run_dag_loop(
    dag: &DagContext<'_>,
    dag_state: &mut DagExecutionState,
) -> Result<(), HubError> {
    let levels = topological_sort_levels(dag.steps, dag.edges).map_err(|_| HubError::DagCycle)?;
    let step_map: HashMap<Uuid, &WorkflowStepRow> = dag.steps.iter().map(|s| (s.id, s)).collect();

    let Some(workflow_id) = dag.steps.first().map(|s| s.workflow_id) else {
        return Ok(()); // Empty DAG — nothing to execute.
    };

    let dead_path_steps = compute_dead_path_steps(dag.steps, dag.edges);
    if !dead_path_steps.is_empty() {
        info!(
            count = dead_path_steps.len(),
            "Dead-path steps identified (all consumers pinned)"
        );
    }

    for level in &levels {
        if dag.cancel.is_some_and(|t| t.is_cancelled()) {
            return Err(HubError::Cancelled);
        }

        // Apply guard clauses to find which steps in this level should execute
        let executable_steps =
            apply_level_guards(dag, dag_state, level, &step_map, &dead_path_steps).await?;

        if executable_steps.is_empty() {
            continue;
        }

        if executable_steps.len() == 1 {
            // Single step — execute directly (no spawn overhead)
            let step = step_map[&executable_steps[0]];
            let step_result = dispatch_step(dag, dag_state, step).await;

            if let Err(ref e) = step_result {
                broadcast_step_failure_if_real(dag.state, dag.ctx, workflow_id, step, e);
            }
            step_result?;

            spawn_summarizer_if_completed(dag.state, step.id, dag_state);
        } else {
            // Multiple steps — execute in parallel via JoinSet
            execute_level_parallel(dag, dag_state, &executable_steps, &step_map, workflow_id)
                .await?;
        }
    }

    Ok(())
}

/// Apply guard clauses (completed, pinned, dead-path, readiness) to each step
/// in a level. Returns the IDs of steps that should be dispatched.
async fn apply_level_guards(
    dag: &DagContext<'_>,
    dag_state: &mut DagExecutionState,
    level: &[Uuid],
    step_map: &HashMap<Uuid, &WorkflowStepRow>,
    dead_path_steps: &HashSet<Uuid>,
) -> Result<Vec<Uuid>, HubError> {
    let mut executable = Vec::new();

    for &step_id in level {
        if dag_state.completed.contains_key(&step_id) {
            continue;
        }
        let step = match step_map.get(&step_id) {
            Some(s) => *s,
            None => continue,
        };

        // Pinned steps replay their last output — skip execution entirely
        if step.pinned {
            if try_replay_pinned(dag, dag_state, step).await? {
                continue;
            }
            warn!(step_id = %step.id, "Pinned step has no prior output, executing normally");
        }

        // Dead-path elimination
        if dead_path_steps.contains(&step_id) {
            dag_state
                .completed
                .insert(step_id, StepOutput::skipped(step_id));
            info!(step_id = %step.id, "Dead-path step skipped (all consumers pinned)");
            continue;
        }

        // Conditional edge readiness
        match check_step_readiness(
            step_id,
            dag.edges,
            &dag_state.completed,
            &dag_state.completed_envelopes,
        ) {
            StepReadiness::Waiting => {
                debug!(step_id = %step_id, "Step waiting on parent completion");
                continue;
            }
            StepReadiness::Skipped => {
                info!(step_id = %step_id, "Step skipped — no matching conditional edges");
                dag_state
                    .completed
                    .insert(step_id, StepOutput::skipped(step_id));
                continue;
            }
            StepReadiness::Ready => {}
        }

        executable.push(step_id);
    }

    Ok(executable)
}

/// Execute multiple steps in parallel using `JoinSet`. Each task gets its own
/// `DagExecutionState` snapshot; results are merged back after all tasks complete.
async fn execute_level_parallel(
    dag: &DagContext<'_>,
    dag_state: &mut DagExecutionState,
    step_ids: &[Uuid],
    step_map: &HashMap<Uuid, &WorkflowStepRow>,
    workflow_id: Uuid,
) -> Result<(), HubError> {
    let mut join_set = tokio::task::JoinSet::new();

    for &step_id in step_ids {
        let step = step_map[&step_id].clone();
        let mut task_state = dag_state.snapshot_for_parallel();

        // Clone owned data for 'static lifetime in spawned task
        let state = dag.state.clone();
        let ctx = dag.ctx.clone();
        let engine = dag.engine.clone_with_provider();
        let steps_owned: Vec<WorkflowStepRow> = dag.steps.to_vec();
        let edges_owned = dag.edges.to_vec();
        let port_meta = dag.port_meta.clone();
        let cancel = dag.cancel.cloned();

        join_set.spawn(async move {
            let cancel_ref = cancel.as_ref();
            let task_dag = DagContext {
                engine: &engine,
                state: &state,
                ctx: &ctx,
                steps: &steps_owned,
                edges: &edges_owned,
                port_meta: &port_meta,
                cancel: cancel_ref,
            };
            let result = dispatch_step(&task_dag, &mut task_state, &step).await;
            (step_id, step, result, task_state)
        });
    }

    // Collect results — fail-fast on first error
    while let Some(join_result) = join_set.join_next().await {
        match join_result {
            Ok((step_id, _step, Ok(()), task_state)) => {
                dag_state.merge_parallel_result(task_state);
                spawn_summarizer_if_completed(dag.state, step_id, dag_state);
            }
            Ok((_step_id, step, Err(e), _task_state)) => {
                broadcast_step_failure_if_real(dag.state, dag.ctx, workflow_id, &step, &e);
                join_set.abort_all();
                return Err(e);
            }
            Err(join_err) => {
                error!("DAG step task panicked: {}", join_err);
                join_set.abort_all();
                return Err(HubError::Internal(anyhow::anyhow!(
                    "DAG step task panicked: {}",
                    join_err
                )));
            }
        }
    }

    Ok(())
}

// ── Step Dispatch ───────────────────────────────────────────────────────────

/// Route a step to the correct executor based on its execution mode.
///
/// - `context` / `input` → pass-through (no LLM call)
/// - `workforce` → pipeline execution (designer pre-phase + agent loop)
/// - Everything else → single agent execution with provider resolution
async fn dispatch_step(
    dag: &DagContext<'_>,
    dag_state: &mut DagExecutionState,
    step: &WorkflowStepRow,
) -> Result<(), HubError> {
    match step.execution_mode.as_str() {
        "context" | "input" => execute_passthrough(dag, dag_state, step).await,
        "workforce" => {
            Pipeline::new()
                .before(DesignerPhase)
                .execute(dag, step, dag_state)
                .await
        }
        _ if step.child_workflow_id.is_some() => {
            warn!(
                step_id = %step.id,
                mode = %step.execution_mode,
                "Step has child_workflow_id but non-workforce mode — routing as workforce"
            );
            Pipeline::new()
                .before(DesignerPhase)
                .execute(dag, step, dag_state)
                .await
        }
        _ => execute_with_agent(dag, dag_state, step).await,
    }
}

/// Context/input pass-through — forward prompt_template as output with no LLM call.
async fn execute_passthrough(
    dag: &DagContext<'_>,
    dag_state: &mut DagExecutionState,
    step: &WorkflowStepRow,
) -> Result<(), HubError> {
    let step_start = std::time::Instant::now();
    let output_key = resolve_output_key(step, &dag.port_meta.step_outputs);
    let content = passthrough_content(step, &dag.ctx.initial_input);
    let (output, value) = StepOutput::passthrough(output_key, content);
    let envelope = wrap_in_agentless_envelope(step.id, Some(value), 0, 0, 0, 0.0);

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
            duration_ms: Some(step_start.elapsed().as_millis() as u64),
        },
    );

    info!(step_id = %step.id, "Context step pass-through completed");
    Ok(())
}

/// Load agent, resolve LLM provider, and execute via the engine.
async fn execute_with_agent(
    dag: &DagContext<'_>,
    dag_state: &mut DagExecutionState,
    step: &WorkflowStepRow,
) -> Result<(), HubError> {
    let agent_id = step.agent_id.ok_or_else(|| {
        HubError::Internal(anyhow::anyhow!(
            "step {} has no agent_id for mode '{}'",
            step.id,
            step.execution_mode
        ))
    })?;

    let agent = load_agent(dag, step.id, agent_id).await?;
    let step_engine = resolve_provider(dag, step.id, &agent).await?;
    let effective_engine = step_engine.as_ref().unwrap_or(dag.engine);

    let step_dag = DagContext {
        engine: effective_engine,
        ..*dag
    };

    execute_single_step(&step_dag, step, &agent, dag_state).await
}

/// Load an agent from snapshot or live DB.
async fn load_agent(
    dag: &DagContext<'_>,
    step_id: Uuid,
    agent_id: Uuid,
) -> Result<crate::db::AgentRow, HubError> {
    if let Some(snap) = &dag.ctx.snapshot {
        snap.agents
            .get(&agent_id)
            .cloned()
            .ok_or(HubError::AgentNotFound { step_id, agent_id })
    } else {
        dag.state
            .repos()
            .agents
            .get_persisted_agent(agent_id)
            .await
            .map_err(|e| anyhow::anyhow!("failed to load agent: {}", e))?
            .ok_or(HubError::AgentNotFound { step_id, agent_id })
    }
}

/// Resolve the LLM provider for an agent. Returns `None` for the default provider.
async fn resolve_provider(
    dag: &DagContext<'_>,
    step_id: Uuid,
    agent: &crate::db::AgentRow,
) -> Result<Option<ExecutionEngine>, HubError> {
    if agent.model_provider.is_empty() || agent.model_provider == crate::constants::ACTIVE_PROVIDER
    {
        return Ok(None);
    }

    if agent.model_provider == "ollama" && !dag.state.is_ollama_enabled().await {
        return Err(HubError::ProviderUnavailable {
            provider: agent.model_provider.clone(),
            step_id,
            agent_name: agent.name.clone(),
        });
    }

    let provider = dag
        .state
        .provider_for(&agent.model_provider)
        .ok_or_else(|| HubError::ProviderUnavailable {
            provider: agent.model_provider.clone(),
            step_id,
            agent_name: agent.name.clone(),
        })?;

    Ok(Some(ExecutionEngine::new(
        provider,
        dag.state.env().debug_stream,
    )))
}

// ── Pinned Replay ───────────────────────────────────────────────────────────

/// Attempt to replay a pinned step's last output. Returns `true` if replayed.
async fn try_replay_pinned(
    dag: &DagContext<'_>,
    dag_state: &mut DagExecutionState,
    step: &WorkflowStepRow,
) -> Result<bool, HubError> {
    let Some((output, envelope)) = load_pinned_output(dag, step).await? else {
        return Ok(false);
    };

    dag_state.record_step_output(step.id, output, envelope);
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
            let content = passthrough_content(step, &dag.ctx.initial_input);
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

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Resolve the content for a passthrough step: prompt_template if set, otherwise initial_input.
fn passthrough_content(step: &WorkflowStepRow, initial_input: &str) -> String {
    if step.prompt_template.is_empty() {
        initial_input.to_owned()
    } else {
        step.prompt_template.clone()
    }
}

/// Spawn a background run results summarization if the step has completed output.
fn spawn_summarizer_if_completed(state: &AppState, step_id: Uuid, dag_state: &DagExecutionState) {
    if let Some(output) = dag_state.completed.get(&step_id) {
        if !output.raw_output.is_empty() {
            crate::server::hub::run_results::spawn_run_results_summary(
                state.clone(),
                state.run_results_tokens(),
                step_id,
                output.raw_output.clone(),
            );
        }
    }
}
