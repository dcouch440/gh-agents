//! Agent runtime and management

mod agent;
mod channels;
pub mod cluster;
mod dispatcher;
pub mod execution_tools;
mod executor;
pub mod pipeline;
mod pool;
pub mod protocol;
mod roles;
pub mod router_agent;
pub mod schedule;
pub mod tool_router;

pub use agent::{Agent, AgentError, AgentId};
pub use channels::{
    create_agent_channel, AgentCommand, AgentHandle, AgentResponse, ApprovalRequest, ContextRequest, ContextResponse, DistillerMode, FileContent, HistoryEntry, ProgressUpdate,
    RoleContext, TaskAssignment, TaskConstraints, TaskContext, TaskResult,
};
pub use cluster::{Cluster, ClusterContext, ClusterError, ClusterId, ClusterManager};
pub use dispatcher::{DispatchError, Dispatcher};
pub use pipeline::{Pipeline, PipelineError, PipelineId, PipelineManager, PipelineRun, PipelineRunStatus, PipelineStage};
pub use pool::{AgentPool, PoolError, PoolStats, PoolTierStats};
pub use roles::{
    CommunicationStyle, LoadedFile, OutputFormat, RequiredReadingLoader, Role, RoleCategory, RoleId, RoleLibrary, RoleManager, RoleTemplate, TemplateVariable, VariableType,
};
pub use router_agent::{ClusterEntry, ToolClusterIndex};
pub use schedule::{Schedule, ScheduleError, ScheduleId, ScheduleManager, Trigger, TriggerEvent, TriggerId};
pub use tool_router::ClusterRoutingContext;
