//! For-each step execution and chained for-each pipeline detection/execution.
//!
//! Contains the fan-out logic for `execution_mode == "for_each"` steps, including
//! label-based routing, and the Phase 6B chained pipeline optimization that runs
//! consecutive for-each steps as per-item concurrent pipelines.

mod tests;

use std::collections::HashMap;

use anyhow::anyhow;
use serde_json::Value as JsonValue;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::db::{
    AgentRow, StepOutputRow, StepRoutingRuleRow, WorkflowStepEdgeRow, WorkflowStepRow,
};
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::error::HubError;
use crate::server::state::AppState;
use crate::server::ws::events::WorkflowEventKind;
use crate::types::{ExecutionError, ExecutionMetadata, ExecutionStatus, StepExecutionEnvelope};

use super::container::{
    create_optional_container, destroy_optional_container, run_with_vpn_watchdog,
};
use super::single::run_step_via_engine;
use super::{
    broadcast_workflow_event, compose_prompt, extract_for_each_label, resolve_for_each_array,
    resolve_output_key, wrap_in_envelope, PortMetadata, StepOutput, WorkflowExecutionContext,
};

// ============================================================================
// Phase 6B: Chained For-Each Pipeline Detection
// ============================================================================

/// A contiguous chain of for-each steps connected by single edges.
/// Items flow through the chain without barriers between stages.
#[derive(Debug, Clone)]
pub(crate) struct ForEachChain {
    /// Ordered step IDs — first is the entry, last feeds the barrier.
    pub(crate) step_ids: Vec<Uuid>,
}

/// Detect chains of consecutive for-each steps.
///
/// A chain is a maximal sequence `[S1, S2, ..., Sn]` (n >= 2) where:
/// - Every step has `execution_mode == "for_each"`
/// - Each `S_{i+1}` has exactly one parent that is `S_i`
/// - Each `S_i` has exactly one for-each child that is `S_{i+1}`
///
/// Fan-out (multiple children) and fan-in (multiple parents) break chains.
pub(crate) fn detect_for_each_chains(
    steps: &[WorkflowStepRow],
    edges: &[WorkflowStepEdgeRow],
) -> Vec<ForEachChain> {
    use std::collections::HashSet;

    let step_map: HashMap<Uuid, &WorkflowStepRow> = steps.iter().map(|s| (s.id, s)).collect();

    // Build adjacency: children and parents per step
    let mut children: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    let mut parents: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for edge in edges {
        children
            .entry(edge.from_step_id)
            .or_default()
            .push(edge.to_step_id);
        parents
            .entry(edge.to_step_id)
            .or_default()
            .push(edge.from_step_id);
    }

    let mut claimed: HashSet<Uuid> = HashSet::new();
    let mut chains = Vec::new();

    for step in steps {
        if claimed.contains(&step.id) {
            continue;
        }
        if step.execution_mode != "for_each" {
            continue;
        }

        // Start building a chain from this step
        let mut chain_ids = vec![step.id];
        let mut current = step.id;

        loop {
            let step_children = children.get(&current).cloned().unwrap_or_default();

            // Find for-each children
            let fe_children: Vec<Uuid> = step_children
                .iter()
                .filter(|cid| {
                    step_map
                        .get(cid)
                        .is_some_and(|s| s.execution_mode == "for_each")
                })
                .copied()
                .collect();

            // Must have exactly one for-each child to continue chain
            if fe_children.len() != 1 {
                break;
            }

            let next = fe_children[0];

            // The child must have exactly one parent (current step)
            let child_parents = parents.get(&next).cloned().unwrap_or_default();
            if child_parents.len() != 1 || child_parents[0] != current {
                break;
            }

            // Don't re-claim
            if claimed.contains(&next) {
                break;
            }

            chain_ids.push(next);
            current = next;
        }

        // Only record chains of length >= 2
        if chain_ids.len() >= 2 {
            for id in &chain_ids {
                claimed.insert(*id);
            }
            chains.push(ForEachChain {
                step_ids: chain_ids,
            });
        }
    }

    chains
}

