//! Cavernous routing step execution — document-based dynamic execution.
//!
//! Searches for routing config documents, selects the best config via LLM,
//! parses it, executes subtasks in dependency order, and aggregates results.

mod tests;

use std::collections::HashMap;

use anyhow::anyhow;
use serde_json::Value as JsonValue;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::db::{AgentRow, WorkflowStepEdgeRow, WorkflowStepRow};
use crate::server::hub::construct_agent_defaults;
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::error::HubError;
use crate::server::hub::recorder::ExecutionRecorder;
use crate::server::hub::strategies::cavernous::{CavernousStepConfig, CavernousStepStrategy};
use crate::server::hub::strategies::dag_step::{compute_cost, DagStepConfig, DagStepStrategy};
use crate::server::hub::streaming::NullSink;
use crate::server::state::AppState;
use crate::server::ws::events::WorkflowEventKind;
use crate::types::{DocumentSummary, RoutingConfigDocument, StepExecutionEnvelope, Subtask};

use super::dag_state::{resolve_output_key, wrap_in_envelope, PortMetadata};
use super::{
    broadcast_workflow_event, build_routing_instruction_block, compose_prompt,
    gather_downstream_routing_context, resolve_dot_path, resolve_port_inputs, StepOutput,
    WorkflowExecutionContext,
};

