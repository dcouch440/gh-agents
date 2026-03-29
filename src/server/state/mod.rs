//! Shared application state for HTTP handlers
//!
//! This module provides `AppState`, the central state container shared across all
//! HTTP handlers. It wraps an inner `Arc<AppStateInner>` for cheap cloning.

use std::net::IpAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use sqlx::PgPool;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::db::pg_repo::PgRepo;
use crate::env::Env;
use crate::llm::{LLMProvider, ProviderRegistry};
use crate::types::{AppConfig, UserId};

use super::hub::protocols::ProtocolEngine;
use super::services::system_store::s3::S3Backend;
use super::services::workspace::WorkspaceManager;
use super::ws::events::{RoomEvent, ServerEvent, SessionEvent, WorkflowEvent};

mod builder;
mod events;
mod repos;
pub mod task_registry;

pub use builder::{AppStateBuilder, BuilderError};
pub use events::{BroadcastEnvelope, EventBus};
pub use repos::Repos;
pub use task_registry::TaskRegistry;

#[cfg(test)]
pub mod test_helpers;
#[cfg(test)]
mod tests;

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
    ToolStart { name: String, tool_id: String, input: String },
    /// A tool has finished executing
    ToolEnd { name: String, tool_id: String, result: String },
    /// A document was created or updated
    DocUpdate { doc_id: String, title: String },
    /// An interactive panel was rendered on the node
    PanelRender {
        content: String,
        submit_label: String,
    },
    /// Stream completed successfully
    Done,
    /// Stream error
    Error(String),
}

/// A buffered broadcast stream that retains all chunks so late-connecting
/// SSE clients can replay missed tokens.
pub(crate) struct BufferedStream {
    tx: broadcast::Sender<StreamChunk>,
    buffer: Vec<StreamChunk>,
    done: bool,
}

/// Inner state wrapped in Arc for cheap cloning.
pub(crate) struct AppStateInner {
    /// Centralized environment configuration (read once at startup).
    pub(crate) env: Arc<Env>,
    /// Database connection pool
    pub(crate) db: Option<PgPool>,
    /// All repository trait objects grouped together
    pub(crate) repos: Repos,
    /// All broadcast channels grouped together
    pub(crate) events: EventBus,
    /// Application configuration (mutable at runtime via API)
    pub(crate) config: Arc<RwLock<AppConfig>>,
    /// LLM provider for agent execution (default / backward-compatible)
    pub(crate) provider: Option<Arc<dyn LLMProvider + Send + Sync>>,
    /// Multi-provider registry for step-level routing
    pub(crate) provider_registry: ProviderRegistry,
    /// In-memory capability → tool resolution (loaded from YAML at startup)
    pub(crate) capability_registry: Arc<crate::config::capability_registry::CapabilityRegistry>,
    /// JWT secret for token signing
    pub(crate) jwt_secret: Vec<u8>,
    /// Channel to send messages to the orchestrator
    pub(crate) chat_tx: mpsc::Sender<ConsumerMessage>,
    /// Map of message IDs to buffered response streams (DashMap for concurrent access)
    pub(crate) response_streams: DashMap<Uuid, BufferedStream>,
    /// Cancellation tokens for running pipelines and agent executions
    pub(crate) cancellation_tokens: DashMap<Uuid, CancellationToken>,
    /// Master shutdown token — cancelled on SIGTERM/SIGINT to signal all background tasks.
    pub(crate) shutdown_token: CancellationToken,
    /// Cached result for `is_ollama_enabled()` to avoid per-step DB round-trips.
    pub(crate) ollama_toggle_cache: Arc<tokio::sync::RwLock<(bool, Instant)>>,
    /// Protocol engine for expanding protocol configurations into workflow primitives.
    pub(crate) protocol_engine: Arc<ProtocolEngine>,
    /// Current number of active WebSocket connections (global).
    pub(crate) ws_connection_count: AtomicUsize,
    /// Active WebSocket connections per IP address.
    pub(crate) ws_connections_by_ip: DashMap<IpAddr, usize>,
    /// Registry for background dispatch tasks.
    pub(crate) task_registry: TaskRegistry,
    /// Cancellation tokens for in-flight run results summarizations (cancel-and-replace).
    pub(crate) run_results_tokens: super::hub::run_results::RunResultsTokens,
    /// S3-compatible storage backend (MinIO in dev, real S3 in prod).
    pub(crate) s3: Option<Arc<S3Backend>>,
    /// JuiceFS workspace manager (None if JuiceFS is not mounted).
    pub(crate) workspace: Option<WorkspaceManager>,
}

