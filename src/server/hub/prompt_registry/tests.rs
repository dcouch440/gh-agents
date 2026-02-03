//! Tests for prompt registry

use crate::server::hub::{error::HubError, prompt_registry::PromptRegistry};
use std::collections::HashMap;
use std::path::Path;
use tempfile::TempDir;

fn setup_test_prompts() -> (TempDir, PromptRegistry) {
    let dir = TempDir::new().unwrap();
    let base = dir.path();

    // Create system/ subdirectory
    std::fs::create_dir_all(base.join("system")).unwrap();
    std::fs::create_dir_all(base.join("agents")).unwrap();

    std::fs::write(base.join("system/distiller.md"), "Distill this: {messages}\nTask: {task_title}").unwrap();

    std::fs::write(base.join("system/router.md"), "Route intent: {intent}\nTools: {tool_specs}").unwrap();

    std::fs::write(base.join("agents/worker.md"), "You are a worker agent.").unwrap();

    let registry = PromptRegistry::load_from_dir(base).unwrap();
    (dir, registry)
}

#[test]
fn load_finds_all_prompts() {
    let (_dir, registry) = setup_test_prompts();
    assert_eq!(registry.len(), 3);
    assert!(registry.get("system/distiller").is_some());
    assert!(registry.get("system/router").is_some());
    assert!(registry.get("agents/worker").is_some());
}

#[test]
fn get_returns_content() {
    let (_dir, registry) = setup_test_prompts();
    let content = registry.get("agents/worker").unwrap();
    assert_eq!(content, "You are a worker agent.");
}

#[test]
fn get_missing_returns_none() {
    let (_dir, registry) = setup_test_prompts();
    assert!(registry.get("nonexistent/prompt").is_none());
}

#[test]
fn require_missing_returns_error() {
    let (_dir, registry) = setup_test_prompts();
    let err = registry.require("ghost").unwrap_err();
    assert!(matches!(err, HubError::PromptNotFound { .. }));
}

#[test]
fn render_substitutes_variables() {
    let (_dir, registry) = setup_test_prompts();
    let mut vars = HashMap::new();
    vars.insert("messages".to_string(), "Hello world".to_string());
    vars.insert("task_title".to_string(), "Fix bug".to_string());

    let rendered = registry.render("system/distiller", &vars).unwrap();
    assert!(rendered.contains("Distill this: Hello world"));
    assert!(rendered.contains("Task: Fix bug"));
}

#[test]
fn render_leaves_unknown_variables() {
    let (_dir, registry) = setup_test_prompts();
    let vars = HashMap::new(); // no vars provided
    let rendered = registry.render("system/distiller", &vars).unwrap();
    assert!(rendered.contains("{messages}"));
    assert!(rendered.contains("{task_title}"));
}

#[test]
fn render_inline_works() {
    let mut vars = HashMap::new();
    vars.insert("name".to_string(), "Alice".to_string());
    let result = PromptRegistry::render_inline("Hello {name}!", &vars);
    assert_eq!(result, "Hello Alice!");
}

#[test]
fn keys_returns_sorted() {
    let (_dir, registry) = setup_test_prompts();
    let keys = registry.keys();
    assert_eq!(keys, vec!["agents/worker", "system/distiller", "system/router"]);
}

#[test]
fn empty_registry() {
    let registry = PromptRegistry::empty();
    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);
    assert!(registry.get("anything").is_none());
}

#[test]
fn load_nonexistent_dir_returns_empty() {
    let registry = PromptRegistry::load_from_dir(Path::new("/nonexistent/path")).unwrap();
    assert!(registry.is_empty());
}

#[test]
fn load_real_prompts_dir() {
    // Test loading the actual prompts/ directory if it exists
    let prompts_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("prompts");
    if prompts_dir.exists() {
        let registry = PromptRegistry::load_from_dir(&prompts_dir).unwrap();
        assert!(registry.len() >= 5, "expected at least 5 system prompts, got {}", registry.len());
        assert!(registry.get("system/distiller").is_some());
        assert!(registry.get("system/router").is_some());
        assert!(registry.get("system/schema_enforcement").is_some());
        assert!(registry.get("system/auto_namer").is_some());
        assert!(registry.get("system/summarizer").is_some());
    }
}
