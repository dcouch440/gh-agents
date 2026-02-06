//! DAG orchestration — topological sort, variable resolution, for-each fan-out,
//! port-based data flow, and workflow execution using the unified ExecutionEngine.
//!
//! This module re-exports the pure graph/variable/port functions from `executors::dag`
//! and provides `execute_workflow_via_engine` which delegates step execution
//! to the hub's `ExecutionEngine` instead of running its own react loop.

use std::collections::HashMap;

use anyhow::anyhow;
use serde_json::Value as JsonValue;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::db::traits::WorkflowCollectionRepo;
use crate::db::{
    AgentRow, StepInputRow, StepOutputRow, StepRoutingRuleRow, WorkflowStepEdgeRow, WorkflowStepRow,
};
use crate::server::state::AppState;
use crate::types::{ExecutionMetadata, ExecutionStatus, StepExecutionEnvelope};

use super::construct_agent_defaults;
use super::engine::ExecutionEngine;
use super::error::HubError;
use super::recorder::ExecutionRecorder;
use super::strategies::dag_step::{compute_cost, DagStepConfig, DagStepStrategy};
use super::streaming::NullSink;

// Re-export pure DAG functions from executors::dag
pub use crate::server::executors::dag::{
    compose_prompt, extract_for_each_label, find_entry_steps, get_child_steps, get_parent_steps,
    resolve_dot_path, resolve_for_each_array, resolve_port_inputs, resolve_variables,
    topological_sort, DagPaused, PortResolutionError, StepOutput, WorkflowExecutionContext,
    WorkflowExecutionResult,
};

/// Wrap a step output into a StepExecutionEnvelope for port-based data flow.
fn wrap_in_envelope(
    output: &StepOutput,
    agent: &AgentRow,
    execution_id: Uuid,
    input_tokens: i64,
    output_tokens: i64,
    cost_usd: f32,
) -> StepExecutionEnvelope {
    StepExecutionEnvelope {
        status: if output.structured_output.is_some() {
            ExecutionStatus::Success
        } else {
            ExecutionStatus::Error
        },
        data: output.structured_output.clone(),
        metadata: ExecutionMetadata {
            execution_id,
            execution_time_ms: 0,
            tokens_in: Some(input_tokens as i32),
            tokens_out: Some(output_tokens as i32),
            cost_usd: Some(cost_usd as f64),
            model: Some(agent.model_id.clone()),
            agent_id: Some(agent.id),
            iteration_index: None,
            iteration_label: None,
            routing_label: None,
            selected_routing_document_id: None,
        },
        error: None,
    }
}

/// Pre-fetched port metadata for all steps in a workflow.
struct PortMetadata {
    step_inputs: HashMap<Uuid, Vec<StepInputRow>>,
    step_outputs: HashMap<Uuid, Vec<StepOutputRow>>,
    routing_rules: HashMap<Uuid, Vec<StepRoutingRuleRow>>,
}

/// Pre-fetch port metadata (inputs, outputs, routing rules) for all steps.
async fn prefetch_port_metadata(state: &AppState, steps: &[WorkflowStepRow]) -> PortMetadata {
    let mut step_inputs: HashMap<Uuid, Vec<StepInputRow>> = HashMap::new();
    let mut step_outputs: HashMap<Uuid, Vec<StepOutputRow>> = HashMap::new();
    let mut routing_rules: HashMap<Uuid, Vec<StepRoutingRuleRow>> = HashMap::new();

    if let Some(ref wf_repo) = state.workflow_repo() {
        for step in steps {
            if let Ok(inputs) = wf_repo.get_step_inputs(step.id).await {
                if !inputs.is_empty() {
                    step_inputs.insert(step.id, inputs);
                }
            }
            if let Ok(outputs) = wf_repo.get_step_outputs(step.id).await {
                if !outputs.is_empty() {
                    step_outputs.insert(step.id, outputs);
                }
            }
            if step.routing_mode.as_deref() == Some("label") {
                if let Ok(rules) = wf_repo.get_step_routing_rules(step.id).await {
                    if !rules.is_empty() {
                        routing_rules.insert(step.id, rules);
                    }
                }
            }
        }
    }

    PortMetadata {
        step_inputs,
        step_outputs,
        routing_rules,
    }
}

