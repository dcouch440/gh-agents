//! Workflow DAG utility functions.
//!
//! Pure functions for DAG operations: topological sort, variable resolution,
//! prompt composition, port-based data flow, and label routing.
//!
//! These are re-exported by `hub::dag` and used throughout the execution system.

pub mod types;

mod conditions;
mod graph;
mod ports;
mod prompts;
mod variables;

// Re-export all public items to preserve the existing API surface.
pub use conditions::{check_step_readiness, evaluate_edge_condition};
pub use graph::{
    compute_dead_path_steps, find_entry_steps, get_child_steps, get_parent_steps, topological_sort,
    topological_sort_levels,
};
pub use ports::{
    collect_upstream_context_data, resolve_dot_path, resolve_port_inputs, PortResolutionError,
};
pub(crate) use prompts::{build_routing_instruction_block, compose_prompt, PromptRepos};
pub(crate) use types::DagPaused;
pub use types::{
    ContainerExecutionConfig, StepOutput, StepReadiness, SubWorkflowParentContext,
    WorkflowExecutionContext, WorkflowExecutionResult,
};
pub use variables::resolve_variables;

use uuid::Uuid;

use crate::server::hub::dag::dag_state::DagContext;
use crate::server::hub::dag::dag_state::DagExecutionState;
use crate::types::StepExecutionEnvelope;

/// Record step output in DAG state and snapshot the envelope for run history.
///
/// Combines `dag_state.record_step_output()` + JSON serialization +
/// `versioning::snapshot_content()` into a single call.
pub(crate) async fn record_and_snapshot_output(
    dag: &DagContext<'_>,
    dag_state: &mut DagExecutionState,
    step_id: Uuid,
    output: StepOutput,
    envelope: StepExecutionEnvelope,
) {
    let envelope_json = serde_json::to_string(&envelope).unwrap_or_default();
    dag_state.record_step_output(step_id, output, envelope);
    let _ = super::versioning::snapshot_content(
        &*dag.state.repos().content_versions,
        dag.ctx.run_id,
        step_id,
        step_id,
        super::versioning::content_types::ENVELOPE,
        "output",
        &envelope_json,
    )
    .await;
}

#[cfg(test)]
mod tests;
