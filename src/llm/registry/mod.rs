//! Provider registry for multi-provider LLM routing.
//!
//! Holds named LLM providers (e.g. "anthropic", "ollama") and provides
//! lookup by name. Immutable after construction — providers are registered
//! at startup and the registry is shared via `Arc` in `AppState`.

use std::collections::HashMap;
use std::sync::Arc;

use super::provider::LLMProvider;

/// Registry of named LLM providers for multi-provider routing.
#[derive(Clone)]
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn LLMProvider + Send + Sync>>,
    default_name: String,
}

impl ProviderRegistry {
    /// Create an empty registry with the given default provider name.
    pub fn new(default_name: impl Into<String>) -> Self {
        Self {
            providers: HashMap::new(),
            default_name: default_name.into(),
        }
    }

    /// Register a provider under a given name.
    pub fn register(
        &mut self,
        name: impl Into<String>,
        provider: Arc<dyn LLMProvider + Send + Sync>,
    ) {
        self.providers.insert(name.into(), provider);
    }

    /// Get a provider by name.
    pub fn get(&self, name: &str) -> Option<&Arc<dyn LLMProvider + Send + Sync>> {
        self.providers.get(name)
    }

    /// Get the default provider.
    pub fn default_provider(&self) -> Option<&Arc<dyn LLMProvider + Send + Sync>> {
        self.providers.get(&self.default_name)
    }

    /// Check if a specific provider is registered.
    pub fn has_provider(&self, name: &str) -> bool {
        self.providers.contains_key(name)
    }

    /// List all registered provider names.
    pub fn provider_names(&self) -> Vec<&str> {
        self.providers.keys().map(|s| s.as_str()).collect()
    }

    /// Get the default provider name.
    pub fn default_name(&self) -> &str {
        &self.default_name
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new("anthropic")
    }
}

#[cfg(test)]
mod tests;
