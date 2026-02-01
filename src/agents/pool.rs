//! Agent pool management

use std::collections::HashMap;
use std::sync::Arc;

use thiserror::Error;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::llm::LLMProvider;
use crate::types::{AgentPersona, AgentPoolConfig, AgentStatus, AgentTier, ModelConfig, Task};

use super::agent::{Agent, AgentError, AgentId};
use super::channels::{create_agent_channel, AgentHandle};
use super::dispatcher::Dispatcher;

/// Error types for pool operations
#[derive(Error, Debug)]
pub enum PoolError {
    #[error("pool limit reached for tier {tier:?}: max {max}")]
    PoolLimitReached { tier: AgentTier, max: u8 },

    #[error("agent not found: {0:?}")]
    AgentNotFound(AgentId),

    #[error("no available agent for tier {0:?}")]
    NoAvailableAgent(AgentTier),

    #[error("agent error: {0}")]
    AgentError(#[from] AgentError),
}

/// Lightweight tracking entry for a running agent.
///
/// The actual `Agent` is consumed by `tokio::spawn(agent.run())`.
/// We keep metadata here for pool bookkeeping.
struct AgentEntry {
    tier: AgentTier,
    /// Shared status updated by the running agent, readable by the pool
    status: Arc<Mutex<AgentStatus>>,
    /// Handle to the spawned agent task (None for test-only agents)
    _join_handle: Option<JoinHandle<Result<(), AgentError>>>,
}

/// Manages pools of agents organized by tier
pub struct AgentPool {
    /// Running agent entries (metadata only — the Agent itself runs in a tokio task)
    agents: HashMap<AgentId, AgentEntry>,
    /// Agent IDs grouped by tier for fast lookup
    by_tier: HashMap<AgentTier, Vec<AgentId>>,
    /// Pool configuration (max agents per tier)
    config: AgentPoolConfig,
    /// Shared LLM provider for all agents
    llm_provider: Arc<dyn LLMProvider + Send + Sync>,
}

impl AgentPool {
    /// Create a new agent pool with the given configuration
    pub fn new(config: AgentPoolConfig, llm_provider: Arc<dyn LLMProvider + Send + Sync>) -> Self {
        let mut by_tier = HashMap::new();
        by_tier.insert(AgentTier::Orchestrator, Vec::new());
        by_tier.insert(AgentTier::Worker, Vec::new());
        by_tier.insert(AgentTier::Utility, Vec::new());

        Self {
            agents: HashMap::new(),
            by_tier,
            config,
            llm_provider,
        }
    }

    /// Get the number of agents in a tier
    pub fn count(&self, tier: AgentTier) -> usize {
        self.by_tier.get(&tier).map(|v| v.len()).unwrap_or(0)
    }

    /// Get the maximum allowed agents for a tier
    pub fn max_for_tier(&self, tier: AgentTier) -> u8 {
        match tier {
            AgentTier::Orchestrator => self.config.max_orchestrators,
            AgentTier::Worker => self.config.max_workers,
            AgentTier::Utility => self.config.max_utilities,
        }
    }

    /// Check if the pool can spawn more agents of the given tier
    pub fn can_spawn(&self, tier: AgentTier) -> bool {
        self.count(tier) < self.max_for_tier(tier) as usize
    }

    /// Get the total number of agents across all tiers
    pub fn total_count(&self) -> usize {
        self.agents.len()
    }

    /// Spawn a new agent of the specified tier (test-only, no dispatcher or run loop).
    ///
    /// The agent is NOT started as a tokio task. Use `spawn_agent_with_dispatcher`
    /// for production agents that need to process commands.
    #[cfg(test)]
    pub fn spawn_agent(&mut self, tier: AgentTier, persona: AgentPersona, model_config: ModelConfig) -> Result<AgentId, PoolError> {
        // Check pool limit
        let max = self.max_for_tier(tier);
        if self.count(tier) >= max as usize {
            return Err(PoolError::PoolLimitReached { tier, max });
        }

        // Create channels (temporary - no dispatcher connection)
        let (_command_tx, command_rx) = create_agent_channel(32);
        let (response_tx, _response_rx) = tokio::sync::mpsc::channel(32);

        // Create the agent
        let agent = Agent::new(tier, persona, model_config, Arc::clone(&self.llm_provider), command_rx, response_tx);
        let agent_id = agent.id.clone();
        let shared_status = agent.shared_status();

        info!(
            agent_id = ?agent_id,
            tier = ?tier,
            "Spawned new agent (test mode, no run loop)"
        );

        // Track the entry (no join handle — agent is not spawned)
        self.agents.insert(
            agent_id.clone(),
            AgentEntry {
                tier,
                status: shared_status,
                _join_handle: None,
            },
        );
        self.by_tier.entry(tier).or_insert_with(Vec::new).push(agent_id.clone());

        Ok(agent_id)
    }

