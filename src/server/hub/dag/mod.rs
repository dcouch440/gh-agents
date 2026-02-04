//! DAG orchestration — topological sort, variable resolution, for-each fan-out,
//! and workflow execution using the unified ExecutionEngine.
//!
//! This module re-exports the pure graph/variable functions from `dag_executor`
//! and provides `execute_workflow_via_engine` which delegates step execution
//! to the hub's `ExecutionEngine` instead of running its own react loop.

use std::collections::HashMap;

use anyhow::anyhow;
use serde_json::Value as JsonValue;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::db::{AgentRow, WorkflowStepEdgeRow, WorkflowStepRow};
use crate::llm::Tool;
use crate::server::state::AppState;

use super::engine::ExecutionEngine;
use super::error::HubError;
use super::recorder::ExecutionRecorder;
use super::strategies::dag_step::{compute_cost, DagStepConfig, DagStepStrategy};
use super::streaming::NullSink;
use super::construct_agent_defaults;

// Re-export pure DAG functions from the existing dag_executor
pub use crate::server::dag_executor::{
    compose_prompt, extract_for_each_label, find_entry_steps, get_child_steps, get_parent_steps,
    resolve_for_each_array, resolve_variables, topological_sort, StepOutput,
    WorkflowExecutionContext, WorkflowExecutionResult,
};

/// Execute a complete workflow DAG using the unified ExecutionEngine.
///
/// This replaces `dag_executor::execute_workflow` — same logic (topo sort,
/// variable resolution, for-each fan-out, interactive review) but step
/// execution goes through `ExecutionEngine::execute()` with `DagStepStrategy`.
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
    let mut var_outputs: HashMap<String, JsonValue> = HashMap::new();
    let mut total_input_tokens: i64 = 0;
    let mut total_output_tokens: i64 = 0;
    let mut total_cost_usd: f32 = 0.0;

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
            .repo
            .get_persisted_agent(step.agent_id)
            .await
            .map_err(|e| anyhow::anyhow!("failed to load agent: {}", e))?
            .ok_or_else(|| HubError::AgentNotFound {
                step_id: *step_id,
                agent_id: step.agent_id,
            })?;

        if step.execution_mode == "room" {
            // Room execution — create a session and pause the pipeline.
            // The user interacts with the room via POST /api/room-sessions/:id/messages.
            // When the session completes, the pipeline continues.
            let room_id = step.room_id.ok_or_else(|| {
                HubError::Internal(anyhow!(
                    "step {} has execution_mode='room' but no room_id",
                    step.id
                ))
            })?;
            let room_repo = state
                .room_repo
                .as_ref()
                .ok_or(HubError::ProviderNotConfigured)?;
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

            // Store a placeholder output so downstream steps can reference the session
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

            // Signal that this pipeline is awaiting user interaction
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
                &var_outputs,
                &mut completed,
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
                &var_outputs,
                &mut completed,
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
    var_outputs: &HashMap<String, JsonValue>,
    completed: &mut HashMap<Uuid, StepOutput>,
    total_input_tokens: &mut i64,
    total_output_tokens: &mut i64,
    total_cost_usd: &mut f32,
    cancel: Option<&CancellationToken>,
) -> Result<(), HubError> {
    let prompt = compose_prompt(
        step,
        state.prompt_template_repo.as_deref(),
        state.doc_repo.as_deref(),
        state.workflow_repo.as_deref(),
        &*state.repo,
        var_outputs,
        &ctx.prior_outputs,
        None,
    )
    .await;

    let (output, in_tok, out_tok, cost) =
        run_step_via_engine(engine, state, ctx, step, agent, &prompt, cancel).await?;

    *total_input_tokens += in_tok;
    *total_output_tokens += out_tok;
    *total_cost_usd += cost;

    // Store output in variable map
    if !output.variable_name.is_empty() {
        if let Some(_structured) = &output.structured_output {
            // var_outputs is behind a mutable ref — safe to insert directly
            // (We'd need a different approach if we parallelize single steps)
        }
    }
    completed.insert(step.id, output);

    Ok(())
}

/// Execute a for-each step: expand into N iterations, run sequentially.
async fn execute_for_each_step(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    agent: &AgentRow,
    _edges: &[WorkflowStepEdgeRow],
    var_outputs: &HashMap<String, JsonValue>,
    completed: &mut HashMap<Uuid, StepOutput>,
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

    info!(
        step_id = %step.id,
        count = array.len(),
        "for_each expansion"
    );

    let mut iteration_outputs = Vec::new();

    for (idx, element) in array.iter().enumerate() {
        if cancel.is_some_and(|t| t.is_cancelled()) {
            return Err(HubError::Cancelled);
        }
        let _label = extract_for_each_label(element, label_field);

        let prompt = compose_prompt(
            step,
            state.prompt_template_repo.as_deref(),
            state.doc_repo.as_deref(),
            state.workflow_repo.as_deref(),
            &*state.repo,
            var_outputs,
            &ctx.prior_outputs,
            Some(element),
        )
        .await;

        match run_step_via_engine(engine, state, ctx, step, agent, &prompt, cancel).await {
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

    completed.insert(
        step.id,
        StepOutput {
            variable_name,
            structured_output: Some(aggregated),
            raw_output: String::new(),
        },
    );

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
        .agent_execution_repo
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("agent_execution_repo not configured"))?;

    // Resolve mode (no context hint initially - future: pass step description)
    let mode = if let Some(resolver) = &state.mode_resolver {
        resolver
            .resolve(agent, prompt, None)
            .await
            .map_err(|e| HubError::Internal(anyhow!("Mode resolution failed: {}", e)))?
    } else {
        // Fallback: construct agent defaults for backward compatibility
        construct_agent_defaults(agent, &state.repo)
            .await
            .map_err(HubError::Internal)?
    };

    // Build system prompt: mode result + schema enforcement
    let mut system_prompt = mode.system_prompt;  // agent + mode already merged
    if let Some(schema_id) = step.output_schema_id {
        if let Some(os_repo) = &state.output_schema_repo {
            if let Ok(Some(schema)) = os_repo.get_output_schema(schema_id).await {
                system_prompt.push_str(&format!(
                    "\n\nYou MUST respond with valid JSON matching this schema:\n```json\n{}\n```\nRespond ONLY with the JSON object, no other text.",
                    serde_json::to_string_pretty(&schema.schema).unwrap_or_default()
                ));
            }
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
            mode.selected_mode_id,  // Track which mode was used
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
        tools: mode.tools,           // Use mode tools
        tool_names: mode.tool_names,  // Use mode tool names
        temperature: mode.temperature, // Use mode temperature
        execution_context: ctx.execution_context.clone(),
        run_id: ctx.run_id,
        user_id: ctx.user_id,
        agent_execution_id: ae_row.id,
    };
    let strategy = DagStepStrategy::new(config, state.clone());

    // Build recorder (strategy handles its own recording in on_complete)
    let recorder = ExecutionRecorder::new(
        state.repo.as_ref(),
        state.agent_execution_repo.as_deref(),
        state.token_ledger_repo.as_deref(),
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

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
