//! File-based agent execution bridge.
//!
//! Reads the system node agent's output files (`config.json`, `topology.json`,
//! `agents/*.json`) and executes the configured agents through the shared
//! runner. This is the execution bridge between the system node agent (which
//! writes files) and the workforce agent executor (which runs them).
//!
//! Used by:
//! - DAG dispatch (slice 4) after the system node agent completes
//! - Future workflow-level agents that produce file-based configs

use std::path::Path;

use anyhow::anyhow;
use tracing::info;

use crate::db::WorkflowStepRow;
use crate::server::hub::error::HubError;
use crate::server::services::system_node::file_reader;
use crate::server::ws::events::WorkflowEventKind;

use super::dag_state::DagExecutionState;
use super::pipeline::{build_upstream_step_output, AgentExecutionInput};
use super::{broadcast_workflow_event, step_display_name, DagContext};

#[cfg(test)]
#[path = "file_executor_tests.rs"]
mod tests;

/// Execute agents configured by files on disk.
///
/// Reads `topology.json` + `agents/*.json` from `base_dir`, converts them to
/// `DesignedAgentPrompt`s, resolves upstream step output, then delegates to
/// the shared agent execution core.
///
/// # Arguments
///
/// * `dag` — DAG execution context (engine, state, steps, edges)
/// * `step` — the workflow step whose agents are being executed
/// * `dag_state` — mutable DAG state for recording outputs and tokens
/// * `base_dir` — the directory where the system node agent wrote its files
pub(crate) async fn execute_from_files(
    dag: &DagContext<'_>,
    step: &WorkflowStepRow,
    dag_state: &mut DagExecutionState,
    base_dir: &Path,
) -> Result<(), HubError> {
    let step_start = std::time::Instant::now();

    // 1. Broadcast step started
    broadcast_workflow_event(
        dag.state,
        dag.ctx,
        step.workflow_id,
        WorkflowEventKind::StepStarted {
            step_id: step.id,
            step_name: step_display_name(step),
            agent_id: None,
            execution_id: None,
        },
    );

    // 2. Read agent configs from filesystem
    let designed_prompts = file_reader::read_agent_configs(base_dir)
        .map_err(|e| HubError::Internal(anyhow!("Failed to read agent configs: {e}")))?;

    if designed_prompts.is_empty() {
        return Err(HubError::Internal(anyhow!(
            "No agent configs found in {}",
            base_dir.display()
        )));
    }

    // 3. Read config.json description for traceability recording
    let original_prompt = file_reader::read_config(base_dir)
        .map(|(_name, description)| description)
        .unwrap_or_else(|_| step.prompt_template.clone());

    info!(
        step_id = %step.id,
        agents = designed_prompts.len(),
        base_dir = %base_dir.display(),
        "Executing agents from file configs"
    );

    // 4. Build upstream step output (same logic as Pipeline)
    let upstream_step_output =
        build_upstream_step_output(dag, step, &dag_state.completed_envelopes);

    // 5. Delegate to the shared agent execution core
    let input = AgentExecutionInput {
        designed_prompts,
        failure_mode: "fail_fast".to_string(),
        upstream_step_output,
        original_prompt,
        designer_run_id: None,
        phase_tokens_in: 0,
        phase_tokens_out: 0,
        phase_cost: 0.0,
    };

    super::pipeline::run_agent_execution(dag, step, dag_state, input, step_start).await
}
