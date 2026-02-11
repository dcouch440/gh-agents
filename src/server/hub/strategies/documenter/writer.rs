//! DocumenterWriterStrategy — Phase 3: document writing.
//!
//! Single-turn text generation. The LLM produces the final document
//! content based on the writer prompt and research output.
//! No tools, no streaming.

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::llm::{Message, Tool};
use crate::server::hub::error::HubError;
use crate::server::hub::strategy::ExecutionStrategy;
use crate::server::state::AppState;
use crate::types::UserId;

/// Configuration for the documenter writer phase.
pub struct DocumenterWriterConfig {
    /// Writer-oriented system prompt with tone/structure guidance.
    pub system_prompt: String,
    /// Model to use.
    pub model_id: String,
    /// LLM temperature.
    pub temperature: f32,
    /// Maximum execution rounds.
    pub max_rounds: u32,
    /// Maximum context size in characters.
    pub context_budget: usize,
    /// Optional state for token ledger writes.
    pub state: Option<AppState>,
    /// Optional user ID for token ledger attribution.
    pub user_id: Option<UserId>,
}

/// Phase 3 strategy: write the final document content.
pub struct DocumenterWriterStrategy {
    config: DocumenterWriterConfig,
}

impl DocumenterWriterStrategy {
    pub fn new(config: DocumenterWriterConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ExecutionStrategy for DocumenterWriterStrategy {
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

    async fn execute_tool(&self, _name: &str, _input: &Value) -> Value {
        serde_json::json!({"error": "documenter writer does not execute tools"})
    }
}