    /// Spawn a new agent with channel connections to the dispatcher.
    ///
    /// The agent's `run()` loop is started as a tokio task immediately,
    /// so it can receive and process commands.
    pub fn spawn_agent_with_dispatcher(&mut self, tier: AgentTier, persona: AgentPersona, model_config: ModelConfig, dispatcher: &mut Dispatcher) -> Result<AgentId, PoolError> {
        // Check pool limit
        let max = self.max_for_tier(tier);
        if self.count(tier) >= max as usize {
            return Err(PoolError::PoolLimitReached { tier, max });
        }

        // Create channels
        let (command_tx, command_rx) = create_agent_channel(32);
        let response_tx = dispatcher.response_sender();

        // Create the agent
        let agent = Agent::new(tier, persona, model_config, Arc::clone(&self.llm_provider), command_rx, response_tx);
        let agent_id = agent.id.clone();
        let shared_status = agent.shared_status();

        // Create handle and register with dispatcher
        let handle = AgentHandle::new(agent_id.clone(), tier, command_tx);
        dispatcher.register_agent(handle);

        // Spawn the agent's run loop as a tokio task
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

        // Track the entry
        self.agents.insert(
            agent_id.clone(),
            AgentEntry {
                tier,
                status: shared_status,
                _join_handle: Some(join_handle),
            },
        );
        self.by_tier.entry(tier).or_default().push(agent_id.clone());

        info!(agent_id = ?agent_id, tier = ?tier, "Spawned agent with dispatcher (run loop started)");

        Ok(agent_id)
    }

    /// Get the ID of an available (idle) agent of the specified tier
    pub fn get_available_agent_id(&self, tier: AgentTier) -> Option<AgentId> {
        let agent_ids = self.by_tier.get(&tier)?;

        for agent_id in agent_ids {
            if let Some(entry) = self.agents.get(agent_id) {
                if let Ok(status) = entry.status.try_lock() {
                    if *status == AgentStatus::Idle {
                        return Some(agent_id.clone());
                    }
                }
            }
        }

        None
    }

    /// Check if a specific agent is available (idle)
    pub fn is_agent_available(&self, id: &AgentId) -> bool {
        self.agents.get(id).and_then(|e| e.status.try_lock().ok()).map(|s| *s == AgentStatus::Idle).unwrap_or(false)
    }

    /// Check if an agent exists in the pool
    pub fn has_agent(&self, id: &AgentId) -> bool {
        self.agents.contains_key(id)
    }

    /// Get the tier of an agent by ID
    pub fn agent_tier(&self, id: &AgentId) -> Option<AgentTier> {
        self.agents.get(id).map(|e| e.tier)
    }

