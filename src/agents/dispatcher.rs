//! Central dispatcher for agent communication

use std::collections::HashMap;
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{info, warn};

use super::agent::AgentId;
use super::channels::{AgentCommand, AgentHandle, AgentResponse};
use crate::types::AgentTier;

/// Central dispatcher for agent communication
pub struct Dispatcher {
    /// Handles for sending commands to agents
    handles: HashMap<AgentId, AgentHandle>,
    /// Handles grouped by tier
    by_tier: HashMap<AgentTier, Vec<AgentId>>,
    /// Channel for receiving responses from agents
    response_rx: mpsc::Receiver<AgentResponse>,
    /// Sender that agents use to send responses (cloned to each agent)
    response_tx: mpsc::Sender<AgentResponse>,
}

impl Dispatcher {
    /// Create a new dispatcher
    pub fn new(response_buffer_size: usize) -> Self {
        let (response_tx, response_rx) = mpsc::channel(response_buffer_size);

        let mut by_tier = HashMap::new();
        by_tier.insert(AgentTier::Orchestrator, Vec::new());
        by_tier.insert(AgentTier::Worker, Vec::new());
        by_tier.insert(AgentTier::Utility, Vec::new());

        Self {
            handles: HashMap::new(),
            by_tier,
            response_rx,
            response_tx,
        }
    }

    /// Get the response sender (clone this for each agent)
    pub fn response_sender(&self) -> mpsc::Sender<AgentResponse> {
        self.response_tx.clone()
    }

    /// Register an agent handle
    pub fn register_agent(&mut self, handle: AgentHandle) {
        let agent_id = handle.agent_id.clone();
        let tier = handle.tier;

        info!(agent_id = ?agent_id, tier = ?tier, "Registering agent with dispatcher");

        self.handles.insert(agent_id.clone(), handle);
        self.by_tier
            .entry(tier)
            .or_insert_with(Vec::new)
            .push(agent_id);
    }

    /// Unregister an agent
    pub fn unregister_agent(&mut self, agent_id: &AgentId) {
        if let Some(handle) = self.handles.remove(agent_id) {
            if let Some(tier_agents) = self.by_tier.get_mut(&handle.tier) {
                tier_agents.retain(|id| id != agent_id);
            }
            info!(agent_id = ?agent_id, "Unregistered agent from dispatcher");
        }
    }

    /// Send a command to a specific agent
    pub async fn send_to_agent(
        &self,
        agent_id: &AgentId,
        command: AgentCommand,
    ) -> Result<(), DispatchError> {
        let handle = self
            .handles
            .get(agent_id)
            .ok_or_else(|| DispatchError::AgentNotFound(agent_id.clone()))?;

        handle
            .send(command)
            .await
            .map_err(|_| DispatchError::ChannelClosed(agent_id.clone()))
    }

    /// Broadcast a command to all agents of a tier
    pub async fn broadcast_to_tier(
        &self,
        tier: AgentTier,
        command: AgentCommand,
    ) -> Vec<DispatchError> {
        let mut errors = Vec::new();

        if let Some(agent_ids) = self.by_tier.get(&tier) {
            for agent_id in agent_ids {
                if let Some(handle) = self.handles.get(agent_id) {
                    if let Err(_e) = handle.send(command.clone()).await {
                        errors.push(DispatchError::ChannelClosed(agent_id.clone()));
                        warn!(agent_id = ?agent_id, "Failed to send command");
                    }
                }
            }
        }

        errors
    }

    /// Receive the next response from any agent
    pub async fn recv_response(&mut self) -> Option<AgentResponse> {
        self.response_rx.recv().await
    }

    /// Try to receive a response without blocking
    pub fn try_recv_response(&mut self) -> Option<AgentResponse> {
        self.response_rx.try_recv().ok()
    }

    /// Get the number of registered agents
    pub fn agent_count(&self) -> usize {
        self.handles.len()
    }

    /// Get the number of registered agents for a tier
    pub fn tier_count(&self, tier: AgentTier) -> usize {
        self.by_tier.get(&tier).map(|v| v.len()).unwrap_or(0)
    }
}

#[derive(Error, Debug)]
pub enum DispatchError {
    #[error("agent not found: {0:?}")]
    AgentNotFound(AgentId),

    #[error("channel closed for agent: {0:?}")]
    ChannelClosed(AgentId),
}
