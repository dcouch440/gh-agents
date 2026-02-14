//! Task force step execution within the DAG.
//!
//! When the DAG encounters a step with `execution_mode = "task_force"`, this
//! module loads the mission brief and agent roster, runs the Agent Designer
//! pre-lifecycle to generate optimized prompts, then executes each roster
//! agent sequentially with designed prompts. Combined results are wrapped
//! in a `StepExecutionEnvelope`.

pub(super) mod designer;
mod tests;

use std::collections::HashMap;

use anyhow::anyhow;
use serde_json::Value as JsonValue;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use uuid::Uuid;

use crate::config::protocols::TASK_FORCE;
use crate::db::{WorkflowStepEdgeRow, WorkflowStepRow};
use crate::server::hub::capability_resolver::resolve_capabilities_to_tools;
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::error::HubError;
use crate::server::hub::protocols::execution_recorder::ProtocolExecutionRecorder;
use crate::server::hub::recorder::ExecutionRecorder;
use crate::server::hub::strategies::compute_cost;
use crate::server::hub::strategies::task_force::{TaskForceAgentConfig, TaskForceAgentStrategy};
use crate::server::hub::streaming::NullSink;
use crate::server::state::AppState;
use crate::server::ws::events::WorkflowEventKind;
use crate::types::{ExecutionMetadata, ExecutionStatus, StepExecutionEnvelope, UserId};

use super::agent_designer::normalize_agent_name;
use super::container::{create_optional_container, destroy_optional_container};
use super::{
    broadcast_workflow_event, compose_prompt, resolve_output_key, resolve_port_inputs,
    step_display_name, PortMetadata, StepOutput, WorkflowExecutionContext,
};

