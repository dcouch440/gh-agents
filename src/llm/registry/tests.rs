//! Tests for ProviderRegistry

use std::sync::Arc;

use crate::llm::{NoOpProvider, ProviderRegistry};

#[test]
fn register_and_get_provider() {
    let mut registry = ProviderRegistry::new("default");
    let provider = Arc::new(NoOpProvider);
    registry.register("test", provider);

    assert!(registry.get("test").is_some());
    assert_eq!(registry.get("test").unwrap().provider_name(), "noop");
}

#[test]
fn get_unknown_returns_none() {
    let registry = ProviderRegistry::new("default");
    assert!(registry.get("nonexistent").is_none());
}

#[test]
fn default_provider_returns_registered_default() {
    let mut registry = ProviderRegistry::new("primary");
    let provider = Arc::new(NoOpProvider);
    registry.register("primary", provider);

    assert!(registry.default_provider().is_some());
}

#[test]
fn default_provider_returns_none_when_not_registered() {
    let registry = ProviderRegistry::new("missing");
    assert!(registry.default_provider().is_none());
}

#[test]
fn has_provider_true_for_registered() {
    let mut registry = ProviderRegistry::new("default");
    registry.register("ollama", Arc::new(NoOpProvider));

    assert!(registry.has_provider("ollama"));
    assert!(!registry.has_provider("openai"));
}

#[test]
fn provider_names_lists_all() {
    let mut registry = ProviderRegistry::new("default");
    registry.register("anthropic", Arc::new(NoOpProvider));
    registry.register("ollama", Arc::new(NoOpProvider));

    let mut names = registry.provider_names();
    names.sort();
    assert_eq!(names, vec!["anthropic", "ollama"]);
}

#[test]
fn default_name_returns_configured_name() {
    let registry = ProviderRegistry::new("anthropic");
    assert_eq!(registry.default_name(), "anthropic");
}

#[test]
fn default_impl_uses_anthropic() {
    let registry = ProviderRegistry::default();
    assert_eq!(registry.default_name(), "anthropic");
}
