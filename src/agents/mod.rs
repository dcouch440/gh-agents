//! Agent runtime and management

mod agent;
mod channels;
mod dispatcher;
mod escalation;
mod executor;
mod pool;
pub mod protocol;
mod roles;

pub use agent::{Agent, AgentError, AgentId};
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