/// Execute a for-each step: expand into N iterations, run sequentially.
///
/// Supports label-based routing: when `routing_mode = "label"`, each element
/// is routed to a different agent based on the value of `routing_field`.
#[allow(clippy::too_many_arguments)]
pub(super) async fn execute_for_each_step(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    agent: &AgentRow,
    _steps: &[WorkflowStepRow],
    _edges: &[WorkflowStepEdgeRow],
    var_outputs: &mut HashMap<String, JsonValue>,
    completed: &mut HashMap<Uuid, StepOutput>,
    completed_envelopes: &mut HashMap<Uuid, StepExecutionEnvelope>,
    port_meta: &PortMetadata,
    total_input_tokens: &mut i64,
    total_output_tokens: &mut i64,
    total_cost_usd: &mut f32,
    cancel: Option<&CancellationToken>,
) -> Result<(), HubError> {
    let for_each_ref = step
        .for_each_ref
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("for_each step {} missing for_each_ref", step.id))?;

    let array =
        resolve_for_each_array(for_each_ref, var_outputs, &ctx.prior_outputs).ok_or_else(|| {
            HubError::ForEachNotArray {
                reference: for_each_ref.to_string(),
            }
        })?;

    let label_field = step.for_each_label_field.as_deref();
    let routing_rules = port_meta.routing_rules.get(&step.id);
    let is_label_routing = step.routing_mode.as_deref() == Some("label") && routing_rules.is_some();

    // Broadcast: step started (for-each)
    broadcast_workflow_event(
        state,
        ctx,
        step.workflow_id,
        WorkflowEventKind::StepStarted {
            step_id: step.id,
            step_name: step
                .output_variable_name
                .clone()
                .unwrap_or_else(|| step.id.to_string()),
            agent_id: Some(agent.id),
            execution_id: None,
        },
    );

    info!(
        step_id = %step.id,
        count = array.len(),
        label_routing = is_label_routing,
        "for_each expansion"
    );

    let total_iterations = array.len();
    let mut iteration_outputs = Vec::new();
    // Cache loaded agents to avoid redundant DB calls
    let mut agent_cache: HashMap<Uuid, AgentRow> = HashMap::new();
    agent_cache.insert(agent.id, agent.clone());

    for (idx, element) in array.iter().enumerate() {
        if cancel.is_some_and(|t| t.is_cancelled()) {
            return Err(HubError::Cancelled);
        }
        let label = extract_for_each_label(element, label_field);

        // Determine which agent to use for this iteration
        let iteration_agent = if is_label_routing {
            let routed_agent_id = label.as_ref().and_then(|lbl| {
                routing_rules
                    .into_iter()
                    .flatten()
                    .find(|r| r.label_value == *lbl)
                    .map(|r| r.agent_id)
            });

            if let Some(routed_id) = routed_agent_id {
                if routed_id == agent.id {
                    agent
                } else if let Some(cached) = agent_cache.get(&routed_id) {
                    cached
                } else {
                    // Load routed agent from DB
                    match state.repo().get_persisted_agent(routed_id).await {
                        Ok(Some(routed_agent)) => {
                            agent_cache.insert(routed_id, routed_agent);
                            agent_cache.get(&routed_id).expect("just inserted")
                        }
                        _ => {
                            warn!(
                                "Routed agent {} not found for label {:?}, falling back to default",
                                routed_id, label
                            );
                            agent
                        }
                    }
                }
            } else {
                debug!(label = ?label, "No routing rule matched, using default agent");
                agent
            }
        } else {
            agent
        };

        let prompt = compose_prompt(
            step,
            state.prompt_template_repo().as_deref(),
            state.doc_repo().as_deref(),
            state.workflow_repo().as_deref(),
            &**state.repo(),
            var_outputs,
            &ctx.prior_outputs,
            Some(element),
            None,
        )
        .await;

        // Create container for this iteration if configured (with optional VPN sidecar)
        let iter_container = create_optional_container(
            ctx.container_config.as_ref(),
            ctx.wg_client.as_deref(),
            "for-each-iter",
        )
        .await?;

        let result = run_with_vpn_watchdog(
            &iter_container,
            run_step_via_engine(
                engine,
                state,
                ctx,
                step,
                iteration_agent,
                &prompt,
                &port_meta.step_outputs,
                cancel,
                iter_container.as_ref().map(|mc| &mc.agent_handle),
            ),
        )
        .await;

        destroy_optional_container(&iter_container, ctx.wg_client.as_deref()).await;

        match result {
            Ok((output, in_tok, out_tok, cost)) => {
                *total_input_tokens += in_tok;
                *total_output_tokens += out_tok;
                *total_cost_usd += cost;
                iteration_outputs.push(output.structured_output.clone());

                // Broadcast: for-each progress
                broadcast_workflow_event(
                    state,
                    ctx,
                    step.workflow_id,
                    WorkflowEventKind::ForEachProgress {
                        step_id: step.id,
                        step_name: step
                            .output_variable_name
                            .clone()
                            .unwrap_or_else(|| step.id.to_string()),
                        completed: idx + 1,
                        total: total_iterations,
                    },
                );
            }
            Err(e) => {
                error!(
                    "for_each iteration {} failed for step {}: {}",
                    idx, step.id, e
                );
            }
        }
    }

    // Aggregate outputs as array
    let aggregated = JsonValue::Array(iteration_outputs.into_iter().flatten().collect());
    let variable_name = resolve_output_key(step, &port_meta.step_outputs);

    let output = StepOutput {
        variable_name: variable_name.clone(),
        structured_output: Some(aggregated.clone()),
        raw_output: String::new(),
    };

    // Store in var_outputs for downstream variable resolution
    if !variable_name.is_empty() {
        var_outputs.insert(variable_name, aggregated);
    }

    // Store envelope for downstream port resolution
    let envelope = wrap_in_envelope(&output, agent, step.id, 0, 0, 0.0);
    completed_envelopes.insert(step.id, envelope);

    completed.insert(step.id, output);

    // Broadcast: step completed (for-each)
    broadcast_workflow_event(
        state,
        ctx,
        step.workflow_id,
        WorkflowEventKind::StepCompleted {
            step_id: step.id,
            step_name: step
                .output_variable_name
                .clone()
                .unwrap_or_else(|| step.id.to_string()),
            agent_id: Some(agent.id),
            output: None,
            input_tokens: None,
            output_tokens: None,
            duration_ms: None,
        },
    );

    Ok(())
}

