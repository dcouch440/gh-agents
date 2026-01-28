//! Agent runtime and management

mod agent;
mod channels;
mod dispatcher;
mod pool;
mod roles;

pub use agent::{Agent, AgentError, AgentId};
pub use channels::{
    create_agent_channel, AgentCommand, AgentHandle, AgentResponse, ApprovalRequest,
    ContextRequest, ContextResponse, FileContent, HistoryEntry, ProgressUpdate, RoleContext,
    TaskAssignment, TaskConstraints, TaskContext, TaskResult,
};
pub use dispatcher::{DispatchError, Dispatcher};
pub use pool::{AgentPool, PoolError, PoolStats, PoolTierStats};
pub use roles::{CommunicationStyle, OutputFormat, RoleId};
