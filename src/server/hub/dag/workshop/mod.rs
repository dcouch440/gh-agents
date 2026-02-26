//! Workshop — interactive node-by-node workflow execution.
//!
//! Execute individual steps on demand, reconstructing prior state
//! from versioned snapshots between runs.

pub(crate) mod context;
pub(crate) mod dispatch;
pub(crate) mod readiness;
pub(crate) mod reconstruct;
pub(crate) mod types;

pub(crate) use context::{build_execution_context, replay_pinned_step, snapshot_error_envelope};
pub(crate) use dispatch::execute_step;
pub(crate) use readiness::next_executable_steps;
pub(crate) use reconstruct::reconstruct_state;

mod tests;
