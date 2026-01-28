//! Shared application state for HTTP handlers

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

use crate::orchestration::Scheduler;
use crate::types::AppConfig;

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
    /// Channel to send messages to the orchestrator
    pub orchestrator_tx: mpsc::Sender<OrchestratorMessage>,
    /// Receiver for orchestrator messages (for the orchestrator to consume)
    orchestrator_rx: Arc<RwLock<mpsc::Receiver<OrchestratorMessage>>>,
    /// Map of message IDs to response broadcast senders
    response_streams: Arc<RwLock<HashMap<Uuid, broadcast::Sender<StreamChunk>>>>,
}

impl AppState {
    /// Create new application state
    pub fn new(db: SqlitePool, scheduler: Arc<RwLock<Scheduler>>, config: AppConfig) -> Self {
        let (orchestrator_tx, orchestrator_rx) = mpsc::channel(100);

        Self {
            db,
            scheduler,
            config: Arc::new(config),
            orchestrator_tx,
            orchestrator_rx: Arc::new(RwLock::new(orchestrator_rx)),
            response_streams: Arc::new(RwLock::new(HashMap::new())),
        }
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