/// Execute a cavernous routing step: search for routing config documents,
/// select the best config via LLM, parse it, execute subtasks, and aggregate.
#[allow(clippy::too_many_arguments)]
pub(super) async fn execute_cavernous_step(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    agent: &AgentRow,
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
    // Broadcast: step started (cavernous)
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

    info!(step_id = %step.id, "Starting cavernous routing step");

    // ── Compose the base prompt (variable + port resolution) ──────────────
    let port_inputs = if let Some(inputs) = port_meta.step_inputs.get(&step.id) {
        match resolve_port_inputs(
            step.id,
            edges,
            inputs,
            &port_meta.step_outputs,
            completed_envelopes,
        ) {
            Ok(resolved) => Some(resolved),
            Err(e) => {
                warn!(
                    "Port resolution failed for cavernous step {}: {}",
                    step.id, e
                );
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

    // Append downstream routing context
    let mut prompt = prompt;
    let local_step_map: HashMap<Uuid, &WorkflowStepRow> = steps.iter().map(|s| (s.id, s)).collect();
    let downstream_contexts =
        gather_downstream_routing_context(step.id, edges, &local_step_map, port_meta, state).await;
    for routing_ctx in &downstream_contexts {
        prompt.push_str(&build_routing_instruction_block(routing_ctx));
    }

    // ── Create parent agent_execution row ─────────────────────────────────
    let ae_repo = state
        .agent_execution_repo()
        .ok_or_else(|| anyhow::anyhow!("agent_execution_repo not configured"))?;

    let parent_ae = ae_repo
        .create_agent_execution(
            agent.id,
            Some(step.id),
            false,
            None,
            &agent.system_prompt,
            &prompt,
            None,
            None,
            None,
            Some(ctx.stage_execution_id),
        )
        .await
        .map_err(|e| anyhow::anyhow!("failed to create cavernous agent execution: {}", e))?;

    // ── Phase 1: LLM generates search query ───────────────────────────────
    let cavernous_config = CavernousStepConfig {
        agent: agent.clone(),
        user_prompt: prompt.clone(),
        user_id: ctx.user_id,
        run_id: ctx.run_id,
    };
    let strategy = CavernousStepStrategy::new(cavernous_config);

    let sink = NullSink;
    let ae_repo_rec = state.agent_execution_repo();
    let tl_repo_rec = state.token_ledger_repo();
    let recorder = ExecutionRecorder::new(
        state.repo().as_ref(),
        ae_repo_rec.as_deref(),
        tl_repo_rec.as_deref(),
    );

    debug!(step_id = %step.id, "Cavernous Phase 1: generating search query");
    let phase1_result = engine
        .execute(&strategy, &prompt, &sink, &recorder, cancel)
        .await?;

    *total_input_tokens += phase1_result.input_tokens as i64;
    *total_output_tokens += phase1_result.output_tokens as i64;
    *total_cost_usd += compute_cost(
        &agent.model_id,
        phase1_result.input_tokens as i64,
        phase1_result.output_tokens as i64,
    );

    let search_query = strategy.search_query().await.ok_or_else(|| {
        HubError::Internal(anyhow!(
            "Cavernous Phase 1 failed: no search query generated"
        ))
    })?;

    info!(step_id = %step.id, query = %search_query, "Cavernous Phase 1 complete");

    // ── Programmatic: search routing documents ────────────────────────────
    let doc_repo = state
        .doc_repo()
        .ok_or_else(|| anyhow::anyhow!("doc_repo not configured"))?;

    let search_results = doc_repo
        .search_routing_documents(ctx.user_id, &search_query, 5)
        .await
        .map_err(|e| HubError::Internal(anyhow!("Routing document search failed: {}", e)))?;

    if search_results.is_empty() {
        return Err(HubError::Internal(anyhow!(
            "No routing documents found for query: {}",
            search_query
        )));
    }

    info!(
        step_id = %step.id,
        results = search_results.len(),
        "Found routing documents"
    );

    // ── Phase 2: LLM selects config ──────────────────────────────────────
    strategy.set_search_results(search_results.clone()).await;

    debug!(step_id = %step.id, "Cavernous Phase 2: selecting routing config");
    let phase2_result = engine
        .execute(&strategy, &prompt, &sink, &recorder, cancel)
        .await?;

    *total_input_tokens += phase2_result.input_tokens as i64;
    *total_output_tokens += phase2_result.output_tokens as i64;
    *total_cost_usd += compute_cost(
        &agent.model_id,
        phase2_result.input_tokens as i64,
        phase2_result.output_tokens as i64,
    );

    let selected_doc_id = strategy.selected_document_id().await.ok_or_else(|| {
        HubError::Internal(anyhow!("Cavernous Phase 2 failed: no config selected"))
    })?;

    info!(
        step_id = %step.id,
        selected_doc = %selected_doc_id,
        "Cavernous Phase 2 complete"
    );

    // ── Phase 3: Parse & validate routing config ─────────────────────────
    let selected_doc = doc_repo
        .get_document(selected_doc_id)
        .await
        .map_err(|e| HubError::Internal(anyhow!("Failed to fetch routing document: {}", e)))?
        .ok_or_else(|| {
            HubError::Internal(anyhow!(
                "Selected routing document {} not found",
                selected_doc_id
            ))
        })?;

    let routing_config: RoutingConfigDocument = serde_json::from_str(&selected_doc.content)
        .map_err(|e| {
            HubError::Internal(anyhow!(
                "Failed to parse routing config from document {}: {}",
                selected_doc_id,
                e
            ))
        })?;

    debug!(
        step_id = %step.id,
        strategy = %routing_config.strategy_name,
        subtasks = routing_config.subtasks.len(),
        "Parsed routing config"
    );

    // Store routing analysis
    let documents_found: Vec<DocumentSummary> = search_results
        .iter()
        .map(|r| DocumentSummary {
            id: r.id,
            title: r.title.clone(),
            description: r.summary.clone(),
            score: 0.0,
        })
        .collect();

    let routing_analysis = strategy
        .build_routing_analysis(documents_found)
        .await
        .ok_or_else(|| HubError::Internal(anyhow!("Failed to build routing analysis")))?;

    let analysis_json = serde_json::to_value(&routing_analysis)
        .map_err(|e| HubError::Internal(anyhow!("Failed to serialize routing analysis: {}", e)))?;

    let _ = ae_repo
        .update_agent_execution_routing(parent_ae.id, &analysis_json, Some(selected_doc_id))
        .await;

    // ── Phase 4: Execute subtasks ────────────────────────────────────────
    let layers = topo_sort_subtasks(&routing_config.subtasks)?;

    let mut subtask_outputs: HashMap<String, StepOutput> = HashMap::new();
    let subtask_order: Vec<String> = layers
        .iter()
        .flat_map(|layer: &Vec<&Subtask>| layer.iter().map(|s| s.id.clone()))
        .collect();

    for layer in &layers {
        if cancel.is_some_and(|c| c.is_cancelled()) {
            return Err(HubError::Internal(anyhow!("Cavernous step cancelled")));
        }

        // Execute subtasks within a layer in parallel (respecting max_parallel)
        let mut join_set = tokio::task::JoinSet::new();
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(
            routing_config.max_parallel.max(1),
        ));

        for subtask in layer {
            // Resolve prompt template with input_mapping from completed subtask outputs
            let resolved_prompt = resolve_subtask_prompt(subtask, &subtask_outputs, &prompt);

            let engine_clone = engine.clone_with_provider();
            let state_clone = state.clone();
            let ctx_clone = ctx.clone();
            let step_clone = step.clone();
            let agent_id = subtask.agent_id;
            let parent_ae_id = parent_ae.id;
            let subtask_id = subtask.id.clone();
            let cancel_token = cancel.cloned();
            let sem = semaphore.clone();

            join_set.spawn(async move {
                let _permit = sem
                    .acquire()
                    .await
                    .map_err(|e| HubError::Internal(anyhow!("Semaphore error: {}", e)))?;

                let result = run_cavernous_subtask(
                    &engine_clone,
                    &state_clone,
                    &ctx_clone,
                    &step_clone,
                    agent_id,
                    &resolved_prompt,
                    parent_ae_id,
                    cancel_token.as_ref(),
                )
                .await;

                result.map(|r| (subtask_id, r))
            });
        }

        // Collect results from this layer
        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(Ok((subtask_id, (output, in_tok, out_tok, cost)))) => {
                    *total_input_tokens += in_tok;
                    *total_output_tokens += out_tok;
                    *total_cost_usd += cost;
                    subtask_outputs.insert(subtask_id, output);
                }
                Ok(Err(e)) => {
                    error!(step_id = %step.id, error = %e, "Subtask execution failed");
                    // Continue with other subtasks; partial results are OK
                }
                Err(e) => {
                    error!(step_id = %step.id, error = %e, "Subtask join error");
                }
            }
        }
    }

    // ── Phase 5: Aggregate results ───────────────────────────────────────
    let aggregated = aggregate_subtask_outputs(
        &subtask_outputs,
        &routing_config.aggregation_mode,
        &subtask_order,
    );

    let variable_name = resolve_output_key(step, &port_meta.step_outputs);
    let output = StepOutput {
        variable_name: variable_name.clone(),
        structured_output: Some(aggregated.clone()),
        raw_output: serde_json::to_string_pretty(&aggregated).unwrap_or_default(),
    };

    if !variable_name.is_empty() {
        var_outputs.insert(variable_name, aggregated);
    }

    // Build envelope for downstream port resolution
    let total_step_in = phase1_result.input_tokens as i64 + phase2_result.input_tokens as i64;
    let total_step_out = phase1_result.output_tokens as i64 + phase2_result.output_tokens as i64;
    let total_step_cost = compute_cost(&agent.model_id, total_step_in, total_step_out);
    let mut envelope = wrap_in_envelope(
        &output,
        agent,
        step.id,
        total_step_in,
        total_step_out,
        total_step_cost,
    );
    envelope.metadata.selected_routing_document_id = Some(selected_doc_id);
    completed_envelopes.insert(step.id, envelope);

    completed.insert(step.id, output);

    // Update parent execution status
    let _ = ae_repo
        .update_agent_execution_status(parent_ae.id, "completed", None, None)
        .await;

    // Broadcast: step completed (cavernous)
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
            input_tokens: Some(*total_input_tokens as u64),
            output_tokens: Some(*total_output_tokens as u64),
            duration_ms: None,
        },
    );

    info!(
        step_id = %step.id,
        subtasks_completed = subtask_outputs.len(),
        "Cavernous routing step complete"
    );

    Ok(())
}

