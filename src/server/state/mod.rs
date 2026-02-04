//! Shared application state for HTTP handlers

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::agents::ToolClusterIndex;
use crate::db::pg_repo::PgRepo;
use crate::db::traits::{
    AgentExecutionRepo, ContextStoreRepo, DocumentRepo, OutputSchemaRepo, PromptTemplateRepo,
    ResultRepo, RoomRepo, RouterRequestRepo, ServerRepo, TokenLedgerRepo, ToolRouterRepo, UserRepo,
    WorkflowRepo,
};
use crate::llm::AnthropicClient;
use crate::types::{AppConfig, UserId};

use super::hub::PromptRegistry;
use super::ws::{
    AgentUpdate, FeedUpdate, PipelineUpdate, RoomUpdateEvent, RoutingUpdate, SessionUpdate,
    TaskUpdate,
};

/// Message sent to the chat consumer
#[derive(Debug, Clone)]
pub struct ConsumerMessage {
    pub id: Uuid,
    pub user_id: UserId,
    pub session_id: Option<Uuid>,
    pub agent_id: Option<Uuid>,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

/// Chunk of a streaming response
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// A token of text
    Token(String),
    /// A tool has started executing
    ToolStart { name: String, tool_id: String },
    /// A tool has finished executing
    ToolEnd { name: String, tool_id: String },
    /// A document was created or updated
    DocUpdate { doc_id: String, title: String },
    /// Stream completed successfully
    Done,
    /// Stream error
    Error(String),
}

/// A buffered broadcast stream that retains all chunks so late-connecting
/// SSE clients can replay missed tokens.
struct BufferedStream {
    tx: broadcast::Sender<StreamChunk>,
    buffer: Vec<StreamChunk>,
    done: bool,
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
    /// Document repository for document CRUD operations (None in legacy/test mode)
    pub doc_repo: Option<Arc<dyn DocumentRepo>>,
    /// Output schema repository (None in legacy/test mode)
    pub output_schema_repo: Option<Arc<dyn OutputSchemaRepo>>,
    /// Prompt template repository (None in legacy/test mode)
    pub prompt_template_repo: Option<Arc<dyn PromptTemplateRepo>>,
    /// Workflow repository (None in legacy/test mode)
    pub workflow_repo: Option<Arc<dyn WorkflowRepo>>,
    pub agent_execution_repo: Option<Arc<dyn AgentExecutionRepo>>,
    pub token_ledger_repo: Option<Arc<dyn TokenLedgerRepo>>,
    pub result_repo: Option<Arc<dyn ResultRepo>>,
    /// Tool router repository (None in legacy/test mode)
    pub tool_router_repo: Option<Arc<dyn ToolRouterRepo>>,
    /// Context store repository (None in legacy/test mode)
    pub context_store_repo: Option<Arc<dyn ContextStoreRepo>>,
    /// Router request repository (None in legacy/test mode)
    pub router_request_repo: Option<Arc<dyn RouterRequestRepo>>,
    /// Room repository for agent room management (None in legacy/test mode)
    pub room_repo: Option<Arc<dyn RoomRepo>>,
    /// Application configuration (mutable at runtime via API)
    pub config: Arc<RwLock<AppConfig>>,
    /// JWT secret for token signing
    pub jwt_secret: Vec<u8>,
    /// Channel to send messages to the orchestrator
    pub chat_tx: mpsc::Sender<ConsumerMessage>,
    /// Map of message IDs to buffered response streams
    response_streams: Arc<RwLock<HashMap<Uuid, Arc<RwLock<BufferedStream>>>>>,
    /// Broadcast channel for feed updates
    pub feed_tx: broadcast::Sender<FeedUpdate>,
    /// Broadcast channel for task updates
    pub task_tx: broadcast::Sender<TaskUpdate>,
    /// Broadcast channel for agent updates
    pub agent_tx: broadcast::Sender<AgentUpdate>,
    /// Broadcast channel for session updates
    pub session_tx: broadcast::Sender<SessionUpdate>,
    /// Broadcast channel for pipeline execution updates
    pub pipeline_tx: broadcast::Sender<PipelineUpdate>,
    /// Broadcast channel for tool routing updates
    pub routing_tx: broadcast::Sender<RoutingUpdate>,
    /// Broadcast channel for router request lifecycle events
    pub router_request_tx: broadcast::Sender<super::ws::RouterRequestEvent>,
    /// Broadcast channel for context store updates
    pub context_update_tx: broadcast::Sender<super::ws::ContextUpdateEvent>,
    /// Broadcast channel for room events
    pub room_update_tx: broadcast::Sender<RoomUpdateEvent>,
    /// Default agent UUID (looked up at startup, agent with name "Home")
    pub default_agent_id: Option<Uuid>,
    /// Tool-to-cluster index for routing tool calls to cluster agents
    pub cluster_index: Option<Arc<ToolClusterIndex>>,
    /// Prompt registry for core system/agent prompts loaded from prompts/ directory
    pub prompt_registry: Arc<PromptRegistry>,
    /// Cancellation tokens for running pipelines and agent executions
    pub cancellation_tokens: Arc<RwLock<HashMap<Uuid, CancellationToken>>>,
}

