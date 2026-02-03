//! Central dispatcher for agent communication

use std::collections::HashMap;
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::info;

use super::agent::AgentId;
use super::channels::{AgentCommand, AgentHandle, AgentResponse};

/// Central dispatcher for agent communication
pub struct Dispatcher {
    /// Handles for sending commands to agents
    handles: HashMap<AgentId, AgentHandle>,
    /// Channel for receiving responses from agents (None if taken by consumer)
    response_rx: Option<mpsc::Receiver<AgentResponse>>,
    /// Sender that agents use to send responses (cloned to each agent)
    response_tx: mpsc::Sender<AgentResponse>,
}

impl Dispatcher {
    /// Create a new dispatcher
    pub fn new(response_buffer_size: usize) -> Self {
        let (response_tx, response_rx) = mpsc::channel(response_buffer_size);

        Self {
            handles: HashMap::new(),
            response_rx: Some(response_rx),
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
        info!(agent_id = ?agent_id, "Registering agent with dispatcher");
        self.handles.insert(agent_id, handle);
    }

    /// Unregister an agent
    pub fn unregister_agent(&mut self, agent_id: &AgentId) {
        if self.handles.remove(agent_id).is_some() {
            info!(agent_id = ?agent_id, "Unregistered agent from dispatcher");
        }
    }

    /// Send a command to a specific agent
    pub async fn send_to_agent(&self, agent_id: &AgentId, command: AgentCommand) -> Result<(), DispatchError> {
        let handle = self.handles.get(agent_id).ok_or_else(|| DispatchError::AgentNotFound(agent_id.clone()))?;

        handle.send(command).await.map_err(|_| DispatchError::ChannelClosed(agent_id.clone()))
    }

    /// Take ownership of the response receiver.
    /// Call this once to hand the receiver to a dedicated consumer task
    /// so it can await responses without holding the dispatcher mutex.
    pub fn take_response_rx(&mut self) -> Option<mpsc::Receiver<AgentResponse>> {
        self.response_rx.take()
    }

    /// Receive the next response from any agent
    pub async fn recv_response(&mut self) -> Option<AgentResponse> {
        match &mut self.response_rx {
            Some(rx) => rx.recv().await,
            None => None,
        }
    }

    /// Try to receive a response without blocking
    pub fn try_recv_response(&mut self) -> Option<AgentResponse> {
        match &mut self.response_rx {
            Some(rx) => rx.try_recv().ok(),
            None => None,
        }
    }

    /// Get the number of registered agents
    pub fn agent_count(&self) -> usize {
        self.handles.len()
    }
}

#[derive(Error, Debug)]
pub enum DispatchError {
    #[error("agent not found: {0:?}")]
    AgentNotFound(AgentId),

    #[error("channel closed for agent: {0:?}")]
    ChannelClosed(AgentId),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    use uuid::Uuid;

    fn make_handle() -> (AgentHandle, mpsc::Receiver<AgentCommand>) {
        let agent_id = AgentId(Uuid::new_v4());
        let (tx, rx) = mpsc::channel(8);
        let handle = AgentHandle::new(agent_id, tx);
        (handle, rx)
    }

    #[test]
    fn new_dispatcher_starts_empty() {
        let d = Dispatcher::new(16);
        assert_eq!(d.agent_count(), 0);
    }

    #[test]
    fn register_agent_increments_counts() {
        let mut d = Dispatcher::new(16);
        let (h, _rx) = make_handle();
        d.register_agent(h);
        assert_eq!(d.agent_count(), 1);
    }

    #[test]
    fn register_multiple_agents() {
        let mut d = Dispatcher::new(16);
        let (h1, _r1) = make_handle();
        let (h2, _r2) = make_handle();
        d.register_agent(h1);
        d.register_agent(h2);
        assert_eq!(d.agent_count(), 2);
    }

    #[test]
    fn unregister_agent_decrements_counts() {
        let mut d = Dispatcher::new(16);
        let (h, _rx) = make_handle();
        let id = h.agent_id.clone();
        d.register_agent(h);
        assert_eq!(d.agent_count(), 1);
        d.unregister_agent(&id);
        assert_eq!(d.agent_count(), 0);
    }

    #[test]
    fn unregister_nonexistent_agent_is_noop() {
        let mut d = Dispatcher::new(16);
        let id = AgentId(Uuid::new_v4());
        d.unregister_agent(&id); // should not panic
        assert_eq!(d.agent_count(), 0);
    }

    #[tokio::test]
    async fn send_to_agent_success() {
        let mut d = Dispatcher::new(16);
        let (h, mut rx) = make_handle();
        let id = h.agent_id.clone();
        d.register_agent(h);

        d.send_to_agent(&id, AgentCommand::Shutdown).await.unwrap();
        let cmd = rx.recv().await.unwrap();
        assert!(matches!(cmd, AgentCommand::Shutdown));
    }

    #[tokio::test]
    async fn send_to_unknown_agent_returns_error() {
        let d = Dispatcher::new(16);
        let id = AgentId(Uuid::new_v4());
        let err = d.send_to_agent(&id, AgentCommand::Shutdown).await.unwrap_err();
        assert!(matches!(err, DispatchError::AgentNotFound(_)));
    }

    #[tokio::test]
    async fn send_to_agent_with_closed_channel() {
        let mut d = Dispatcher::new(16);
        let (h, rx) = make_handle();
        let id = h.agent_id.clone();
        d.register_agent(h);
        drop(rx); // close receiver

        let err = d.send_to_agent(&id, AgentCommand::Shutdown).await.unwrap_err();
        assert!(matches!(err, DispatchError::ChannelClosed(_)));
    }

    #[tokio::test]
    async fn recv_response_receives_sent_message() {
        let mut d = Dispatcher::new(16);
        let tx = d.response_sender();
        let agent_id = AgentId(Uuid::new_v4());
        let task_id = Uuid::new_v4();

        tx.send(AgentResponse::TaskStarted { agent_id: agent_id.clone(), task_id }).await.unwrap();

        let resp = d.recv_response().await.unwrap();
        assert!(matches!(resp, AgentResponse::TaskStarted { .. }));
    }

    #[tokio::test]
    async fn try_recv_response_returns_none_when_empty() {
        let mut d = Dispatcher::new(16);
        assert!(d.try_recv_response().is_none());
    }

    #[tokio::test]
    async fn try_recv_response_returns_message() {
        let mut d = Dispatcher::new(16);
        let tx = d.response_sender();
        let agent_id = AgentId(Uuid::new_v4());

        tx.send(AgentResponse::ShutdownComplete { agent_id: agent_id.clone() }).await.unwrap();

        let resp = d.try_recv_response();
        assert!(resp.is_some());
    }

    #[test]
    fn response_sender_clones_are_independent() {
        let d = Dispatcher::new(16);
        let tx1 = d.response_sender();
        let tx2 = d.response_sender();
        assert!(!tx1.is_closed());
        assert!(!tx2.is_closed());
    }

    #[test]
    fn dispatch_error_display() {
        let id = AgentId(Uuid::nil());
        let err = DispatchError::AgentNotFound(id.clone());
        assert!(err.to_string().contains("agent not found"));
        let err2 = DispatchError::ChannelClosed(id);
        assert!(err2.to_string().contains("channel closed"));
    }
}