// ============================================================================
// Phase 6B: Chained For-Each Pipeline Execution
// ============================================================================

/// Result of executing one item through all stages of a for-each chain.
struct PipelineItemResult {
    index: usize,
    #[allow(dead_code)]
    label: Option<String>,
    /// One envelope per stage, ordered by chain stage index.
    stage_envelopes: Vec<(Uuid, StepExecutionEnvelope)>,
    input_tokens: i64,
    output_tokens: i64,
    cost_usd: f32,
}

/// Data needed for one stage of the pipeline, pre-loaded before task spawning.
#[derive(Clone)]
struct PipelineStageData {
    step: WorkflowStepRow,
    default_agent: AgentRow,
    routing_rules: Option<Vec<StepRoutingRuleRow>>,
    /// Pre-loaded routed agents for label routing.
    agent_cache: HashMap<Uuid, AgentRow>,
}

/// Execute a chain of for-each steps using per-item pipeline streaming.
///
/// For chain `[A, B]` with N items:
/// - Spawns N pipeline tasks, one per item index
/// - Each pipeline task runs: A[i] → B[i] sequentially
/// - All N pipelines run concurrently via JoinSet
/// - Collects results into aggregates for each step in the chain
#[allow(clippy::too_many_arguments)]
pub(super) async fn execute_for_each_chain(
    _engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    chain: &ForEachChain,
    step_map: &HashMap<Uuid, &WorkflowStepRow>,
    _edges: &[WorkflowStepEdgeRow],
    var_outputs: &mut HashMap<String, JsonValue>,
    completed: &mut HashMap<Uuid, StepOutput>,
    completed_envelopes: &mut HashMap<Uuid, StepExecutionEnvelope>,
    port_meta: &PortMetadata,
    total_input_tokens: &mut i64,
    total_output_tokens: &mut i64,
    total_cost_usd: &mut f32,
    cancel: Option<&CancellationToken>,
) -> Result<(), HubError> {
    // 1. Get the first step and resolve the for-each array
    let first_step = step_map
        .get(&chain.step_ids[0])
        .ok_or_else(|| HubError::Internal(anyhow!("chain head step not found")))?;

    let for_each_ref = first_step.for_each_ref.as_deref().ok_or_else(|| {
        anyhow::anyhow!("chain head step {} missing for_each_ref", chain.step_ids[0])
    })?;

    let array =
        resolve_for_each_array(for_each_ref, var_outputs, &ctx.prior_outputs).ok_or_else(|| {
            HubError::ForEachNotArray {
                reference: for_each_ref.to_string(),
            }
        })?;

    info!(
        chain_len = chain.step_ids.len(),
        items = array.len(),
        head_step = %chain.step_ids[0],
        "Starting chained for-each pipeline"
    );

    // 2. Pre-load all stage data (agents + routing rules)
    let mut stages: Vec<PipelineStageData> = Vec::with_capacity(chain.step_ids.len());

    for step_id in &chain.step_ids {
        let step = step_map
            .get(step_id)
            .ok_or_else(|| HubError::Internal(anyhow!("chain step {} not found", step_id)))?;

        let default_agent_id = step.agent_id.ok_or_else(|| {
            HubError::Internal(anyhow::anyhow!(
                "chain step {} has no agent_id for mode '{}'",
                step_id,
                step.execution_mode
            ))
        })?;
        let default_agent = state
            .repo()
            .get_persisted_agent(default_agent_id)
            .await
            .map_err(|e| anyhow::anyhow!("failed to load agent: {}", e))?
            .ok_or_else(|| HubError::AgentNotFound {
                step_id: *step_id,
                agent_id: default_agent_id,
            })?;

        let routing_rules = port_meta.routing_rules.get(step_id).cloned();

        // Pre-load routed agents into cache
        let mut agent_cache: HashMap<Uuid, AgentRow> = HashMap::new();
        agent_cache.insert(default_agent.id, default_agent.clone());

        if let Some(ref rules) = routing_rules {
            for rule in rules {
                use std::collections::hash_map::Entry;
                if let Entry::Vacant(entry) = agent_cache.entry(rule.agent_id) {
                    if let Ok(Some(routed_agent)) =
                        state.repo().get_persisted_agent(rule.agent_id).await
                    {
                        entry.insert(routed_agent);
                    }
                }
            }
        }

        stages.push(PipelineStageData {
            step: (*step).clone(),
            default_agent,
            routing_rules,
            agent_cache,
        });
    }

    // 3. Spawn per-item pipeline tasks
    let engine_provider = state
        .provider()
        .ok_or(HubError::ProviderNotConfigured)?
        .clone();
    let cancel_token = cancel.cloned();

    let mut join_set = tokio::task::JoinSet::new();

    for (idx, element) in array.iter().enumerate() {
        let stages_clone = stages.clone();
        let state_clone = state.clone();
        let ctx_clone = ctx.clone();
        let element_clone = element.clone();
        let cancel_clone = cancel_token.clone();
        let provider_clone = engine_provider.clone();
        let step_outputs_clone = port_meta.step_outputs.clone();

        join_set.spawn(async move {
            let task_engine = ExecutionEngine::new(provider_clone);
            execute_pipeline_item(
                &task_engine,
                &state_clone,
                &ctx_clone,
                &stages_clone,
                idx,
                &element_clone,
                &step_outputs_clone,
                cancel_clone.as_ref(),
            )
            .await
        });
    }

    // 4. Collect results
    let mut item_results: Vec<PipelineItemResult> = Vec::with_capacity(array.len());

    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(item_result)) => {
                item_results.push(item_result);
            }
            Ok(Err(e)) => {
                error!("Pipeline item failed: {}", e);
                // Continue collecting other results — partial success
            }
            Err(join_err) => {
                error!("Pipeline task panicked: {}", join_err);
            }
        }
    }

    // Sort by index for deterministic ordering
    item_results.sort_by_key(|r| r.index);

    // 5. Build per-step aggregates
    for (stage_idx, step_id) in chain.step_ids.iter().enumerate() {
        let step = match step_map.get(step_id) {
            Some(s) => s,
            None => continue,
        };
        let variable_name = resolve_output_key(step, &port_meta.step_outputs);

        // Collect iteration outputs for this stage
        let mut iteration_outputs: Vec<Option<JsonValue>> = Vec::with_capacity(array.len());
        let mut stage_input_tokens: i64 = 0;
        let mut stage_output_tokens: i64 = 0;
        let mut stage_cost: f32 = 0.0;

        for item in &item_results {
            if stage_idx < item.stage_envelopes.len() {
                let (_, ref envelope) = item.stage_envelopes[stage_idx];
                iteration_outputs.push(envelope.data.clone());
            } else {
                // Item failed before reaching this stage
                iteration_outputs.push(None);
            }
        }

        // Accumulate tokens only for the final stage (avoid double-counting)
        if stage_idx == chain.step_ids.len() - 1 {
            for item in &item_results {
                stage_input_tokens += item.input_tokens;
                stage_output_tokens += item.output_tokens;
                stage_cost += item.cost_usd;
            }
        }

        let aggregated = JsonValue::Array(iteration_outputs.into_iter().flatten().collect());

        let output = StepOutput {
            variable_name: variable_name.clone(),
            structured_output: Some(aggregated.clone()),
            raw_output: String::new(),
        };

        if !variable_name.is_empty() {
            var_outputs.insert(variable_name, aggregated);
        }

        let default_agent = &stages[stage_idx].default_agent;
        let envelope = wrap_in_envelope(
            &output,
            default_agent,
            *step_id,
            stage_input_tokens,
            stage_output_tokens,
            stage_cost,
        );
        completed_envelopes.insert(*step_id, envelope);
        completed.insert(*step_id, output);

        // Only accumulate totals once (at the end)
        if stage_idx == chain.step_ids.len() - 1 {
            *total_input_tokens += stage_input_tokens;
            *total_output_tokens += stage_output_tokens;
            *total_cost_usd += stage_cost;
        }
    }

    info!(
        chain_len = chain.step_ids.len(),
        items_completed = item_results.len(),
        "Chained for-each pipeline complete"
    );

    Ok(())
}