impl AppState {
    /// Create new application state, returning the orchestrator receiver separately
    /// so it can be passed to the orchestrator consumer task.
    ///
    /// Loads persisted agents and clusters from the database on startup.
    pub async fn new(db: PgPool, config: AppConfig) -> (Self, mpsc::Receiver<ConsumerMessage>) {
        let repo: Arc<dyn ServerRepo> = Arc::new(PgRepo::new(db.clone()));
        let user_repo: Arc<dyn UserRepo> = Arc::new(PgRepo::new(db.clone()));
        let doc_repo: Arc<dyn DocumentRepo> = Arc::new(PgRepo::new(db.clone()));
        let output_schema_repo: Arc<dyn OutputSchemaRepo> = Arc::new(PgRepo::new(db.clone()));
        let prompt_template_repo: Arc<dyn PromptTemplateRepo> = Arc::new(PgRepo::new(db.clone()));
        let workflow_repo: Arc<dyn WorkflowRepo> = Arc::new(PgRepo::new(db.clone()));
        let agent_execution_repo: Arc<dyn AgentExecutionRepo> = Arc::new(PgRepo::new(db.clone()));
        let token_ledger_repo: Arc<dyn TokenLedgerRepo> = Arc::new(PgRepo::new(db.clone()));
        let result_repo: Arc<dyn ResultRepo> = Arc::new(PgRepo::new(db.clone()));
        let tool_router_repo: Arc<dyn ToolRouterRepo> = Arc::new(PgRepo::new(db.clone()));
        let context_store_repo: Arc<dyn ContextStoreRepo> = Arc::new(PgRepo::new(db.clone()));
        let router_request_repo: Arc<dyn RouterRequestRepo> = Arc::new(PgRepo::new(db.clone()));
        let room_repo: Arc<dyn RoomRepo> = Arc::new(PgRepo::new(db.clone()));
        let (mut state, rx) = Self::with_repo(Some(db), repo, config.clone());

        // Load prompt registry from prompts/ directory
        let prompts_dir = std::env::current_dir().unwrap_or_default().join("prompts");
        match PromptRegistry::load_from_dir(&prompts_dir) {
            Ok(registry) => {
                tracing::info!(
                    "Loaded {} prompts from {}",
                    registry.len(),
                    prompts_dir.display()
                );
                state.prompt_registry = Arc::new(registry);
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to load prompts from {}: {} — using empty registry",
                    prompts_dir.display(),
                    e
                );
            }
        }
        state.user_repo = Some(user_repo);
        state.doc_repo = Some(doc_repo);
        state.output_schema_repo = Some(output_schema_repo);
        state.prompt_template_repo = Some(prompt_template_repo);
        state.workflow_repo = Some(workflow_repo);
        state.agent_execution_repo = Some(agent_execution_repo);
        state.token_ledger_repo = Some(token_ledger_repo);
        state.result_repo = Some(result_repo);
        state.tool_router_repo = Some(tool_router_repo);
        state.context_store_repo = Some(context_store_repo);
        state.router_request_repo = Some(router_request_repo);
        state.room_repo = Some(room_repo);

        // Look up default agent from DB (for workflow system)
        let legacy_user =
            UserId(uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap());
        if let Ok(agent_rows) = state.repo.list_persisted_agents(legacy_user).await {
            // Look up default agent (name = "Home")
            if let Some(home) = agent_rows
                .iter()
                .find(|r| r.name.eq_ignore_ascii_case("home"))
            {
                tracing::info!("Default agent: {} ({})", home.name, home.id);
                state.default_agent_id = Some(home.id);
            }
        }

        // Build tool-to-cluster index for routing (if API key available)
        if AnthropicClient::from_env().is_ok() {
            // Build tool-to-cluster index for routing
            match crate::db::list_clusters_with_tools(state.db.as_ref().unwrap()).await {
                Ok(pairs) => {
                    let tool_count: usize = pairs.iter().map(|(_, tools)| tools.len()).sum();
                    tracing::info!(
                        "Built ToolClusterIndex: {} clusters, {} tools",
                        pairs.len(),
                        tool_count
                    );
                    state.cluster_index = Some(Arc::new(ToolClusterIndex::new(pairs)));
                }
                Err(e) => {
                    tracing::warn!("Failed to build ToolClusterIndex: {}", e);
                }
            }

            // LEGACY: Pipeline reconstruction removed (workflows replaced pipelines)
        }

