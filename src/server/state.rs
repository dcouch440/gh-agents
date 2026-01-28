//! Shared application state for HTTP handlers

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

use crate::orchestration::Scheduler;
use crate::types::AppConfig;

use super::ws::{AgentUpdate, FeedUpdate, TaskUpdate};

/// Message sent to the orchestrator
#[derive(Debug, Clone)]
pub struct OrchestratorMessage {
    pub id: Uuid,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

/// Chunk of a streaming response
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// A token of text
    Token(String),
    /// Stream completed successfully
    Done,
    /// Stream error
    Error(String),
}

/// Application state shared across all HTTP handlers
#[derive(Clone)]
pub struct AppState {
    /// Database connection pool
    pub db: SqlitePool,
    /// Task scheduler for orchestration
    pub scheduler: Arc<RwLock<Scheduler>>,
    /// Application configuration
    pub config: Arc<AppConfig>,
    /// JWT secret for token signing
    pub jwt_secret: Vec<u8>,
    /// Channel to send messages to the orchestrator
    pub orchestrator_tx: mpsc::Sender<OrchestratorMessage>,
    /// Receiver for orchestrator messages (for the orchestrator to consume)
    orchestrator_rx: Arc<RwLock<mpsc::Receiver<OrchestratorMessage>>>,
    /// Map of message IDs to response broadcast senders
    response_streams: Arc<RwLock<HashMap<Uuid, broadcast::Sender<StreamChunk>>>>,
    /// Broadcast channel for feed updates
    pub feed_tx: broadcast::Sender<FeedUpdate>,
    /// Broadcast channel for task updates
    pub task_tx: broadcast::Sender<TaskUpdate>,
    /// Broadcast channel for agent updates
    pub agent_tx: broadcast::Sender<AgentUpdate>,
}

impl AppState {
    /// Create new application state
    pub fn new(db: SqlitePool, scheduler: Arc<RwLock<Scheduler>>, config: AppConfig) -> Self {
        let (orchestrator_tx, orchestrator_rx) = mpsc::channel(100);
        let (feed_tx, _) = broadcast::channel(100);
        let (task_tx, _) = broadcast::channel(100);
        let (agent_tx, _) = broadcast::channel(100);

        // Generate a random JWT secret
        // In production, this should be persisted or configured via environment variable
        let jwt_secret = rand::random::<[u8; 32]>().to_vec();

        Self {
            db,
            scheduler,
            config: Arc::new(config),
            jwt_secret,
            orchestrator_tx,
            orchestrator_rx: Arc::new(RwLock::new(orchestrator_rx)),
            response_streams: Arc::new(RwLock::new(HashMap::new())),
            feed_tx,
            task_tx,
            agent_tx,
        }
    }

    /// Subscribe to feed updates
    pub fn subscribe_feed(&self) -> broadcast::Receiver<FeedUpdate> {
        self.feed_tx.subscribe()
    }

    /// Subscribe to task updates
    pub fn subscribe_tasks(&self) -> broadcast::Receiver<TaskUpdate> {
        self.task_tx.subscribe()
    }

    /// Subscribe to agent updates
    pub fn subscribe_agents(&self) -> broadcast::Receiver<AgentUpdate> {
        self.agent_tx.subscribe()
    }

    /// Broadcast a feed update to all subscribers
    pub fn broadcast_feed(&self, update: FeedUpdate) {
        let _ = self.feed_tx.send(update);
    }

    /// Broadcast a task update to all subscribers
    pub fn broadcast_task(&self, update: TaskUpdate) {
        let _ = self.task_tx.send(update);
    }

    /// Broadcast an agent update to all subscribers
    pub fn broadcast_agent(&self, update: AgentUpdate) {
        let _ = self.agent_tx.send(update);
    }

    /// Get a receiver for streaming responses for a specific message
    ///
    /// Creates a new broadcast channel for this message if one doesn't exist.
    pub async fn get_response_stream(&self, message_id: Uuid) -> broadcast::Receiver<StreamChunk> {
        let mut streams = self.response_streams.write().await;

        if let Some(tx) = streams.get(&message_id) {
            tx.subscribe()
        } else {
            // Create a new broadcast channel with buffer for 100 chunks
            let (tx, rx) = broadcast::channel(100);
            streams.insert(message_id, tx);
            rx
        }
    }

    /// Send a chunk to a message's response stream
    ///
    /// Returns false if no stream exists for this message.
    pub async fn send_stream_chunk(&self, message_id: Uuid, chunk: StreamChunk) -> bool {
        let streams = self.response_streams.read().await;

        if let Some(tx) = streams.get(&message_id) {
            // Send to all subscribers, ignore if no receivers
            let _ = tx.send(chunk);
            true
        } else {
            false
        }
    }

    /// Remove a response stream (call when streaming is complete)
    pub async fn remove_response_stream(&self, message_id: Uuid) {
        let mut streams = self.response_streams.write().await;
        streams.remove(&message_id);
    }

    /// Get access to the orchestrator message receiver
    ///
    /// Note: In practice, the orchestrator would be initialized with the receiver directly.
    /// This method is provided for potential future use cases.
    #[allow(dead_code)]
    pub async fn take_orchestrator_rx(&self) -> Option<mpsc::Receiver<OrchestratorMessage>> {
        // This is a one-time operation - the orchestrator takes ownership
        let _rx_guard = self.orchestrator_rx.write().await;
        // We can't actually take it since it's behind RwLock, but this signals intent
        // In practice, the orchestrator would be initialized with the receiver directly
        None
    }
}
