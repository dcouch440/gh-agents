//! Agent pool management

use std::collections::HashMap;
use std::sync::Arc;

use thiserror::Error;
use tracing::{info, warn};

use crate::llm::LLMProvider;
use crate::types::{AgentPersona, AgentPoolConfig, AgentTier, ModelConfig};

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

/// Manages pools of agents organized by tier
pub struct AgentPool {
    /// All agents managed by this pool
    agents: HashMap<AgentId, Agent>,
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

    /// Spawn a new agent of the specified tier
    ///
    /// Returns the new agent's ID, or an error if the pool limit is reached.
    ///
    /// # Model Configuration
    /// The agent uses the provided `model_config`. For difficulty-based model
    /// selection, tasks are routed to appropriate tiers:
    /// - `difficulty=complex` → Orchestrator tier → Opus
    /// - `difficulty=standard` → Worker tier → Sonnet
    /// - `difficulty=simple` → Utility tier → Sonnet
    ///
    /// # Future: Model Override
    /// TODO: Support per-task model override via task.metadata["model_override"].
    /// This would require architectural changes to allow agents to switch models
    /// per task, rather than being bound to a model at spawn time.
    pub fn spawn_agent(
        &mut self,
        tier: AgentTier,
        persona: AgentPersona,
        model_config: ModelConfig,
    ) -> Result<AgentId, PoolError> {
        // Check pool limit
        let max = self.max_for_tier(tier);
        if self.count(tier) >= max as usize {
            return Err(PoolError::PoolLimitReached { tier, max });
        }

        // Create channels (temporary - no dispatcher connection)
        let (_command_tx, command_rx) = create_agent_channel(32);
        let (response_tx, _response_rx) = tokio::sync::mpsc::channel(32);

        // Create the agent
        let agent = Agent::new(
            tier,
            persona,
            model_config,
            Arc::clone(&self.llm_provider),
            command_rx,
            response_tx,
        );
        let agent_id = agent.id.clone();

        info!(
            agent_id = ?agent_id,
            tier = ?tier,
            "Spawned new agent"
        );

        // Track the agent
        self.agents.insert(agent_id.clone(), agent);
        self.by_tier
            .entry(tier)
            .or_insert_with(Vec::new)
            .push(agent_id.clone());

        Ok(agent_id)
    }

    /// Spawn a new agent with channel connections to the dispatcher
    ///
    /// This method properly wires up both command and response channels.
    pub fn spawn_agent_with_dispatcher(
        &mut self,
        tier: AgentTier,
        persona: AgentPersona,
        model_config: ModelConfig,
        dispatcher: &mut Dispatcher,
    ) -> Result<AgentId, PoolError> {
        // Check pool limit
        let max = self.max_for_tier(tier);
        if self.count(tier) >= max as usize {
            return Err(PoolError::PoolLimitReached { tier, max });
        }

        // Create channels
        let (command_tx, command_rx) = create_agent_channel(32);
        let response_tx = dispatcher.response_sender();

        // Create the agent
        let agent = Agent::new(
            tier,
            persona,
            model_config,
            Arc::clone(&self.llm_provider),
            command_rx,
            response_tx,
        );
        let agent_id = agent.id.clone();

        // Create handle and register with dispatcher
        let handle = AgentHandle::new(agent_id.clone(), tier, command_tx);
        dispatcher.register_agent(handle);

        // Track in pool
        self.agents.insert(agent_id.clone(), agent);
        self.by_tier
            .entry(tier)
            .or_insert_with(Vec::new)
            .push(agent_id.clone());

        info!(agent_id = ?agent_id, tier = ?tier, "Spawned agent with dispatcher connection");

        Ok(agent_id)
    }

    /// Get the ID of an available (idle) agent of the specified tier
    ///
    /// Returns the ID without borrowing mutably, useful when you need to
    /// check availability before taking action.
    pub fn get_available_agent_id(&self, tier: AgentTier) -> Option<AgentId> {
        let agent_ids = self.by_tier.get(&tier)?;

        for agent_id in agent_ids {
            if let Some(agent) = self.agents.get(agent_id) {
                if agent.is_available() {
                    return Some(agent_id.clone());
                }
            }
        }

        None
    }

    /// Get an available (idle) agent of the specified tier
    ///
    /// Returns a mutable reference to the agent, or None if no agents are available.
    pub fn get_available_agent(&mut self, tier: AgentTier) -> Option<&mut Agent> {
        // First find an available agent ID without holding mutable borrow
        let agent_id = self.get_available_agent_id(tier)?;
        // Then get mutable reference
        self.agents.get_mut(&agent_id)
    }

