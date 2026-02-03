//! Agent pool management

use std::collections::HashMap;
use std::sync::Arc;

use thiserror::Error;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::llm::LLMProvider;
use crate::types::{AgentPersona, AgentPoolConfig, AgentStatus, ModelConfig};

use super::agent::{Agent, AgentError, AgentId};
use super::channels::{create_agent_channel, AgentHandle};
use super::dispatcher::Dispatcher;

/// Error types for pool operations
#[derive(Error, Debug)]
pub enum PoolError {
    #[error("pool limit reached: max {max}")]
    PoolLimitReached { max: u8 },

    #[error("agent not found: {0:?}")]
    AgentNotFound(AgentId),

    #[error("no available agent")]
    NoAvailableAgent,

    #[error("agent error: {0}")]
    AgentError(#[from] AgentError),
}

/// Lightweight tracking entry for a running agent.
struct AgentEntry {
    /// Shared status updated by the running agent, readable by the pool
    status: Arc<Mutex<AgentStatus>>,
    /// Handle to the spawned agent task (None for test-only agents)
    _join_handle: Option<JoinHandle<Result<(), AgentError>>>,
}

/// Manages a pool of agents
pub struct AgentPool {
    /// Running agent entries (metadata only — the Agent itself runs in a tokio task)
    agents: HashMap<AgentId, AgentEntry>,
    /// Pool configuration
    config: AgentPoolConfig,
    /// Shared LLM provider for all agents
    llm_provider: Arc<dyn LLMProvider + Send + Sync>,
}

impl AgentPool {
    /// Create a new agent pool with the given configuration
    pub fn new(config: AgentPoolConfig, llm_provider: Arc<dyn LLMProvider + Send + Sync>) -> Self {
        Self {
            agents: HashMap::new(),
            config,
            llm_provider,
        }
    }

    /// Get the total number of agents
    pub fn total_count(&self) -> usize {
        self.agents.len()
    }

    /// Check if the pool can spawn more agents
    pub fn can_spawn(&self) -> bool {
        self.total_count() < self.config.max_agents as usize
    }

    /// Spawn a new agent (test-only, no dispatcher or run loop).
    #[cfg(test)]
    pub fn spawn_agent(&mut self, persona: AgentPersona, model_config: ModelConfig) -> Result<AgentId, PoolError> {
        if !self.can_spawn() {
            return Err(PoolError::PoolLimitReached { max: self.config.max_agents });
        }

        let (_command_tx, command_rx) = create_agent_channel(32);
        let (response_tx, _response_rx) = tokio::sync::mpsc::channel(32);

        let agent = Agent::new(persona, model_config, Arc::clone(&self.llm_provider), command_rx, response_tx);
        let agent_id = agent.id.clone();
        let shared_status = agent.shared_status();

        info!(agent_id = ?agent_id, "Spawned new agent (test mode)");

        self.agents.insert(
            agent_id.clone(),
            AgentEntry {
                status: shared_status,
                _join_handle: None,
            },
        );

        Ok(agent_id)
    }

    /// Spawn a new agent with channel connections to the dispatcher.
    pub fn spawn_agent_with_dispatcher(
        &mut self,
        persona: AgentPersona,
        model_config: ModelConfig,
        dispatcher: &mut Dispatcher,
    ) -> Result<AgentId, PoolError> {
        if !self.can_spawn() {
            return Err(PoolError::PoolLimitReached { max: self.config.max_agents });
        }

        let (command_tx, command_rx) = create_agent_channel(32);
        let response_tx = dispatcher.response_sender();

        let agent = Agent::new(persona, model_config, Arc::clone(&self.llm_provider), command_rx, response_tx);
        let agent_id = agent.id.clone();
        let shared_status = agent.shared_status();

        let handle = AgentHandle::new(agent_id.clone(), command_tx);
        dispatcher.register_agent(handle);

        let spawned_id = agent_id.clone();
        let join_handle = tokio::spawn(async move {
            let result = agent.run().await;
            if let Err(ref e) = result {
                tracing::error!(agent_id = ?spawned_id, error = ?e, "Agent run loop exited with error");
            } else {
                tracing::info!(agent_id = ?spawned_id, "Agent run loop completed");
            }
            result
        });

        self.agents.insert(
            agent_id.clone(),
            AgentEntry {
                status: shared_status,
                _join_handle: Some(join_handle),
            },
        );

        info!(agent_id = ?agent_id, "Spawned agent with dispatcher");

        Ok(agent_id)
    }

    /// Get the ID of any available (idle) agent
    pub fn get_available_agent_id(&self) -> Option<AgentId> {
        for (agent_id, entry) in &self.agents {
            if let Ok(status) = entry.status.try_lock() {
                if *status == AgentStatus::Idle {
                    return Some(agent_id.clone());
                }
            }
        }
        None
    }

    /// Check if a specific agent is available (idle)
    pub fn is_agent_available(&self, id: &AgentId) -> bool {
        self.agents
            .get(id)
            .and_then(|e| e.status.try_lock().ok())
            .map(|s| *s == AgentStatus::Idle)
            .unwrap_or(false)
    }

    /// Check if an agent exists in the pool
    pub fn has_agent(&self, id: &AgentId) -> bool {
        self.agents.contains_key(id)
    }

