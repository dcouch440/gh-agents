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
pub use graph::{find_entry_steps, get_child_steps, get_parent_steps, topological_sort};
pub use ports::{
    collect_upstream_context_data, resolve_dot_path, resolve_port_inputs, PortResolutionError,
};
pub use prompts::{build_routing_instruction_block, compose_prompt};
pub use types::{
    ContainerExecutionConfig, DagPaused, StepOutput, StepReadiness, SubWorkflowParentContext,
    WorkflowExecutionContext, WorkflowExecutionResult,
};
pub use variables::{extract_for_each_label, resolve_for_each_array, resolve_variables};

#[cfg(test)]
mod tests;
