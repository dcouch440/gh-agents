//! Builder pattern for constructing AppState.
//!
//! Provides a fluent API for creating AppState instances, particularly useful
//! for tests where you want fine-grained control over which components are set.

use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;
use sqlx::PgPool;
use thiserror::Error;
use tokio::sync::{mpsc, RwLock};
use tokio_util::sync::CancellationToken;

use crate::llm::{LLMProvider, ProviderRegistry};
use crate::types::AppConfig;

use crate::server::hub::PromptRegistry;

use super::{AppState, AppStateInner, ConsumerMessage, EventBus, Repos};

/// Errors that can occur during AppState building.
#[derive(Debug, Error)]
pub enum BuilderError {
    /// The grouped repositories are required but were not provided.
    #[error("repos is required")]
    MissingRepos,

    /// The application configuration is required but was not provided.
    #[error("config is required")]
    MissingConfig,
}

/// Builder for constructing AppState instances.
///
/// # Example
///
/// ```ignore
/// let (state, rx) = AppStateBuilder::new()
///     .with_repos(repos)
///     .with_config(config)
///     .build()?;
/// ```
pub struct AppStateBuilder {
    db: Option<PgPool>,
    repos: Option<Repos>,
    events: Option<EventBus>,
    config: Option<AppConfig>,
    provider: Option<Arc<dyn LLMProvider + Send + Sync>>,
    provider_registry: Option<ProviderRegistry>,
    prompt_registry: Option<Arc<PromptRegistry>>,
    jwt_secret: Option<Vec<u8>>,
}

impl AppStateBuilder {
    /// Create a new builder with no fields set.
    pub fn new() -> Self {
        Self {
            db: None,
            repos: None,
            events: None,
            config: None,
            provider: None,
            provider_registry: None,
            prompt_registry: None,
            jwt_secret: None,
        }
    }

    /// Set the database connection pool.
    pub fn with_db(mut self, db: PgPool) -> Self {
        self.db = Some(db);
        self
    }

    /// Set the grouped repositories (required).
    pub fn with_repos(mut self, repos: Repos) -> Self {
        self.repos = Some(repos);
        self
    }

    /// Set the event bus. If not provided, a default one will be created.
    pub fn with_events(mut self, events: EventBus) -> Self {
        self.events = Some(events);
        self
    }

    /// Set the application configuration (required).
    pub fn with_config(mut self, config: AppConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Set the LLM provider.
    pub fn with_provider(mut self, provider: Arc<dyn LLMProvider + Send + Sync>) -> Self {
        self.provider = Some(provider);
        self
    }

    /// Set the provider registry for multi-provider routing.
    pub fn with_provider_registry(mut self, registry: ProviderRegistry) -> Self {
        self.provider_registry = Some(registry);
        self
    }

    /// Set the prompt registry. If not provided, an empty one will be used.
    pub fn with_prompt_registry(mut self, registry: Arc<PromptRegistry>) -> Self {
        self.prompt_registry = Some(registry);
        self
    }

    /// Set the JWT secret. If not provided, a random one will be generated.
    pub fn with_jwt_secret(mut self, secret: Vec<u8>) -> Self {
        self.jwt_secret = Some(secret);
        self
    }

    /// Build the AppState and return it along with the orchestrator message receiver.
    ///
    /// # Errors
    ///
    /// Returns `BuilderError::MissingRepos` if `with_repos()` was not called.
    /// Returns `BuilderError::MissingConfig` if `with_config()` was not called.
    pub fn build(self) -> Result<(AppState, mpsc::Receiver<ConsumerMessage>), BuilderError> {
        let repos = self.repos.ok_or(BuilderError::MissingRepos)?;
        let config = self.config.ok_or(BuilderError::MissingConfig)?;

        let (chat_tx, orchestrator_rx) = mpsc::channel(crate::constants::CHANNEL_ORCHESTRATOR);
        let events = self.events.unwrap_or_default();
        let jwt_secret = self
            .jwt_secret
            .unwrap_or_else(|| rand::random::<[u8; 32]>().to_vec());
        let prompt_registry = self
            .prompt_registry
            .unwrap_or_else(|| Arc::new(PromptRegistry::empty()));

        let provider_registry = self.provider_registry.unwrap_or_default();

        let state = AppState::from_inner(AppStateInner {
            db: self.db,
            repos,
            events,
            config: Arc::new(RwLock::new(config)),
            provider: self.provider,
            provider_registry,
            prompt_registry,
            jwt_secret,
            chat_tx,
            response_streams: DashMap::new(),
            cancellation_tokens: DashMap::new(),
            shutdown_token: CancellationToken::new(),
            ollama_toggle_cache: Arc::new(tokio::sync::RwLock::new((false, Instant::now()))),
            protocol_engine: Arc::new(crate::server::hub::protocols::ProtocolEngine::new()),
            ws_connection_count: std::sync::atomic::AtomicUsize::new(0),
            ws_connections_by_ip: dashmap::DashMap::new(),
            pending_scan_items: dashmap::DashMap::new(),
        });

        Ok((state, orchestrator_rx))
    }

    /// Build the AppState for tests, panicking on error.
    ///
    /// This is a convenience method for tests that don't want to handle errors.
    ///
    /// # Panics
    ///
    /// Panics if required fields are missing.
    #[cfg(test)]
    pub fn build_for_test(self) -> (AppState, mpsc::Receiver<ConsumerMessage>) {
        self.build()
            .expect("AppStateBuilder: missing required fields")
    }
}

impl Default for AppStateBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::state::test_helpers::default_mock_repos;

    fn default_config() -> AppConfig {
        AppConfig::default()
    }

    #[test]
    fn build_with_required_fields_succeeds() {
        let result = AppStateBuilder::new()
            .with_repos(default_mock_repos())
            .with_config(default_config())
            .build();

        assert!(result.is_ok());
    }

    #[test]
    fn build_without_repos_fails() {
        let result = AppStateBuilder::new().with_config(default_config()).build();

        assert!(matches!(result, Err(BuilderError::MissingRepos)));
    }

    #[test]
    fn build_without_config_fails() {
        let result = AppStateBuilder::new()
            .with_repos(default_mock_repos())
            .build();

        assert!(matches!(result, Err(BuilderError::MissingConfig)));
    }

    #[test]
    fn build_for_test_with_required_fields_succeeds() {
        let (_state, _rx) = AppStateBuilder::new()
            .with_repos(default_mock_repos())
            .with_config(default_config())
            .build_for_test();
    }

    #[test]
    #[should_panic(expected = "missing required fields")]
    fn build_for_test_panics_on_missing_fields() {
        let _ = AppStateBuilder::new().build_for_test();
    }

    #[test]
    fn builder_sets_optional_fields() {
        let custom_secret = vec![1, 2, 3, 4];

        let (state, _rx) = AppStateBuilder::new()
            .with_repos(default_mock_repos())
            .with_config(default_config())
            .with_jwt_secret(custom_secret.clone())
            .build_for_test();

        assert_eq!(state.jwt_secret(), &custom_secret);
    }
}
