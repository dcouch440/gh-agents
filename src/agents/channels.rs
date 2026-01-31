//! Channel types for inter-agent communication

use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::agent::AgentId;
use super::roles::{CommunicationStyle, OutputFormat, RoleId};
use crate::execution::ExecutionContext;
use crate::types::{AgentTier, TaskStatus};

/// Commands sent to agents
#[derive(Debug, Clone)]
pub enum AgentCommand {
    /// Assign a task to this agent
    AssignTask(TaskAssignment),
    /// Provide context the agent requested
    ProvideContext(ContextResponse),
    /// Grant approval for a pending action
    GrantApproval,
    /// Deny approval for a pending action
    DenyApproval { reason: String },
    /// Request the agent to shut down gracefully
    Shutdown,
}

/// Responses from agents
#[derive(Debug, Clone)]
pub enum AgentResponse {
    /// Agent has started working on a task
    TaskStarted { agent_id: AgentId, task_id: Uuid },
    /// Agent completed a task successfully
    TaskCompleted {
        agent_id: AgentId,
        result: TaskResult,
    },
    /// Agent failed to complete a task
    TaskFailed {
        agent_id: AgentId,
        result: TaskResult,
    },
    /// Agent needs additional context
    ContextRequest {
        agent_id: AgentId,
        request: ContextRequest,
    },
    /// Agent needs approval before proceeding
    ApprovalRequest {
        agent_id: AgentId,
        request: ApprovalRequest,
    },
    /// Progress update for display in feed
    ProgressUpdate {
        agent_id: AgentId,
        update: ProgressUpdate,
    },
    /// Agent has shut down
    ShutdownComplete { agent_id: AgentId },
}

/// Task assignment details
#[derive(Debug, Clone)]
pub struct TaskAssignment {
    pub task_id: Uuid,
    pub title: String,
    pub description: String,
    pub context: TaskContext,
    pub constraints: TaskConstraints,
    pub timeout: Duration,
    /// The role this agent should assume for this task
    pub role_id: RoleId,
}

/// Context provided with a task
#[derive(Debug, Clone)]
pub struct TaskContext {
    /// Pre-loaded file contents (from role's required_reading)
    pub required_reading: Vec<FileContent>,
    /// Additional task-specific files
    pub files: Vec<FileContent>,
    /// Relevant prior work
    pub history: Vec<HistoryEntry>,
    /// Project conventions (CLAUDE.md content)
    pub conventions: String,
    /// Role-specific prompt additions
    pub role_context: RoleContext,
    /// Chat conversation messages (for chat-mode tasks).
    /// When non-empty, the executor uses these directly as LLM messages
    /// instead of wrapping in a "complete this task" prompt.
    pub chat_messages: Vec<crate::llm::Message>,
    /// Execution context for file/git/test/sandbox operations.
    pub execution_context: Option<ExecutionContext>,
}

/// Role-specific context for prompt building
#[derive(Debug, Clone)]
pub struct RoleContext {
    /// The role's system prompt
    pub system_prompt: String,
    /// The role's communication style
    pub style: CommunicationStyle,
    /// Expected output format
    pub output_format: OutputFormat,
}

/// File content for context
#[derive(Debug, Clone)]
pub struct FileContent {
    pub path: String,
    pub content: String,
}

/// History entry for context
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub task_id: Uuid,
    pub summary: String,
}

/// Constraints on task execution
#[derive(Debug, Clone)]
pub struct TaskConstraints {
    pub max_files_modified: Option<u32>,
    pub allowed_paths: Vec<String>,
    pub require_tests: bool,
    pub require_review: bool,
    /// If set, only these execution tools are available to the agent.
    pub allowed_tools: Option<Vec<String>>,
}

impl Default for TaskConstraints {
    fn default() -> Self {
        Self {
            max_files_modified: None,
            allowed_paths: Vec::new(),
            require_tests: false,
            require_review: true,
            allowed_tools: None,
        }
    }
}

/// Result of task execution
#[derive(Debug, Clone)]
pub struct TaskResult {
    pub task_id: Uuid,
    pub status: TaskStatus,
    pub output: String,
    pub files_modified: Vec<String>,
    pub errors: Vec<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub duration_ms: u64,
}

/// Request for additional context
#[derive(Debug, Clone)]
pub struct ContextRequest {
    pub task_id: Uuid,
    pub files_needed: Vec<String>,
    pub questions: Vec<String>,
}

/// Response to context request
#[derive(Debug, Clone)]
pub struct ContextResponse {
    pub task_id: Uuid,
    pub files: Vec<FileContent>,
    pub answers: Vec<String>,
}

/// Request for user approval
#[derive(Debug, Clone)]
pub struct ApprovalRequest {
    pub task_id: Uuid,
    pub action: String,
    pub details: String,
}

/// Progress update for feed
#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    pub task_id: Uuid,
    pub message: String,
    pub progress_percent: Option<u8>,
}

/// Handle for sending commands to an agent
#[derive(Clone)]
pub struct AgentHandle {
    pub agent_id: AgentId,
    pub tier: AgentTier,
    command_tx: mpsc::Sender<AgentCommand>,
}

impl AgentHandle {
    pub fn new(agent_id: AgentId, tier: AgentTier, command_tx: mpsc::Sender<AgentCommand>) -> Self {
        Self {
            agent_id,
            tier,
            command_tx,
        }
    }

    /// Send a command to the agent
    pub async fn send(
        &self,
        command: AgentCommand,
    ) -> Result<(), mpsc::error::SendError<AgentCommand>> {
        self.command_tx.send(command).await
    }

    /// Try to send without blocking
    pub fn try_send(
        &self,
        command: AgentCommand,
    ) -> Result<(), mpsc::error::TrySendError<AgentCommand>> {
        self.command_tx.try_send(command)
    }
}

/// Create an agent channel pair
pub fn create_agent_channel(
    buffer_size: usize,
) -> (mpsc::Sender<AgentCommand>, mpsc::Receiver<AgentCommand>) {
    mpsc::channel(buffer_size)
}
