//! Tool definitions, registry, and selection prompts.
//!
//! This module provides:
//! - `ToolDefinition` - Schema for defining tools agents can use
//! - `ToolRegistry` - Registry of available tools
//! - Pre-defined tools for file, git, and test operations
//! - Tool selection prompts

mod definitions;
mod selection;

pub use definitions::*;
pub use selection::*;
