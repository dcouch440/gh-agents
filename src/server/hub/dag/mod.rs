//! DAG orchestration — topological sort, variable resolution, for-each fan-out,
//! port-based data flow, and workflow execution using the unified ExecutionEngine.
//!
//! Pure utility functions live in the `utils` submodule and are re-exported here.
//! Execution functions use the hub's `ExecutionEngine` for step execution.

use std::collections::HashMap;
use std::collections::HashSet;

use serde_json::Value as JsonValue;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use uuid::Uuid;

use crate::db::{WorkflowStepEdgeRow, WorkflowStepRow};
use crate::server::state::AppState;
use crate::server::ws::events::{WorkflowEvent, WorkflowEventKind};
use crate::types::{DownstreamRoutingContext, RouteDescription, StepExecutionEnvelope};

use super::engine::ExecutionEngine;
use super::error::HubError;

/// Emit a workflow lifecycle event via WebSocket broadcast.
///
/// When executing inside a sub-workflow (i.e. `ctx.parent_context` is set),
/// step-level events are also relayed to the parent's channel as
/// `SubWorkflowStepProgress` so the frontend can render nested execution.
pub fn broadcast_workflow_event(
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    workflow_id: Uuid,
    kind: WorkflowEventKind,
) {
    // If executing inside a sub-workflow, relay step events to parent's channel
    if let Some(parent) = &ctx.parent_context {
        if let Some(relay_kind) = build_parent_relay(&kind, parent, ctx.run_id) {
            state.broadcast_workflow(WorkflowEvent {
                run_id: Some(parent.parent_run_id),
                workflow_id: parent.parent_workflow_id,
                user_id: Some(ctx.user_id),
                kind: relay_kind,
            });
        }
    }

    // Broadcast original event on the current channel
    state.broadcast_workflow(WorkflowEvent {
        run_id: Some(ctx.run_id),
        workflow_id,
        user_id: Some(ctx.user_id),
        kind,
    });
}

/// Build a `SubWorkflowStepProgress` relay event for the parent's channel.
///
/// Returns `Some` for step lifecycle events (started/completed/failed),
/// `None` for workflow-level or progress events.
pub(crate) fn build_parent_relay(
    kind: &WorkflowEventKind,
    parent: &utils::SubWorkflowParentContext,
    child_execution_id: Uuid,
) -> Option<WorkflowEventKind> {
    match kind {
        WorkflowEventKind::StepStarted {
            step_id, step_name, ..
        } => Some(WorkflowEventKind::SubWorkflowStepProgress {
            parent_step_id: parent.parent_step_id,
            child_execution_id,
            child_step_id: *step_id,
            child_step_name: step_name.clone(),
            status: "started".into(),
            input_tokens: None,
            output_tokens: None,
            duration_ms: None,
            error: None,
        }),
        WorkflowEventKind::StepCompleted {
            step_id,
            step_name,
            input_tokens,
            output_tokens,
            duration_ms,
            ..
        } => Some(WorkflowEventKind::SubWorkflowStepProgress {
            parent_step_id: parent.parent_step_id,
            child_execution_id,
            child_step_id: *step_id,
            child_step_name: step_name.clone(),
            status: "completed".into(),
            input_tokens: *input_tokens,
            output_tokens: *output_tokens,
            duration_ms: *duration_ms,
            error: None,
        }),
        WorkflowEventKind::StepFailed {
            step_id,
            step_name,
            error,
        } => Some(WorkflowEventKind::SubWorkflowStepProgress {
            parent_step_id: parent.parent_step_id,
            child_execution_id,
            child_step_id: *step_id,
            child_step_name: step_name.clone(),
            status: "failed".into(),
            input_tokens: None,
            output_tokens: None,
            duration_ms: None,
            error: Some(error.clone()),
        }),
        _ => None,
    }
}

// ── Submodules ──────────────────────────────────────────────────────────────

pub(crate) mod agent_designer;
pub(crate) mod belief_capture;
pub(crate) mod container;
pub(crate) mod dag_state;
pub(crate) mod designer_input;
pub(crate) mod for_each;
pub(crate) mod resume;
pub(crate) mod room_step;
pub(crate) mod single;
pub(crate) mod staging;
pub(crate) mod sub_workflow;
pub mod templates;
pub(crate) mod utils;
pub(crate) mod versioning;
pub(crate) mod workforce;

pub(crate) use dag_state::{
    broadcast_step_failure_if_real, build_incoming_edge_index, prefetch_port_metadata,
    resolve_output_key, resolve_step_port_inputs, step_display_name, wrap_in_agentless_envelope,
    wrap_in_envelope, DagContext, DagExecutionState, PortMetadata,
};

