//! AgentDesignerStrategy — single-shot prompt generation for task force agents.
//!
//! The Agent Designer makes one LLM call to generate optimized (system prompt,
//! task prompt, tool assignment) tuples for each roster agent. No tools,
//! no streaming — a single prompt→response cycle that produces structured JSON.

mod tests;

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::llm::{Message, TokenUsage, Tool};
use crate::server::hub::error::HubError;
use crate::server::hub::strategies;
use crate::server::hub::strategy::ExecutionStrategy;
use crate::server::state::AppState;
use crate::types::UserId;

/// Configuration for the Agent Designer LLM call.
pub struct AgentDesignerConfig {
    /// System prompt with beliefs and output schema instructions.
    pub system_prompt: String,
    /// Model to use for prompt generation.
    pub model_id: String,
    /// LLM temperature.
    pub temperature: f32,
    /// Maximum execution rounds (always 1).
    pub max_rounds: u32,
    /// Maximum context size in characters.
    pub context_budget: usize,
    /// Optional state for token ledger writes.
    pub state: Option<AppState>,
    /// Optional user ID for token ledger attribution.
    pub user_id: Option<UserId>,
    /// Agent execution ID for message persistence and on_complete updates.
    pub agent_execution_id: Option<Uuid>,
}

/// Strategy for the Agent Designer pre-lifecycle LLM call.
///
/// Single-shot, no tools. Persists execution results via `on_complete`.
pub struct AgentDesignerStrategy {
    config: AgentDesignerConfig,
}

impl AgentDesignerStrategy {
    pub fn new(config: AgentDesignerConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ExecutionStrategy for AgentDesignerStrategy {
    fn system_prompt(&self) -> &str {
        &self.config.system_prompt
    }

    fn tools(&self) -> Vec<Tool> {
        vec![]
    }

    fn model_id(&self) -> &str {
        &self.config.model_id
    }

    fn max_rounds(&self) -> u32 {
        self.config.max_rounds
    }

    fn context_budget(&self) -> usize {
        self.config.context_budget
    }

    fn streaming(&self) -> bool {
        false
    }

    fn temperature(&self) -> f32 {
        self.config.temperature
    }

    async fn build_messages(&self, input: &str) -> Result<Vec<Message>, HubError> {
        Ok(vec![Message::user(input)])
    }

    fn state(&self) -> Option<&AppState> {
        self.config.state.as_ref()
    }

    fn user_id(&self) -> Option<Uuid> {
        self.config.user_id.map(|u| u.0)
    }

    fn agent_execution_id(&self) -> Option<Uuid> {
        self.config.agent_execution_id
    }

    async fn execute_tool(&self, _name: &str, _input: &Value) -> Value {
        serde_json::json!({"error": "agent designer does not execute tools"})
    }

    async fn on_complete(&self, response: &str, usage: &TokenUsage) -> Result<(), HubError> {
        strategies::complete_agent_execution(
            self.config.state.as_ref(),
            self.user_id(),
            self.config.agent_execution_id,
            self.model_id(),
            response,
            usage,
            true,
        )
        .await;
        Ok(())
    }
}
