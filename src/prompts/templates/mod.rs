//! Prompt templates for different agent types.
//!
//! This module provides pre-built prompt templates for:
//! - Orchestrator: Planning, reviewing, routing, conversation, and recovery
//! - Worker: Implementation, testing, bug fixing (see ticket 4.3)
//! - Utility: Formatting, linting, simple tasks (see ticket 4.4)

mod orchestrator;

pub use orchestrator::*;
