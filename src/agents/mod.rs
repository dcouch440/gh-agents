//! Agent runtime and management

mod agent;
mod channels;
pub mod cluster;
mod dispatcher;
pub mod pipeline;
pub mod schedule;
mod escalation;
pub mod execution_tools;
mod executor;
pub mod planner_bot;
mod pool;
pub mod protocol;
mod roles;

pub use agent::{Agent, AgentError, AgentId};
pub use cluster::{Cluster, ClusterContext, ClusterError, ClusterId, ClusterManager};
pub use pipeline::{Pipeline, PipelineError, PipelineId, PipelineManager, PipelineRun, PipelineRunStatus, PipelineStage};
pub use schedule::{Schedule, ScheduleError, ScheduleId, ScheduleManager, Trigger, TriggerEvent, TriggerId};
pub use channels::{
    create_agent_channel, AgentCommand, AgentHandle, AgentResponse, ApprovalRequest,
    ContextRequest, ContextResponse, FileContent, HistoryEntry, ProgressUpdate, RoleContext,
    TaskAssignment, TaskConstraints, TaskContext, TaskResult,
};
pub use dispatcher::{DispatchError, Dispatcher};
pub use escalation::{
    EscalationDecision, EscalationError, EscalationManager, EscalationPolicy, HumanAction,
    HumanReviewRequest, HumanReviewSummary, TaskEscalationState, TierAttempt,
};
pub use pool::{AgentPool, PoolError, PoolStats, PoolTierStats};
pub use roles::{
    CommunicationStyle, LoadedFile, OutputFormat, RequiredReadingLoader, Role, RoleCategory,
    RoleId, RoleLibrary, RoleManager, RoleTemplate, TemplateVariable, VariableType,
};