    /// Remove an agent from the pool entirely
    ///
    /// The agent's tokio task will be aborted.
    pub fn remove_agent(&mut self, id: &AgentId) -> Result<(), PoolError> {
        let entry = self.agents.remove(id).ok_or_else(|| PoolError::AgentNotFound(id.clone()))?;

        let tier = entry.tier;

        // Abort the agent's tokio task if running
        if let Some(handle) = entry._join_handle {
            handle.abort();
        }

        // Remove from tier index
        if let Some(tier_agents) = self.by_tier.get_mut(&tier) {
            tier_agents.retain(|a| a != id);
        }

        info!(agent_id = ?id, tier = ?tier, "Agent removed from pool");
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

    /// Count available (idle) agents in a tier
    fn count_available(&self, tier: AgentTier) -> usize {
        self.by_tier
            .get(&tier)
            .map(|ids| {
                ids.iter()
                    .filter(|id| self.agents.get(*id).and_then(|e| e.status.try_lock().ok()).map(|s| *s == AgentStatus::Idle).unwrap_or(false))
                    .count()
            })
            .unwrap_or(0)
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            orchestrators: PoolTierStats {
                total: self.count(AgentTier::Orchestrator),
                available: self.count_available(AgentTier::Orchestrator),
                max: self.config.max_orchestrators as usize,
            },
            workers: PoolTierStats {
                total: self.count(AgentTier::Worker),
                available: self.count_available(AgentTier::Worker),
                max: self.config.max_workers as usize,
            },
            utilities: PoolTierStats {
                total: self.count(AgentTier::Utility),
                available: self.count_available(AgentTier::Utility),
                max: self.config.max_utilities as usize,
            },
        }
    }
}

/// Statistics for a single tier
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolTierStats {
    pub total: usize,
    pub available: usize,
    pub max: usize,
}

/// Pool-wide statistics
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolStats {
    pub orchestrators: PoolTierStats,
    pub workers: PoolTierStats,
    pub utilities: PoolTierStats,
}

/// Resolve the model config for a task, applying any `model_override` from task metadata.
///
/// If the task's metadata contains a `model_override` key, that model ID is used
/// instead of the tier default. Otherwise, `tier_model` is returned unchanged.
#[allow(dead_code)]
pub fn resolve_model_for_task(tier_model: ModelConfig, task: &Task) -> ModelConfig {
    if let Some(override_id) = task.metadata.as_ref().and_then(|m| m.get("model_override")) {
        ModelConfig {
            model_id: override_id.clone(),
            ..tier_model
        }
    } else {
        tier_model
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{LLMError, LLMRequest, LLMResponse, StopReason, StreamChunk, TokenUsage};
    use async_trait::async_trait;
    use futures::Stream;
    use std::pin::Pin;

    /// Mock LLM provider for testing
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

        async fn send_message_stream(&self, _request: LLMRequest) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>>, LLMError> {
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
        let config = AgentPoolConfig {
            max_orchestrators: 2,
            max_workers: 3,
            max_utilities: 4,
        };
        let provider = Arc::new(MockLLMProvider);
        AgentPool::new(config, provider)
    }

    #[test]
    fn new_pool_is_empty() {
        let pool = create_test_pool();
        assert_eq!(pool.count(AgentTier::Orchestrator), 0);
        assert_eq!(pool.count(AgentTier::Worker), 0);
        assert_eq!(pool.count(AgentTier::Utility), 0);
        assert_eq!(pool.total_count(), 0);
    }

    #[test]
    fn max_for_tier_returns_config_values() {
        let pool = create_test_pool();
        assert_eq!(pool.max_for_tier(AgentTier::Orchestrator), 2);
        assert_eq!(pool.max_for_tier(AgentTier::Worker), 3);
        assert_eq!(pool.max_for_tier(AgentTier::Utility), 4);
    }

    #[test]
    fn can_spawn_returns_true_for_empty_pool() {
        let pool = create_test_pool();
        assert!(pool.can_spawn(AgentTier::Orchestrator));
        assert!(pool.can_spawn(AgentTier::Worker));
        assert!(pool.can_spawn(AgentTier::Utility));
    }

    #[test]
    fn spawn_agent_creates_worker() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        let result = pool.spawn_agent(AgentTier::Worker, persona, model_config);
        assert!(result.is_ok());

        let agent_id = result.unwrap();
        assert_eq!(pool.count(AgentTier::Worker), 1);
        assert_eq!(pool.total_count(), 1);

        // Verify the agent is tracked
        assert!(pool.has_agent(&agent_id));
        assert!(pool.by_tier.get(&AgentTier::Worker).unwrap().contains(&agent_id));
    }

    #[test]
    fn spawn_agent_respects_tier_limits() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        // Spawn up to limit (max_orchestrators = 2)
        pool.spawn_agent(AgentTier::Orchestrator, persona.clone(), model_config.clone()).unwrap();
        pool.spawn_agent(AgentTier::Orchestrator, persona.clone(), model_config.clone()).unwrap();

        assert_eq!(pool.count(AgentTier::Orchestrator), 2);
        assert!(!pool.can_spawn(AgentTier::Orchestrator));

        // Third spawn should fail
        let result = pool.spawn_agent(AgentTier::Orchestrator, persona, model_config);
        assert!(result.is_err());

        match result {
            Err(PoolError::PoolLimitReached { tier, max }) => {
                assert_eq!(tier, AgentTier::Orchestrator);
                assert_eq!(max, 2);
            }
            _ => panic!("Expected PoolLimitReached error"),
        }
    }

