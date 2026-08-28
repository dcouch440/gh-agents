//! Step dispatch — routes each step to its executor and handles agent/provider resolution.

use tracing::info;
use uuid::Uuid;

use crate::db::WorkflowStepRow;
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::error::HubError;
use crate::server::state::AppState;
use crate::server::ws::events::WorkflowEventKind;

use super::super::broadcast::broadcast_workflow_event;
use super::super::single::execute_single_step;
use super::super::utils;
use super::super::{
    missing_agent_error, resolve_output_key, step_display_name, wrap_in_agentless_envelope,
    DagContext, DagExecutionState, StepOutput,
};

/// Route a step to the correct executor based on its execution mode.
///
/// - `context` / `input` → pass-through (no LLM call)
/// - `workforce` → file-based agent execution (reads system node agent's files)
///   Falls back to Pipeline with DesignerPhase for legacy steps without files.
/// - Everything else → single agent execution with provider resolution
pub(crate) async fn dispatch_step(
    dag: &DagContext<'_>,
    dag_state: &mut DagExecutionState,
    step: &WorkflowStepRow,
) -> Result<(), HubError> {
    match step.execution_mode.as_str() {
        "context" | "input" => execute_passthrough(dag, dag_state, step).await,
        _ if step.child_workflow_id.is_some() => {
            // File-based execution: read system node agent's config files
            let base_dir = crate::server::services::system_node::resolve_base_dir(
                dag.state,
                step.workflow_id,
                step.id,
            );
            super::super::file_executor::execute_from_files(dag, step, dag_state, &base_dir).await
        }
        _ => execute_with_agent(dag, dag_state, step).await,
    }
}

/// Context/input pass-through — forward prompt_template as output with no LLM call.
async fn execute_passthrough(
    dag: &DagContext<'_>,
    dag_state: &mut DagExecutionState,
    step: &WorkflowStepRow,
) -> Result<(), HubError> {
    let step_start = std::time::Instant::now();
    let output_key = resolve_output_key(step, &dag.port_meta.step_outputs);
    let content = passthrough_content(step, &dag.ctx.initial_input);
    let (output, value) = StepOutput::passthrough(output_key, content);
    let envelope = wrap_in_agentless_envelope(step.id, Some(value), 0, 0, 0, 0.0);

    utils::record_and_snapshot_output(dag, dag_state, step.id, output, envelope).await;

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

    info!(step_id = %step.id, "Context step pass-through completed");
    Ok(())
}

/// Load agent, resolve LLM provider, and execute via the engine.
async fn execute_with_agent(
    dag: &DagContext<'_>,
    dag_state: &mut DagExecutionState,
    step: &WorkflowStepRow,
) -> Result<(), HubError> {
    let agent_id = step.agent_id.ok_or_else(|| missing_agent_error(step))?;

    let agent = load_agent(dag, step.id, agent_id).await?;
    let step_engine = resolve_provider(dag, step.id, &agent).await?;
    let effective_engine = step_engine.as_ref().unwrap_or(dag.engine);

    let step_dag = DagContext {
        engine: effective_engine,
        ..*dag
    };

    execute_single_step(&step_dag, step, &agent, dag_state).await
}

/// Load an agent from snapshot or live DB.
async fn load_agent(
    dag: &DagContext<'_>,
    step_id: Uuid,
    agent_id: Uuid,
) -> Result<crate::db::AgentRow, HubError> {
    if let Some(snap) = &dag.ctx.snapshot {
        snap.agents
            .get(&agent_id)
            .cloned()
            .ok_or(HubError::AgentNotFound { step_id, agent_id })
    } else {
        dag.state
            .repos()
            .agents
            .get_persisted_agent(agent_id)
            .await
            .map_err(|e| anyhow::anyhow!("failed to load agent: {}", e))?
            .ok_or(HubError::AgentNotFound { step_id, agent_id })
    }
}

/// Resolve the LLM provider for an agent. Returns `None` for the default provider.
async fn resolve_provider(
    dag: &DagContext<'_>,
    step_id: Uuid,
    agent: &crate::db::AgentRow,
) -> Result<Option<ExecutionEngine>, HubError> {
    if agent.model_provider.is_empty() || agent.model_provider == crate::constants::ACTIVE_PROVIDER
    {
        return Ok(None);
    }

    if agent.model_provider == "ollama" && !dag.state.is_ollama_enabled().await {
        return Err(HubError::ProviderUnavailable {
            provider: agent.model_provider.clone(),
            step_id,
            agent_name: agent.name.clone(),
        });
    }

    let provider = dag
        .state
        .provider_for(&agent.model_provider)
        .ok_or_else(|| HubError::ProviderUnavailable {
            provider: agent.model_provider.clone(),
            step_id,
            agent_name: agent.name.clone(),
        })?;

    Ok(Some(ExecutionEngine::new(
        provider,
        dag.state.env().debug_stream,
    )))
}

/// Resolve the content for a passthrough step: prompt_template if set, otherwise initial_input.
fn passthrough_content(step: &WorkflowStepRow, initial_input: &str) -> String {
    if step.prompt_template.is_empty() {
        initial_input.to_owned()
    } else {
        step.prompt_template.clone()
    }
}

/// Spawn a background run results summarization if the step has completed output.
pub(super) fn spawn_summarizer_if_completed(
    state: &AppState,
    step_id: Uuid,
    dag_state: &DagExecutionState,
) {
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
