//! Prompt templates for different agent types.
//!
//! This module contains specialized prompt templates for:
//! - Workers: Focused development tasks
//! - (Future) Orchestrators: Task decomposition and routing
//! - (Future) Utilities: Specialized helper tasks

mod worker;

pub use worker::*;
