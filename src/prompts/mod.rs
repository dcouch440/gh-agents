//! Prompt engineering infrastructure for nexor.
//!
//! This module provides:
//! - `PromptTemplate` - The canonical structure for all prompts
//! - `PromptBuilder` - Fluent API for assembling prompts
//! - `ContextInjector` - Priority-based context injection
//! - `PromptVersion` - Version tracking for debugging/replay
//! - `templates` - Pre-built prompt templates for different agent types

mod builder;
mod context;
pub mod templates;
mod version;

pub use builder::*;
pub use context::*;
pub use version::*;
