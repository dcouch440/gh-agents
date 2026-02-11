//! For-each step execution and chained for-each pipeline detection/execution.
//!
//! - `detection` — pure graph analysis to identify chains of consecutive for-each steps
//! - `iteration` — single for-each step fan-out execution
//! - `pipeline` — Phase 6B chained pipeline execution (per-item concurrency)

mod detection;
mod iteration;
mod pipeline;
mod tests;

pub(crate) use detection::{detect_for_each_chains, ForEachChain};
pub(in crate::server::hub::dag) use iteration::execute_for_each_step;
pub(in crate::server::hub::dag) use pipeline::execute_for_each_chain;
