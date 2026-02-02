//! Shared application state for HTTP handlers

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

use crate::agents::{AgentPool, AgentResponse, ClusterManager, Dispatcher, PipelineManager, RoleManager, ScheduleManager, ToolClusterIndex};
use crate::db::pg_repo::PgRepo;
use crate::db::traits::{AgentExecutionRepo, ContextStoreRepo, DocumentRepo, OutputSchemaRepo, PipelineStageMemberRepo, PromptTemplateRepo, ResultRepo, RouterRequestRepo, ServerRepo, TokenLedgerRepo, ToolRouterRepo, UserRepo, WorkflowRepo};
use crate::llm::AnthropicClient;
use crate::types::{AgentPoolConfig, AppConfig, UserId};

use super::agent_mode::{AgentModeId, ModeRegistry};
use super::ws::{AgentUpdate, FeedUpdate, PipelineUpdate, RoutingUpdate, SessionUpdate, TaskUpdate};

/// Message sent to the orchestrator
#[derive(Debug, Clone)]
pub struct OrchestratorMessage {
    pub id: Uuid,
    pub user_id: UserId,
    pub session_id: Option<Uuid>,
    pub mode_id: AgentModeId,
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
    /// Pipeline stage member repository (None in legacy/test mode)
    pub stage_member_repo: Option<Arc<dyn PipelineStageMemberRepo>>,
    pub agent_execution_repo: Option<Arc<dyn AgentExecutionRepo>>,
    pub token_ledger_repo: Option<Arc<dyn TokenLedgerRepo>>,
    pub result_repo: Option<Arc<dyn ResultRepo>>,
    /// Tool router repository (None in legacy/test mode)
    pub tool_router_repo: Option<Arc<dyn ToolRouterRepo>>,
    /// Context store repository (None in legacy/test mode)
    pub context_store_repo: Option<Arc<dyn ContextStoreRepo>>,
    /// Router request repository (None in legacy/test mode)
    pub router_request_repo: Option<Arc<dyn RouterRequestRepo>>,
    /// Application configuration (mutable at runtime via API)
    pub config: Arc<RwLock<AppConfig>>,
    /// JWT secret for token signing
    pub jwt_secret: Vec<u8>,
    /// Channel to send messages to the orchestrator
    pub orchestrator_tx: mpsc::Sender<OrchestratorMessage>,
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
    /// Pipeline manager for chained agent workflows
    pub pipeline_manager: Arc<RwLock<PipelineManager>>,
    /// Schedule manager for cron-like and event-driven agent execution
    pub schedule_manager: Arc<RwLock<ScheduleManager>>,
    /// Registry of available agent modes
    pub mode_registry: Arc<ModeRegistry>,
    /// Tool-to-cluster index for routing tool calls to cluster agents
    pub cluster_index: Option<Arc<ToolClusterIndex>>,
}

