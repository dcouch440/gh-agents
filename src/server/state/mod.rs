//! Shared application state for HTTP handlers
//!
//! This module provides `AppState`, the central state container shared across all
//! HTTP handlers. It wraps an inner `Arc<AppStateInner>` for cheap cloning.

use std::sync::Arc;

use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use sqlx::PgPool;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::db::pg_repo::PgRepo;
use crate::db::traits::ServerRepo;
use crate::llm::{LLMProvider, ProviderRegistry};
use crate::types::{AppConfig, UserId};

use super::hub::protocols::ProtocolEngine;
use super::hub::PromptRegistry;
use super::ws::events::{RoomEvent, ServerEvent, SessionEvent, WorkflowEvent};

mod builder;
mod events;
mod repos;

pub use builder::{AppStateBuilder, BuilderError};
pub use events::EventBus;
pub use repos::Repos;

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
    ToolStart { name: String, tool_id: String },
    /// A tool has finished executing
    ToolEnd { name: String, tool_id: String },
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
    /// Database connection pool
    pub(crate) db: Option<PgPool>,
    /// Repository trait object for DB operations used by API handlers
    pub(crate) server_repo: Arc<dyn ServerRepo>,
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
    /// Prompt registry for core system/agent prompts
    pub(crate) prompt_registry: Arc<PromptRegistry>,
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
    pub async fn new(db: PgPool, config: AppConfig) -> (Self, mpsc::Receiver<ConsumerMessage>) {
        let server_repo: Arc<dyn ServerRepo> = Arc::new(PgRepo::new(db.clone()));

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
            Arc::new(PgRepo::new(db.clone())), // tool_routers
            Arc::new(PgRepo::new(db.clone())), // context_store
            Arc::new(PgRepo::new(db.clone())), // router_requests
            Arc::new(PgRepo::new(db.clone())), // rooms
            Arc::new(PgRepo::new(db.clone())), // tool_capabilities
            Arc::new(PgRepo::new(db.clone())), // system_config
            Arc::new(PgRepo::new(db.clone())), // protocols
        );

        let (chat_tx, orchestrator_rx) = mpsc::channel(crate::constants::CHANNEL_ORCHESTRATOR);
        let events = EventBus::new();

        // JWT secret: require via env var, fall back to random for dev only
        let jwt_secret = Self::load_jwt_secret();

        // Load prompt registry from prompts/ directory
        let prompt_registry = Self::load_prompt_registry();

        // Initialize LLM providers
        let (provider, provider_registry) = Self::init_providers().await;

        let state = Self(Arc::new(AppStateInner {
            db: Some(db),
            server_repo,
            repos,
            events,
            config: Arc::new(RwLock::new(config)),
            provider,
            provider_registry,
            prompt_registry,
            jwt_secret,
            chat_tx,
            response_streams: DashMap::new(),
            cancellation_tokens: DashMap::new(),
            shutdown_token: CancellationToken::new(),
            ollama_toggle_cache: Arc::new(tokio::sync::RwLock::new((false, Instant::now()))),
            protocol_engine: Arc::new(ProtocolEngine::new()),
        }));

        (state, orchestrator_rx)
    }

    /// Create application state with a custom repo (for testing).
    /// Returns the state and the orchestrator message receiver.
    pub fn with_repo(
        db: Option<PgPool>,
        server_repo: Arc<dyn ServerRepo>,
        repos: Repos,
        config: AppConfig,
    ) -> (Self, mpsc::Receiver<ConsumerMessage>) {
        let (chat_tx, orchestrator_rx) = mpsc::channel(crate::constants::CHANNEL_ORCHESTRATOR);
        let events = EventBus::new();
        let jwt_secret = Self::load_jwt_secret();

        (
            Self(Arc::new(AppStateInner {
                db,
                server_repo,
                repos,
                events,
                config: Arc::new(RwLock::new(config)),
                provider: None,
                provider_registry: ProviderRegistry::default(),
                prompt_registry: Arc::new(PromptRegistry::empty()),
                jwt_secret,
                chat_tx,
                response_streams: DashMap::new(),
                cancellation_tokens: DashMap::new(),
                shutdown_token: CancellationToken::new(),
                ollama_toggle_cache: Arc::new(tokio::sync::RwLock::new((false, Instant::now()))),
                protocol_engine: Arc::new(ProtocolEngine::new()),
            })),
            orchestrator_rx,
        )
    }

    // =========================================================================
    // Private initialization helpers
    // =========================================================================

    fn load_jwt_secret() -> Vec<u8> {
        match std::env::var(crate::constants::ENV_JWT_SECRET) {
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
        }
    }

    fn load_prompt_registry() -> Arc<PromptRegistry> {
        let prompts_dir = std::env::current_dir().unwrap_or_default().join("prompts");
        match PromptRegistry::load_from_dir(&prompts_dir) {
            Ok(registry) => {
                tracing::info!(
                    "Loaded {} prompts from {}",
                    registry.len(),
                    prompts_dir.display()
                );
                Arc::new(registry)
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to load prompts from {}: {} — using empty registry",
                    prompts_dir.display(),
                    e
                );
                Arc::new(PromptRegistry::empty())
            }
        }
    }

    async fn init_providers() -> (Option<Arc<dyn LLMProvider + Send + Sync>>, ProviderRegistry) {
        let mut registry = ProviderRegistry::new("anthropic");

        // Initialize Anthropic provider (default)
        let provider = match crate::llm::AnthropicClient::from_env() {
            Ok(p) => {
                tracing::info!("Initialized LLM provider: {}", p.model_id().to_string());
                let provider: Arc<dyn LLMProvider + Send + Sync> =
                    Arc::new(crate::llm::SafeStreamProvider::new(
                        crate::llm::RetryingProvider::with_defaults(
                            crate::llm::RateLimitedProvider::with_defaults(p),
                        ),
                    ));

                registry.register("anthropic", provider.clone());

                Some(provider)
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to initialize LLM provider: {}. Set ANTHROPIC_API_KEY.",
                    e
                );
                None
            }
        };

        // Initialize Ollama provider if enabled
        let ollama_enabled = std::env::var(crate::constants::ENV_OLLAMA_ENABLED)
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        if ollama_enabled {
            match crate::llm::OllamaClient::from_env() {
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
                            std::env::var(crate::constants::ENV_OLLAMA_BASE_URL).unwrap_or_else(
                                |_| { crate::constants::OLLAMA_DEFAULT_BASE_URL.to_string() }
                            )
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
        } else {
            tracing::debug!(
                "Ollama provider disabled (set {}=true to enable)",
                crate::constants::ENV_OLLAMA_ENABLED
            );
        }

        (provider, registry)
    }

    // =========================================================================
    // Accessor methods
    // =========================================================================

    /// Access the database connection pool.
    pub fn db(&self) -> Option<&PgPool> {
        self.0.db.as_ref()
    }

    /// Access the server repository (legacy).
    pub fn server_repo(&self) -> &Arc<dyn ServerRepo> {
        &self.0.server_repo
    }

    /// Backward-compatible alias for `server_repo()`.
    /// This is the primary method used by existing handlers.
    pub fn repo(&self) -> &Arc<dyn ServerRepo> {
        &self.0.server_repo
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
            _ => std::env::var(crate::constants::ENV_OLLAMA_ENABLED)
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
        };

        // Update cache
        {
            let mut cache = self.0.ollama_toggle_cache.write().await;
            *cache = (enabled, Instant::now());
        }

        enabled
    }

    /// Access the prompt registry.
    pub fn prompt_registry(&self) -> &Arc<PromptRegistry> {
        &self.0.prompt_registry
    }

    /// Access the protocol engine.
    pub fn protocol_engine(&self) -> &Arc<ProtocolEngine> {
        &self.0.protocol_engine
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
    // Backward-compatible accessors (delegate to repos)
    // These maintain the old API while we migrate handlers.
    // After migration, handlers should use state.repos().* directly.
    // =========================================================================

    /// Backward-compatible: User repository.
    pub fn user_repo(&self) -> Option<Arc<dyn crate::db::traits::UserRepo>> {
        Some(self.0.repos.users.clone())
    }

    /// Backward-compatible: Document repository.
    pub fn doc_repo(&self) -> Option<Arc<dyn crate::db::traits::DocumentRepo>> {
        Some(self.0.repos.documents.clone())
    }

    /// Backward-compatible: Output schema repository.
    pub fn output_schema_repo(&self) -> Option<Arc<dyn crate::db::traits::OutputSchemaRepo>> {
        Some(self.0.repos.output_schemas.clone())
    }

    /// Backward-compatible: Prompt template repository.
    pub fn prompt_template_repo(&self) -> Option<Arc<dyn crate::db::traits::PromptTemplateRepo>> {
        Some(self.0.repos.prompt_templates.clone())
    }

    /// Backward-compatible: Workflow repository.
    pub fn workflow_repo(&self) -> Option<Arc<dyn crate::db::traits::WorkflowRepo>> {
        Some(self.0.repos.workflows.clone())
    }

    /// Backward-compatible: Agent execution repository.
    pub fn agent_execution_repo(&self) -> Option<Arc<dyn crate::db::traits::AgentExecutionRepo>> {
        Some(self.0.repos.agent_executions.clone())
    }

    /// Backward-compatible: Token ledger repository.
    pub fn token_ledger_repo(&self) -> Option<Arc<dyn crate::db::traits::TokenLedgerRepo>> {
        Some(self.0.repos.token_ledger.clone())
    }

    /// Backward-compatible: Room repository.
    pub fn room_repo(&self) -> Option<Arc<dyn crate::db::traits::RoomRepo>> {
        Some(self.0.repos.rooms.clone())
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
}
