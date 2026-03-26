//! DAG orchestration — topological sort, variable resolution,
//! port-based data flow, and workflow execution using the unified ExecutionEngine.
//!
//! Pure utility functions live in the `utils` submodule.
//! Event broadcasting lives in `broadcast`.
//! The core dispatch loop lives in `orchestration`.
//! Routing context assembly lives in `routing`.

// ── Submodules ──────────────────────────────────────────────────────────────

pub(crate) mod broadcast;
pub(crate) mod container;
pub(crate) mod dag_state;
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
pub use orchestration::execute_workflow_via_engine;

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

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
