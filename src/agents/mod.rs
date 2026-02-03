//! Agent runtime utilities and management
//!
//! LEGACY modules removed: agent, dispatcher, executor, pool, schedule, cluster, pipeline, channels
//! Note: roles.rs contains legacy RoleManager code but core types still used

pub mod execution_tools;
pub mod gatekeeper;
pub mod protocol;
pub mod roles;
pub mod router_agent;
pub mod tool_router;

// Re-export commonly used types
pub use protocol::AgentId;
pub use roles::{CommunicationStyle, OutputFormat, RoleId}; // Note: RoleManager is legacy/unused
pub use router_agent::{ClusterEntry, ToolClusterIndex};
