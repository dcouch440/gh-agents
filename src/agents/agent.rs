//! Agent runtime and lifecycle

use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{mpsc, Mutex};
use tracing::{info, warn};
use uuid::Uuid;

use super::channels::{AgentCommand, AgentResponse};
use crate::llm::LLMProvider;
use crate::types::{AgentPersona, AgentStatus, AgentTier, ModelConfig};

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("invalid state transition: cannot {action} while in {current_status:?} state")]
    InvalidStateTransition { action: String, current_status: AgentStatus },
    #[error("agent has no current task")]
    NoCurrentTask,
    #[error("response channel closed")]
    ResponseChannelClosed,
    #[error("LLM error: {0}")]
    LLMError(String),
    #[error("task {task_id} timed out after {timeout:?}")]
    TaskTimeout { task_id: uuid::Uuid, timeout: std::time::Duration },
}

/// Unique identifier for an agent
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct AgentId(pub Uuid);

impl AgentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

/// An AI agent that can execute tasks
pub struct Agent {
    /// Unique identifier
    pub id: AgentId,
    /// Agent tier (Orchestrator, Worker, Utility)
    pub tier: AgentTier,
    /// Persona defining behavior and communication style
    pub persona: AgentPersona,
    /// LLM configuration for this agent
    pub model_config: ModelConfig,
    /// Current task being worked on (if any)
    pub current_task: Option<Uuid>,
    /// Current status
    pub status: AgentStatus,
    /// Shared status observable by the pool (updated alongside self.status)
    shared_status: Arc<Mutex<AgentStatus>>,
    /// Reference to LLM provider (shared across agents)
    llm_provider: Arc<dyn LLMProvider + Send + Sync>,
    /// Channel for receiving commands
    command_rx: mpsc::Receiver<AgentCommand>,
    /// Channel for sending responses
    response_tx: mpsc::Sender<AgentResponse>,
    /// Whether the agent has been shut down
    is_shutdown: bool,
}

impl Agent {
    /// Create a new agent with the specified configuration
    pub fn new(
        tier: AgentTier,
        persona: AgentPersona,
        model_config: ModelConfig,
        llm_provider: Arc<dyn LLMProvider + Send + Sync>,
        command_rx: mpsc::Receiver<AgentCommand>,
        response_tx: mpsc::Sender<AgentResponse>,
    ) -> Self {
        Self {
            id: AgentId::new(),
            tier,
            persona,
            model_config,
            current_task: None,
            status: AgentStatus::Idle,
            shared_status: Arc::new(Mutex::new(AgentStatus::Idle)),
            llm_provider,
            command_rx,
            response_tx,
            is_shutdown: false,
        }
    }

    /// Get the shared status handle (for external observation by the pool)
    pub fn shared_status(&self) -> Arc<Mutex<AgentStatus>> {
        Arc::clone(&self.shared_status)
    }

    /// Get the agent's ID
    pub fn id(&self) -> &AgentId {
        &self.id
    }

    /// Get the agent's tier
    pub fn tier(&self) -> AgentTier {
        self.tier
    }

    /// Get the agent's current status
    pub fn status(&self) -> AgentStatus {
        self.status
    }

    /// Check if the agent is available for work
    pub fn is_available(&self) -> bool {
        matches!(self.status, AgentStatus::Idle)
    }

    /// Update both local and shared status
    fn set_status(&mut self, status: AgentStatus) {
        self.status = status;
        // Use try_lock to avoid blocking; shared_status is only read externally
        if let Ok(mut s) = self.shared_status.try_lock() {
            *s = status;
        }
    }

    /// Start working on a task
    /// Transitions: Idle → Working
    pub fn start_task(&mut self, task_id: Uuid) -> Result<(), AgentError> {
        if self.status != AgentStatus::Idle {
            return Err(AgentError::InvalidStateTransition {
                action: "start task".to_string(),
                current_status: self.status,
            });
        }
        self.current_task = Some(task_id);
        self.set_status(AgentStatus::Working);
        Ok(())
    }

