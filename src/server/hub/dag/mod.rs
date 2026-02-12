//! DAG orchestration — topological sort, variable resolution, for-each fan-out,
//! port-based data flow, and workflow execution using the unified ExecutionEngine.
//!
//! Pure utility functions live in the `utils` submodule and are re-exported here.
//! Execution functions use the hub's `ExecutionEngine` for step execution.

use std::collections::HashMap;
use std::collections::HashSet;

use serde_json::Value as JsonValue;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::db::{WorkflowStepEdgeRow, WorkflowStepRow};
use crate::server::state::AppState;
use crate::server::ws::events::{WorkflowEvent, WorkflowEventKind};
use crate::types::{
    DownstreamRoutingContext, ExecutionMetadata, ExecutionStatus, RouteDescription,
    StepExecutionEnvelope,
};

use super::engine::ExecutionEngine;
use super::error::HubError;

/// Emit a workflow lifecycle event via WebSocket broadcast.
pub fn broadcast_workflow_event(
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    workflow_id: Uuid,
    kind: WorkflowEventKind,
) {
    state.broadcast_workflow(WorkflowEvent {
        run_id: ctx.run_id,
        workflow_id,
        user_id: Some(ctx.user_id),
        kind,
    });
}

// ── Submodules ──────────────────────────────────────────────────────────────

pub(crate) mod container;
pub(crate) mod dag_state;
pub mod documenter;
pub(crate) mod for_each;
pub(crate) mod resume;
pub(crate) mod room_step;
pub(crate) mod single;
pub mod utils;

pub(crate) use dag_state::{
    prefetch_port_metadata, resolve_output_key, wrap_in_envelope, PortMetadata,
};

pub use utils::{
    build_routing_instruction_block, check_step_readiness, collect_upstream_context_data,
    compose_prompt, evaluate_edge_condition, extract_for_each_label, find_entry_steps,
    get_child_steps, get_parent_steps, resolve_dot_path, resolve_for_each_array,
    resolve_port_inputs, resolve_variables, topological_sort, ContainerExecutionConfig, DagPaused,
    PortResolutionError, StepOutput, StepReadiness, WorkflowExecutionContext,
    WorkflowExecutionResult,
};

// Re-export public functions from submodules
pub use resume::{resume_dag_from_approval, resume_workflow_via_engine};