/// Application state shared across all HTTP handlers.
///
/// Wraps an inner Arc for cheap cloning. All handlers receive
/// the same underlying state.
#[derive(Clone)]
pub struct AppState(Arc<AppStateInner>);

impl AppState {
    /// Create AppState from inner state (used by AppStateBuilder).
    pub(crate) fn from_inner(inner: AppStateInner) -> Self {
        Self(Arc::new(inner))
    }

    /// Create new application state, returning the orchestrator receiver separately
    /// so it can be passed to the orchestrator consumer task.
    ///
    /// Loads persisted agents and clusters from the database on startup.
    pub async fn new(
        db: PgPool,
        config: AppConfig,
        env: Arc<Env>,
    ) -> (Self, mpsc::Receiver<ConsumerMessage>) {
        // Create all repos from PgRepo
        let repos = Repos::new(
            Arc::new(PgRepo::new(db.clone())), // users
            Arc::new(PgRepo::new(db.clone())), // documents
            Arc::new(PgRepo::new(db.clone())), // output_schemas
            Arc::new(PgRepo::new(db.clone())), // prompt_templates
            Arc::new(PgRepo::new(db.clone())), // workflows
            Arc::new(PgRepo::new(db.clone())), // agent_executions
            Arc::new(PgRepo::new(db.clone())), // token_ledger
            Arc::new(PgRepo::new(db.clone())), // results
            Arc::new(PgRepo::new(db.clone())), // rooms
            Arc::new(PgRepo::new(db.clone())), // tool_capabilities
            Arc::new(PgRepo::new(db.clone())), // system_config
            Arc::new(PgRepo::new(db.clone())), // protocols
            Arc::new(PgRepo::new(db.clone())), // content_versions
            Arc::new(PgRepo::new(db.clone())), // agents
            Arc::new(PgRepo::new(db.clone())), // tools
            Arc::new(PgRepo::new(db.clone())), // sessions
            Arc::new(PgRepo::new(db.clone())), // chat_messages
            Arc::new(PgRepo::new(db.clone())), // auth_config
            Arc::new(PgRepo::new(db.clone())), // system_files
        );

        let (chat_tx, orchestrator_rx) = mpsc::channel(crate::constants::CHANNEL_ORCHESTRATOR);
        let events = EventBus::new();

        // JWT secret: require via env var, fall back to random for dev only
        let jwt_secret = Self::load_jwt_secret(&env);

        // Load capability registry from YAML config files
        let capability_registry = Self::load_capability_registry();

        // Initialize LLM providers
        let (provider, provider_registry) = Self::init_providers(&env).await;

        // Initialize S3 backend (panics if unavailable — required for system store)
        let s3 = Some(Self::init_s3(&env).await);

        // Initialize workspace manager (optional — requires JuiceFS mount)
        let workspace = WorkspaceManager::from_env(crate::constants::ENV_WORKSPACE_MOUNT_POINT)
            .or_else(|| {
                // Fall back to default mount point
                let default = crate::constants::WORKSPACE_DEFAULT_MOUNT_POINT;
                WorkspaceManager::new(default).ok()
            });

        let state = Self(Arc::new(AppStateInner {
            env,
            db: Some(db),
            repos,
            events,
            config: Arc::new(RwLock::new(config)),
            provider,
            provider_registry,
            capability_registry,
            jwt_secret,
            chat_tx,
            response_streams: DashMap::new(),
            cancellation_tokens: DashMap::new(),
            shutdown_token: CancellationToken::new(),
            ollama_toggle_cache: Arc::new(tokio::sync::RwLock::new((false, Instant::now()))),
            protocol_engine: Arc::new(ProtocolEngine::new()),
            ws_connection_count: AtomicUsize::new(0),
            ws_connections_by_ip: DashMap::new(),
            task_registry: TaskRegistry::new(),
            run_results_tokens: super::hub::run_results::new_run_results_tokens(),
            s3,
            workspace,
        }));

        (state, orchestrator_rx)
    }

