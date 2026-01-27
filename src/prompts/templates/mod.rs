//! Prompt templates for different agent tiers.
//!
//! This module contains specialized prompt templates:
//! - `utility` - Quick, well-defined tasks (formatting, linting, etc.)
//!
//! Future modules (pending implementation):
//! - `orchestrator` - Decomposition and coordination prompts
//! - `worker` - Implementation and coding prompts

mod utility;

pub use utility::*;
