//! Resume / approval orchestration for paused DAG workflows.
//!
//! Contains two entry points:
//! - `resume_workflow_via_engine` — re-enters the DAG main loop with
//!   pre-completed state and continues executing downstream steps.
//! - `resume_dag_from_approval` — top-level handler that reconstructs
//!   completed state from the database, injects the approved output,
//!   and delegates to `resume_workflow_via_engine`.

use std::collections::{HashMap, HashSet};

use anyhow::anyhow;
use serde_json::Value as JsonValue;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::db::traits::WorkflowCollectionRepo;
use crate::db::{WorkflowStepEdgeRow, WorkflowStepRow};
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::error::HubError;
use crate::server::state::AppState;
use crate::server::ws::events::WorkflowEventKind;
use crate::types::{ExecutionMetadata, ExecutionStatus, StepExecutionEnvelope};

use super::dag_state::{prefetch_port_metadata, resolve_output_key};
use super::{
    broadcast_workflow_event, get_parent_steps, topological_sort, StepOutput,
    WorkflowExecutionContext, WorkflowExecutionResult,
};

// Step execution functions from sibling modules
use super::for_each::{detect_for_each_chains, execute_for_each_chain, execute_for_each_step};
use super::room_step::execute_room_step;
use super::single::execute_single_step;

/// Resume a workflow DAG from a paused state after an interactive step is approved.
///
/// Reconstructs the completed step outputs from agent_executions in the DB,
/// injects the approved output for the paused step, then continues executing
/// downstream steps.
pub async fn resume_workflow_via_engine(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    steps: &[WorkflowStepRow],
    edges: &[WorkflowStepEdgeRow],
    pre_completed: HashMap<Uuid, StepOutput>,
    pre_var_outputs: HashMap<String, JsonValue>,
    cancel: Option<&CancellationToken>,
) -> Result<WorkflowExecutionResult, HubError> {
    let start_time = std::time::Instant::now();
    let sorted = topological_sort(steps, edges).map_err(|_| HubError::DagCycle)?;
    let step_map: HashMap<Uuid, &WorkflowStepRow> = steps.iter().map(|s| (s.id, s)).collect();

    let mut completed = pre_completed;
    let mut var_outputs = pre_var_outputs;
    let mut completed_envelopes: HashMap<Uuid, StepExecutionEnvelope> = HashMap::new();
    let mut total_input_tokens: i64 = 0;
    let mut total_output_tokens: i64 = 0;
    let mut total_cost_usd: f32 = 0.0;

    // Build synthetic envelopes for pre-completed steps
    for (step_id, output) in &completed {
        let envelope = StepExecutionEnvelope {
            status: if output.structured_output.is_some() {
                ExecutionStatus::Success
            } else {
                ExecutionStatus::Error
            },
            data: output.structured_output.clone(),
            metadata: ExecutionMetadata {
                execution_id: *step_id,
                execution_time_ms: 0,
                tokens_in: None,
                tokens_out: None,
                cost_usd: None,
                model: None,
                agent_id: None,
                iteration_index: None,
                iteration_label: None,
                routing_label: None,

                upstream_agent_id: None,
                upstream_routing_label: None,
                room_session_id: None,
                room_id: None,
                total_rounds: None,
            },
            error: None,
        };
        completed_envelopes.insert(*step_id, envelope);
    }

    // Pre-fetch port metadata
    let port_meta = prefetch_port_metadata(state, steps).await;

    // Phase 6B: Detect chained for-each pipelines
    let chains = detect_for_each_chains(steps, edges);
    let chain_member_set: HashSet<Uuid> = chains
        .iter()
        .flat_map(|c| c.step_ids.iter().copied())
        .collect();

    for step_id in &sorted {
        // Skip already-completed steps
        if completed.contains_key(step_id) {
            continue;
        }

        let step = match step_map.get(step_id) {
            Some(s) => *s,
            None => continue,
        };

        if cancel.is_some_and(|t| t.is_cancelled()) {
            return Err(HubError::Cancelled);
        }

        let parents = get_parent_steps(*step_id, edges);
        let all_parents_done = parents.iter().all(|pid| completed.contains_key(pid));
        if !all_parents_done {
            warn!("Step {} has uncompleted parents, skipping", step_id);
            continue;
        }

        // Phase 6B: If this is the head of a for-each chain, execute the whole chain
        if let Some(chain) = chains.iter().find(|c| c.step_ids[0] == *step_id) {
            execute_for_each_chain(
                engine,
                state,
                ctx,
                chain,
                &step_map,
                edges,
                &mut var_outputs,
                &mut completed,
                &mut completed_envelopes,
                &port_meta,
                &mut total_input_tokens,
                &mut total_output_tokens,
                &mut total_cost_usd,
                cancel,
            )
            .await?;
            continue;
        }

        // Skip non-head chain members (already executed by chain head)
        if chain_member_set.contains(step_id) {
            continue;
        }

        let agent_id = step.agent_id.ok_or_else(|| {
            HubError::Internal(anyhow::anyhow!(
                "step {} has no agent_id for mode '{}'",
                step_id,
                step.execution_mode
            ))
        })?;
        let agent = state
            .repo()
            .get_persisted_agent(agent_id)
            .await
            .map_err(|e| anyhow::anyhow!("failed to load agent: {}", e))?
            .ok_or_else(|| HubError::AgentNotFound {
                step_id: *step_id,
                agent_id,
            })?;

        if step.execution_mode == "room" {
            execute_room_step(
                engine,
                state,
                ctx,
                step,
                steps,
                edges,
                &mut var_outputs,
                &mut completed,
                &mut completed_envelopes,
                &port_meta,
                &mut total_input_tokens,
                &mut total_output_tokens,
                &mut total_cost_usd,
                cancel,
            )
            .await?;
        } else if step.execution_mode == "for_each" {
            execute_for_each_step(
                engine,
                state,
                ctx,
                step,
                &agent,
                steps,
                edges,
                &mut var_outputs,
                &mut completed,
                &mut completed_envelopes,
                &port_meta,
                &mut total_input_tokens,
                &mut total_output_tokens,
                &mut total_cost_usd,
                cancel,
            )
            .await?;
        } else {
            execute_single_step(
                engine,
                state,
                ctx,
                step,
                &agent,
                steps,
                edges,
                &mut var_outputs,
                &mut completed,
                &mut completed_envelopes,
                &port_meta,
                &mut total_input_tokens,
                &mut total_output_tokens,
                &mut total_cost_usd,
                cancel,
            )
            .await?;
        }
    }

    let final_outputs: HashMap<String, StepOutput> = completed
        .into_iter()
        .map(|(id, out)| (id.to_string(), out))
        .collect();

    Ok(WorkflowExecutionResult {
        outputs: final_outputs,
        total_input_tokens,
        total_output_tokens,
        total_cost_usd,
        duration_ms: start_time.elapsed().as_millis() as u64,
    })
}