    /// Create application state with custom repos (for testing).
    /// Returns the state and the orchestrator message receiver.
    pub fn with_repos(
        db: Option<PgPool>,
        repos: Repos,
        config: AppConfig,
    ) -> (Self, mpsc::Receiver<ConsumerMessage>) {
        #[cfg(test)]
        let env = Arc::new(Env::test_default());
        #[cfg(not(test))]
        let env = Arc::new(Env::load());

        let (chat_tx, orchestrator_rx) = mpsc::channel(crate::constants::CHANNEL_ORCHESTRATOR);
        let events = EventBus::new();
        let jwt_secret = Self::load_jwt_secret(&env);

        (
            Self(Arc::new(AppStateInner {
                env,
                db,
                repos,
                events,
                config: Arc::new(RwLock::new(config)),
                provider: None,
                provider_registry: ProviderRegistry::default(),
                capability_registry: Arc::new(
                    crate::config::capability_registry::CapabilityRegistry::empty(),
                ),
                jwt_secret,
                chat_tx,
                response_streams: DashMap::new(),
                cancellation_tokens: DashMap::new(),
                shutdown_token: CancellationToken::new(),
                ollama_toggle_cache: Arc::new(tokio::sync::RwLock::new((false, Instant::now()))),
                protocol_engine: Arc::new(ProtocolEngine::new()),
                ws_connection_count: AtomicUsize::new(0),
                ws_connections_by_ip: DashMap::new(),
                task_registry: TaskRegistry::new(),
                run_results_tokens: super::hub::run_results::new_run_results_tokens(),
                s3: None,
                workspace: None,
            })),
            orchestrator_rx,
        )
    }

    // =========================================================================
    // Private initialization helpers
    // =========================================================================