    /// Get a reference to an agent by ID
    pub fn get_agent(&self, id: &AgentId) -> Option<&Agent> {
        self.agents.get(id)
    }

    /// Get a mutable reference to an agent by ID
    pub fn get_agent_mut(&mut self, id: &AgentId) -> Option<&mut Agent> {
        self.agents.get_mut(id)
    }

    /// Release an agent back to idle state
    ///
    /// If the agent is working on a task, the task will be failed.
    pub fn release_agent(&mut self, id: &AgentId) -> Result<(), PoolError> {
        let agent = self
            .agents
            .get_mut(id)
            .ok_or_else(|| PoolError::AgentNotFound(id.clone()))?;

        // If agent is not idle, fail any current task
        if !agent.is_available() {
            if let Err(e) = agent.fail_task() {
                warn!(agent_id = ?id, error = ?e, "Error failing task during release");
            }
        }

        info!(agent_id = ?id, "Agent released to pool");
        Ok(())
    }

    /// Remove an agent from the pool entirely
    ///
    /// The agent will be shut down before removal.
    pub fn remove_agent(&mut self, id: &AgentId) -> Result<(), PoolError> {
        // Get and shutdown the agent
        let mut agent = self
            .agents
            .remove(id)
            .ok_or_else(|| PoolError::AgentNotFound(id.clone()))?;

        let tier = agent.tier();
        agent.shutdown()?;

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
                    .filter(|id| {
                        self.agents
                            .get(*id)
                            .map(|a| a.is_available())
                            .unwrap_or(false)
                    })
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
                model: "test-model".to_string(),
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage {
                    input_tokens: 10,
                    output_tokens: 20,
                },
            })
        }

        async fn send_message_stream(
            &self,
            _request: LLMRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>>, LLMError>
        {
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
        assert!(pool.agents.contains_key(&agent_id));
        assert!(pool
            .by_tier
            .get(&AgentTier::Worker)
            .unwrap()
            .contains(&agent_id));
    }

    #[test]
    fn spawn_agent_respects_tier_limits() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        // Spawn up to limit (max_orchestrators = 2)
        pool.spawn_agent(
            AgentTier::Orchestrator,
            persona.clone(),
            model_config.clone(),
        )
        .unwrap();
        pool.spawn_agent(
            AgentTier::Orchestrator,
            persona.clone(),
            model_config.clone(),
        )
        .unwrap();

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
        pool.spawn_agent(
            AgentTier::Orchestrator,
            persona.clone(),
            model_config.clone(),
        )
        .unwrap();
        pool.spawn_agent(
            AgentTier::Orchestrator,
            persona.clone(),
            model_config.clone(),
        )
        .unwrap();

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
        let agent_id = pool
            .spawn_agent(AgentTier::Worker, persona, model_config)
            .unwrap();

        // Should find the available agent
        let available_id = pool.get_available_agent_id(AgentTier::Worker);
        assert!(available_id.is_some());
        assert_eq!(available_id.unwrap(), agent_id);

        // get_available_agent should also work
        let agent = pool.get_available_agent(AgentTier::Worker);
        assert!(agent.is_some());
        assert!(agent.unwrap().is_available());
    }

    #[test]
    fn get_available_agent_returns_none_when_all_busy() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        // Spawn an agent and make it busy
        let agent_id = pool
            .spawn_agent(AgentTier::Worker, persona, model_config)
            .unwrap();

        // Start a task to make the agent busy
        let agent = pool.get_agent_mut(&agent_id).unwrap();
        agent.start_task(uuid::Uuid::new_v4()).unwrap();

        // Now get_available_agent should return None
        let available = pool.get_available_agent_id(AgentTier::Worker);
        assert!(available.is_none());
    }

    #[test]
    fn get_available_agent_returns_none_for_empty_tier() {
        let pool = create_test_pool();

        // No agents spawned, should return None
        let available = pool.get_available_agent_id(AgentTier::Worker);
        assert!(available.is_none());
    }

    #[test]
    fn get_agent_by_id() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        let agent_id = pool
            .spawn_agent(AgentTier::Worker, persona, model_config)
            .unwrap();

        // Get by ID should work
        let agent = pool.get_agent(&agent_id);
        assert!(agent.is_some());
        assert_eq!(agent.unwrap().tier(), AgentTier::Worker);

        // Unknown ID should return None
        let unknown_id = AgentId::new();
        assert!(pool.get_agent(&unknown_id).is_none());
    }

    #[test]
    fn get_available_agent_finds_first_idle_among_busy() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        // Spawn two agents
        let agent1_id = pool
            .spawn_agent(AgentTier::Worker, persona.clone(), model_config.clone())
            .unwrap();
        let agent2_id = pool
            .spawn_agent(AgentTier::Worker, persona, model_config)
            .unwrap();

        // Make first agent busy
        pool.get_agent_mut(&agent1_id)
            .unwrap()
            .start_task(uuid::Uuid::new_v4())
            .unwrap();

        // Should find the second agent (still idle)
        let available_id = pool.get_available_agent_id(AgentTier::Worker);
        assert!(available_id.is_some());
        assert_eq!(available_id.unwrap(), agent2_id);
    }

    #[test]
    fn release_idle_agent_is_noop() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        let agent_id = pool
            .spawn_agent(AgentTier::Worker, persona, model_config)
            .unwrap();

        // Agent is already idle, release should succeed
        let result = pool.release_agent(&agent_id);
        assert!(result.is_ok());

        // Agent should still be available
        assert!(pool.get_agent(&agent_id).unwrap().is_available());
    }

    #[test]
    fn release_working_agent_returns_to_idle() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        let agent_id = pool
            .spawn_agent(AgentTier::Worker, persona, model_config)
            .unwrap();

        // Make the agent busy
        pool.get_agent_mut(&agent_id)
            .unwrap()
            .start_task(uuid::Uuid::new_v4())
            .unwrap();

        assert!(!pool.get_agent(&agent_id).unwrap().is_available());

        // Release should return it to idle
        let result = pool.release_agent(&agent_id);
        assert!(result.is_ok());
        assert!(pool.get_agent(&agent_id).unwrap().is_available());
    }

    #[test]
    fn release_unknown_agent_fails() {
        let mut pool = create_test_pool();

        let unknown_id = AgentId::new();
        let result = pool.release_agent(&unknown_id);
        assert!(result.is_err());

        match result {
            Err(PoolError::AgentNotFound(_)) => {}
            _ => panic!("Expected AgentNotFound error"),
        }
    }

    #[test]
    fn remove_agent_cleans_up() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        let agent_id = pool
            .spawn_agent(AgentTier::Worker, persona, model_config)
            .unwrap();

        assert_eq!(pool.count(AgentTier::Worker), 1);
        assert_eq!(pool.total_count(), 1);

        // Remove the agent
        let result = pool.remove_agent(&agent_id);
        assert!(result.is_ok());

        // Agent should be gone
        assert_eq!(pool.count(AgentTier::Worker), 0);
        assert_eq!(pool.total_count(), 0);
        assert!(pool.get_agent(&agent_id).is_none());
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
        pool.spawn_agent(AgentTier::Worker, persona.clone(), model_config.clone())
            .unwrap();
        pool.spawn_agent(AgentTier::Worker, persona.clone(), model_config.clone())
            .unwrap();
        pool.spawn_agent(AgentTier::Orchestrator, persona, model_config)
            .unwrap();

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

        // Spawn some agents
        let agent1_id = pool
            .spawn_agent(AgentTier::Worker, persona.clone(), model_config.clone())
            .unwrap();
        pool.spawn_agent(AgentTier::Worker, persona.clone(), model_config.clone())
            .unwrap();

        let stats = pool.stats();
        assert_eq!(stats.workers.total, 2);
        assert_eq!(stats.workers.available, 2);

        // Make one busy
        pool.get_agent_mut(&agent1_id)
            .unwrap()
            .start_task(uuid::Uuid::new_v4())
            .unwrap();

        let stats = pool.stats();
        assert_eq!(stats.workers.total, 2);
        assert_eq!(stats.workers.available, 1);
    }

    #[test]
    fn stats_shows_all_tiers() {
        let mut pool = create_test_pool();
        let persona = AgentPersona::default();
        let model_config = ModelConfig::default();

        pool.spawn_agent(
            AgentTier::Orchestrator,
            persona.clone(),
            model_config.clone(),
        )
        .unwrap();
        pool.spawn_agent(AgentTier::Worker, persona.clone(), model_config.clone())
            .unwrap();
        pool.spawn_agent(AgentTier::Worker, persona.clone(), model_config.clone())
            .unwrap();
        pool.spawn_agent(AgentTier::Utility, persona, model_config)
            .unwrap();

        let stats = pool.stats();
        assert_eq!(stats.orchestrators.total, 1);
        assert_eq!(stats.orchestrators.max, 2);
        assert_eq!(stats.workers.total, 2);
        assert_eq!(stats.workers.max, 3);
        assert_eq!(stats.utilities.total, 1);
        assert_eq!(stats.utilities.max, 4);
    }
}
