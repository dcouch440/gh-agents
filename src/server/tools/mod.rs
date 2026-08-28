//! Tool handlers for step-scoped and service-layer tools.
//!
//! Sub-modules contain handlers for different tool domains:
//! `documents` (CRUD), `haiku` (utility LLM), `manager` (topology),
//! `node_assistant` (step config), `system_node` (system node agent),
//! `web` (outbound HTTP: search and page fetch).

pub mod documents;
pub mod execution;
pub mod haiku;
pub mod manager;
pub mod node_assistant;
pub mod shared;
pub mod system_node;
pub mod system_store;
pub mod web;

// Re-exports for chat completion and message building.
pub use haiku::{haiku_extract_context, haiku_summarize, haiku_summarize_title};
