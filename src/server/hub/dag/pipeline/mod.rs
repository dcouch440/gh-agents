//! Pipeline step execution within the DAG.
//!
//! Contains the agent execution machinery: level scheduling, output
//! composition, and the shared runner used by both the legacy Pipeline
//! and the file executor.

mod agent_executor;
mod output;
mod runner;
mod tests;
mod types;

// Re-exports for crate-wide access
pub(crate) use types::DesignedAgentPrompt;

// Re-exports for the file executor (file-based execution bridge)
pub(crate) use output::build_upstream_step_output;
pub(crate) use runner::{run_agent_execution, AgentExecutionInput};

// Re-exports for test access (tests.rs imports via crate path)
#[cfg(test)]
pub(crate) use agent_executor::passdown_entries;
#[cfg(test)]
pub(crate) use output::{
    build_filtered_outputs_block, build_team_blocks, build_upstream_outputs_block,
    compose_workforce_output, compute_execution_levels, filter_outputs_for_agent,
};