/// Execute a task force step within the DAG.
///
/// Loads the mission brief and agent roster, runs the Agent Designer
/// pre-lifecycle to generate optimized prompts and tool assignments, then
/// executes each roster agent sequentially with designed prompts.
/// The combined output is a JSON object keyed by agent name.
#[allow(clippy::too_many_arguments)]
pub(super) async fn execute_task_force_step(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    _steps: &[WorkflowStepRow],
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

    // 1. Broadcast step started
    broadcast_workflow_event(
        state,
        ctx,
        step.workflow_id,
        WorkflowEventKind::StepStarted {
            step_id: step.id,
            step_name: step_display_name(step),
            agent_id: None,
            execution_id: None,
        },
    );

    // 2. Load mission brief
    let brief = state
        .repos()
        .workflows
        .get_mission_brief(step.id)
        .await
        .map_err(|e| HubError::Internal(anyhow!("failed to load mission brief: {}", e)))?
        .ok_or_else(|| {
            HubError::Internal(anyhow!("task_force step {} has no mission brief", step.id))
        })?;

    // 3. Load agent roster (sorted by execution_order)
    let roster = state
        .repos()
        .workflows
        .list_agent_roster(brief.id)
        .await
        .map_err(|e| HubError::Internal(anyhow!("failed to load agent roster: {}", e)))?;

    if roster.is_empty() {
        return Err(HubError::Internal(anyhow!(
            "task_force step {} has empty agent roster",
            step.id
        )));
    }

    info!(
        step_id = %step.id,
        task = %brief.task_description,
        agents = roster.len(),
        failure_mode = %brief.failure_mode,
        "Starting task force step execution"
    );

    // 4. Resolve port inputs
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
                    "Port resolution failed for task_force step {}: {}",
                    step.id, e
                );
                None
            }
        }
    } else {
        None
    };

    // 5. Compose base prompt
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

    // 6. Create protocol execution recorder
    let recorder = ProtocolExecutionRecorder::new(&*state.repos().protocols, step.id, ctx.run_id);

    // 7. Create optional container
    let managed_container = create_optional_container(
        ctx.container_config.as_ref(),
        ctx.wg_client.as_deref(),
        "task_force",
    )
    .await?;

    // 8. Run Agent Designer pre-lifecycle to generate optimized prompts
    let (designed_prompts, designer_usage) = designer::run_agent_designer(
        engine,
        state,
        ctx,
        step,
        &brief,
        &roster,
        completed_envelopes,
        cancel,
    )
    .await?;

    // 9. Sequential agent execution loop (using designer-generated prompts)
    let mut agent_outputs: Vec<(String, String)> = Vec::new();
    let mut step_in_tokens: i64 = designer_usage.input_tokens;
    let mut step_out_tokens: i64 = designer_usage.output_tokens;
    let mut step_cost: f32 = designer_usage.cost_usd;
    let total_agents = designed_prompts.len();
    let agent_cfg = TASK_FORCE.agent("agent");

    for (idx, designed) in designed_prompts.iter().enumerate() {
        // Check cancellation
        if cancel.is_some_and(|t| t.is_cancelled()) {
            destroy_optional_container(&managed_container, ctx.wg_client.as_deref()).await;
            return Err(HubError::Cancelled);
        }

        // Broadcast: agent started
        broadcast_workflow_event(
            state,
            ctx,
            step.workflow_id,
            WorkflowEventKind::TaskForceAgentProgress {
                step_id: step.id,
                agent_name: designed.agent_name.clone(),
                agent_index: idx,
                total_agents,
                status: "started".to_string(),
            },
        );

        // Create protocol execution row
        let exec_row = recorder
            .create_phase(&format!("agent_{}", idx), None, Some(&prompt))
            .await?;

        // Resolve capabilities from designer-assigned tools
        let (tools, tool_names) =
            resolve_capabilities_to_tools(&designed.tools, &*state.repos().tool_capabilities)
                .await
                .unwrap_or_else(|e| {
                    warn!(
                        agent = %designed.agent_name,
                        "Capability resolution failed: {}", e
                    );
                    (vec![], vec![])
                });

        // Inject previous outputs at runtime, filtered by designer routing
        let filtered = filter_outputs_for_agent(&agent_outputs, &designed.receives_from);
        let task_prompt = if filtered.is_empty() {
            designed.task_prompt.clone()
        } else {
            let previous_outputs = build_filtered_outputs_block(&filtered);
            format!(
                "{}\n\n<previous_agent_outputs>\n{}\n</previous_agent_outputs>",
                designed.task_prompt, previous_outputs
            )
        };

        // Build strategy with designer-generated system prompt
        let strategy = TaskForceAgentStrategy::new(TaskForceAgentConfig {
            system_prompt: designed.system_prompt.clone(),
            model_id: agent_cfg.model_id.clone(),
            temperature: agent_cfg.temperature,
            max_rounds: agent_cfg.max_rounds,
            context_budget: agent_cfg.context_budget,
            tools,
            tool_names,
            execution_context: ctx.execution_context.clone(),
            container_handle: managed_container.as_ref().map(|mc| mc.agent_handle.clone()),
            state: Some(state.clone()),
            user_id: Some(UserId(ctx.user_id)),
        });

        // Execute via engine
        let inner_recorder = ExecutionRecorder::new(&**state.repo(), None, None);
        let result = engine
            .clone_with_provider()
            .execute(&strategy, &task_prompt, &NullSink, &inner_recorder, cancel)
            .await;

        match result {
            Ok(exec_result) => {
                let cost = compute_cost(
                    &agent_cfg.model_id,
                    exec_result.input_tokens as i64,
                    exec_result.output_tokens as i64,
                );
                step_in_tokens += exec_result.input_tokens as i64;
                step_out_tokens += exec_result.output_tokens as i64;
                step_cost += cost;

                agent_outputs.push((designed.agent_name.clone(), exec_result.content.clone()));

                recorder
                    .update_phase(
                        exec_row.id,
                        "complete",
                        Some(&exec_result.content),
                        None,
                        exec_result.input_tokens as i64,
                        exec_result.output_tokens as i64,
                        cost,
                        Some(&agent_cfg.model_id),
                    )
                    .await;

                info!(
                    agent = %designed.agent_name,
                    idx = idx,
                    tokens_in = exec_result.input_tokens,
                    tokens_out = exec_result.output_tokens,
                    "Task force agent completed"
                );

                // Broadcast: agent completed
                broadcast_workflow_event(
                    state,
                    ctx,
                    step.workflow_id,
                    WorkflowEventKind::TaskForceAgentProgress {
                        step_id: step.id,
                        agent_name: designed.agent_name.clone(),
                        agent_index: idx,
                        total_agents,
                        status: "completed".to_string(),
                    },
                );
            }
            Err(e) => {
                let err_msg = format!("{}", e);
                recorder
                    .update_phase(
                        exec_row.id,
                        "failed",
                        None,
                        Some(&err_msg),
                        0,
                        0,
                        0.0,
                        Some(&agent_cfg.model_id),
                    )
                    .await;

                // Broadcast: agent failed
                broadcast_workflow_event(
                    state,
                    ctx,
                    step.workflow_id,
                    WorkflowEventKind::TaskForceAgentProgress {
                        step_id: step.id,
                        agent_name: designed.agent_name.clone(),
                        agent_index: idx,
                        total_agents,
                        status: "failed".to_string(),
                    },
                );

                match brief.failure_mode.as_str() {
                    "fail_fast" => {
                        warn!(
                            agent = %designed.agent_name,
                            error = %err_msg,
                            "Task force agent failed (fail_fast)"
                        );
                        destroy_optional_container(&managed_container, ctx.wg_client.as_deref())
                            .await;
                        return Err(e);
                    }
                    _ => {
                        // skip_and_continue (or any other mode)
                        warn!(
                            agent = %designed.agent_name,
                            error = %err_msg,
                            "Task force agent failed, skipping ({})", brief.failure_mode
                        );
                        agent_outputs.push((
                            designed.agent_name.clone(),
                            format!("[AGENT FAILED: {}]", err_msg),
                        ));
                    }
                }
            }
        }
    }

    // 10. Destroy optional container
    destroy_optional_container(&managed_container, ctx.wg_client.as_deref()).await;

    // 11. Compose combined output
    let combined_data = compose_combined_output(&agent_outputs);
    let output_key = resolve_output_key(step, &port_meta.step_outputs);

    // 12. Store results
    *total_input_tokens += step_in_tokens;
    *total_output_tokens += step_out_tokens;
    *total_cost_usd += step_cost;

    let output = StepOutput {
        variable_name: output_key,
        raw_output: serde_json::to_string(&combined_data).unwrap_or_default(),
        structured_output: Some(combined_data.clone()),
    };

    if !output.variable_name.is_empty() {
        if let Some(ref structured) = output.structured_output {
            var_outputs.insert(output.variable_name.clone(), structured.clone());
        }
    }

    let envelope = StepExecutionEnvelope {
        status: ExecutionStatus::Success,
        data: Some(combined_data),
        metadata: ExecutionMetadata {
            execution_id: step.id,
            execution_time_ms: step_start.elapsed().as_millis() as u64,
            tokens_in: Some(step_in_tokens as i32),
            tokens_out: Some(step_out_tokens as i32),
            cost_usd: Some(step_cost as f64),
            model: Some(agent_cfg.model_id.clone()),
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

    // 13. Broadcast step completed
    broadcast_workflow_event(
        state,
        ctx,
        step.workflow_id,
        WorkflowEventKind::StepCompleted {
            step_id: step.id,
            step_name: step_display_name(step),
            agent_id: None,
            output: None,
            input_tokens: Some(step_in_tokens as u64),
            output_tokens: Some(step_out_tokens as u64),
            duration_ms: Some(step_start.elapsed().as_millis() as u64),
        },
    );

    info!(
        step_id = %step.id,
        agents = total_agents,
        tokens_in = step_in_tokens,
        tokens_out = step_out_tokens,
        duration_ms = step_start.elapsed().as_millis(),
        "Task force step execution completed"
    );

    Ok(())
}

// ── Helper functions ─────────────────────────────────────────────────────

/// Build a human-readable team roster string for prompt injection.
#[cfg(test)]
fn build_team_roster_string(roster: &[crate::db::TaskAgentRosterRow]) -> String {
    roster
        .iter()
        .map(|a| {
            let caps = if a.capabilities.is_empty() {
                String::new()
            } else {
                format!(" [{}]", a.capabilities.join(", "))
            };
            format!(
                "- **{}** (order {}): {}{}",
                a.name, a.execution_order, a.role_description, caps
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Build the "previous outputs" block for injection into an agent's system prompt.
#[cfg(test)]
fn build_previous_outputs_block(agent_outputs: &[(String, String)]) -> String {
    if agent_outputs.is_empty() {
        "No previous agent outputs yet. You are the first agent to execute.".to_string()
    } else {
        agent_outputs
            .iter()
            .map(|(name, output)| format!("### {}\n{}", name, output))
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

/// Filter agent outputs based on receives_from routing.
/// If receives_from is empty, returns all outputs (backwards-compatible default).
/// If receives_from is non-empty, returns only outputs from the named agents.
/// Uses normalized matching to handle case style differences.
fn filter_outputs_for_agent<'a>(
    agent_outputs: &'a [(String, String)],
    receives_from: &[String],
) -> Vec<&'a (String, String)> {
    if receives_from.is_empty() {
        agent_outputs.iter().collect()
    } else {
        let normalized_receives: Vec<String> = receives_from
            .iter()
            .map(|n| normalize_agent_name(n))
            .collect();
        agent_outputs
            .iter()
            .filter(|(name, _)| normalized_receives.contains(&normalize_agent_name(name)))
            .collect()
    }
}

/// Build the "previous outputs" block from a filtered set of agent outputs.
fn build_filtered_outputs_block(outputs: &[&(String, String)]) -> String {
    if outputs.is_empty() {
        "No previous agent outputs yet. You are the first agent to execute.".to_string()
    } else {
        outputs
            .iter()
            .map(|(name, output)| format!("### {}\n{}", name, output))
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

/// Compose combined output from all agents' results, keyed by normalized agent name.
fn compose_combined_output(agent_outputs: &[(String, String)]) -> JsonValue {
    let mut composite = serde_json::Map::new();
    for (name, output) in agent_outputs {
        let key = name.to_lowercase().replace(' ', "_");
        let value: JsonValue =
            serde_json::from_str(output).unwrap_or_else(|_| JsonValue::String(output.clone()));
        composite.insert(key, value);
    }
    JsonValue::Object(composite)
}