/// Resolve a subtask's prompt_template by substituting `{input.<subtask_id>.<path>}`
/// references with outputs from completed subtasks.
pub(super) fn resolve_subtask_prompt(
    subtask: &Subtask,
    completed_outputs: &HashMap<String, StepOutput>,
    base_prompt: &str,
) -> String {
    let mut resolved = subtask.prompt_template.clone();

    // Replace {base_prompt} with the parent step's composed prompt
    resolved = resolved.replace("{base_prompt}", base_prompt);

    // Replace input_mapping references: {input.<dep_id>} or {input.<dep_id>.<path>}
    for (placeholder, source) in &subtask.input_mapping {
        let value: String = if let Some(output) = completed_outputs.get(source) {
            if let Some(ref structured) = output.structured_output {
                serde_json::to_string(structured).unwrap_or_default()
            } else {
                output.raw_output.clone()
            }
        } else {
            // Try resolving dot-path: "subtask_id.field.path"
            let parts: Vec<&str> = source.splitn(2, '.').collect();
            if parts.len() == 2 {
                if let Some(output) = completed_outputs.get(parts[0]) {
                    if let Some(ref structured) = output.structured_output {
                        resolve_dot_path(structured, parts[1])
                            .map(|v| {
                                if let Some(s) = v.as_str() {
                                    s.to_string()
                                } else {
                                    serde_json::to_string(&v).unwrap_or_default()
                                }
                            })
                            .unwrap_or_default()
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        };

        let pattern = format!("{{{}}}", placeholder);
        resolved = resolved.replace(&pattern, &value);
    }

    resolved
}

/// Execute a single subtask within a cavernous routing step.
async fn run_cavernous_subtask(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    parent_step: &WorkflowStepRow,
    agent_id: Uuid,
    prompt: &str,
    parent_ae_id: Uuid,
    cancel: Option<&CancellationToken>,
) -> Result<(StepOutput, i64, i64, f32), HubError> {
    let agent = state
        .repo()
        .get_persisted_agent(agent_id)
        .await
        .map_err(|e| anyhow::anyhow!("failed to load subtask agent: {}", e))?
        .ok_or_else(|| HubError::AgentNotFound {
            step_id: parent_step.id,
            agent_id,
        })?;

    // Resolve mode
    let mode = if let Some(resolver) = state.mode_resolver() {
        resolver
            .resolve(&agent, prompt, None)
            .await
            .map_err(|e| HubError::Internal(anyhow!("Mode resolution failed: {}", e)))?
    } else {
        construct_agent_defaults(&agent, state.repo())
            .await
            .map_err(HubError::Internal)?
    };

    let mut system_prompt = mode.system_prompt;
    if let Some(ref suffix) = parent_step.system_prompt_suffix {
        if !suffix.trim().is_empty() {
            system_prompt.push_str("\n\n");
            system_prompt.push_str(suffix);
        }
    }
    if let Some(schema_id) = parent_step.output_schema_id {
        let os_repo = &state.repos().output_schemas;
        if let Ok(Some(schema)) = os_repo.get_output_schema(schema_id).await {
            system_prompt.push_str(&format!(
                "\n\n<schema>\nYour response is parsed directly by a JSON parser. Respond with a valid JSON object matching this schema:\n```json\n{}\n```\n</schema>",
                serde_json::to_string_pretty(&schema.schema).unwrap_or_default()
            ));
        }
    }

    // Create agent_execution row linked to parent
    let ae_repo = state
        .agent_execution_repo()
        .ok_or_else(|| anyhow::anyhow!("agent_execution_repo not configured"))?;

    let ae_row = ae_repo
        .create_agent_execution(
            agent.id,
            Some(parent_step.id),
            false,
            Some(parent_ae_id),
            &system_prompt,
            prompt,
            mode.selected_mode_id,
            None,
            None,
            Some(ctx.stage_execution_id),
        )
        .await
        .map_err(|e| anyhow::anyhow!("failed to create subtask agent execution: {}", e))?;

    // Build strategy using existing DagStepStrategy
    let config = DagStepConfig {
        agent: agent.clone(),
        step: parent_step.clone(),
        system_prompt,
        user_prompt: prompt.to_string(),
        tools: mode.tools,
        tool_names: mode.tool_names,
        temperature: mode.temperature,
        execution_context: ctx.execution_context.clone(),
        container_handle: None, // Subtasks don't get their own container
        run_id: ctx.run_id,
        user_id: ctx.user_id,
        agent_execution_id: ae_row.id,
    };
    let strategy = DagStepStrategy::new(config, state.clone());

    let sink = NullSink;
    let ae_repo_for_recorder = state.agent_execution_repo();
    let tl_repo_for_recorder = state.token_ledger_repo();
    let recorder = ExecutionRecorder::new(
        state.repo().as_ref(),
        ae_repo_for_recorder.as_deref(),
        tl_repo_for_recorder.as_deref(),
    );

    let result = engine
        .execute(&strategy, prompt, &sink, &recorder, cancel)
        .await?;

    let cost = compute_cost(
        &agent.model_id,
        result.input_tokens as i64,
        result.output_tokens as i64,
    );

    let structured = DagStepStrategy::parse_output(&result.content);

    let output = StepOutput {
        variable_name: String::new(), // Subtasks don't have variable names
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

/// Topological sort of subtasks into execution layers using Kahn's algorithm.
///
/// Returns layers where tasks within the same layer can execute in parallel,
/// and layers must execute sequentially (earlier layers before later ones).
pub(crate) fn topo_sort_subtasks(subtasks: &[Subtask]) -> Result<Vec<Vec<&Subtask>>, HubError> {
    if subtasks.is_empty() {
        return Ok(vec![]);
    }

    let id_to_subtask: HashMap<&str, &Subtask> =
        subtasks.iter().map(|s| (s.id.as_str(), s)).collect();

    // Build in-degree map and adjacency list
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();

    for subtask in subtasks {
        in_degree.entry(subtask.id.as_str()).or_insert(0);
        for dep in &subtask.depends_on {
            let dep_str: &str = dep.as_str();
            if !id_to_subtask.contains_key(dep_str) {
                return Err(HubError::Internal(anyhow!(
                    "Subtask '{}' depends on unknown subtask '{}'",
                    subtask.id,
                    dep
                )));
            }
            *in_degree.entry(subtask.id.as_str()).or_insert(0) += 1;
            dependents
                .entry(dep.as_str())
                .or_default()
                .push(subtask.id.as_str());
        }
    }

    let mut layers: Vec<Vec<&Subtask>> = Vec::new();
    let mut queue: Vec<&str> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&id, _)| id)
        .collect();
    queue.sort(); // Deterministic ordering

    let mut processed = 0;

    while !queue.is_empty() {
        let mut next_queue: Vec<&str> = Vec::new();
        let mut layer: Vec<&Subtask> = Vec::new();

        for &id in &queue {
            layer.push(id_to_subtask[id]);
            processed += 1;

            if let Some(deps) = dependents.get(id) {
                for &dep_id in deps {
                    let deg = in_degree.get_mut(dep_id).expect("dep validated in setup");
                    *deg -= 1;
                    if *deg == 0 {
                        next_queue.push(dep_id);
                    }
                }
            }
        }

        layer.sort_by_key(|s| &s.id); // Deterministic within layer
        layers.push(layer);
        next_queue.sort();
        queue = next_queue;
    }

    if processed != subtasks.len() {
        return Err(HubError::Internal(anyhow!(
            "Dependency cycle detected in subtasks"
        )));
    }

    Ok(layers)
}

/// Aggregate subtask outputs according to the specified mode.
///
/// Modes:
/// - `"all_outputs"`: Map of subtask_id → output
/// - `"final_output"`: Last subtask's output only
/// - `"merge"`: Shallow merge of all structured outputs into one object
pub(crate) fn aggregate_subtask_outputs(
    results: &HashMap<String, StepOutput>,
    aggregation_mode: &str,
    subtask_order: &[String],
) -> JsonValue {
    match aggregation_mode {
        "final_output" => {
            // Return the output of the last subtask in topo order
            for id in subtask_order.iter().rev() {
                if let Some(output) = results.get(id) {
                    return output
                        .structured_output
                        .clone()
                        .unwrap_or_else(|| JsonValue::String(output.raw_output.clone()));
                }
            }
            JsonValue::Null
        }
        "merge" => {
            // Shallow merge all structured outputs into one object
            let mut merged = serde_json::Map::new();
            for id in subtask_order {
                if let Some(output) = results.get(id) {
                    if let Some(JsonValue::Object(obj)) = &output.structured_output {
                        for (k, v) in obj {
                            merged.insert(k.clone(), v.clone());
                        }
                    }
                }
            }
            JsonValue::Object(merged)
        }
        // "all_outputs" and any unknown mode
        _ => {
            let mut map = serde_json::Map::new();
            for id in subtask_order {
                if let Some(output) = results.get(id) {
                    let value = output
                        .structured_output
                        .clone()
                        .unwrap_or_else(|| JsonValue::String(output.raw_output.clone()));
                    map.insert(id.clone(), value);
                }
            }
            JsonValue::Object(map)
        }
    }
}
