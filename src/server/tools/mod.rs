//! Tool handlers for step-scoped and service-layer tools.
//!
//! Sub-modules contain handlers for different tool domains:
//! `documents` (CRUD), `haiku` (utility LLM), `manager` (topology),
//! `node_assistant` (step config), `workforce` (team config).

pub mod documents;
pub mod haiku;
pub mod manager;
pub mod node_assistant;
pub mod shared;
pub mod workforce;

// Re-exports for chat completion and message building.
pub use haiku::{haiku_extract_context, haiku_summarize, haiku_summarize_title};