/// Execute one item through all stages of a pipeline chain sequentially.
///
/// This runs as a spawned Tokio task. Each stage's output becomes the input
/// for the next stage, with upstream routing context propagated forward.
#[allow(clippy::too_many_arguments)]
async fn execute_pipeline_item(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    stages: &[PipelineStageData],
    item_index: usize,
    element: &JsonValue,
    step_outputs: &HashMap<Uuid, Vec<StepOutputRow>>,
    cancel: Option<&CancellationToken>,
) -> Result<PipelineItemResult, HubError> {
    let mut stage_envelopes: Vec<(Uuid, StepExecutionEnvelope)> = Vec::with_capacity(stages.len());
    let mut total_input_tokens: i64 = 0;
    let mut total_output_tokens: i64 = 0;
    let mut total_cost: f32 = 0.0;

    // Track upstream context for propagation
    let mut upstream_agent_id: Option<Uuid> = None;
    let mut upstream_routing_label: Option<String> = None;

    // The current element being processed — starts as the original, then becomes
    // the previous stage's output data for subsequent stages.
    let mut current_element = element.clone();

    let label = extract_for_each_label(element, stages[0].step.for_each_label_field.as_deref());

    for (stage_idx, stage) in stages.iter().enumerate() {
        // Check cancellation
        if cancel.is_some_and(|t| t.is_cancelled()) {
            return Err(HubError::Cancelled);
        }

        let step = &stage.step;

        // Determine which agent handles this item at this stage
        let iteration_agent = resolve_pipeline_agent(
            &current_element,
            element, // original element for routing field lookup
            step,
            &stage.routing_rules,
            &stage.default_agent,
            &stage.agent_cache,
        );

        let routing_label = if step.routing_mode.as_deref() == Some("label") {
            extract_for_each_label(element, step.routing_field.as_deref())
        } else {
            None
        };

        // Build prompt with the current element
        let prompt = compose_prompt(
            step,
            state.prompt_template_repo().as_deref(),
            state.doc_repo().as_deref(),
            state.workflow_repo().as_deref(),
            &**state.repo(),
            &HashMap::new(), // pipeline items don't use var_outputs
            &ctx.prior_outputs,
            Some(&current_element),
            None,
        )
        .await;

        // Append upstream context to prompt for stages > 0
        let prompt = if stage_idx > 0 {
            let mut p = prompt;
            if let Some(ref ua_id) = upstream_agent_id {
                p.push_str(&format!(
                    "\n\n<upstream>\nThis item was processed by agent {} (routing label: {}).\n</upstream>",
                    ua_id,
                    upstream_routing_label.as_deref().unwrap_or("none")
                ));
            }
            p
        } else {
            prompt
        };

        // Create container for this pipeline stage if configured (with optional VPN sidecar)
        let stage_container = create_optional_container(
            ctx.container_config.as_ref(),
            ctx.wg_client.as_deref(),
            "pipeline-stage",
        )
        .await?;

        let result = run_with_vpn_watchdog(
            &stage_container,
            run_step_via_engine(
                engine,
                state,
                ctx,
                step,
                iteration_agent,
                &prompt,
                step_outputs,
                cancel,
                stage_container.as_ref().map(|mc| &mc.agent_handle),
            ),
        )
        .await;

        destroy_optional_container(&stage_container, ctx.wg_client.as_deref()).await;

        match result {
            Ok((output, in_tok, out_tok, cost)) => {
                total_input_tokens += in_tok;
                total_output_tokens += out_tok;
                total_cost += cost;

                let envelope = StepExecutionEnvelope {
                    status: ExecutionStatus::Success,
                    data: output.structured_output.clone(),
                    metadata: ExecutionMetadata {
                        execution_id: Uuid::new_v4(),
                        execution_time_ms: 0,
                        tokens_in: Some(in_tok as i32),
                        tokens_out: Some(out_tok as i32),
                        cost_usd: Some(cost as f64),
                        model: Some(iteration_agent.model_id.clone()),
                        agent_id: Some(iteration_agent.id),
                        iteration_index: Some(item_index),
                        iteration_label: label.clone(),
                        routing_label: routing_label.clone(),
                        selected_routing_document_id: None,
                        upstream_agent_id,
                        upstream_routing_label: upstream_routing_label.clone(),
                        room_session_id: None,
                        room_id: None,
                        total_rounds: None,
                    },
                    error: None,
                };

                // Propagate context for next stage
                upstream_agent_id = Some(iteration_agent.id);
                upstream_routing_label = routing_label;

                // Next stage uses this stage's output as its element
                if let Some(ref data) = output.structured_output {
                    current_element = data.clone();
                }

                stage_envelopes.push((step.id, envelope));
            }
            Err(e) => {
                error!(
                    "Pipeline item {} failed at stage {} (step {}): {}",
                    item_index, stage_idx, step.id, e
                );

                // Record error envelope for this stage
                let error_envelope = StepExecutionEnvelope {
                    status: ExecutionStatus::Error,
                    data: None,
                    metadata: ExecutionMetadata {
                        execution_id: Uuid::new_v4(),
                        execution_time_ms: 0,
                        tokens_in: None,
                        tokens_out: None,
                        cost_usd: None,
                        model: Some(iteration_agent.model_id.clone()),
                        agent_id: Some(iteration_agent.id),
                        iteration_index: Some(item_index),
                        iteration_label: label.clone(),
                        routing_label: None,
                        selected_routing_document_id: None,
                        upstream_agent_id,
                        upstream_routing_label: upstream_routing_label.clone(),
                        room_session_id: None,
                        room_id: None,
                        total_rounds: None,
                    },
                    error: Some(ExecutionError {
                        message: format!("{}", e),
                        error_type: "PipelineStageError".to_string(),
                        retryable: false,
                        details: None,
                    }),
                };

                stage_envelopes.push((step.id, error_envelope));

                // Don't continue to next stages — item failed
                break;
            }
        }
    }

    Ok(PipelineItemResult {
        index: item_index,
        label,
        stage_envelopes,
        input_tokens: total_input_tokens,
        output_tokens: total_output_tokens,
        cost_usd: total_cost,
    })
}

/// Resolve which agent should handle an item at a pipeline stage.
///
/// Uses the ORIGINAL element (from the planner output) for routing field lookup,
/// so that both stages can route by the same field (e.g., "category").
fn resolve_pipeline_agent<'a>(
    _current_element: &JsonValue,
    original_element: &JsonValue,
    step: &WorkflowStepRow,
    routing_rules: &Option<Vec<StepRoutingRuleRow>>,
    default_agent: &'a AgentRow,
    agent_cache: &'a HashMap<Uuid, AgentRow>,
) -> &'a AgentRow {
    let is_label_routing = step.routing_mode.as_deref() == Some("label") && routing_rules.is_some();

    if !is_label_routing {
        return default_agent;
    }

    let routing_field = step.routing_field.as_deref().unwrap_or("category");
    let label = original_element.get(routing_field).and_then(|v| v.as_str());

    if let Some(label_str) = label {
        let routed_agent_id = routing_rules
            .as_ref()
            .into_iter()
            .flatten()
            .find(|r| r.label_value == label_str)
            .map(|r| r.agent_id);

        if let Some(routed_id) = routed_agent_id {
            if let Some(agent) = agent_cache.get(&routed_id) {
                return agent;
            }
        }
    }

    default_agent
}