/// Execute a complete workflow DAG using the unified ExecutionEngine.
///
/// This replaces `executors::dag::execute_workflow` — same logic (topo sort,
/// variable resolution, for-each fan-out, interactive review) but step
/// execution goes through `ExecutionEngine::execute()` with `DagStepStrategy`.
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

    let mut completed: HashMap<Uuid, StepOutput> = HashMap::new();
    let mut completed_envelopes: HashMap<Uuid, StepExecutionEnvelope> = HashMap::new();
    let mut var_outputs: HashMap<String, JsonValue> = HashMap::new();
    let mut total_input_tokens: i64 = 0;
    let mut total_output_tokens: i64 = 0;
    let mut total_cost_usd: f32 = 0.0;

    // Pre-fetch port metadata for all steps
    let port_meta = prefetch_port_metadata(state, steps).await;

    for step_id in &sorted {
        let step = match step_map.get(step_id) {
            Some(s) => *s,
            None => continue,
        };

        // Check cancellation before each step
        if cancel.is_some_and(|t| t.is_cancelled()) {
            return Err(HubError::Cancelled);
        }

        // Check all parents are completed
        let parents = get_parent_steps(*step_id, edges);
        let all_parents_done = parents.iter().all(|pid| completed.contains_key(pid));
        if !all_parents_done {
            warn!("Step {} has uncompleted parents, skipping", step_id);
            continue;
        }

        // Load agent
        let agent = state
            .repo()
            .get_persisted_agent(step.agent_id)
            .await
            .map_err(|e| anyhow::anyhow!("failed to load agent: {}", e))?
            .ok_or_else(|| HubError::AgentNotFound {
                step_id: *step_id,
                agent_id: step.agent_id,
            })?;

        if step.execution_mode == "room" {
            // Room execution — create a session and pause the pipeline.
            let room_id = step.room_id.ok_or_else(|| {
                HubError::Internal(anyhow!(
                    "step {} has execution_mode='room' but no room_id",
                    step.id
                ))
            })?;
            let room_repo = &state.repos().rooms;
            let session = room_repo
                .create_room_session(room_id, Some(ctx.run_id))
                .await
                .map_err(|e| HubError::Internal(anyhow!("failed to create room session: {}", e)))?;

            info!(
                step_id = %step.id,
                room_id = %room_id,
                session_id = %session.id,
                "Room step paused — awaiting interactive room conversation"
            );

            let output = StepOutput {
                variable_name: step.output_variable_name.clone().unwrap_or_default(),
                raw_output: format!(
                    "{{\"room_session_id\":\"{}\",\"status\":\"awaiting_room\"}}",
                    session.id
                ),
                structured_output: Some(serde_json::json!({
                    "room_session_id": session.id.to_string(),
                    "status": "awaiting_room"
                })),
            };
            if let Some(ref var_name) = step.output_variable_name {
                var_outputs.insert(
                    var_name.clone(),
                    output.structured_output.clone().unwrap_or_default(),
                );
            }
            completed.insert(step.id, output);

            return Err(HubError::AwaitingUser {
                step_id: step.id,
                execution_id: session.id,
            });
        } else if step.execution_mode == "for_each" {
            execute_for_each_step(
                engine,
                state,
                ctx,
                step,
                &agent,
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
    })
}