/// Orchestrator: resume the DAG after an interactive step is approved.
///
/// Finds the paused workflow execution, reconstructs completed state from
/// agent_executions in the DB, then continues executing downstream steps.
pub async fn resume_dag_from_approval(
    state: &AppState,
    paused_step_id: Uuid,
    approved_output: StepOutput,
) -> Result<(), HubError> {
    let wf_repo = &state.repos().workflows;
    let ae_repo = &state.repos().agent_executions;

    // Load the step to get workflow_id
    let step = wf_repo
        .get_step(paused_step_id)
        .await
        .map_err(|e| HubError::Internal(anyhow!("step load failed: {}", e)))?
        .ok_or_else(|| HubError::Internal(anyhow!("step {} not found", paused_step_id)))?;

    // Load workflow steps and edges
    let steps = wf_repo
        .list_steps(step.workflow_id)
        .await
        .map_err(|e| HubError::Internal(anyhow!("steps load failed: {}", e)))?;
    let edges = wf_repo
        .list_edges(step.workflow_id)
        .await
        .map_err(|e| HubError::Internal(anyhow!("edges load failed: {}", e)))?;

    let port_meta = prefetch_port_metadata(state, &steps).await;

    // Find the paused workflow_execution via agent_executions
    let step_ids: Vec<Uuid> = steps.iter().map(|s| s.id).collect();
    let completed_executions = ae_repo
        .list_completed_executions_for_step_ids(&step_ids)
        .await
        .map_err(|e| HubError::Internal(anyhow!("failed to load completed executions: {}", e)))?;

    let workflow_execution_id = completed_executions
        .iter()
        .find_map(|ae| ae.workflow_execution_id);

    let user_id = completed_executions
        .first()
        .map(|ae| ae.agent_id)
        .unwrap_or(Uuid::nil());

    // Reconstruct completed step outputs from DB
    let step_map: HashMap<Uuid, &WorkflowStepRow> = steps.iter().map(|s| (s.id, s)).collect();
    let mut completed: HashMap<Uuid, StepOutput> = HashMap::new();
    let mut var_outputs: HashMap<String, JsonValue> = HashMap::new();

    for ae in &completed_executions {
        if let Some(ae_step_id) = ae.workflow_step_id {
            if let Some(ws) = step_map.get(&ae_step_id) {
                let variable_name = resolve_output_key(ws, &port_meta.step_outputs);
                let output = StepOutput {
                    variable_name: variable_name.clone(),
                    raw_output: ae.output.clone().unwrap_or_default(),
                    structured_output: ae.structured_output.clone(),
                };
                if !variable_name.is_empty() {
                    if let Some(structured) = &output.structured_output {
                        var_outputs.insert(variable_name, structured.clone());
                    }
                }
                completed.insert(ae_step_id, output);
            }
        }
    }

    // Inject the approved step's output
    let approved_var_name = resolve_output_key(&step, &port_meta.step_outputs);
    let mut approved_output = approved_output;
    approved_output.variable_name = approved_var_name.clone();
    if !approved_var_name.is_empty() {
        if let Some(structured) = &approved_output.structured_output {
            var_outputs.insert(approved_var_name, structured.clone());
        }
    }
    completed.insert(paused_step_id, approved_output);

    // Broadcast: workflow resumed
    let workflow_id = step.workflow_id;

    // Build execution context
    let ctx = WorkflowExecutionContext {
        stage_execution_id: workflow_execution_id.unwrap_or(Uuid::new_v4()),
        run_id: Uuid::new_v4(),
        user_id,
        initial_input: String::new(),
        prior_outputs: HashMap::new(),
        execution_context: None,
        container_config: None,
        wg_client: None,
    };

    // Create engine and resume
    let provider = state
        .provider()
        .ok_or(HubError::ProviderNotConfigured)?
        .clone();
    let engine = ExecutionEngine::new(provider);

    // Broadcast: resumed
    broadcast_workflow_event(
        state,
        &ctx,
        workflow_id,
        WorkflowEventKind::Resumed {
            step_id: paused_step_id,
        },
    );

    info!(
        paused_step_id = %paused_step_id,
        completed_steps = completed.len(),
        remaining_steps = steps.len() - completed.len(),
        "Resuming DAG after interactive approval"
    );

    let result = resume_workflow_via_engine(
        &engine,
        state,
        &ctx,
        &steps,
        &edges,
        completed,
        var_outputs,
        None,
    )
    .await;

    // Update workflow_execution status if we know which one it is
    if let Some(wf_exec_id) = workflow_execution_id {
        if let Some(db) = state.db() {
            let coll_repo = crate::db::pg_repo::PgRepo::new(db.clone());
            match &result {
                Ok(wf_result) => {
                    let outputs_json: Option<JsonValue> = {
                        let mut map = serde_json::Map::new();
                        for (step_id_str, output) in &wf_result.outputs {
                            let val = output
                                .structured_output
                                .clone()
                                .unwrap_or_else(|| JsonValue::String(output.raw_output.clone()));
                            map.insert(step_id_str.clone(), val);
                        }
                        Some(JsonValue::Object(map))
                    };
                    let _ = coll_repo
                        .update_workflow_execution_status(
                            wf_exec_id,
                            "completed",
                            outputs_json,
                            None,
                        )
                        .await;

                    info!(
                        workflow_execution_id = %wf_exec_id,
                        "Resumed workflow execution completed"
                    );
                }
                Err(HubError::AwaitingUser { .. }) => {
                    info!(
                        workflow_execution_id = %wf_exec_id,
                        "Resumed workflow hit another interactive step, re-pausing"
                    );
                }
                Err(e) => {
                    // Broadcast: workflow failed
                    broadcast_workflow_event(
                        state,
                        &ctx,
                        workflow_id,
                        WorkflowEventKind::Failed {
                            error: format!("{}", e),
                        },
                    );

                    let _ = coll_repo
                        .update_workflow_execution_status(
                            wf_exec_id,
                            "failed",
                            None,
                            Some(format!("{}", e)),
                        )
                        .await;
                    error!(
                        workflow_execution_id = %wf_exec_id,
                        error = %e,
                        "Resumed workflow execution failed"
                    );
                }
            }
        }
    }

    result.map(|_| ())
}