    /// Indicate waiting for additional context
    /// Transitions: Working → WaitingForContext
    pub fn wait_for_context(&mut self) -> Result<(), AgentError> {
        if self.status != AgentStatus::Working {
            return Err(AgentError::InvalidStateTransition {
                action: "wait for context".to_string(),
                current_status: self.status,
            });
        }
        self.set_status(AgentStatus::WaitingForContext);
        Ok(())
    }

    /// Indicate waiting for user approval
    /// Transitions: Working → WaitingForApproval
    pub fn wait_for_approval(&mut self) -> Result<(), AgentError> {
        if self.status != AgentStatus::Working {
            return Err(AgentError::InvalidStateTransition {
                action: "wait for approval".to_string(),
                current_status: self.status,
            });
        }
        self.set_status(AgentStatus::WaitingForApproval);
        Ok(())
    }

    /// Resume working after context received or approval granted
    /// Transitions: WaitingForContext|WaitingForApproval → Working
    pub fn resume(&mut self) -> Result<(), AgentError> {
        match self.status {
            AgentStatus::WaitingForContext | AgentStatus::WaitingForApproval => {
                self.set_status(AgentStatus::Working);
                Ok(())
            }
            _ => Err(AgentError::InvalidStateTransition {
                action: "resume".to_string(),
                current_status: self.status,
            }),
        }
    }

    /// Mark current task as complete
    /// Transitions: Working → Idle
    pub fn complete_task(&mut self) -> Result<Uuid, AgentError> {
        if self.status != AgentStatus::Working {
            return Err(AgentError::InvalidStateTransition {
                action: "complete task".to_string(),
                current_status: self.status,
            });
        }
        let task_id = self.current_task.take().ok_or(AgentError::NoCurrentTask)?;
        self.set_status(AgentStatus::Idle);
        Ok(task_id)
    }

    /// Mark current task as failed
    /// Transitions: Working|WaitingFor* → Idle
    pub fn fail_task(&mut self) -> Result<Uuid, AgentError> {
        match self.status {
            AgentStatus::Idle => Err(AgentError::InvalidStateTransition {
                action: "fail task".to_string(),
                current_status: self.status,
            }),
            _ => {
                let task_id = self.current_task.take().ok_or(AgentError::NoCurrentTask)?;
                self.set_status(AgentStatus::Idle);
                Ok(task_id)
            }
        }
    }

    /// Cleanly shut down the agent
    ///
    /// If the agent is working on a task, the task will be failed first.
    /// After shutdown, the agent should not be used.
    pub fn shutdown(&mut self) -> Result<(), AgentError> {
        info!(agent_id = ?self.id, tier = ?self.tier, "Agent shutting down");

        // If working, fail the current task
        if self.current_task.is_some() {
            let task_id = self.fail_task()?;
            warn!(
                agent_id = ?self.id,
                task_id = ?task_id,
                "Agent shutdown while task in progress - task failed"
            );
        }

        self.is_shutdown = true;
        info!(agent_id = ?self.id, "Agent shutdown complete");
        Ok(())
    }

    /// Check if the agent has been shut down
    pub fn is_shutdown(&self) -> bool {
        self.is_shutdown
    }

    /// Receive the next command (blocking)
    pub async fn recv_command(&mut self) -> Option<AgentCommand> {
        self.command_rx.recv().await
    }

    /// Try to receive a command without blocking
    pub fn try_recv_command(&mut self) -> Option<AgentCommand> {
        self.command_rx.try_recv().ok()
    }

    /// Send a response to the dispatcher
    pub async fn send_response(&self, response: AgentResponse) -> Result<(), AgentError> {
        self.response_tx.send(response).await.map_err(|_| AgentError::ResponseChannelClosed)
    }