    fn load_jwt_secret(env: &Env) -> Vec<u8> {
        match &env.jwt_secret {
            Some(s) => {
                tracing::info!(
                    "{} loaded from environment",
                    crate::constants::ENV_JWT_SECRET
                );
                s.clone().into_bytes()
            }
            None => {
                if env.is_production() {
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
        }
    }

    fn load_capability_registry() -> Arc<crate::config::capability_registry::CapabilityRegistry> {
        let config_dir = std::env::current_dir().unwrap_or_default().join("config");
        match crate::config::capability_registry::CapabilityRegistry::load(&config_dir) {
            Ok(registry) => {
                tracing::info!("Loaded capability registry from {}", config_dir.display());
                Arc::new(registry)
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to load capability registry from {}: {} — using empty registry",
                    config_dir.display(),
                    e
                );
                Arc::new(crate::config::capability_registry::CapabilityRegistry::empty())
            }
        }
    }

    async fn init_providers(
        env: &Env,
    ) -> (Option<Arc<dyn LLMProvider + Send + Sync>>, ProviderRegistry) {
        let active = crate::constants::ACTIVE_PROVIDER;
        let mut registry = ProviderRegistry::new(active);

        // Initialize Anthropic provider
        if let Some(ref api_key) = env.anthropic_api_key {
            let config = crate::llm::AnthropicConfig::new(api_key.clone())
                .with_model(env.anthropic_model.clone());
            match crate::llm::AnthropicClient::with_config(config) {
                Ok(p) => {
                    tracing::info!("Initialized Anthropic provider: {}", p.model_id());
                    let provider: Arc<dyn LLMProvider + Send + Sync> =
                        Arc::new(crate::llm::SafeStreamProvider::new(
                            crate::llm::RetryingProvider::with_defaults(
                                crate::llm::RateLimitedProvider::with_defaults(p),
                            ),
                        ));
                    registry.register("anthropic", provider);
                }
                Err(e) => {
                    tracing::warn!(
                        "Anthropic provider not initialized: {}. Set ANTHROPIC_API_KEY.",
                        e
                    );
                }
            }
        } else {
            tracing::warn!(
                "Anthropic provider not initialized: {} not set. Set ANTHROPIC_API_KEY.",
                crate::constants::ENV_ANTHROPIC_API_KEY
            );
        }

        // Initialize Ollama provider if enabled
        if env.ollama_enabled {
            match &env.ollama_model {
                Some(model) => {
                    let config = crate::llm::OllamaConfig {
                        base_url: env.ollama_base_url.clone(),
                        model: model.clone(),
                        timeout_secs: crate::constants::OLLAMA_DEFAULT_TIMEOUT_SECS,
                    };
                    match crate::llm::OllamaClient::new(config) {
                        Ok(client) => {
                            // Verify Ollama is reachable before registering
                            if let Err(e) = client.health_check().await {
                                tracing::warn!(
                                    "Ollama enabled but not reachable — skipping registration: {}",
                                    e
                                );
                            } else if let Err(e) = client.validate_model().await {
                                tracing::warn!(
                                    "Ollama reachable but model validation failed — skipping registration: {}",
                                    e
                                );
                            } else {
                                tracing::info!(
                                    "Initialized Ollama provider: {} ({})",
                                    client.model_id(),
                                    env.ollama_base_url
                                );
                                let ollama_provider: Arc<dyn LLMProvider + Send + Sync> =
                                    Arc::new(crate::llm::SafeStreamProvider::new(
                                        crate::llm::RetryingProvider::with_defaults(client),
                                    ));
                                registry.register("ollama", ollama_provider);
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Ollama enabled but failed to initialize: {}. Set {}.",
                                e,
                                crate::constants::ENV_OLLAMA_MODEL
                            );
                        }
                    }
                }
                None => {
                    tracing::warn!(
                        "Ollama enabled but {} not set — skipping.",
                        crate::constants::ENV_OLLAMA_MODEL
                    );
                }
            }
        } else {
            tracing::debug!(
                "Ollama provider disabled (set {}=true to enable)",
                crate::constants::ENV_OLLAMA_ENABLED
            );
        }

        // Initialize xAI provider (with web + X search enabled for all requests).
        if let Some(ref api_key) = env.xai_api_key {
            let config = crate::llm::XaiConfig {
                api_key: api_key.clone(),
                base_url: crate::constants::XAI_DEFAULT_BASE_URL.to_string(),
                model: env.xai_model.clone(),
                timeout_secs: crate::constants::XAI_CHAT_TIMEOUT_SECS,
                web_search: true,
                x_search: true,
            };
            match crate::llm::XaiClient::with_config(config) {
                Ok(client) => {
                    tracing::info!(
                        "Initialized xAI provider: {} ({}) [web_search + x_search enabled]",
                        client.model_id(),
                        crate::constants::XAI_DEFAULT_BASE_URL
                    );
                    let xai_provider: Arc<dyn LLMProvider + Send + Sync> =
                        Arc::new(crate::llm::SafeStreamProvider::new(
                            crate::llm::RetryingProvider::with_defaults(
                                crate::llm::RateLimitedProvider::with_defaults(client),
                            ),
                        ));
                    registry.register("xai", xai_provider);
                }
                Err(e) => {
                    tracing::debug!(
                        "xAI provider not initialized: {}. Set {} to enable.",
                        e,
                        crate::constants::ENV_XAI_API_KEY
                    );
                }
            }
        } else {
            tracing::debug!(
                "xAI provider not initialized: {} not set. Set {} to enable.",
                crate::constants::ENV_XAI_API_KEY,
                crate::constants::ENV_XAI_API_KEY
            );
        }

        // Resolve the default provider from the active profile
        let provider = registry.default_provider().cloned();
        if provider.is_some() {
            tracing::info!("Active provider profile: '{active}'");
        } else {
            tracing::warn!(
                "Active provider '{active}' not available — no default LLM provider configured"
            );
        }

        (provider, registry)
    }

    async fn init_s3(env: &Env) -> Arc<S3Backend> {
        let endpoint = env.s3_endpoint.as_deref();
        match S3Backend::new(endpoint, &env.s3_bucket).await {
            Ok(backend) => {
                tracing::info!(
                    "Initialized S3 backend: bucket={}, endpoint={}",
                    env.s3_bucket,
                    endpoint.unwrap_or("AWS default")
                );
                Arc::new(backend)
            }
            Err(e) => {
                panic!(
                    "S3 backend failed to initialize: {e}. \
                     Set S3_ENDPOINT=http://localhost:9000, \
                     AWS_ACCESS_KEY_ID=minioadmin, \
                     AWS_SECRET_ACCESS_KEY=minioadmin, \
                     and ensure MinIO is running (docker compose up -d minio)"
                );
            }
        }
    }

    // =========================================================================
    // Accessor methods
    // =========================================================================

    /// Access the centralized environment configuration.
    pub fn env(&self) -> &Env {
        &self.0.env
    }

    /// Access the database connection pool.
    pub fn db(&self) -> Option<&PgPool> {
        self.0.db.as_ref()
    }

    /// Access the grouped repositories.
    pub fn repos(&self) -> &Repos {
        &self.0.repos
    }

    /// Access the event bus.
    pub fn events(&self) -> &EventBus {
        &self.0.events
    }

    /// Access the application configuration.
    pub fn config(&self) -> &Arc<RwLock<AppConfig>> {
        &self.0.config
    }

    /// Access the default LLM provider (backward-compatible).
    pub fn provider(&self) -> Option<&Arc<dyn LLMProvider + Send + Sync>> {
        self.0.provider.as_ref()
    }

    /// Access the provider registry for multi-provider routing.
    pub fn provider_registry(&self) -> &ProviderRegistry {
        &self.0.provider_registry
    }

    /// Get a specific provider by name (e.g. "anthropic", "ollama").
    pub fn provider_for(&self, provider_name: &str) -> Option<Arc<dyn LLMProvider + Send + Sync>> {
        self.0.provider_registry.get(provider_name).cloned()
    }

    /// Check whether the Ollama provider is enabled at runtime.
    ///
    /// Uses a 60-second cache to avoid per-step DB round-trips. Checks the
    /// system_config DB table first (runtime toggle), then falls back to the
    /// `NEXOR_OLLAMA_ENABLED` env var.
    pub async fn is_ollama_enabled(&self) -> bool {
        const CACHE_TTL: Duration = Duration::from_secs(60);

        // Check cache first
        {
            let cache = self.0.ollama_toggle_cache.read().await;
            if cache.1.elapsed() < CACHE_TTL {
                return cache.0;
            }
        }

        // Cache miss — query DB
        let enabled = match self
            .0
            .repos
            .system_config
            .get_system_config("ollama_enabled")
            .await
        {
            Ok(Some(config)) => config.config_value.as_bool().unwrap_or(false),
            _ => self.0.env.ollama_enabled,
        };

        // Update cache
        {
            let mut cache = self.0.ollama_toggle_cache.write().await;
            *cache = (enabled, Instant::now());
        }

        enabled
    }

    /// Access the capability registry (YAML-backed, in-memory).
    pub fn capability_registry(
        &self,
    ) -> &Arc<crate::config::capability_registry::CapabilityRegistry> {
        &self.0.capability_registry
    }

    /// Access the protocol engine.
    pub fn protocol_engine(&self) -> &Arc<ProtocolEngine> {
        &self.0.protocol_engine
    }

    /// Access the background dispatch task registry.
    pub fn task_registry(&self) -> &TaskRegistry {
        &self.0.task_registry
    }

    /// Access the S3-compatible storage backend.
    pub fn s3(&self) -> Option<&Arc<S3Backend>> {
        self.0.s3.as_ref()
    }

    /// Access the JuiceFS workspace manager (None if not mounted).
    pub fn workspace(&self) -> Option<&WorkspaceManager> {
        self.0.workspace.as_ref()
    }

    /// Access the run results summarization tokens (cancel-and-replace map).
    pub fn run_results_tokens(&self) -> &super::hub::run_results::RunResultsTokens {
        &self.0.run_results_tokens
    }

    /// Access the JWT secret.
    pub fn jwt_secret(&self) -> &[u8] {
        &self.0.jwt_secret
    }

    /// Access the chat sender (for sending messages to orchestrator).
    pub fn chat_tx(&self) -> &mpsc::Sender<ConsumerMessage> {
        &self.0.chat_tx
    }

    // =========================================================================
    // Broadcast methods (delegate to EventBus)
    // =========================================================================

    /// Broadcast any event to all WebSocket subscribers.
    pub fn broadcast(&self, event: ServerEvent) {
        self.0.events.broadcast(event);
    }

    /// Broadcast a workflow execution event.
    pub fn broadcast_workflow(&self, event: WorkflowEvent) {
        self.broadcast(ServerEvent::Workflow(event));
    }

    /// Broadcast a room event.
    pub fn broadcast_room(&self, event: RoomEvent) {
        self.broadcast(ServerEvent::Room(event));
    }

    /// Broadcast a session event.
    pub fn broadcast_session(&self, event: SessionEvent) {
        self.broadcast(ServerEvent::Session(event));
    }

    // =========================================================================
    // Response stream methods (using DashMap for concurrent access)
    // =========================================================================

    /// Ensure a response stream exists for this message (creates if missing).
    ///
    /// Call this before queuing work to the orchestrator so the broadcast
    /// channel exists when tokens start arriving.
    pub fn ensure_response_stream(&self, message_id: Uuid) {
        self.0
            .response_streams
            .entry(message_id)
            .or_insert_with(|| {
                let (tx, _) = broadcast::channel(100);
                BufferedStream {
                    tx,
                    buffer: Vec::new(),
                    done: false,
                }
            });
    }

    /// Get the buffered chunks, a live receiver, and whether the stream is done.
    ///
    /// The caller should replay the buffer first, then listen on the receiver.
    pub fn get_response_stream(
        &self,
        message_id: Uuid,
    ) -> (Vec<StreamChunk>, broadcast::Receiver<StreamChunk>, bool) {
        let entry = self
            .0
            .response_streams
            .entry(message_id)
            .or_insert_with(|| {
                let (tx, _) = broadcast::channel(100);
                BufferedStream {
                    tx,
                    buffer: Vec::new(),
                    done: false,
                }
            });
        let rx = entry.tx.subscribe();
        (entry.buffer.clone(), rx, entry.done)
    }

    /// Send a chunk to a message's response stream
    ///
    /// Appends to the buffer and broadcasts. Returns false if no stream exists.
    pub fn send_stream_chunk(&self, message_id: Uuid, chunk: StreamChunk) -> bool {
        if let Some(mut entry) = self.0.response_streams.get_mut(&message_id) {
            if matches!(&chunk, StreamChunk::Done) {
                entry.done = true;
            }
            entry.buffer.push(chunk.clone());
            let _ = entry.tx.send(chunk);
            true
        } else {
            false
        }
    }

    /// Remove a response stream (call when streaming is complete)
    pub fn remove_response_stream(&self, message_id: Uuid) {
        self.0.response_streams.remove(&message_id);
    }

    // =========================================================================
    // Cancellation token methods (using DashMap for concurrent access)
    // =========================================================================

    /// Register a cancellation token for a running execution (pipeline run or agent execution).
    /// Returns a clone of the token to pass through the execution chain.
    pub fn register_cancellation(&self, id: Uuid) -> CancellationToken {
        let token = CancellationToken::new();
        self.0.cancellation_tokens.insert(id, token.clone());
        token
    }

    /// Create a child cancellation token linked to a parent.
    /// Cancelling the parent automatically cancels all children.
    pub fn register_child_cancellation(
        &self,
        id: Uuid,
        parent: &CancellationToken,
    ) -> CancellationToken {
        let child = parent.child_token();
        self.0.cancellation_tokens.insert(id, child.clone());
        child
    }

    /// Cancel a running execution by its ID. Returns true if the token existed.
    pub fn cancel_execution(&self, id: Uuid) -> bool {
        if let Some(entry) = self.0.cancellation_tokens.get(&id) {
            entry.cancel();
            true
        } else {
            false
        }
    }

    /// Remove a cancellation token after execution completes.
    pub fn remove_cancellation(&self, id: Uuid) {
        self.0.cancellation_tokens.remove(&id);
    }

    /// Access the master shutdown cancellation token.
    pub fn shutdown_token(&self) -> &CancellationToken {
        &self.0.shutdown_token
    }

    /// Cancel all running executions and return the count cancelled.
    /// Called during graceful shutdown to drain active workflows.
    pub fn cancel_all_executions(&self) -> usize {
        let mut cancelled = 0;
        for entry in self.0.cancellation_tokens.iter() {
            entry.cancel();
            cancelled += 1;
        }
        cancelled
    }

    /// Return the number of active execution tokens.
    pub fn active_execution_count(&self) -> usize {
        self.0.cancellation_tokens.len()
    }

    // =========================================================================
    // WebSocket connection tracking
    // =========================================================================

    /// Try to acquire a WebSocket connection slot for the given IP.
    /// Returns `true` if the connection is allowed, `false` if limits are exceeded.
    pub fn try_acquire_ws_connection(&self, ip: IpAddr) -> bool {
        let current = self.0.ws_connection_count.load(Ordering::Relaxed);
        if current >= crate::constants::WS_MAX_CONNECTIONS {
            return false;
        }

        let mut ip_count = self.0.ws_connections_by_ip.entry(ip).or_insert(0);
        if *ip_count >= crate::constants::WS_MAX_CONNECTIONS_PER_IP {
            return false;
        }

        *ip_count += 1;
        self.0.ws_connection_count.fetch_add(1, Ordering::Relaxed);
        true
    }

    /// Release a WebSocket connection slot for the given IP.
    pub fn release_ws_connection(&self, ip: IpAddr) {
        self.0.ws_connection_count.fetch_sub(1, Ordering::Relaxed);
        if let Some(mut entry) = self.0.ws_connections_by_ip.get_mut(&ip) {
            *entry = entry.saturating_sub(1);
            if *entry == 0 {
                drop(entry);
                self.0.ws_connections_by_ip.remove(&ip);
            }
        }
    }

    /// Return the current number of active WebSocket connections.
    pub fn ws_connection_count(&self) -> usize {
        self.0.ws_connection_count.load(Ordering::Relaxed)
    }
}