pub use utils::{
    build_routing_instruction_block, check_step_readiness, collect_upstream_context_data,
    compute_dead_path_steps, evaluate_edge_condition, extract_for_each_label, find_entry_steps,
    get_child_steps, get_parent_steps, resolve_dot_path, resolve_for_each_array,
    resolve_port_inputs, resolve_variables, topological_sort, ContainerExecutionConfig, DagPaused,
    PortResolutionError, StepOutput, StepReadiness, WorkflowExecutionContext,
    WorkflowExecutionResult,
};
pub(crate) use utils::{compose_prompt, PromptRepos};

// Re-export public functions from submodules
pub use resume::{resume_dag_from_approval, resume_workflow_via_engine, ResumeState};

// Internal imports for the main orchestration loop
use belief_capture::execute_belief_capture_step;

use for_each::{detect_for_each_chains, execute_for_each_chain, execute_for_each_step};
use room_step::execute_room_step;
use single::execute_single_step;
use sub_workflow::execute_sub_workflow_step;
use workforce::execute_workforce_step;

// ── Routing Context ─────────────────────────────────────────────────────────

/// For a given step, find downstream label-routing steps and build
/// routing context for prompt injection.
///
/// Uses edges (in memory), the step map, and pre-fetched routing rules.
/// Loads agent names and tools from the database on demand.
async fn gather_downstream_routing_context(
    step_id: Uuid,
    edges: &[WorkflowStepEdgeRow],
    step_map: &HashMap<Uuid, &WorkflowStepRow>,
    port_meta: &PortMetadata,
    state: &AppState,
) -> Vec<DownstreamRoutingContext> {
    let mut contexts = Vec::new();

    let child_step_ids = get_child_steps(step_id, edges);

    for child_id in child_step_ids {
        let Some(child_step) = step_map.get(&child_id) else {
            continue;
        };

        if child_step.routing_mode.as_deref() != Some("label") {
            continue;
        }

        let Some(routing_field) = child_step.routing_field.as_ref() else {
            continue;
        };

        let Some(rules) = port_meta.routing_rules.get(&child_id) else {
            continue;
        };

        if rules.is_empty() {
            continue;
        }

        let mut routes = Vec::new();
        for rule in rules {
            let agent_name = match state
                .repos()
                .agents
                .get_persisted_agent(rule.agent_id)
                .await
            {
                Ok(Some(agent)) => agent.name,
                _ => format!("Agent {}", rule.agent_id),
            };

            let agent_tools = match state.repos().tools.get_agent_tools(rule.agent_id).await {
                Ok(tools) => tools.into_iter().map(|t| t.name).collect(),
                Err(_) => vec![],
            };

            routes.push(RouteDescription {
                label_value: rule.label_value.clone(),
                description: rule.description.clone(),
                agent_name,
                agent_tools,
            });
        }

        contexts.push(DownstreamRoutingContext {
            downstream_step_id: child_id,
            routing_field: routing_field.clone(),
            routes,
        });
    }

    contexts
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
            let value = JsonValue::String(content.clone());
            let output = StepOutput {
                variable_name: output_key,
                structured_output: Some(value.clone()),
                raw_output: content,
            };
            let envelope = wrap_in_agentless_envelope(step.id, Some(value), 0, 0, 0, 0.0);
            Ok(Some((output, envelope)))
        }
        _ => {
            // For single/other modes: load last envelope from DB
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
                            .map(|d| serde_json::to_string(d).unwrap_or_default())
                            .unwrap_or_default(),
                    };
                    Ok(Some((output, envelope)))
                }
                None => Ok(None),
            }
        }
    }
}

// ── Shared DAG Loop ─────────────────────────────────────────────────────────

