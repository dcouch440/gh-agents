//! Shared application state for HTTP handlers

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

use crate::agents::{AgentPool, AgentResponse, ClusterManager, Dispatcher, RoleManager};
use crate::db::pg_repo::PgRepo;
use crate::db::traits::{ServerRepo, UserRepo};
use crate::llm::AnthropicClient;
use crate::orchestration::Scheduler;
use crate::types::{AgentPoolConfig, AppConfig, UserId};

use super::ws::{AgentUpdate, FeedUpdate, TaskUpdate};

/// Message sent to the orchestrator
#[derive(Debug, Clone)]
pub struct OrchestratorMessage {
    pub id: Uuid,
    pub user_id: UserId,
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
    /// User repository for authentication operations (None in legacy/test mode)
    pub user_repo: Option<Arc<dyn UserRepo>>,
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
    /// Agent pool for managing agents (None in tests that don't need agents)
    pub pool: Option<Arc<tokio::sync::Mutex<AgentPool>>>,
    /// Dispatcher for routing commands to agents (None in tests)
    pub dispatcher: Option<Arc<tokio::sync::Mutex<Dispatcher>>>,
    /// Task results from agents, keyed by task_id
    pub task_results: Arc<RwLock<HashMap<Uuid, AgentResponse>>>,
    /// Role manager for building role-aware agent context
    pub role_manager: Option<Arc<RoleManager>>,
    /// Cluster manager for agent grouping
    pub cluster_manager: Arc<RwLock<ClusterManager>>,
}

impl AppState {
    /// Create new application state, returning the orchestrator receiver separately
    /// so it can be passed to the orchestrator consumer task.
    ///
    /// Loads persisted agents and clusters from the database on startup.
    pub async fn new(
        db: PgPool,
        scheduler: Arc<RwLock<Scheduler>>,
        config: AppConfig,
    ) -> (Self, mpsc::Receiver<OrchestratorMessage>) {
        let repo: Arc<dyn ServerRepo> = Arc::new(PgRepo::new(db.clone()));
        let user_repo: Arc<dyn UserRepo> = Arc::new(PgRepo::new(db.clone()));
        let (mut state, rx) = Self::with_repo(Some(db), repo, Some(scheduler), config);
        state.user_repo = Some(user_repo);

        // Initialize role manager with current working directory as project root
        let project_root = std::env::current_dir().unwrap_or_default();
        state.role_manager = Some(Arc::new(RoleManager::new(project_root)));

        // Initialize agent pool + dispatcher if API key is available
        if let Ok(provider) = AnthropicClient::from_env() {
            let provider = Arc::new(provider);
            let mut pool = AgentPool::new(AgentPoolConfig::default(), provider);
            let mut dispatcher = Dispatcher::new(64);

            // Reconstruct agents from DB
            let legacy_user = UserId(uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap());
            if let Ok(agent_rows) = state.repo.list_persisted_agents(legacy_user).await {
                for row in agent_rows {
                    let tier = match row.tier.as_str() {
                        "orchestrator" => crate::types::AgentTier::Orchestrator,
                        "utility" => crate::types::AgentTier::Utility,
                        _ => crate::types::AgentTier::Worker,
                    };
                    let persona = crate::types::AgentPersona {
                        name: row.persona_name.clone(),
                        ..Default::default()
                    };
                    match pool.spawn_agent_with_dispatcher(tier, persona, crate::types::ModelConfig::default(), &mut dispatcher) {
                        Ok(id) => tracing::info!("Restored agent {} ({})", row.persona_name, id.0),
                        Err(e) => tracing::warn!("Failed to restore agent {}: {}", row.persona_name, e),
                    }
                }
            }

            state.pool = Some(Arc::new(tokio::sync::Mutex::new(pool)));
            state.dispatcher = Some(Arc::new(tokio::sync::Mutex::new(dispatcher)));

            // Reconstruct clusters from DB
            if let Ok(cluster_rows) = state.repo.list_persisted_clusters(legacy_user).await {
                let mut mgr = state.cluster_manager.write().await;
                for row in cluster_rows {
                    let cid = crate::agents::ClusterId(row.id);
                    mgr.create_cluster_with_id(cid, row.name.clone(), row.description.clone());
                    tracing::info!("Restored cluster {} ({})", row.name, row.id);
                    if let Ok(members) = state.repo.list_cluster_members(row.id).await {
                        for agent_uuid in members {
                            let _ = mgr.add_agent(cid, crate::agents::AgentId(agent_uuid));
                        }
                    }
                }
            }
        }

        (state, rx)
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
                user_repo: None,
                scheduler,
                config: Arc::new(config),
                jwt_secret,
                orchestrator_tx,
                response_streams: Arc::new(RwLock::new(HashMap::new())),
                feed_tx,
                task_tx,
                agent_tx,
                pool: None,
                dispatcher: None,
                task_results: Arc::new(RwLock::new(HashMap::new())),
                role_manager: None,
                cluster_manager: Arc::new(RwLock::new(ClusterManager::new())),
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
            user_id: UserId::new(),
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
            user_id: None,
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
            user_id: None,
        });
    }

    #[test]
    fn broadcast_agent_no_panic() {
        let state = make_state();
        state.broadcast_agent(AgentUpdate {
            id: "agent-1".into(),
            status: "idle".into(),
            current_task: None,
            user_id: None,
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