        (state, rx)
    }

    /// Create application state with a custom repo (for testing).
    /// Returns the state and the orchestrator message receiver.
    pub fn with_repo(
        db: Option<PgPool>,
        repo: Arc<dyn ServerRepo>,
        config: AppConfig,
    ) -> (Self, mpsc::Receiver<ConsumerMessage>) {
        let (chat_tx, orchestrator_rx) = mpsc::channel(crate::constants::CHANNEL_ORCHESTRATOR);
        let (feed_tx, _) = broadcast::channel(crate::constants::CHANNEL_BROADCAST_HIGH);
        let (task_tx, _) = broadcast::channel(crate::constants::CHANNEL_BROADCAST);
        let (agent_tx, _) = broadcast::channel(crate::constants::CHANNEL_BROADCAST_LOW);
        let (session_tx, _) = broadcast::channel(crate::constants::CHANNEL_BROADCAST_LOW);
        let (pipeline_tx, _) = broadcast::channel(crate::constants::CHANNEL_BROADCAST);
        let (routing_tx, _) = broadcast::channel(crate::constants::CHANNEL_BROADCAST_HIGH);
        let (router_request_tx, _) = broadcast::channel(crate::constants::CHANNEL_BROADCAST);
        let (context_update_tx, _) = broadcast::channel(crate::constants::CHANNEL_BROADCAST_LOW);
        let (room_update_tx, _) = broadcast::channel(crate::constants::CHANNEL_BROADCAST);

        // JWT secret: require via env var, fall back to random for dev only
        let jwt_secret = match std::env::var(crate::constants::ENV_JWT_SECRET) {
            Ok(s) if !s.is_empty() => {
                tracing::info!(
                    "{} loaded from environment",
                    crate::constants::ENV_JWT_SECRET
                );
                s.into_bytes()
            }
            _ => {
                let is_production = std::env::var(crate::constants::ENV_RUST_ENV)
                    .map(|v| v.eq_ignore_ascii_case("production"))
                    .unwrap_or(false);
                if is_production {
                    panic!(
                        "{} must be set in production ({}=production)",
                        crate::constants::ENV_JWT_SECRET,
                        crate::constants::ENV_RUST_ENV
                    );
                }
                tracing::warn!(
                    "{} not set — using random secret. Tokens will not survive restarts.",
                    crate::constants::ENV_JWT_SECRET
                );
                rand::random::<[u8; 32]>().to_vec()
            }
        };

        (
            Self {
                db,
                repo,
                user_repo: None,
                doc_repo: None,
                output_schema_repo: None,
                prompt_template_repo: None,
                workflow_repo: None,
                agent_execution_repo: None,
                token_ledger_repo: None,
                result_repo: None,
                tool_router_repo: None,
                context_store_repo: None,
                router_request_repo: None,
                room_repo: None,
                config: Arc::new(RwLock::new(config)),
                jwt_secret,
                chat_tx,
                response_streams: Arc::new(RwLock::new(HashMap::new())),
                feed_tx,
                task_tx,
                agent_tx,
                session_tx,
                pipeline_tx,
                routing_tx,
                router_request_tx,
                context_update_tx,
                room_update_tx,
                default_agent_id: None,
                cluster_index: None,
                prompt_registry: Arc::new(PromptRegistry::empty()),
                cancellation_tokens: Arc::new(RwLock::new(HashMap::new())),
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

    /// Subscribe to session updates
    pub fn subscribe_sessions(&self) -> broadcast::Receiver<SessionUpdate> {
        self.session_tx.subscribe()
    }

    /// Broadcast a session update to all subscribers
    pub fn broadcast_session(&self, update: SessionUpdate) {
        let _ = self.session_tx.send(update);
    }

    /// Subscribe to pipeline execution updates
    pub fn subscribe_pipelines(&self) -> broadcast::Receiver<PipelineUpdate> {
        self.pipeline_tx.subscribe()
    }

    /// Broadcast a pipeline execution update to all subscribers
    pub fn broadcast_pipeline(&self, update: PipelineUpdate) {
        let _ = self.pipeline_tx.send(update);
    }

    /// Subscribe to routing updates
    pub fn subscribe_routing(&self) -> broadcast::Receiver<RoutingUpdate> {
        self.routing_tx.subscribe()
    }

    /// Broadcast a routing update to all subscribers
    pub fn broadcast_routing(&self, update: RoutingUpdate) {
        let _ = self.routing_tx.send(update);
    }

    /// Subscribe to router request lifecycle events
    pub fn subscribe_router_requests(&self) -> broadcast::Receiver<super::ws::RouterRequestEvent> {
        self.router_request_tx.subscribe()
    }

    /// Broadcast a router request event
    pub fn broadcast_router_request(&self, event: super::ws::RouterRequestEvent) {
        let _ = self.router_request_tx.send(event);
    }

    /// Subscribe to context store updates
    pub fn subscribe_context_updates(&self) -> broadcast::Receiver<super::ws::ContextUpdateEvent> {
        self.context_update_tx.subscribe()
    }

    /// Broadcast a context update event
    pub fn broadcast_context_update(&self, event: super::ws::ContextUpdateEvent) {
        let _ = self.context_update_tx.send(event);
    }

    /// Subscribe to room events
    pub fn subscribe_room_updates(&self) -> broadcast::Receiver<RoomUpdateEvent> {
        self.room_update_tx.subscribe()
    }

    /// Broadcast a room event
    pub fn broadcast_room_update(&self, event: RoomUpdateEvent) {
        let _ = self.room_update_tx.send(event);
    }

    /// Ensure a response stream exists for this message (creates if missing).
    ///
    /// Call this before queuing work to the orchestrator so the broadcast
    /// channel exists when tokens start arriving.
    pub async fn ensure_response_stream(&self, message_id: Uuid) {
        let mut streams = self.response_streams.write().await;
        streams.entry(message_id).or_insert_with(|| {
            let (tx, _) = broadcast::channel(100);
            Arc::new(RwLock::new(BufferedStream {
                tx,
                buffer: Vec::new(),
                done: false,
            }))
        });
    }

    /// Get the buffered chunks, a live receiver, and whether the stream is done.
    ///
    /// The caller should replay the buffer first, then listen on the receiver.
    /// Holding the inner read lock while snapshotting + subscribing guarantees
    /// no chunks are missed or duplicated.
    pub async fn get_response_stream(
        &self,
        message_id: Uuid,
    ) -> (Vec<StreamChunk>, broadcast::Receiver<StreamChunk>, bool) {
        let mut streams = self.response_streams.write().await;
        let entry = streams.entry(message_id).or_insert_with(|| {
            let (tx, _) = broadcast::channel(100);
            Arc::new(RwLock::new(BufferedStream {
                tx,
                buffer: Vec::new(),
                done: false,
            }))
        });
        let inner = entry.read().await;
        let rx = inner.tx.subscribe();
        (inner.buffer.clone(), rx, inner.done)
    }

    /// Send a chunk to a message's response stream
    ///
    /// Appends to the buffer and broadcasts. Returns false if no stream exists.
    pub async fn send_stream_chunk(&self, message_id: Uuid, chunk: StreamChunk) -> bool {
        let streams = self.response_streams.read().await;

        if let Some(entry) = streams.get(&message_id) {
            let mut inner = entry.write().await;
            if matches!(&chunk, StreamChunk::Done) {
                inner.done = true;
            }
            inner.buffer.push(chunk.clone());
            let _ = inner.tx.send(chunk);
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

    /// Register a cancellation token for a running execution (pipeline run or agent execution).
    /// Returns a clone of the token to pass through the execution chain.
    pub async fn register_cancellation(&self, id: Uuid) -> CancellationToken {
        let token = CancellationToken::new();
        let mut tokens = self.cancellation_tokens.write().await;
        tokens.insert(id, token.clone());
        token
    }

    /// Create a child cancellation token linked to a parent.
    /// Cancelling the parent automatically cancels all children.
    pub async fn register_child_cancellation(
        &self,
        id: Uuid,
        parent: &CancellationToken,
    ) -> CancellationToken {
        let child = parent.child_token();
        let mut tokens = self.cancellation_tokens.write().await;
        tokens.insert(id, child.clone());
        child
    }

    /// Cancel a running execution by its ID. Returns true if the token existed.
    pub async fn cancel_execution(&self, id: Uuid) -> bool {
        let tokens = self.cancellation_tokens.read().await;
        if let Some(token) = tokens.get(&id) {
            token.cancel();
            true
        } else {
            false
        }
    }

    /// Remove a cancellation token after execution completes.
    pub async fn remove_cancellation(&self, id: Uuid) {
        let mut tokens = self.cancellation_tokens.write().await;
        tokens.remove(&id);
    }
}

#[cfg(test)]
mod tests;