// Internal imports for the main orchestration loop
use for_each::{detect_for_each_chains, execute_for_each_chain, execute_for_each_step};
use room_step::execute_room_step;
use single::execute_single_step;

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
            let agent_name = match state.repo().get_persisted_agent(rule.agent_id).await {
                Ok(Some(agent)) => agent.name,
                _ => format!("Agent {}", rule.agent_id),
            };

            let agent_tools = match state.repo().get_agent_tools(rule.agent_id).await {
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
    let sorted = topological_sort(steps, edges).map_err(|_| HubError::DagCycle)?;
    let step_map: HashMap<Uuid, &WorkflowStepRow> = steps.iter().map(|s| (s.id, s)).collect();
    let workflow_id = steps.first().map(|s| s.workflow_id).unwrap_or(Uuid::nil());
    let start_time = std::time::Instant::now();

    let mut completed: HashMap<Uuid, StepOutput> = HashMap::new();
    let mut completed_envelopes: HashMap<Uuid, StepExecutionEnvelope> = HashMap::new();
    let mut var_outputs: HashMap<String, JsonValue> = HashMap::new();
    let mut total_input_tokens: i64 = 0;
    let mut total_output_tokens: i64 = 0;
    let mut total_cost_usd: f32 = 0.0;

    // Pre-fetch port metadata for all steps
    let port_meta = prefetch_port_metadata(state, steps).await;

    // Broadcast: workflow started
    broadcast_workflow_event(
        state,
        ctx,
        workflow_id,
        WorkflowEventKind::Started {
            total_steps: sorted.len(),
        },
    );

    // Phase 6B: Detect chained for-each pipelines
    let chains = detect_for_each_chains(steps, edges);
    let chain_member_set: HashSet<Uuid> = chains
        .iter()
        .flat_map(|c| c.step_ids.iter().copied())
        .collect();

    if !chains.is_empty() {
        info!(
            chain_count = chains.len(),
            "Detected chained for-each pipelines"
        );
    }

    for step_id in &sorted {
        // Skip steps already executed as part of a chain
        if completed.contains_key(step_id) {
            continue;
        }

        let step = match step_map.get(step_id) {
            Some(s) => *s,
            None => continue,
        };

        // Check cancellation before each step
        if cancel.is_some_and(|t| t.is_cancelled()) {
            return Err(HubError::Cancelled);
        }

        // Check step readiness (handles conditional edges)
        match check_step_readiness(*step_id, edges, &completed, &completed_envelopes) {
            StepReadiness::Waiting => {
                warn!("Step {} has uncompleted parents, skipping", step_id);
                continue;
            }
            StepReadiness::Skipped => {
                info!("Step {} skipped — no matching conditional edges", step_id);
                completed.insert(*step_id, StepOutput::skipped(*step_id));
                continue;
            }
            StepReadiness::Ready => { /* proceed with execution */ }
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

        // Context steps pass through their prompt_template as output — no LLM call
        if step.execution_mode == "context" {
            let step_start = std::time::Instant::now();
            let output_key = resolve_output_key(step, &port_meta.step_outputs);
            let content = if step.prompt_template.is_empty() {
                ctx.initial_input.clone()
            } else {
                step.prompt_template.clone()
            };
            let value = JsonValue::String(content.clone());

            var_outputs.insert(output_key.clone(), value.clone());

            let output = StepOutput {
                variable_name: output_key,
                structured_output: Some(value.clone()),
                raw_output: content,
            };

            let envelope = StepExecutionEnvelope {
                status: ExecutionStatus::Success,
                data: Some(value),
                metadata: ExecutionMetadata {
                    execution_id: step.id,
                    execution_time_ms: 0,
                    tokens_in: Some(0),
                    tokens_out: Some(0),
                    cost_usd: Some(0.0),
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
            completed_envelopes.insert(step.id, envelope);
            completed.insert(step.id, output);

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
                    agent_id: None,
                    output: None,
                    input_tokens: Some(0),
                    output_tokens: Some(0),
                    duration_ms: Some(step_start.elapsed().as_millis() as u64),
                },
            );

            info!(step_id = %step.id, "Context step pass-through completed");
            continue;
        }

        // Documenter steps — phased pipeline, no agent needed
        if step.execution_mode == "documenter" {
            let step_result = execute_documenter_step(
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
            .await;

            if let Err(ref e) = step_result {
                if !matches!(e, HubError::AwaitingUser { .. }) {
                    broadcast_workflow_event(
                        state,
                        ctx,
                        workflow_id,
                        WorkflowEventKind::StepFailed {
                            step_id: step.id,
                            step_name: step
                                .output_variable_name
                                .clone()
                                .unwrap_or_else(|| step.id.to_string()),
                            error: format!("{}", e),
                        },
                    );
                }
            }
            step_result?;
            continue;
        }

        // Load agent
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

        // Resolve provider: use registry if agent targets non-default provider
        let step_engine = if agent.model_provider == "anthropic" || agent.model_provider.is_empty()
        {
            None // Use default engine
        } else {
            // Check runtime toggle for non-default providers
            if agent.model_provider == "ollama" && !state.is_ollama_enabled().await {
                return Err(HubError::ProviderUnavailable {
                    provider: agent.model_provider.clone(),
                    step_id: *step_id,
                    agent_name: agent.name.clone(),
                });
            }
            let provider = state.provider_for(&agent.model_provider).ok_or_else(|| {
                HubError::ProviderUnavailable {
                    provider: agent.model_provider.clone(),
                    step_id: *step_id,
                    agent_name: agent.name.clone(),
                }
            })?;
            Some(ExecutionEngine::new(provider))
        };
        let effective_engine = step_engine.as_ref().unwrap_or(engine);

        let step_result = if step.execution_mode == "room" {
            execute_room_step(
                effective_engine,
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
            .await
        } else if step.execution_mode == "for_each" {
            execute_for_each_step(
                effective_engine,
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
            .await
        } else {
            execute_single_step(
                effective_engine,
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
            .await
        };

        if let Err(ref e) = step_result {
            if !matches!(e, HubError::AwaitingUser { .. }) {
                broadcast_workflow_event(
                    state,
                    ctx,
                    workflow_id,
                    WorkflowEventKind::StepFailed {
                        step_id: step.id,
                        step_name: step
                            .output_variable_name
                            .clone()
                            .unwrap_or_else(|| step.id.to_string()),
                        error: format!("{}", e),
                    },
                );
            }
        }
        step_result?;
    }

    let duration_ms = start_time.elapsed().as_millis() as u64;

    let final_outputs: HashMap<String, StepOutput> = completed
        .into_iter()
        .map(|(id, out)| (id.to_string(), out))
        .collect();

    Ok(WorkflowExecutionResult {
        outputs: final_outputs,
        total_input_tokens,
        total_output_tokens,
        total_cost_usd,
        duration_ms,
    })
}

// ── Documenter Step Execution ───────────────────────────────────────────────

/// Execute a documenter step through the phased pipeline (strategy → research → write).
///
/// Unlike other step types, documenter steps have no agent. They dispatch to
/// `DocumenterExecutor` which runs three LLM phases internally, recording each
/// as a `protocol_execution` row and broadcasting WebSocket progress events.
async fn execute_documenter_step(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    steps: &[WorkflowStepRow],
    edges: &[WorkflowStepEdgeRow],
    var_outputs: &mut HashMap<String, JsonValue>,
    completed: &mut HashMap<Uuid, StepOutput>,
    completed_envelopes: &mut HashMap<Uuid, StepExecutionEnvelope>,
    port_meta: &PortMetadata,
    total_input_tokens: &mut i64,
    total_output_tokens: &mut i64,
    total_cost_usd: &mut f32,
    cancel: Option<&CancellationToken>,
) -> Result<(), HubError> {
    let step_start = std::time::Instant::now();

    // Broadcast: step started (no agent)
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
            agent_id: None,
            execution_id: None,
        },
    );

    // Resolve port inputs
    let port_inputs = if let Some(inputs) = port_meta.step_inputs.get(&step.id) {
        match resolve_port_inputs(
            step.id,
            edges,
            inputs,
            &port_meta.step_outputs,
            completed_envelopes,
        ) {
            Ok(resolved) => {
                debug!(step_id = %step.id, ports = resolved.len(), "Resolved documenter port inputs");
                Some(resolved)
            }
            Err(e) => {
                warn!(
                    "Port resolution failed for documenter step {}: {}",
                    step.id, e
                );
                None
            }
        }
    } else {
        None
    };

    // Collect upstream context data from bare (portless) edges connecting context steps
    let upstream_context =
        collect_upstream_context_data(step.id, edges, steps, completed_envelopes);

    // Compose prompt
    let mut prompt = compose_prompt(
        step,
        state.prompt_template_repo().as_deref(),
        state.doc_repo().as_deref(),
        state.workflow_repo().as_deref(),
        &**state.repo(),
        var_outputs,
        &ctx.prior_outputs,
        None,
        port_inputs.as_ref(),
    )
    .await;

    // Build ContextDocument objects from upstream context (stable short_ids via title hash)
    let upstream_docs: Vec<documenter::ContextDocument> = upstream_context
        .iter()
        .map(|(title, content)| {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            title.hash(&mut hasher);
            let short_id = format!("{:08x}", hasher.finish() & 0xFFFF_FFFF);
            documenter::ContextDocument {
                short_id,
                title: title.clone(),
                content: content.clone(),
            }
        })
        .collect();

    // Append upstream context using the same <document_XXXXXXXX> format
    // so the strategy LLM sees the short_ids it needs for context_document_ids routing
    let context_block = documenter::build_context_block(&[], &upstream_docs);
    if !context_block.is_empty() {
        prompt.push_str("\n\n");
        prompt.push_str(&context_block);
    }

    // Execute the documenter pipeline
    let executor = documenter::DocumenterExecutor::new(
        engine,
        state,
        ctx,
        step,
        &prompt,
        cancel,
        &upstream_docs,
    );
    let result = executor.execute(&port_meta.step_outputs).await?;

    // Accumulate tokens
    *total_input_tokens += result.input_tokens;
    *total_output_tokens += result.output_tokens;
    *total_cost_usd += result.cost_usd;

    // Store output in variable map
    if !result.output.variable_name.is_empty() {
        if let Some(ref structured) = result.output.structured_output {
            var_outputs.insert(result.output.variable_name.clone(), structured.clone());
        }
    }

    // Store envelope for downstream port resolution (agent-less)
    let envelope = StepExecutionEnvelope {
        status: if result.output.structured_output.is_some() {
            ExecutionStatus::Success
        } else {
            ExecutionStatus::Error
        },
        data: result.output.structured_output.clone(),
        metadata: ExecutionMetadata {
            execution_id: step.id,
            execution_time_ms: step_start.elapsed().as_millis() as u64,
            tokens_in: Some(result.input_tokens as i32),
            tokens_out: Some(result.output_tokens as i32),
            cost_usd: Some(result.cost_usd as f64),
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
    completed_envelopes.insert(step.id, envelope);
    completed.insert(step.id, result.output);

    // Broadcast: step completed (no agent)
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
            agent_id: None,
            output: None,
            input_tokens: Some(result.input_tokens as u64),
            output_tokens: Some(result.output_tokens as u64),
            duration_ms: Some(step_start.elapsed().as_millis() as u64),
        },
    );

    info!(
        step_id = %step.id,
        input_tokens = result.input_tokens,
        output_tokens = result.output_tokens,
        "Documenter step completed"
    );

    Ok(())
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
