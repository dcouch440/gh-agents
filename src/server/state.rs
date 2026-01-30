//! Shared application state for HTTP handlers

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

use crate::db::pg_repo::PgRepo;
use crate::db::traits::ServerRepo;
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
    /// Database connection pool (used by Scheduler; None in mock-based tests)
    pub db: Option<PgPool>,
    /// Repository trait object for DB operations used by API handlers
    pub repo: Arc<dyn ServerRepo>,
    /// Task scheduler for orchestration (None in mock-based tests)
    pub scheduler: Option<Arc<RwLock<Scheduler>>>,
    /// Application configuration
    pub config: Arc<AppConfig>,
    /// JWT secret for token signing
    pub jwt_secret: Vec<u8>,
    /// Channel to send messages to the orchestrator
    pub orchestrator_tx: mpsc::Sender<OrchestratorMessage>,
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
    /// Create new application state, returning the orchestrator receiver separately
    /// so it can be passed to the orchestrator consumer task.
    pub fn new(
        db: PgPool,
        scheduler: Arc<RwLock<Scheduler>>,
        config: AppConfig,
    ) -> (Self, mpsc::Receiver<OrchestratorMessage>) {
        let repo: Arc<dyn ServerRepo> = Arc::new(PgRepo::new(db.clone()));
        Self::with_repo(Some(db), repo, Some(scheduler), config)
    }

    /// Create application state with a custom repo (for testing).
    /// Returns the state and the orchestrator message receiver.
    pub fn with_repo(
        db: Option<PgPool>,
        repo: Arc<dyn ServerRepo>,
        scheduler: Option<Arc<RwLock<Scheduler>>>,
        config: AppConfig,
    ) -> (Self, mpsc::Receiver<OrchestratorMessage>) {
        let (orchestrator_tx, orchestrator_rx) = mpsc::channel(100);
        let (feed_tx, _) = broadcast::channel(100);
        let (task_tx, _) = broadcast::channel(100);
        let (agent_tx, _) = broadcast::channel(100);

        // Generate a random JWT secret
        // In production, this should be persisted or configured via environment variable
        let jwt_secret = rand::random::<[u8; 32]>().to_vec();

        (
            Self {
                db,
                repo,
                scheduler,
                config: Arc::new(config),
                jwt_secret,
                orchestrator_tx,
                response_streams: Arc::new(RwLock::new(HashMap::new())),
                feed_tx,
                task_tx,
                agent_tx,
            },
            orchestrator_rx,
        )
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

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::traits::MockServerRepo;

    fn make_state() -> AppState {
        let mut mock = MockServerRepo::new();
        mock.expect_health_check().returning(|| true);
        let repo: Arc<dyn ServerRepo> = Arc::new(mock);
        let (state, _rx) = AppState::with_repo(None, repo, None, AppConfig::default());
        state
    }

    #[test]
    fn stream_chunk_variants() {
        let token = StreamChunk::Token("hello".into());
        let done = StreamChunk::Done;
        let err = StreamChunk::Error("oops".into());
        match token {
            StreamChunk::Token(s) => assert_eq!(s, "hello"),
            _ => panic!(),
        }
        assert!(matches!(done, StreamChunk::Done));
        match err {
            StreamChunk::Error(s) => assert_eq!(s, "oops"),
            _ => panic!(),
        }
    }

    #[test]
    fn orchestrator_message_construction() {
        let msg = OrchestratorMessage {
            id: Uuid::new_v4(),
            content: "do stuff".into(),
            timestamp: Utc::now(),
        };
        assert_eq!(msg.content, "do stuff");
    }

    #[test]
    fn app_state_new_creates_valid_state() {
        let state = make_state();
        assert_eq!(state.jwt_secret.len(), 32);
    }

    #[test]
    fn subscribe_feed_returns_receiver() {
        let state = make_state();
        let _rx = state.subscribe_feed();
    }

    #[test]
    fn subscribe_tasks_returns_receiver() {
        let state = make_state();
        let _rx = state.subscribe_tasks();
    }

    #[test]
    fn subscribe_agents_returns_receiver() {
        let state = make_state();
        let _rx = state.subscribe_agents();
    }

    #[test]
    fn broadcast_feed_no_panic() {
        let state = make_state();
        state.broadcast_feed(FeedUpdate {
            id: Uuid::new_v4(),
            agent_id: "a".into(),
            content: "c".into(),
            item_type: "info".into(),
            timestamp: Utc::now(),
        });
    }

    #[test]
    fn broadcast_task_no_panic() {
        let state = make_state();
        state.broadcast_task(TaskUpdate {
            id: Uuid::new_v4(),
            status: "pending".into(),
            progress: None,
            assigned_agent: None,
        });
    }

    #[test]
    fn broadcast_agent_no_panic() {
        let state = make_state();
        state.broadcast_agent(AgentUpdate {
            id: "agent-1".into(),
            status: "idle".into(),
            current_task: None,
        });
    }

    #[tokio::test]
    async fn get_response_stream_creates_new() {
        let state = make_state();
        let msg_id = Uuid::new_v4();
        let _rx = state.get_response_stream(msg_id).await;
    }

    #[tokio::test]
    async fn get_response_stream_returns_existing() {
        let state = make_state();
        let msg_id = Uuid::new_v4();
        let _rx1 = state.get_response_stream(msg_id).await;
        let _rx2 = state.get_response_stream(msg_id).await;
    }

    #[tokio::test]
    async fn send_stream_chunk_no_stream() {
        let state = make_state();
        let result = state
            .send_stream_chunk(Uuid::new_v4(), StreamChunk::Token("hi".into()))
            .await;
        assert!(!result);
    }

    #[tokio::test]
    async fn send_stream_chunk_with_stream() {
        let state = make_state();
        let msg_id = Uuid::new_v4();
        let _rx = state.get_response_stream(msg_id).await;
        let result = state
            .send_stream_chunk(msg_id, StreamChunk::Token("hi".into()))
            .await;
        assert!(result);
    }

    #[tokio::test]
    async fn remove_response_stream() {
        let state = make_state();
        let msg_id = Uuid::new_v4();
        let _rx = state.get_response_stream(msg_id).await;
        state.remove_response_stream(msg_id).await;
        let result = state.send_stream_chunk(msg_id, StreamChunk::Done).await;
        assert!(!result);
    }
}
