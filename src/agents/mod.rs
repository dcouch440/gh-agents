//! Agent runtime utilities and management
//!
//! LEGACY modules removed: agent, dispatcher, executor, pool, schedule
//! Core agent pool system has been replaced by the hub/ExecutionEngine.

pub mod channels;
pub mod cluster;
pub mod execution_tools;
pub mod gatekeeper;
pub mod pipeline;
pub mod protocol;
pub mod roles;
pub mod router_agent;
pub mod tool_router;

// Re-export commonly used types
pub use channels::{
    AgentCommand, AgentHandle, AgentResponse, ApprovalRequest, ContextRequest, ContextResponse, DistillerMode, FileContent, HistoryEntry, ProgressUpdate, RoleContext,
    TaskAssignment, TaskConstraints, TaskContext, TaskResult,
};
pub use cluster::{Cluster, ClusterContext, ClusterError, ClusterId, ClusterManager};
pub use pipeline::{Pipeline, PipelineError, PipelineId, PipelineManager, PipelineRun, PipelineRunStatus, PipelineStage};
pub use protocol::AgentId;
pub use roles::{CommunicationStyle, LoadedFile, OutputFormat, RequiredReadingLoader, Role, RoleCategory, RoleId, RoleLibrary, RoleManager, RoleTemplate, TemplateVariable, VariableType};
pub use router_agent::{ClusterEntry, ToolClusterIndex};
// pub use tool_router::ClusterRoutingContext; // LEGACY: Removed (used pool/dispatcher)
