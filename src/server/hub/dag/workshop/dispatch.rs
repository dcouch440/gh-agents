//! Step execution dispatch for workshop mode.
//!
//! Mirrors the dispatch logic in `run_dag_loop()` but for a single step.
//! All existing step executors are reused unchanged.

use std::time::Instant;

use serde_json::Value as JsonValue;
use tracing::info;
use uuid::Uuid;

use crate::db::{WorkflowStepEdgeRow, WorkflowStepRow};
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::error::HubError;
use crate::server::state::AppState;

use super::super::dag_state::{
    resolve_output_key, wrap_in_agentless_envelope, DagContext, DagExecutionState, PortMetadata,
};
use super::super::pipeline::{DesignerPhase, Pipeline};
use super::super::single::execute_single_step;
use super::super::utils::{StepOutput, WorkflowExecutionContext};
use super::super::versioning;
use super::types::WorkshopStepResult;

/// Execute a single workshop step, dispatching to the appropriate executor.
pub(crate) async fn execute_step(
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    steps: &[WorkflowStepRow],
    edges: &[WorkflowStepEdgeRow],
    dag_state: &mut DagExecutionState,
    port_meta: &PortMetadata,
) -> Result<WorkshopStepResult, HubError> {
    let step_start = Instant::now();

    let provider = state
        .provider()
        .ok_or_else(|| HubError::Internal(anyhow::anyhow!("LLM provider not configured")))?
        .clone();
    let engine = ExecutionEngine::new(provider);

    info!(
        step_id = %step.id,
        mode = %step.execution_mode,
        "Workshop step execution starting"
    );

    // Context/input pass-through — no LLM call needed
    if step.execution_mode == "context" || step.execution_mode == "input" {
        return execute_passthrough(state, ctx, step, dag_state, port_meta, step_start).await;
    }

    // Build DagContext for step executors
    let dag = DagContext {
        engine: &engine,
        state,
        ctx,
        steps,
        edges,
        port_meta,
        cancel: None,
    };

    match step.execution_mode.as_str() {
        "workforce" => execute_workforce(&dag, step, dag_state, step_start).await,
        _ => execute_agent(&dag, state, step, dag_state, &engine, step_start).await,
    }
}

/// Pass-through execution for context/input steps — no LLM call.
async fn execute_passthrough(
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    dag_state: &mut DagExecutionState,
    port_meta: &PortMetadata,
    step_start: Instant,
) -> Result<WorkshopStepResult, HubError> {
    let data = if step.prompt_template.is_empty() {
        None
    } else {
        Some(JsonValue::String(step.prompt_template.clone()))
    };

    let envelope = wrap_in_agentless_envelope(step.id, data.clone(), 0, 0, 0, 0.0);

    // Snapshot the envelope
    let envelope_json = serde_json::to_string(&envelope).unwrap_or_default();
    let _ = versioning::snapshot_content(
        &*state.repos().content_versions,
        ctx.run_id,
        step.id,
        step.id,
        versioning::content_types::ENVELOPE,
        "output",
        &envelope_json,
    )
    .await;

    let output_key = resolve_output_key(step, &port_meta.step_outputs);
    let output = StepOutput {
        variable_name: output_key,
        structured_output: data.clone(),
        raw_output: step.prompt_template.clone(),
    };
    dag_state.record_step_output(step.id, output, envelope);

    Ok(WorkshopStepResult {
        step_id: step.id,
        status: "completed".to_string(),
        output: data,
        tokens_in: 0,
        tokens_out: 0,
        cost_usd: 0.0,
        duration_ms: step_start.elapsed().as_millis() as u64,
    })
}

/// Workforce (pipeline) step execution — agentless, designer + agent loop.
async fn execute_workforce(
    dag: &DagContext<'_>,
    step: &WorkflowStepRow,
    dag_state: &mut DagExecutionState,
    step_start: Instant,
) -> Result<WorkshopStepResult, HubError> {
    let pre_tokens_in = dag_state.total_input_tokens;
    let pre_tokens_out = dag_state.total_output_tokens;
    let pre_cost = dag_state.total_cost_usd;

    Pipeline::new()
        .before(DesignerPhase)
        .execute(dag, step, dag_state)
        .await?;

    let output = dag_state
        .completed
        .get(&step.id)
        .and_then(|o| o.structured_output.clone());

    Ok(WorkshopStepResult {
        step_id: step.id,
        status: "completed".to_string(),
        output,
        tokens_in: dag_state.total_input_tokens - pre_tokens_in,
        tokens_out: dag_state.total_output_tokens - pre_tokens_out,
        cost_usd: dag_state.total_cost_usd - pre_cost,
        duration_ms: step_start.elapsed().as_millis() as u64,
    })
}

/// Agent-bearing step execution — loads agent, resolves provider, dispatches.
async fn execute_agent(
    dag: &DagContext<'_>,
    state: &AppState,
    step: &WorkflowStepRow,
    dag_state: &mut DagExecutionState,
    default_engine: &ExecutionEngine,
    step_start: Instant,
) -> Result<WorkshopStepResult, HubError> {
    let agent_id = step.agent_id.ok_or_else(|| {
        HubError::Internal(anyhow::anyhow!(
            "step {} has no agent_id for mode '{}'",
            step.id,
            step.execution_mode
        ))
    })?;

    let agent = state
        .repos()
        .agents
        .get_persisted_agent(agent_id)
        .await
        .map_err(|e| anyhow::anyhow!("failed to load agent: {}", e))?
        .ok_or_else(|| HubError::AgentNotFound {
            step_id: step.id,
            agent_id,
        })?;

    // Resolve provider: empty or active → default engine, explicit name → named provider
    let step_engine = if agent.model_provider.is_empty()
        || agent.model_provider == crate::constants::ACTIVE_PROVIDER
    {
        None
    } else {
        let provider = state.provider_for(&agent.model_provider).ok_or_else(|| {
            HubError::ProviderUnavailable {
                provider: agent.model_provider.clone(),
                step_id: step.id,
                agent_name: agent.name.clone(),
            }
        })?;
        Some(ExecutionEngine::new(provider))
    };
    let effective_engine = step_engine.as_ref().unwrap_or(default_engine);

    let step_dag = DagContext {
        engine: effective_engine,
        ..*dag
    };

    let pre_tokens_in = dag_state.total_input_tokens;
    let pre_tokens_out = dag_state.total_output_tokens;
    let pre_cost = dag_state.total_cost_usd;

    execute_single_step(&step_dag, step, &agent, dag_state).await?;

    let output = dag_state
        .completed
        .get(&step.id)
        .and_then(|o| o.structured_output.clone());

    Ok(WorkshopStepResult {
        step_id: step.id,
        status: "completed".to_string(),
        output,
        tokens_in: dag_state.total_input_tokens - pre_tokens_in,
        tokens_out: dag_state.total_output_tokens - pre_tokens_out,
        cost_usd: dag_state.total_cost_usd - pre_cost,
        duration_ms: step_start.elapsed().as_millis() as u64,
    })
}
