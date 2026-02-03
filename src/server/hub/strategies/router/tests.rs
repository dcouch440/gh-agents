//! Tests for router strategy

use crate::llm::TokenUsage;
use crate::server::hub::strategies::router::{RouterConfig, RouterStrategy};
use crate::server::hub::strategy::ExecutionStrategy;

#[test]
fn router_strategy_properties() {
    let strategy = RouterStrategy::new(RouterConfig {
        system_prompt: "You are a router.".into(),
        model_id: "claude-3-haiku".into(),
        state: None,
        user_id: None,
    });

    assert_eq!(strategy.system_prompt(), "You are a router.");
    assert_eq!(strategy.model_id(), "claude-3-haiku");
    assert_eq!(strategy.max_rounds(), 1);
    assert!(!strategy.streaming());
    assert!(strategy.tools().is_empty());
    assert!((strategy.temperature() - 0.0).abs() < f32::EPSILON);
}

#[tokio::test]
async fn router_build_messages() {
    let strategy = RouterStrategy::new(RouterConfig {
        system_prompt: "route".into(),
        model_id: "m".into(),
        state: None,
        user_id: None,
    });

    let messages = strategy.build_messages("route this intent").await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].text(), "route this intent");
}

#[tokio::test]
async fn router_on_complete_is_noop() {
    let strategy = RouterStrategy::new(RouterConfig {
        system_prompt: "route".into(),
        model_id: "m".into(),
        state: None,
        user_id: None,
    });

    let usage = TokenUsage { input_tokens: 100, output_tokens: 50 };
    strategy.on_complete("{}", &usage).await.unwrap();
}
