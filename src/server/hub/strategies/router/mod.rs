//! RouterStrategy — wraps tool routing as a single-round execution.
//!
//! The router LLM outputs a JSON decision (not Anthropic tool_use).
//! Single round, no streaming, no tools.

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::llm::{Message, Tool};
use crate::server::state::AppState;
use crate::types::UserId;

use super::super::error::HubError;
use super::super::strategy::ExecutionStrategy;

/// Configuration for a router execution.
pub struct RouterConfig {
    /// The router's system prompt (from DB config).
    pub system_prompt: String,
    /// The model to use for routing decisions.
    pub model_id: String,
    /// Optional state + user for token ledger writes.
    pub state: Option<AppState>,
    pub user_id: Option<UserId>,
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

    fn state(&self) -> Option<&AppState> {
        self.config.state.as_ref()
    }

    fn user_id(&self) -> Option<Uuid> {
        self.config.user_id.map(|u| u.0)
    }

    async fn execute_tool(&self, _name: &str, _input: &Value) -> Value {
        // Router never uses Anthropic tool_use
        serde_json::json!({"error": "router does not execute tools"})
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