/// Execute a single (non-for-each) step through the engine.
async fn execute_single_step(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    agent: &AgentRow,
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
    // Resolve port inputs if this step has input ports defined
    let port_inputs = if let Some(inputs) = port_meta.step_inputs.get(&step.id) {
        match resolve_port_inputs(
            step.id,
            edges,
            inputs,
            &port_meta.step_outputs,
            completed_envelopes,
        ) {
            Ok(resolved) => {
                debug!(step_id = %step.id, ports = resolved.len(), "Resolved port inputs");
                Some(resolved)
            }
            Err(e) => {
                warn!("Port resolution failed for step {}: {}", step.id, e);
                None
            }
        }
    } else {
        None
    };

    let prompt = compose_prompt(
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

    let (output, in_tok, out_tok, cost) =
        run_step_via_engine(engine, state, ctx, step, agent, &prompt, cancel).await?;

    *total_input_tokens += in_tok;
    *total_output_tokens += out_tok;
    *total_cost_usd += cost;

    // Store output in variable map (fixes var_outputs propagation bug)
    if !output.variable_name.is_empty() {
        if let Some(ref structured) = output.structured_output {
            var_outputs.insert(output.variable_name.clone(), structured.clone());
        }
    }

    // Store envelope for downstream port resolution
    let envelope = wrap_in_envelope(&output, agent, step.id, in_tok, out_tok, cost);
    completed_envelopes.insert(step.id, envelope);

    completed.insert(step.id, output);

    Ok(())
}

/// Execute a for-each step: expand into N iterations, run sequentially.
///
/// Supports label-based routing: when `routing_mode = "label"`, each element
/// is routed to a different agent based on the value of `routing_field`.
async fn execute_for_each_step(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    agent: &AgentRow,
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

    info!(
        step_id = %step.id,
        count = array.len(),
        label_routing = is_label_routing,
        "for_each expansion"
    );

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
                    .unwrap()
                    .iter()
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
                            agent_cache.get(&routed_id).unwrap()
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

        match run_step_via_engine(engine, state, ctx, step, iteration_agent, &prompt, cancel).await
        {
            Ok((output, in_tok, out_tok, cost)) => {
                *total_input_tokens += in_tok;
                *total_output_tokens += out_tok;
                *total_cost_usd += cost;
                iteration_outputs.push(output.structured_output.clone());
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
    let variable_name = step.output_variable_name.clone().unwrap_or_default();

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

    Ok(())
}

/// Run a single step execution through the ExecutionEngine.
///
/// Creates the agent_execution record, builds a DagStepStrategy, and
/// calls `engine.execute()`. Returns (StepOutput, input_tokens, output_tokens, cost).
async fn run_step_via_engine(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    agent: &AgentRow,
    prompt: &str,
    cancel: Option<&CancellationToken>,
) -> Result<(StepOutput, i64, i64, f32), HubError> {
    let ae_repo = state
        .agent_execution_repo()
        .ok_or_else(|| anyhow::anyhow!("agent_execution_repo not configured"))?;

    // Resolve mode (no context hint initially - future: pass step description)
    let mode = if let Some(resolver) = state.mode_resolver() {
        resolver
            .resolve(agent, prompt, None)
            .await
            .map_err(|e| HubError::Internal(anyhow!("Mode resolution failed: {}", e)))?
    } else {
        // Fallback: construct agent defaults for backward compatibility
        construct_agent_defaults(agent, state.repo())
            .await
            .map_err(HubError::Internal)?
    };

    // Build system prompt: mode result + schema enforcement
    let mut system_prompt = mode.system_prompt; // agent + mode already merged
    if let Some(schema_id) = step.output_schema_id {
        let os_repo = &state.repos().output_schemas;
        if let Ok(Some(schema)) = os_repo.get_output_schema(schema_id).await {
            system_prompt.push_str(&format!(
                "\n\nYou MUST respond with valid JSON matching this schema:\n```json\n{}\n```\nRespond ONLY with the JSON object, no other text.",
                serde_json::to_string_pretty(&schema.schema).unwrap_or_default()
            ));
        }
    }

    // Create agent_execution row
    let ae_row = ae_repo
        .create_agent_execution(
            agent.id,
            Some(step.id),
            false,
            None,
            &system_prompt,
            prompt,
            mode.selected_mode_id, // Track which mode was used
            None,
            None,
        )
        .await
        .map_err(|e| anyhow::anyhow!("failed to create agent execution: {}", e))?;

    // Record initial messages
    let _ = ae_repo
        .create_execution_message(ae_row.id, "system", &system_prompt, None, 0, 0)
        .await;
    let _ = ae_repo
        .create_execution_message(ae_row.id, "user", prompt, None, 0, 0)
        .await;

    // Build strategy
    let config = DagStepConfig {
        agent: agent.clone(),
        step: step.clone(),
        system_prompt,
        user_prompt: prompt.to_string(),
        tools: mode.tools,             // Use mode tools
        tool_names: mode.tool_names,   // Use mode tool names
        temperature: mode.temperature, // Use mode temperature
        execution_context: ctx.execution_context.clone(),
        run_id: ctx.run_id,
        user_id: ctx.user_id,
        agent_execution_id: ae_row.id,
    };
    let strategy = DagStepStrategy::new(config, state.clone());

    // Build recorder (strategy handles its own recording in on_complete)
    let ae_repo = state.agent_execution_repo();
    let tl_repo = state.token_ledger_repo();
    let recorder = ExecutionRecorder::new(
        state.repo().as_ref(),
        ae_repo.as_deref(),
        tl_repo.as_deref(),
    );

    let sink = NullSink;

    // Execute
    let result = engine
        .execute(&strategy, prompt, &sink, &recorder, cancel)
        .await?;

    let cost = compute_cost(
        &agent.model_id,
        result.input_tokens as i64,
        result.output_tokens as i64,
    );

    let variable_name = step.output_variable_name.clone().unwrap_or_default();
    let structured = super::strategies::dag_step::DagStepStrategy::parse_output(&result.content);

    let output = StepOutput {
        variable_name,
        structured_output: structured,
        raw_output: result.content,
    };

    Ok((
        output,
        result.input_tokens as i64,
        result.output_tokens as i64,
        cost,
    ))
}

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
                selected_routing_document_id: None,
            },
            error: None,
        };
        completed_envelopes.insert(*step_id, envelope);
    }

    // Pre-fetch port metadata
    let port_meta = prefetch_port_metadata(state, steps).await;

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

        let agent = state
            .repo()
            .get_persisted_agent(step.agent_id)
            .await
            .map_err(|e| anyhow::anyhow!("failed to load agent: {}", e))?
            .ok_or_else(|| HubError::AgentNotFound {
                step_id: *step_id,
                agent_id: step.agent_id,
            })?;

        if step.execution_mode == "room" {
            return Err(HubError::AwaitingUser {
                step_id: step.id,
                execution_id: Uuid::nil(),
            });
        } else if step.execution_mode == "for_each" {
            execute_for_each_step(
                engine,
                state,
                ctx,
                step,
                &agent,
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
                let variable_name = ws.output_variable_name.clone().unwrap_or_default();
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
    let approved_var_name = step.output_variable_name.clone().unwrap_or_default();
    let mut approved_output = approved_output;
    approved_output.variable_name = approved_var_name.clone();
    if !approved_var_name.is_empty() {
        if let Some(structured) = &approved_output.structured_output {
            var_outputs.insert(approved_var_name, structured.clone());
        }
    }
    completed.insert(paused_step_id, approved_output);

    // Build execution context
    let ctx = WorkflowExecutionContext {
        stage_execution_id: workflow_execution_id.unwrap_or(Uuid::new_v4()),
        run_id: Uuid::new_v4(),
        user_id,
        initial_input: String::new(),
        prior_outputs: HashMap::new(),
        execution_context: None,
    };

    // Create engine and resume
    let provider = state
        .provider()
        .ok_or(HubError::ProviderNotConfigured)?
        .clone();
    let engine = ExecutionEngine::new(provider);

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

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
