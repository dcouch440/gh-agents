//! Prompt engineering infrastructure for nexor.
//!
//! This module provides:
//! - `PromptTemplate` - The canonical structure for all prompts
//! - `PromptBuilder` - Fluent API for assembling prompts
//! - `ContextInjector` - Priority-based context injection
//! - `PromptVersion` - Version tracking for debugging/replay
//! - `tools` - Tool definitions and selection
//! - `recovery` - Self-correction and recovery prompts

mod builder;
mod context;
mod version;

pub mod recovery;
pub mod tools;

pub use builder::*;
pub use context::*;
pub use version::*;
