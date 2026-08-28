//! Core DAG orchestration — the inner engine of workflow execution.
//!
//! `run_dag_loop` iterates topologically-sorted steps, applying guard
//! clauses (cancellation, pinned replay, dead-path elimination, conditional
//! edges) before routing each step to its executor. Submodules handle
//! step dispatch, overlay persistence, and pinned output replay.

mod dispatch;
mod overlay;
mod replay;

use std::collections::{HashMap, HashSet};

use tracing::{debug, error, info, warn};
use uuid::Uuid;

use tokio_util::sync::CancellationToken;

use crate::db::{WorkflowStepEdgeRow, WorkflowStepRow};
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::error::HubError;
use crate::server::state::AppState;
use crate::server::ws::events::WorkflowEventKind;

use super::broadcast::broadcast_workflow_event;
use super::{
    broadcast_step_failure_if_real, check_step_readiness, compute_dead_path_steps,
    prefetch_port_metadata, topological_sort_levels, DagContext, DagExecutionState, StepOutput,
    StepReadiness, WorkflowExecutionContext, WorkflowExecutionResult,
};

use dispatch::{dispatch_step, spawn_summarizer_if_completed};
use overlay::{
    capture_pinned_step_files, merge_and_persist_overlays, persist_step_overlay_if_present,
};
use replay::try_replay_pinned;

// ── Public Entry Point ────────────────────────────────────────────────────

