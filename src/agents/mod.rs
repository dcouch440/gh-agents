//! Agent runtime utilities and management
//!
//! LEGACY modules removed: agent, dispatcher, executor, pool, schedule, cluster, pipeline, channels, roles

pub mod execution_tools;
pub mod gatekeeper;
pub mod router_agent;
pub mod tool_router;

// Re-export commonly used types
pub use router_agent::{ClusterEntry, ToolClusterIndex};
