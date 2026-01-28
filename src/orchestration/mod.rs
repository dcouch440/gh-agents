//! Task orchestration and scheduling
//!
//! This module provides:
//! - `Planner` - Decomposes tickets into vertical slices
//! - `Scheduler` - Controls work assignment based on production mode

mod planner;
mod scheduler;

pub use planner::{DecompositionError, DecompositionResult, Planner, PlannerConfig, PlannerOutput};
pub use scheduler::Scheduler;