impl AppState {
    /// Create new application state, returning the orchestrator receiver separately
    /// so it can be passed to the orchestrator consumer task.
    ///
    /// Loads persisted agents and clusters from the database on startup.
    pub async fn new(db: PgPool, config: AppConfig) -> (Self, mpsc::Receiver<OrchestratorMessage>) {
        let repo: Arc<dyn ServerRepo> = Arc::new(PgRepo::new(db.clone()));
        let user_repo: Arc<dyn UserRepo> = Arc::new(PgRepo::new(db.clone()));
        let doc_repo: Arc<dyn DocumentRepo> = Arc::new(PgRepo::new(db.clone()));
        let output_schema_repo: Arc<dyn OutputSchemaRepo> = Arc::new(PgRepo::new(db.clone()));
        let prompt_template_repo: Arc<dyn PromptTemplateRepo> = Arc::new(PgRepo::new(db.clone()));
        let workflow_repo: Arc<dyn WorkflowRepo> = Arc::new(PgRepo::new(db.clone()));
        let stage_member_repo: Arc<dyn PipelineStageMemberRepo> = Arc::new(PgRepo::new(db.clone()));
        let agent_execution_repo: Arc<dyn AgentExecutionRepo> = Arc::new(PgRepo::new(db.clone()));
        let token_ledger_repo: Arc<dyn TokenLedgerRepo> = Arc::new(PgRepo::new(db.clone()));
        let result_repo: Arc<dyn ResultRepo> = Arc::new(PgRepo::new(db.clone()));
        let tool_router_repo: Arc<dyn ToolRouterRepo> = Arc::new(PgRepo::new(db.clone()));
        let context_store_repo: Arc<dyn ContextStoreRepo> = Arc::new(PgRepo::new(db.clone()));
        let router_request_repo: Arc<dyn RouterRequestRepo> = Arc::new(PgRepo::new(db.clone()));
        let (mut state, rx) = Self::with_repo(Some(db), repo, config.clone());
        state.user_repo = Some(user_repo);
        state.doc_repo = Some(doc_repo);
        state.output_schema_repo = Some(output_schema_repo);
        state.prompt_template_repo = Some(prompt_template_repo);
        state.workflow_repo = Some(workflow_repo);
        state.stage_member_repo = Some(stage_member_repo);
        state.agent_execution_repo = Some(agent_execution_repo);
        state.token_ledger_repo = Some(token_ledger_repo);
        state.result_repo = Some(result_repo);
        state.tool_router_repo = Some(tool_router_repo);
        state.context_store_repo = Some(context_store_repo);
        state.router_request_repo = Some(router_request_repo);

        // Initialize role manager with current working directory as project root
        let project_root = std::env::current_dir().unwrap_or_default();
        state.role_manager = Some(Arc::new(RoleManager::new(project_root.clone())));

        // Initialize agent pool + dispatcher if API key is available
        if let Ok(provider) = AnthropicClient::from_env() {
            let provider = Arc::new(provider);
            let mut pool = AgentPool::new(AgentPoolConfig::default(), provider);
            let mut dispatcher = Dispatcher::new(64);

            // Reconstruct agents from DB
            let legacy_user = UserId(uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap());
            if let Ok(agent_rows) = state.repo.list_persisted_agents(legacy_user).await {
                for row in agent_rows {
                    let tier = match row.tier.as_deref().unwrap_or("worker") {
                        "orchestrator" => crate::types::AgentTier::Orchestrator,
                        "utility" => crate::types::AgentTier::Utility,
                        _ => crate::types::AgentTier::Worker,
                    };
                    let persona = crate::types::AgentPersona {
                        name: row.name.clone(),
                        ..Default::default()
                    };
                    match pool.spawn_agent_with_dispatcher(tier, persona, crate::types::ModelConfig::default(), &mut dispatcher) {
                        Ok(id) => tracing::info!("Restored agent {} ({})", row.name, id.0),
                        Err(e) => {
                            tracing::warn!("Failed to restore agent {}: {}", row.name, e)
                        }
                    }
                }
            }

            state.pool = Some(Arc::new(tokio::sync::Mutex::new(pool)));
            state.dispatcher = Some(Arc::new(tokio::sync::Mutex::new(dispatcher)));

            // Build tool-to-cluster index for routing
            match crate::db::list_clusters_with_tools(state.db.as_ref().unwrap()).await {
                Ok(pairs) => {
                    let tool_count: usize = pairs.iter().map(|(_, tools)| tools.len()).sum();
                    tracing::info!("Built ToolClusterIndex: {} clusters, {} tools", pairs.len(), tool_count);
                    state.cluster_index = Some(Arc::new(ToolClusterIndex::new(pairs)));
                }
                Err(e) => {
                    tracing::warn!("Failed to build ToolClusterIndex: {}", e);
                }
            }

            // Reconstruct pipelines from DB
            if let Ok(pipeline_rows) = state.repo.list_pipelines(legacy_user).await {
                let mut mgr = state.pipeline_manager.write().await;
                for row in pipeline_rows {
                    let pid = crate::agents::PipelineId(row.id);
                    mgr.create_pipeline_with_id(pid, row.name.clone());
                    tracing::info!("Restored pipeline {} ({})", row.name, row.id);
                    if let Ok(stages) = state.repo.list_pipeline_stages(row.id).await {
                        for stage in stages {
                            let _ = mgr.add_stage(
                                pid,
                                stage.agent_id.map(crate::agents::AgentId),
                                stage.cluster_id.map(crate::agents::ClusterId),
                                stage.role,
                                stage.approval_required.unwrap_or(false),
                                stage.fan_out.unwrap_or(false),
                                stage.stage_name,
                                stage.input_definitions.unwrap_or_else(|| serde_json::json!([])),
                                stage.output_description.unwrap_or_default(),
                                stage.output_schema.unwrap_or_else(|| serde_json::json!({"fields": []})),
                            );
                        }
                    }
                }
            }
        }

        (state, rx)
    }

