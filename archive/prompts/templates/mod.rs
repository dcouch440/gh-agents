//! Prompt templates for different agent types.
//!
//! This module provides pre-built prompt templates for:
//! - Orchestrator: Planning, reviewing, routing, conversation, and recovery
//! - Refactor: Mid-stream plan modifications through conversation
//! - Worker: Implementation, context-gathering, progress, self-check, stuck-detection
//! - Utility: Formatting, linting, boilerplate, docs, renaming

mod orchestrator;
mod refactor;
mod utility;
mod worker;

pub use orchestrator::*;
pub use refactor::*;
pub use utility::*;
pub use worker::*;
