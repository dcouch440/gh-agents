//! Tool registry and definitions.
//!
//! Provides a static registry for mapping tool names to tool definitions.
//! This is the single source of truth for all tools in the system.

pub mod registry;

// Re-export the main registry function
pub use registry::get_tool_definition;