    /// Remove an agent from the pool
    pub fn remove_agent(&mut self, id: &AgentId) -> Result<(), PoolError> {
        let entry = self.agents.remove(id).ok_or_else(|| PoolError::AgentNotFound(id.clone()))?;

        if let Some(handle) = entry._join_handle {
            handle.abort();
        }

        info!(agent_id = ?id, "Agent removed from pool");
        Ok(())
    }

    /// Shut down all agents and clear the pool
    pub fn shutdown_all(&mut self) {
        info!("Shutting down all agents in pool");

        let agent_ids: Vec<AgentId> = self.agents.keys().cloned().collect();
        for id in agent_ids {
            if let Err(e) = self.remove_agent(&id) {
                warn!(agent_id = ?id, error = ?e, "Error removing agent during shutdown");
            }
        }

        info!("All agents shut down");
    }

    /// Count available (idle) agents
    fn count_available(&self) -> usize {
        self.agents
            .values()
            .filter(|e| e.status.try_lock().ok().map(|s| *s == AgentStatus::Idle).unwrap_or(false))
            .count()
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            total: self.total_count(),
            available: self.count_available(),
            max: self.config.max_agents as usize,
        }
    }
}

/// Pool statistics
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolStats {
    pub total: usize,
    pub available: usize,
    pub max: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{LLMError, LLMRequest, LLMResponse, StopReason, StreamChunk, TokenUsage};
    use async_trait::async_trait;
    use futures::Stream;
    use std::pin::Pin;

    struct MockLLMProvider;

    #[async_trait]
    impl LLMProvider for MockLLMProvider {
        async fn send_message(&self, _request: LLMRequest) -> Result<LLMResponse, LLMError> {
            Ok(LLMResponse {
                content: "test response".to_string(),
                content_blocks: vec![],
                model: "test-model".to_string(),
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage { input_tokens: 10, output_tokens: 20 },
            })
        }

        async fn send_message_stream(
            &self,
            _request: LLMRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>>, LLMError> {
            unimplemented!("not needed for these tests")
        }

        fn provider_name(&self) -> &'static str {
            "mock"
        }

        fn model_id(&self) -> &str {
            "test-model"
        }
    }

    fn create_test_pool() -> AgentPool {
        let config = AgentPoolConfig { max_agents: 5 };
        let provider = Arc::new(MockLLMProvider);
        AgentPool::new(config, provider)
    }

    #[test]
    fn new_pool_is_empty() {
        let pool = create_test_pool();
        assert_eq!(pool.total_count(), 0);
    }

    #[test]
    fn can_spawn_returns_true_for_empty_pool() {
        let pool = create_test_pool();
        assert!(pool.can_spawn());
    }

    #[test]
    fn spawn_agent_creates_agent() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        let result = pool.spawn_agent(persona, model_config);
        assert!(result.is_ok());

        let agent_id = result.unwrap();
        assert_eq!(pool.total_count(), 1);
        assert!(pool.has_agent(&agent_id));
    }

    #[test]
    fn spawn_agent_respects_limit() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        for _ in 0..5 {
            pool.spawn_agent(persona.clone(), model_config.clone()).unwrap();
        }

        assert!(!pool.can_spawn());

        let result = pool.spawn_agent(persona, model_config);
        assert!(matches!(result, Err(PoolError::PoolLimitReached { max: 5 })));
    }

    #[test]
    fn get_available_agent_returns_idle_agent() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        let agent_id = pool.spawn_agent(persona, model_config).unwrap();

        let available_id = pool.get_available_agent_id();
        assert!(available_id.is_some());
        assert_eq!(available_id.unwrap(), agent_id);
        assert!(pool.is_agent_available(&agent_id));
    }

    #[test]
    fn get_available_agent_returns_none_for_empty_pool() {
        let pool = create_test_pool();
        assert!(pool.get_available_agent_id().is_none());
    }

    #[test]
    fn remove_agent_cleans_up() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        let agent_id = pool.spawn_agent(persona, model_config).unwrap();
        assert_eq!(pool.total_count(), 1);

        pool.remove_agent(&agent_id).unwrap();

        assert_eq!(pool.total_count(), 0);
        assert!(!pool.has_agent(&agent_id));
    }

    #[test]
    fn remove_unknown_agent_fails() {
        let mut pool = create_test_pool();
        let unknown_id = AgentId::new();
        let result = pool.remove_agent(&unknown_id);
        assert!(matches!(result, Err(PoolError::AgentNotFound(_))));
    }

    #[test]
    fn shutdown_all_clears_pool() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        pool.spawn_agent(persona.clone(), model_config.clone()).unwrap();
        pool.spawn_agent(persona, model_config).unwrap();

        assert_eq!(pool.total_count(), 2);

        pool.shutdown_all();

        assert_eq!(pool.total_count(), 0);
    }

    #[test]
    fn stats_returns_correct_values() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        let stats = pool.stats();
        assert_eq!(stats.total, 0);
        assert_eq!(stats.available, 0);
        assert_eq!(stats.max, 5);

        pool.spawn_agent(persona.clone(), model_config.clone()).unwrap();
        pool.spawn_agent(persona, model_config).unwrap();

        let stats = pool.stats();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.available, 2);
    }

    #[test]
    fn pool_error_display_messages() {
        let e1 = PoolError::PoolLimitReached { max: 5 };
        assert!(e1.to_string().contains("pool limit reached"));

        let e2 = PoolError::AgentNotFound(AgentId::new());
        assert!(e2.to_string().contains("agent not found"));

        let e3 = PoolError::NoAvailableAgent;
        assert!(e3.to_string().contains("no available agent"));
    }
}