    #[test]
    fn spawn_agent_independent_tier_limits() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        // Fill orchestrators
        pool.spawn_agent(AgentTier::Orchestrator, persona.clone(), model_config.clone()).unwrap();
        pool.spawn_agent(AgentTier::Orchestrator, persona.clone(), model_config.clone()).unwrap();

        // Can still spawn workers
        assert!(pool.can_spawn(AgentTier::Worker));
        let result = pool.spawn_agent(AgentTier::Worker, persona, model_config);
        assert!(result.is_ok());
        assert_eq!(pool.count(AgentTier::Worker), 1);
    }

    #[test]
    fn get_available_agent_returns_idle_agent() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        // Spawn an agent
        let agent_id = pool.spawn_agent(AgentTier::Worker, persona, model_config).unwrap();

        // Should find the available agent
        let available_id = pool.get_available_agent_id(AgentTier::Worker);
        assert!(available_id.is_some());
        assert_eq!(available_id.unwrap(), agent_id);

        // is_agent_available should also work
        assert!(pool.is_agent_available(&agent_id));
    }

    #[test]
    fn get_available_agent_returns_none_for_empty_tier() {
        let pool = create_test_pool();

        // No agents spawned, should return None
        let available = pool.get_available_agent_id(AgentTier::Worker);
        assert!(available.is_none());
    }

    #[test]
    fn has_agent_by_id() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        let agent_id = pool.spawn_agent(AgentTier::Worker, persona, model_config).unwrap();

        // Has agent should work
        assert!(pool.has_agent(&agent_id));
        assert_eq!(pool.agent_tier(&agent_id), Some(AgentTier::Worker));

        // Unknown ID should return false/None
        let unknown_id = AgentId::new();
        assert!(!pool.has_agent(&unknown_id));
        assert_eq!(pool.agent_tier(&unknown_id), None);
    }

    #[test]
    fn remove_agent_cleans_up() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        let agent_id = pool.spawn_agent(AgentTier::Worker, persona, model_config).unwrap();

        assert_eq!(pool.count(AgentTier::Worker), 1);
        assert_eq!(pool.total_count(), 1);

        // Remove the agent
        let result = pool.remove_agent(&agent_id);
        assert!(result.is_ok());

        // Agent should be gone
        assert_eq!(pool.count(AgentTier::Worker), 0);
        assert_eq!(pool.total_count(), 0);
        assert!(!pool.has_agent(&agent_id));
        assert!(pool.by_tier.get(&AgentTier::Worker).unwrap().is_empty());
    }

    #[test]
    fn remove_unknown_agent_fails() {
        let mut pool = create_test_pool();

        let unknown_id = AgentId::new();
        let result = pool.remove_agent(&unknown_id);
        assert!(result.is_err());

        match result {
            Err(PoolError::AgentNotFound(_)) => {}
            _ => panic!("Expected AgentNotFound error"),
        }
    }

    #[test]
    fn shutdown_all_clears_pool() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        // Spawn multiple agents
        pool.spawn_agent(AgentTier::Worker, persona.clone(), model_config.clone()).unwrap();
        pool.spawn_agent(AgentTier::Worker, persona.clone(), model_config.clone()).unwrap();
        pool.spawn_agent(AgentTier::Orchestrator, persona, model_config).unwrap();

        assert_eq!(pool.total_count(), 3);

        // Shutdown all
        pool.shutdown_all();

        // All agents should be gone
        assert_eq!(pool.total_count(), 0);
        assert_eq!(pool.count(AgentTier::Worker), 0);
        assert_eq!(pool.count(AgentTier::Orchestrator), 0);
    }

    #[test]
    fn stats_returns_correct_values() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        // Initial stats
        let stats = pool.stats();
        assert_eq!(stats.workers.total, 0);
        assert_eq!(stats.workers.available, 0);
        assert_eq!(stats.workers.max, 3);

        // Spawn some agents (in test mode, agents start Idle)
        pool.spawn_agent(AgentTier::Worker, persona.clone(), model_config.clone()).unwrap();
        pool.spawn_agent(AgentTier::Worker, persona.clone(), model_config.clone()).unwrap();

        let stats = pool.stats();
        assert_eq!(stats.workers.total, 2);
        assert_eq!(stats.workers.available, 2);
    }

    #[test]
    fn pool_error_display_messages() {
        let e1 = PoolError::PoolLimitReached { tier: AgentTier::Worker, max: 3 };
        assert!(e1.to_string().contains("pool limit reached"));
        assert!(e1.to_string().contains("Worker"));

        let e2 = PoolError::AgentNotFound(AgentId::new());
        assert!(e2.to_string().contains("agent not found"));

        let e3 = PoolError::NoAvailableAgent(AgentTier::Utility);
        assert!(e3.to_string().contains("no available agent"));
    }

    #[test]
    fn spawn_all_tiers() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        let orch_id = pool.spawn_agent(AgentTier::Orchestrator, persona.clone(), model_config.clone()).unwrap();
        let worker_id = pool.spawn_agent(AgentTier::Worker, persona.clone(), model_config.clone()).unwrap();
        let util_id = pool.spawn_agent(AgentTier::Utility, persona, model_config).unwrap();

        assert_eq!(pool.agent_tier(&orch_id), Some(AgentTier::Orchestrator));
        assert_eq!(pool.agent_tier(&worker_id), Some(AgentTier::Worker));
        assert_eq!(pool.agent_tier(&util_id), Some(AgentTier::Utility));
        assert_eq!(pool.total_count(), 3);
    }

    #[test]
    fn spawn_worker_limit_reached() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        // max_workers = 3
        for _ in 0..3 {
            pool.spawn_agent(AgentTier::Worker, persona.clone(), model_config.clone()).unwrap();
        }
        assert!(!pool.can_spawn(AgentTier::Worker));
        let result = pool.spawn_agent(AgentTier::Worker, persona, model_config);
        match result {
            Err(PoolError::PoolLimitReached { tier, max }) => {
                assert_eq!(tier, AgentTier::Worker);
                assert_eq!(max, 3);
            }
            _ => panic!("Expected PoolLimitReached"),
        }
    }

    #[test]
    fn spawn_utility_limit_reached() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        // max_utilities = 4
        for _ in 0..4 {
            pool.spawn_agent(AgentTier::Utility, persona.clone(), model_config.clone()).unwrap();
        }
        assert!(!pool.can_spawn(AgentTier::Utility));
        let result = pool.spawn_agent(AgentTier::Utility, persona, model_config);
        assert!(matches!(result, Err(PoolError::PoolLimitReached { .. })));
    }

    #[test]
    fn get_available_agent_id_returns_agent() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        let agent_id = pool.spawn_agent(AgentTier::Worker, persona, model_config).unwrap();

        let available = pool.get_available_agent_id(AgentTier::Worker);
        assert!(available.is_some());
        assert_eq!(available.unwrap(), agent_id);
    }

    #[test]
    fn get_available_agent_id_returns_none_when_empty() {
        let pool = create_test_pool();
        assert!(pool.get_available_agent_id(AgentTier::Orchestrator).is_none());
    }

    #[test]
    fn remove_agent_then_can_spawn_again() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        // Fill orchestrators (max=2)
        let id1 = pool.spawn_agent(AgentTier::Orchestrator, persona.clone(), model_config.clone()).unwrap();
        pool.spawn_agent(AgentTier::Orchestrator, persona.clone(), model_config.clone()).unwrap();
        assert!(!pool.can_spawn(AgentTier::Orchestrator));

        // Remove one
        pool.remove_agent(&id1).unwrap();
        assert!(pool.can_spawn(AgentTier::Orchestrator));

        // Can spawn again
        pool.spawn_agent(AgentTier::Orchestrator, persona, model_config).unwrap();
        assert_eq!(pool.count(AgentTier::Orchestrator), 2);
    }

    #[test]
    fn shutdown_all_with_agents() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        pool.spawn_agent(AgentTier::Worker, persona.clone(), model_config.clone()).unwrap();
        pool.spawn_agent(AgentTier::Utility, persona, model_config).unwrap();

        pool.shutdown_all();
        assert_eq!(pool.total_count(), 0);
    }

    #[test]
    fn count_available_empty_tier() {
        let pool = create_test_pool();
        // count_available is private but tested through stats
        let stats = pool.stats();
        assert_eq!(stats.orchestrators.available, 0);
        assert_eq!(stats.workers.available, 0);
        assert_eq!(stats.utilities.available, 0);
    }

    #[test]
    fn stats_with_multiple_agents() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        pool.spawn_agent(AgentTier::Utility, persona.clone(), model_config.clone()).unwrap();
        pool.spawn_agent(AgentTier::Utility, persona.clone(), model_config.clone()).unwrap();
        pool.spawn_agent(AgentTier::Utility, persona, model_config).unwrap();

        let stats = pool.stats();
        assert_eq!(stats.utilities.total, 3);
        assert_eq!(stats.utilities.available, 3);
        assert_eq!(stats.utilities.max, 4);
    }

    #[test]
    fn stats_shows_all_tiers() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        pool.spawn_agent(AgentTier::Orchestrator, persona.clone(), model_config.clone()).unwrap();
        pool.spawn_agent(AgentTier::Worker, persona.clone(), model_config.clone()).unwrap();
        pool.spawn_agent(AgentTier::Worker, persona.clone(), model_config.clone()).unwrap();
        pool.spawn_agent(AgentTier::Utility, persona, model_config).unwrap();

        let stats = pool.stats();
        assert_eq!(stats.orchestrators.total, 1);
        assert_eq!(stats.orchestrators.max, 2);
        assert_eq!(stats.workers.total, 2);
        assert_eq!(stats.workers.max, 3);
        assert_eq!(stats.utilities.total, 1);
        assert_eq!(stats.utilities.max, 4);
    }

    // === resolve_model_for_task tests ===

    fn make_task_for_model(metadata: Option<std::collections::HashMap<String, String>>) -> Task {
        use crate::types::{Priority, TaskId, TaskStatus};
        Task {
            id: TaskId::new(),
            slice_id: None,
            title: "test".to_string(),
            description: String::new(),
            assigned_tier: AgentTier::Worker,
            assigned_agent: None,
            status: TaskStatus::Pending,
            priority: Priority::Normal,
            context_files: vec![],
            metadata,
            depends_on: vec![],
            retry_count: 0,
            max_retries: 3,
            last_error: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn resolve_model_no_override_returns_tier_default() {
        let tier_model = ModelConfig {
            model_id: "claude-sonnet-4-20250514".to_string(),
            ..Default::default()
        };
        let task = make_task_for_model(None);

        let resolved = resolve_model_for_task(tier_model.clone(), &task);
        assert_eq!(resolved.model_id, "claude-sonnet-4-20250514");
    }

    #[test]
    fn resolve_model_with_override_uses_override() {
        let tier_model = ModelConfig {
            model_id: "claude-sonnet-4-20250514".to_string(),
            max_tokens: 4096,
            ..Default::default()
        };
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("model_override".to_string(), "claude-opus-4-5-20251101".to_string());
        let task = make_task_for_model(Some(metadata));

        let resolved = resolve_model_for_task(tier_model, &task);
        assert_eq!(resolved.model_id, "claude-opus-4-5-20251101");
        // Other config preserved
        assert_eq!(resolved.max_tokens, 4096);
    }

    #[test]
    fn resolve_model_empty_metadata_returns_tier_default() {
        let tier_model = ModelConfig {
            model_id: "claude-sonnet-4-20250514".to_string(),
            ..Default::default()
        };
        let task = make_task_for_model(Some(std::collections::HashMap::new()));

        let resolved = resolve_model_for_task(tier_model, &task);
        assert_eq!(resolved.model_id, "claude-sonnet-4-20250514");
    }
}