    /// Create application state with a custom repo (for testing).
    /// Returns the state and the orchestrator message receiver.
    pub fn with_repo(db: Option<PgPool>, repo: Arc<dyn ServerRepo>, config: AppConfig) -> (Self, mpsc::Receiver<OrchestratorMessage>) {
        let (orchestrator_tx, orchestrator_rx) = mpsc::channel(crate::constants::CHANNEL_ORCHESTRATOR);
        let (feed_tx, _) = broadcast::channel(crate::constants::CHANNEL_BROADCAST_HIGH);
        let (task_tx, _) = broadcast::channel(crate::constants::CHANNEL_BROADCAST);
        let (agent_tx, _) = broadcast::channel(crate::constants::CHANNEL_BROADCAST_LOW);
        let (session_tx, _) = broadcast::channel(crate::constants::CHANNEL_BROADCAST_LOW);
        let (pipeline_tx, _) = broadcast::channel(crate::constants::CHANNEL_BROADCAST);
        let (routing_tx, _) = broadcast::channel(crate::constants::CHANNEL_BROADCAST_HIGH);

        // JWT secret: require via env var, fall back to random for dev only
        let jwt_secret = match std::env::var(crate::constants::ENV_JWT_SECRET) {
            Ok(s) if !s.is_empty() => {
                tracing::info!("{} loaded from environment", crate::constants::ENV_JWT_SECRET);
                s.into_bytes()
            }
            _ => {
                let is_production = std::env::var(crate::constants::ENV_RUST_ENV).map(|v| v.eq_ignore_ascii_case("production")).unwrap_or(false);
                if is_production {
                    panic!("{} must be set in production ({}=production)", crate::constants::ENV_JWT_SECRET, crate::constants::ENV_RUST_ENV);
                }
                tracing::warn!("{} not set — using random secret. Tokens will not survive restarts.", crate::constants::ENV_JWT_SECRET);
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
                stage_member_repo: None,
                agent_execution_repo: None,
                token_ledger_repo: None,
                result_repo: None,
                tool_router_repo: None,
                context_store_repo: None,
                router_request_repo: None,
                config: Arc::new(RwLock::new(config)),
                jwt_secret,
                orchestrator_tx,
                response_streams: Arc::new(RwLock::new(HashMap::new())),
                feed_tx,
                task_tx,
                agent_tx,
                session_tx,
                pipeline_tx,
                routing_tx,
                pool: None,
                dispatcher: None,
                task_results: Arc::new(RwLock::new(HashMap::new())),
                role_manager: None,
                cluster_manager: Arc::new(RwLock::new(ClusterManager::new())),
                pipeline_manager: Arc::new(RwLock::new(PipelineManager::new())),
                schedule_manager: Arc::new(RwLock::new(ScheduleManager::new())),
                mode_registry: Arc::new(ModeRegistry::new()),
                cluster_index: None,
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

    /// Ensure a response stream exists for this message (creates if missing).
    ///
    /// Call this before queuing work to the orchestrator so the broadcast
    /// channel exists when tokens start arriving.
    pub async fn ensure_response_stream(&self, message_id: Uuid) {
        let mut streams = self.response_streams.write().await;
        streams.entry(message_id).or_insert_with(|| {
            let (tx, _) = broadcast::channel(100);
            Arc::new(RwLock::new(BufferedStream { tx, buffer: Vec::new(), done: false }))
        });
    }

    /// Get the buffered chunks, a live receiver, and whether the stream is done.
    ///
    /// The caller should replay the buffer first, then listen on the receiver.
    /// Holding the inner read lock while snapshotting + subscribing guarantees
    /// no chunks are missed or duplicated.
    pub async fn get_response_stream(&self, message_id: Uuid) -> (Vec<StreamChunk>, broadcast::Receiver<StreamChunk>, bool) {
        let mut streams = self.response_streams.write().await;
        let entry = streams.entry(message_id).or_insert_with(|| {
            let (tx, _) = broadcast::channel(100);
            Arc::new(RwLock::new(BufferedStream { tx, buffer: Vec::new(), done: false }))
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::traits::MockServerRepo;

    fn make_state() -> AppState {
        let mut mock = MockServerRepo::new();
        mock.expect_health_check().returning(|| true);
        let repo: Arc<dyn ServerRepo> = Arc::new(mock);
        let (state, _rx) = AppState::with_repo(None, repo, AppConfig::default());
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
            session_id: None,
            mode_id: AgentModeId::new("home"),
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
        let (buf, _rx, done) = state.get_response_stream(msg_id).await;
        assert!(buf.is_empty());
        assert!(!done);
    }

    #[tokio::test]
    async fn get_response_stream_returns_existing() {
        let state = make_state();
        let msg_id = Uuid::new_v4();
        let (_buf1, _rx1, _) = state.get_response_stream(msg_id).await;
        let (_buf2, _rx2, _) = state.get_response_stream(msg_id).await;
    }

    #[tokio::test]
    async fn send_stream_chunk_no_stream() {
        let state = make_state();
        let result = state.send_stream_chunk(Uuid::new_v4(), StreamChunk::Token("hi".into())).await;
        assert!(!result);
    }

    #[tokio::test]
    async fn send_stream_chunk_with_stream() {
        let state = make_state();
        let msg_id = Uuid::new_v4();
        state.ensure_response_stream(msg_id).await;
        let result = state.send_stream_chunk(msg_id, StreamChunk::Token("hi".into())).await;
        assert!(result);
    }

    #[tokio::test]
    async fn buffered_stream_replays_chunks() {
        let state = make_state();
        let msg_id = Uuid::new_v4();
        state.ensure_response_stream(msg_id).await;

        // Send chunks with no SSE client connected
        state.send_stream_chunk(msg_id, StreamChunk::Token("hello ".into())).await;
        state.send_stream_chunk(msg_id, StreamChunk::Token("world".into())).await;
        state.send_stream_chunk(msg_id, StreamChunk::Done).await;

        // Late subscriber gets the full buffer
        let (buf, _rx, done) = state.get_response_stream(msg_id).await;
        assert_eq!(buf.len(), 3);
        assert!(done);
        assert!(matches!(&buf[0], StreamChunk::Token(t) if t == "hello "));
        assert!(matches!(&buf[1], StreamChunk::Token(t) if t == "world"));
        assert!(matches!(&buf[2], StreamChunk::Done));
    }

    #[tokio::test]
    async fn remove_response_stream() {
        let state = make_state();
        let msg_id = Uuid::new_v4();
        state.ensure_response_stream(msg_id).await;
        state.remove_response_stream(msg_id).await;
        let result = state.send_stream_chunk(msg_id, StreamChunk::Done).await;
        assert!(!result);
    }
}