/// Execute a complete workflow DAG using the unified ExecutionEngine.
///
/// Runs topological sort, resolves variables and port-based data flow,
/// then dispatches each step by execution mode: passthrough, pipeline,
/// or agent-based. Supports pinned replay, dead-path elimination, and
/// conditional edge routing.
pub async fn execute_workflow_via_engine(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    steps: &[WorkflowStepRow],
    edges: &[WorkflowStepEdgeRow],
    cancel: Option<&CancellationToken>,
) -> Result<WorkflowExecutionResult, HubError> {
    let workflow_id = steps.first().map(|s| s.workflow_id).unwrap_or(Uuid::nil());
    let start_time = std::time::Instant::now();

    // Pre-fetch port metadata: from snapshot if template-based, otherwise from live DB
    let port_meta = match &ctx.snapshot {
        Some(snap) => super::templates::port_metadata_from_snapshot(snap),
        None => prefetch_port_metadata(state, steps, edges).await,
    };

    let mut dag_state = DagExecutionState::new();

    // Broadcast: workflow started (compute step count from levels to avoid double sort)
    let levels = topological_sort_levels(steps, edges).map_err(|_| HubError::DagCycle)?;
    let sorted_len: usize = levels.iter().map(|l| l.len()).sum();

    broadcast_workflow_event(
        state,
        ctx,
        workflow_id,
        WorkflowEventKind::Started {
            total_steps: sorted_len,
        },
    );

    let dag = DagContext {
        engine,
        state,
        ctx,
        steps,
        edges,
        port_meta: &port_meta,
        cancel,
    };
    run_dag_loop(&dag, &mut dag_state, &levels).await?;

    let duration_ms = start_time.elapsed().as_millis() as u64;

    let final_outputs: HashMap<String, StepOutput> = dag_state
        .completed
        .into_iter()
        .map(|(id, out)| (id.to_string(), out))
        .collect();

    Ok(WorkflowExecutionResult {
        outputs: final_outputs,
        total_input_tokens: dag_state.total_input_tokens,
        total_output_tokens: dag_state.total_output_tokens,
        total_cost_usd: dag_state.total_cost_usd,
        duration_ms,
    })
}

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
    levels: &[Vec<Uuid>],
) -> Result<(), HubError> {
    let step_map: HashMap<Uuid, &WorkflowStepRow> = dag.steps.iter().map(|s| (s.id, s)).collect();

    let Some(workflow_id) = dag.steps.first().map(|s| s.workflow_id) else {
        return Ok(()); // Empty DAG — nothing to execute.
    };

    // C3: Fresh workspace per run + pre-load pinned step files
    if let Some(ws) = dag.state.workspace() {
        if let Some(cc) = dag.ctx.container_config.as_ref() {
            if let (Some(wf_id), Some(run_id)) = (cc.workflow_id, cc.run_id) {
                // Clear stale workspace from previous runs
                let ws_init = ws.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    if ws_init.workspace_exists(wf_id, run_id) {
                        let _ = ws_init.destroy_run_workspace(wf_id, run_id);
                    }
                    ws_init.create_run_workspace(wf_id, run_id)
                })
                .await;

                // Pre-load pinned step files (topo order)
                for step in dag.steps.iter().filter(|s| s.pinned) {
                    let pinned_dir = ws.pinned_step_path(wf_id, step.id);
                    if pinned_dir.exists() {
                        let ws_pin = ws.clone();
                        let sid = step.id;
                        match tokio::task::spawn_blocking(move || {
                            ws_pin.preload_pinned_files(wf_id, run_id, sid)
                        })
                        .await
                        {
                            Ok(Ok(n)) if n > 0 => {
                                info!(
                                    step_id = %step.id,
                                    files = n,
                                    "Pre-loaded pinned step files into workspace"
                                );
                            }
                            Ok(Err(e)) => {
                                warn!(
                                    step_id = %step.id,
                                    error = %e,
                                    "Failed to pre-load pinned step files"
                                );
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    let dead_path_steps = compute_dead_path_steps(dag.steps, dag.edges);
    if !dead_path_steps.is_empty() {
        info!(
            count = dead_path_steps.len(),
            "Dead-path steps identified (all consumers pinned)"
        );
    }

    for level in levels {
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

            // Persist before propagating. The executor populates
            // `dag_state.step_overlay` whether or not the step succeeded, and a
            // partial deliverable beats no deliverable — this `?` sitting above
            // the persist call is the second of the three places run dd27d008's
            // homepage was discarded.
            persist_step_overlay_if_present(dag, dag_state).await;

            if let Err(ref e) = step_result {
                broadcast_step_failure_if_real(dag.state, dag.ctx, workflow_id, step, e);
            }
            step_result?;

            // Pinning stays below the `?`: pinning a failed step's files would
            // preload broken state into every future run.
            if step.pinned {
                capture_pinned_step_files(dag, step.id).await;
            }

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

        // Conditional edge readiness — use pre-built incoming edge index
        let incoming = dag
            .port_meta
            .incoming_edges
            .get(&step_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        match check_step_readiness(
            step_id,
            incoming,
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
/// Overlay diffs are collected from each task and merged before persisting.
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
            let overlay = task_state.step_overlay.take();
            (step_id, step, result, task_state, overlay)
        });
    }

    // Collect results and overlay diffs.
    //
    // The error arm used to bind `_overlay`, drop it, and return immediately —
    // so a level where one step failed lost every step's files, including the
    // failing step's own partial work. Record the first real error instead,
    // drain the set fully, persist what survived, and propagate afterwards.
    let mut overlays: Vec<super::merge::types::StepOverlay> = Vec::new();
    let mut first_error: Option<HubError> = None;

    while let Some(join_result) = join_set.join_next().await {
        match join_result {
            Ok((step_id, _step, Ok(()), task_state, overlay)) => {
                dag_state.merge_parallel_result(task_state);
                if let Some(ov) = overlay {
                    overlays.push(ov);
                }
                spawn_summarizer_if_completed(dag.state, step_id, dag_state);
            }
            Ok((_step_id, step, Err(e), _task_state, overlay)) => {
                // Keep the failing step's overlay: it may hold most of the work.
                if let Some(ov) = overlay {
                    overlays.push(ov);
                }
                broadcast_step_failure_if_real(dag.state, dag.ctx, workflow_id, &step, &e);
                if first_error.is_none() {
                    first_error = Some(e);
                }
                join_set.abort_all();
            }
            Err(join_err) => {
                // After `abort_all()` every remaining sibling yields
                // `JoinError::Cancelled`. Those must not overwrite the real
                // error, nor invent one when the level was otherwise fine.
                if join_err.is_cancelled() {
                    continue;
                }
                error!("DAG step task panicked: {}", join_err);
                if first_error.is_none() {
                    first_error = Some(HubError::Internal(anyhow::anyhow!(
                        "DAG step task panicked: {}",
                        join_err
                    )));
                }
                join_set.abort_all();
            }
        }
    }

    // Merge and persist parallel overlays — on the failure path too.
    if !overlays.is_empty() {
        merge_and_persist_overlays(dag, &mut overlays).await;
    }

    match first_error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}
