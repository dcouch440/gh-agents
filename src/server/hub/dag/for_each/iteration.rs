//! Single for-each step execution: expand into N iterations, run sequentially.
//!
//! Supports label-based routing: when `routing_mode = "label"`, each element
//! is routed to a different agent based on the value of `routing_field`.

use std::collections::HashMap;

use serde_json::Value as JsonValue;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::db::{AgentRow, WorkflowStepEdgeRow, WorkflowStepRow};
use crate::server::hub::dag::container::{
    create_optional_container, destroy_optional_container, run_with_vpn_watchdog,
};
use crate::server::hub::dag::single::run_step_via_engine;
use crate::server::hub::dag::{
    broadcast_workflow_event, compose_prompt, extract_for_each_label, resolve_for_each_array,
    resolve_output_key, wrap_in_envelope, PortMetadata, StepOutput, WorkflowExecutionContext,
};
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::error::HubError;
use crate::server::state::AppState;
use crate::server::ws::events::WorkflowEventKind;
use crate::types::StepExecutionEnvelope;

/// Execute a for-each step: expand into N iterations, run sequentially.
#[allow(clippy::too_many_arguments)]
pub(in crate::server::hub::dag) async fn execute_for_each_step(
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
    let mut iteration_outputs = Vec::with_capacity(array.len());
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
