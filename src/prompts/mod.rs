//! Prompt engineering infrastructure for nexor.
//!
//! This module provides:
//! - `PromptTemplate` - The canonical structure for all prompts
//! - `PromptBuilder` - Fluent API for assembling prompts
//! - `ContextInjector` - Priority-based context injection
//! - `PromptVersion` - Version tracking for debugging/replay
//! - `templates` - Agent-specific prompt templates
//! - `schemas` - JSON schemas for structured LLM outputs
//! - `tools` - Tool definitions and selection
//! - `examples` - Few-shot examples library
//! - `recovery` - Self-correction and recovery prompts

mod builder;
mod context;
mod version;

pub mod examples;
pub mod recovery;
pub mod schemas;
pub mod templates;
pub mod tools;

pub use builder::*;
pub use context::*;
pub use version::*;