/// Core step dispatch loop shared by both fresh execution and resume paths.
///
/// Iterates through topologically-sorted steps, executing each according to its
/// mode. Handles cancellation, conditional edges, for-each chains, and
/// provider resolution for non-default LLM providers.
async fn run_dag_loop(
    dag: &DagContext<'_>,
    dag_state: &mut DagExecutionState,
) -> Result<(), HubError> {
    let sorted = topological_sort(dag.steps, dag.edges).map_err(|_| HubError::DagCycle)?;
    let step_map: HashMap<Uuid, &WorkflowStepRow> = dag.steps.iter().map(|s| (s.id, s)).collect();
    let workflow_id = dag
        .steps
        .first()
        .map(|s| s.workflow_id)
        .unwrap_or(Uuid::nil());

    // Dead-path elimination: skip non-pinned steps whose output has no unpinned consumers
    let dead_path_steps = compute_dead_path_steps(dag.steps, dag.edges);
    if !dead_path_steps.is_empty() {
        info!(
            count = dead_path_steps.len(),
            "Dead-path steps identified (all consumers pinned)"
        );
    }

    // Phase 6B: Detect chained for-each pipelines
    let chains = detect_for_each_chains(dag.steps, dag.edges);
    let chain_member_set: HashSet<Uuid> = chains
        .iter()
        .flat_map(|c| c.step_ids.iter().copied())
        .collect();
    let chain_by_head: HashMap<Uuid, _> = chains.iter().map(|c| (c.step_ids[0], c)).collect();

    if !chains.is_empty() {
        info!(
            chain_count = chains.len(),
            "Detected chained for-each pipelines"
        );
    }

    for step_id in &sorted {
        // Skip steps already executed as part of a chain
        if dag_state.completed.contains_key(step_id) {
            continue;
        }

        let step = match step_map.get(step_id) {
            Some(s) => *s,
            None => continue,
        };

        // Check cancellation before each step
        if dag.cancel.is_some_and(|t| t.is_cancelled()) {
            return Err(HubError::Cancelled);
        }

        // Pinned steps replay their last output — skip execution entirely
        if step.pinned {
            if let Some((output, envelope)) = load_pinned_output(dag, step).await? {
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
                continue;
            }
            // No prior output for single step: warn and fall through to normal execution
            warn!(step_id = %step.id, "Pinned step has no prior output, executing normally");
        }

        // Dead-path elimination: skip steps whose output has no unpinned consumers
        if dead_path_steps.contains(step_id) {
            dag_state
                .completed
                .insert(*step_id, StepOutput::skipped(*step_id));
            info!(step_id = %step.id, "Dead-path step skipped (all consumers pinned)");
            continue;
        }

        // Check step readiness (handles conditional edges)
        match check_step_readiness(
            *step_id,
            dag.edges,
            &dag_state.completed,
            &dag_state.completed_envelopes,
        ) {
            StepReadiness::Waiting => {
                warn!("Step {} has uncompleted parents, skipping", step_id);
                continue;
            }
            StepReadiness::Skipped => {
                info!("Step {} skipped — no matching conditional edges", step_id);
                dag_state
                    .completed
                    .insert(*step_id, StepOutput::skipped(*step_id));
                continue;
            }
            StepReadiness::Ready => { /* proceed with execution */ }
        }

        // Phase 6B: If this is the head of a for-each chain, execute the whole chain
        if let Some(chain) = chain_by_head.get(step_id) {
            execute_for_each_chain(dag, chain, &step_map, dag_state).await?;
            continue;
        }

        // Skip non-head chain members (already executed by chain head)
        if chain_member_set.contains(step_id) {
            continue;
        }

        // Context / input steps pass through their prompt_template as output — no LLM call
        if step.execution_mode == "context" || step.execution_mode == "input" {
            let step_start = std::time::Instant::now();
            let output_key = resolve_output_key(step, &dag.port_meta.step_outputs);
            let content = if step.prompt_template.is_empty() {
                dag.ctx.initial_input.clone()
            } else {
                step.prompt_template.clone()
            };
            let value = JsonValue::String(content.clone());

            let output = StepOutput {
                variable_name: output_key,
                structured_output: Some(value.clone()),
                raw_output: content,
            };

            let envelope = wrap_in_agentless_envelope(step.id, Some(value), 0, 0, 0, 0.0);

            // Snapshot envelope for run history
            let envelope_json = serde_json::to_string(&envelope).unwrap_or_default();
            dag_state.record_step_output(step.id, output, envelope);
            let _ = versioning::snapshot_content(
                &*dag.state.repos().content_versions,
                dag.ctx.run_id,
                step.id,
                step.id,
                versioning::content_types::ENVELOPE,
                "output",
                &envelope_json,
            )
            .await;

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

            spawn_summarizer_if_completed(dag.state, step.id, dag_state);

            info!(step_id = %step.id, "Context step pass-through completed");
            continue;
        }

        // Belief capture steps — per-source LLM extraction, no agent_id needed
        if step.execution_mode == "belief_capture" {
            let step_result = execute_belief_capture_step(dag, step, dag_state).await;

            if let Err(ref e) = step_result {
                broadcast_step_failure_if_real(dag.state, dag.ctx, workflow_id, step, e);
            }
            step_result?;
            spawn_summarizer_if_completed(dag.state, step.id, dag_state);
            continue;
        }

        // Sub-workflow steps — execute child workflow from template, no agent needed
        if step.execution_mode == "sub_workflow" {
            let step_result = execute_sub_workflow_step(dag, step, dag_state).await;

            if let Err(ref e) = step_result {
                broadcast_step_failure_if_real(dag.state, dag.ctx, workflow_id, step, e);
            }
            step_result?;
            spawn_summarizer_if_completed(dag.state, step.id, dag_state);
            continue;
        }

        // Workforce steps — designer + sequential agent execution with deliverables
        if step.execution_mode == "workforce" {
            let step_result = execute_workforce_step(dag, step, dag_state).await;

            if let Err(ref e) = step_result {
                broadcast_step_failure_if_real(dag.state, dag.ctx, workflow_id, step, e);
            }
            step_result?;
            spawn_summarizer_if_completed(dag.state, step.id, dag_state);
            continue;
        }

        // Load agent (from snapshot if template-based, else from live DB)
        let agent_id = step.agent_id.ok_or_else(|| {
            HubError::Internal(anyhow::anyhow!(
                "step {} has no agent_id for mode '{}'",
                step_id,
                step.execution_mode
            ))
        })?;
        let agent = if let Some(snap) = &dag.ctx.snapshot {
            snap.agents
                .get(&agent_id)
                .cloned()
                .ok_or_else(|| HubError::AgentNotFound {
                    step_id: *step_id,
                    agent_id,
                })?
        } else {
            dag.state
                .repos()
                .agents
                .get_persisted_agent(agent_id)
                .await
                .map_err(|e| anyhow::anyhow!("failed to load agent: {}", e))?
                .ok_or_else(|| HubError::AgentNotFound {
                    step_id: *step_id,
                    agent_id,
                })?
        };

        // Resolve provider: use registry if agent targets non-default provider
        let step_engine = if agent.model_provider == "anthropic" || agent.model_provider.is_empty()
        {
            None // Use default engine
        } else {
            // Check runtime toggle for non-default providers
            if agent.model_provider == "ollama" && !dag.state.is_ollama_enabled().await {
                return Err(HubError::ProviderUnavailable {
                    provider: agent.model_provider.clone(),
                    step_id: *step_id,
                    agent_name: agent.name.clone(),
                });
            }
            let provider = dag
                .state
                .provider_for(&agent.model_provider)
                .ok_or_else(|| HubError::ProviderUnavailable {
                    provider: agent.model_provider.clone(),
                    step_id: *step_id,
                    agent_name: agent.name.clone(),
                })?;
            Some(ExecutionEngine::new(provider))
        };
        let effective_engine = step_engine.as_ref().unwrap_or(dag.engine);

        // Build a step-local DagContext with the effective engine for provider overrides
        let step_dag = DagContext {
            engine: effective_engine,
            ..*dag
        };

        let step_result = if step.execution_mode == "room" {
            execute_room_step(&step_dag, step, dag_state).await
        } else if step.execution_mode == "for_each" {
            execute_for_each_step(&step_dag, step, &agent, dag_state).await
        } else {
            execute_single_step(&step_dag, step, &agent, dag_state).await
        };

        if let Err(ref e) = step_result {
            broadcast_step_failure_if_real(dag.state, dag.ctx, workflow_id, step, e);
        }
        step_result?;

        spawn_summarizer_if_completed(dag.state, step.id, dag_state);
    }

    Ok(())
}

// ── Main DAG Orchestration ──────────────────────────────────────────────────

/// Execute a complete workflow DAG using the unified ExecutionEngine.
///
/// Executes a DAG via topo sort, variable resolution, for-each fan-out,
/// and interactive review. Step execution goes through
/// `ExecutionEngine::execute()` with `DagStepStrategy`.
///
/// Supports port-based data flow: if steps define input/output ports and edges
/// connect them, data flows through envelopes with structured extraction.
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
    let sorted_len = topological_sort(steps, edges)
        .map_err(|_| HubError::DagCycle)?
        .len();

    let mut dag_state = DagExecutionState::new();

    // Pre-fetch port metadata: from snapshot if template-based, otherwise from live DB
    let port_meta = match &ctx.snapshot {
        Some(snap) => templates::port_metadata_from_snapshot(snap),
        None => prefetch_port_metadata(state, steps, edges).await,
    };

    // Broadcast: workflow started
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
    run_dag_loop(&dag, &mut dag_state).await?;

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

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
