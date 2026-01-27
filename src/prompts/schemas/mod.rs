//! Output schemas for structured LLM responses.
//!
//! This module provides JSON schemas for all LLM output types:
//! - `DecompositionOutput` - Orchestrator ticket breakdown
//! - `TaskResultOutput` - Worker task completion
//! - `ReviewOutput` - Orchestrator code review
//! - `ErrorOutput` - Error/failure reporting

mod decomposition;
mod error;
mod review;
mod task_result;
mod validation;

pub use decomposition::*;
pub use error::*;
pub use review::*;
pub use task_result::*;
pub use validation::*;
