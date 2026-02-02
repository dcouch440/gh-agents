//! RouterStrategy — wraps tool routing as a single-round execution.
//!
//! The router LLM outputs a JSON decision (not Anthropic tool_use).
//! Single round, no streaming, no tools.

use async_trait::async_trait;
use serde_json::Value;

use crate::llm::{Message, TokenUsage, Tool};

use super::super::error::HubError;
use super::super::strategy::ExecutionStrategy;

/// Configuration for a router execution.
pub struct RouterConfig {
    /// The router's system prompt (from DB config).
    pub system_prompt: String,
    /// The model to use for routing decisions.
    pub model_id: String,
}

/// Strategy for tool routing — single LLM call that outputs a JSON decision.
///
/// No tools (the router outputs JSON directly, not via tool_use).
/// No streaming. Max 1 round.
pub struct RouterStrategy {
    config: RouterConfig,
}

impl RouterStrategy {
    pub fn new(config: RouterConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ExecutionStrategy for RouterStrategy {
    fn system_prompt(&self) -> &str {
        &self.config.system_prompt
    }

    fn tools(&self) -> Vec<Tool> {
        vec![] // Router outputs JSON directly, no tool_use
    }

    fn model_id(&self) -> &str {
        &self.config.model_id
    }

    fn max_rounds(&self) -> u32 {
        1
    }

    fn context_budget(&self) -> usize {
        100_000 // Routing prompts are small
    }

    fn streaming(&self) -> bool {
        false
    }

    fn temperature(&self) -> f32 {
        0.0 // Deterministic routing
    }

    async fn build_messages(&self, input: &str) -> Result<Vec<Message>, HubError> {
        // Input is the full routing prompt (tool specs + context + intent)
        Ok(vec![Message::user(input)])
    }

    async fn execute_tool(&self, _name: &str, _input: &Value) -> Value {
        // Router never uses Anthropic tool_use
        serde_json::json!({"error": "router does not execute tools"})
    }

    async fn on_complete(&self, _response: &str, _usage: &TokenUsage) -> Result<(), HubError> {
        // Logging and request tracking are handled by the caller (RouterService)
        Ok(())
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn router_strategy_properties() {
        let strategy = RouterStrategy::new(RouterConfig {
            system_prompt: "You are a router.".into(),
            model_id: "claude-3-haiku".into(),
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
        });

        let usage = TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
        };
        strategy.on_complete("{}", &usage).await.unwrap();
    }
}