    /// Get a clone of the response sender (for spawned tasks)
    pub fn response_sender(&self) -> mpsc::Sender<AgentResponse> {
        self.response_tx.clone()
    }

    /// Get a reference to the LLM provider
    pub fn llm_provider(&self) -> &(dyn LLMProvider + Send + Sync) {
        self.llm_provider.as_ref()
    }
}

impl Drop for Agent {
    fn drop(&mut self) {
        if !self.is_shutdown {
            warn!(
                agent_id = ?self.id,
                tier = ?self.tier,
                "Agent dropped without explicit shutdown"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{LLMRequest, LLMResponse, StopReason, StreamChunk, TokenUsage};
    use async_trait::async_trait;
    use futures::Stream;
    use std::pin::Pin;

    /// Mock LLM provider for testing
    struct MockLLMProvider;

    #[async_trait]
    impl LLMProvider for MockLLMProvider {
        async fn send_message(&self, _request: LLMRequest) -> Result<LLMResponse, crate::llm::LLMError> {
            Ok(LLMResponse {
                content: "test response".to_string(),
                content_blocks: vec![],
                model: "test-model".to_string(),
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage { input_tokens: 10, output_tokens: 20 },
            })
        }

        async fn send_message_stream(&self, _request: LLMRequest) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, crate::llm::LLMError>> + Send>>, crate::llm::LLMError> {
            unimplemented!("not needed for these tests")
        }

        fn provider_name(&self) -> &'static str {
            "mock"
        }

        fn model_id(&self) -> &str {
            "test-model"
        }
    }

    fn create_test_agent() -> Agent {
        let provider = Arc::new(MockLLMProvider);
        let (_command_tx, command_rx) = mpsc::channel(32);
        let (response_tx, _response_rx) = mpsc::channel(32);
        Agent::new(AgentTier::Worker, AgentPersona::default(), ModelConfig::default(), provider, command_rx, response_tx)
    }

    #[test]
    fn state_transitions_valid_flow() {
        let mut agent = create_test_agent();
        assert_eq!(agent.status(), AgentStatus::Idle);
        assert!(agent.is_available());

        // Idle → Working
        let task_id = Uuid::new_v4();
        agent.start_task(task_id).unwrap();
        assert_eq!(agent.status(), AgentStatus::Working);
        assert_eq!(agent.current_task, Some(task_id));
        assert!(!agent.is_available());

        // Working → WaitingForContext
        agent.wait_for_context().unwrap();
        assert_eq!(agent.status(), AgentStatus::WaitingForContext);

        // WaitingForContext → Working
        agent.resume().unwrap();
        assert_eq!(agent.status(), AgentStatus::Working);

        // Working → WaitingForApproval
        agent.wait_for_approval().unwrap();
        assert_eq!(agent.status(), AgentStatus::WaitingForApproval);

        // WaitingForApproval → Working
        agent.resume().unwrap();
        assert_eq!(agent.status(), AgentStatus::Working);

        // Working → Idle
        let completed_task_id = agent.complete_task().unwrap();
        assert_eq!(completed_task_id, task_id);
        assert_eq!(agent.status(), AgentStatus::Idle);
        assert!(agent.current_task.is_none());
    }

    #[test]
    fn state_transitions_invalid_rejected() {
        let mut agent = create_test_agent();
        let task_id = Uuid::new_v4();

        // Can't start task when already working
        agent.start_task(task_id).unwrap();
        let result = agent.start_task(Uuid::new_v4());
        assert!(result.is_err());
        match result {
            Err(AgentError::InvalidStateTransition { action, .. }) => {
                assert!(action.contains("start task"));
            }
            _ => panic!("expected InvalidStateTransition error"),
        }

        // Can't wait_for_context from non-Working state
        agent.wait_for_context().unwrap(); // Now WaitingForContext
        let result = agent.wait_for_context();
        assert!(result.is_err());

        // Can't wait_for_approval from non-Working state
        let result = agent.wait_for_approval();
        assert!(result.is_err());

        // Can't complete_task from non-Working state
        let result = agent.complete_task();
        assert!(result.is_err());
    }

    #[test]
    fn fail_task_from_working() {
        let mut agent = create_test_agent();
        let task_id = Uuid::new_v4();

        agent.start_task(task_id).unwrap();
        let failed_task_id = agent.fail_task().unwrap();
        assert_eq!(failed_task_id, task_id);
        assert_eq!(agent.status(), AgentStatus::Idle);
        assert!(agent.current_task.is_none());
    }

    #[test]
    fn fail_task_from_waiting_states() {
        // From WaitingForContext
        let mut agent = create_test_agent();
        let task_id = Uuid::new_v4();
        agent.start_task(task_id).unwrap();
        agent.wait_for_context().unwrap();
        let failed_task_id = agent.fail_task().unwrap();
        assert_eq!(failed_task_id, task_id);
        assert_eq!(agent.status(), AgentStatus::Idle);

        // From WaitingForApproval
        let mut agent = create_test_agent();
        let task_id = Uuid::new_v4();
        agent.start_task(task_id).unwrap();
        agent.wait_for_approval().unwrap();
        let failed_task_id = agent.fail_task().unwrap();
        assert_eq!(failed_task_id, task_id);
        assert_eq!(agent.status(), AgentStatus::Idle);
    }

    #[test]
    fn fail_task_from_idle_rejected() {
        let mut agent = create_test_agent();
        let result = agent.fail_task();
        assert!(result.is_err());
        match result {
            Err(AgentError::InvalidStateTransition { action, .. }) => {
                assert!(action.contains("fail task"));
            }
            _ => panic!("expected InvalidStateTransition error"),
        }
    }

    #[test]
    fn resume_only_from_waiting_states() {
        let mut agent = create_test_agent();

        // Can't resume from Idle
        let result = agent.resume();
        assert!(result.is_err());

        // Can't resume from Working
        agent.start_task(Uuid::new_v4()).unwrap();
        let result = agent.resume();
        assert!(result.is_err());
    }

    #[test]
    fn shutdown_idle_agent() {
        let mut agent = create_test_agent();
        assert_eq!(agent.status(), AgentStatus::Idle);
        assert!(!agent.is_shutdown());

        // Shutdown should succeed for idle agent
        agent.shutdown().unwrap();
        assert!(agent.is_shutdown());
        assert_eq!(agent.status(), AgentStatus::Idle);
        assert!(agent.current_task.is_none());
    }

    #[test]
    fn shutdown_working_agent_fails_task() {
        let mut agent = create_test_agent();
        let task_id = Uuid::new_v4();

        // Start working on a task
        agent.start_task(task_id).unwrap();
        assert_eq!(agent.status(), AgentStatus::Working);
        assert_eq!(agent.current_task, Some(task_id));

        // Shutdown should fail the task first
        agent.shutdown().unwrap();
        assert!(agent.is_shutdown());
        assert_eq!(agent.status(), AgentStatus::Idle);
        assert!(agent.current_task.is_none());
    }

    #[test]
    fn shutdown_from_waiting_states() {
        // From WaitingForContext
        let mut agent = create_test_agent();
        let task_id = Uuid::new_v4();
        agent.start_task(task_id).unwrap();
        agent.wait_for_context().unwrap();
        agent.shutdown().unwrap();
        assert!(agent.is_shutdown());
        assert_eq!(agent.status(), AgentStatus::Idle);

        // From WaitingForApproval
        let mut agent = create_test_agent();
        let task_id = Uuid::new_v4();
        agent.start_task(task_id).unwrap();
        agent.wait_for_approval().unwrap();
        agent.shutdown().unwrap();
        assert!(agent.is_shutdown());
        assert_eq!(agent.status(), AgentStatus::Idle);
    }
}
