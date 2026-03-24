//! DAG orchestration — topological sort, variable resolution,
//! port-based data flow, and workflow execution using the unified ExecutionEngine.
//!
//! Pure utility functions live in the `utils` submodule.
//! Event broadcasting lives in `broadcast`.
//! The core dispatch loop lives in `orchestration`.
//! Routing context assembly lives in `routing`.

use std::collections::HashMap;

use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::db::{WorkflowStepEdgeRow, WorkflowStepRow};
use crate::server::state::AppState;
use crate::server::ws::events::WorkflowEventKind;

use super::engine::ExecutionEngine;
use super::error::HubError;

// ── Submodules ──────────────────────────────────────────────────────────────

pub(crate) mod agent_designer;
pub(crate) mod broadcast;
pub(crate) mod container;
pub(crate) mod dag_state;
pub(crate) mod designer_input;
pub(crate) mod file_executor;
pub(crate) mod merge;
mod orchestration;
pub(crate) mod pipeline;
pub(crate) mod resume;
mod routing;
pub(crate) mod single;
pub mod templates;
pub(crate) mod utils;
pub(crate) mod versioning;
pub(crate) mod workshop;

// ── Re-exports ──────────────────────────────────────────────────────────────

pub use broadcast::broadcast_workflow_event;

pub(crate) use dag_state::{
    broadcast_step_failure_if_real, build_incoming_edge_index, prefetch_port_metadata,
    resolve_output_key, resolve_step_port_inputs, step_display_name, wrap_in_agentless_envelope,
    wrap_in_envelope, DagContext, DagExecutionState, PortMetadata,
};

pub(crate) use orchestration::run_dag_loop;
pub(crate) use routing::gather_downstream_routing_context;
pub(crate) use utils::{build_routing_instruction_block, compose_prompt, DagPaused, PromptRepos};

pub use utils::{
    check_step_readiness, collect_upstream_context_data, compute_dead_path_steps,
    evaluate_edge_condition, find_entry_steps, get_child_steps, get_parent_steps, resolve_dot_path,
    resolve_port_inputs, resolve_variables, topological_sort, topological_sort_levels,
    ContainerExecutionConfig, PortResolutionError, StepOutput, StepReadiness,
    WorkflowExecutionContext, WorkflowExecutionResult,
};

pub use resume::{resume_dag_from_approval, resume_workflow_via_engine, ResumeState};

// ── Main DAG Orchestration ──────────────────────────────────────────────────

/// Execute a complete workflow DAG using the unified ExecutionEngine.
///
/// Runs topological sort, resolves variables and port-based data flow,
/// then dispatches each step by execution mode: passthrough, pipeline,
/// or agent-based. Supports pinned replay, dead-path elimination, and
/// conditional edge routing.
pub async fn execute_workflow_via_engine(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    steps: &[WorkflowStepRow],
    edges: &[WorkflowStepEdgeRow],
    cancel: Option<&CancellationToken>,
) -> Result<WorkflowExecutionResult, HubError> {
    let workflow_id = steps.first().map(|s| s.workflow_id).unwrap_or(Uuid::nil());
    let start_time = std::time::Instant::now();
    let sorted_len = topological_sort(steps, edges)
        .map_err(|_| HubError::DagCycle)?
        .len();

    let mut dag_state = DagExecutionState::new();

    // Pre-fetch port metadata: from snapshot if template-based, otherwise from live DB
    let port_meta = match &ctx.snapshot {
        Some(snap) => templates::port_metadata_from_snapshot(snap),
        None => prefetch_port_metadata(state, steps, edges).await,
    };

    // Broadcast: workflow started
    broadcast_workflow_event(
        state,
        ctx,
        workflow_id,
        WorkflowEventKind::Started {
            total_steps: sorted_len,
        },
    );

    let dag = DagContext {
        engine,
        state,
        ctx,
        steps,
        edges,
        port_meta: &port_meta,
        cancel,
    };
    run_dag_loop(&dag, &mut dag_state).await?;

    let duration_ms = start_time.elapsed().as_millis() as u64;

    let final_outputs: HashMap<String, StepOutput> = dag_state
        .completed
        .into_iter()
        .map(|(id, out)| (id.to_string(), out))
        .collect();

    Ok(WorkflowExecutionResult {
        outputs: final_outputs,
        total_input_tokens: dag_state.total_input_tokens,
        total_output_tokens: dag_state.total_output_tokens,
        total_cost_usd: dag_state.total_cost_usd,
        duration_ms,
    })
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
