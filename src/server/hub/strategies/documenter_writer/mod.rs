//! DocumenterWriterStrategy — Phase 3: document writing.
//!
//! Single-turn text generation. The LLM produces the final document
//! content based on the writer prompt and research output.
//! No tools, no streaming.

use async_trait::async_trait;
use serde_json::Value;

use crate::llm::{Message, TokenUsage, Tool};
use crate::server::state::AppState;
use crate::types::UserId;

use super::super::error::HubError;
use super::super::strategy::ExecutionStrategy;

mod tests;

/// Configuration for the documenter writer phase.
pub struct DocumenterWriterConfig {
    /// Writer-oriented system prompt with tone/structure guidance.
    pub system_prompt: String,
    /// Model to use.
    pub model_id: String,
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
        1
    }

    fn context_budget(&self) -> usize {
        480_000
    }

    fn streaming(&self) -> bool {
        false
    }

    fn temperature(&self) -> f32 {
        0.5
    }

    async fn build_messages(&self, input: &str) -> Result<Vec<Message>, HubError> {
        Ok(vec![Message::user(input)])
    }

    async fn execute_tool(&self, _name: &str, _input: &Value) -> Value {
        serde_json::json!({"error": "documenter writer does not execute tools"})
    }

    async fn on_complete(&self, _response: &str, usage: &TokenUsage) -> Result<(), HubError> {
        if let (Some(state), Some(user_id)) = (&self.config.state, &self.config.user_id) {
            let tl_repo = &state.repos().token_ledger;
            let cost = super::compute_cost(
                &self.config.model_id,
                usage.input_tokens as i64,
                usage.output_tokens as i64,
            );
            let _ = tl_repo
                .insert_ledger_entry(
                    user_id.0,
                    None,
                    &self.config.model_id,
                    usage.input_tokens as i64,
                    usage.output_tokens as i64,
                    cost,
                )
                .await;
        }
        Ok(())
    }
}
