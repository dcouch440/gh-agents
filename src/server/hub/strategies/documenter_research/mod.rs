//! DocumenterResearchStrategy — Phase 2: research gathering.
//!
//! Multi-round tool-using strategy. The LLM uses capability-resolved tools
//! to gather information for a single document. Supports both execution-context
//! tools (filesystem, git) and context-free tools (web research).

use async_trait::async_trait;
use serde_json::Value;
use tracing::info;
use uuid::Uuid;

use crate::agents::execution_tools;
use crate::execution::ExecutionContext;
use crate::llm::{Message, Tool};
use crate::server::state::AppState;
use crate::types::UserId;

use super::super::error::HubError;
use super::super::strategy::ExecutionStrategy;

mod tests;

/// Configuration for the documenter research phase.
pub struct DocumenterResearchConfig {
    /// Research-oriented system prompt with document context.
    pub system_prompt: String,
    /// Model to use.
    pub model_id: String,
    /// Resolved tools from capability resolution.
    pub tools: Vec<Tool>,
    /// Allow-list for tool execution filtering.
    pub tool_names: Vec<String>,
    /// Optional execution context for filesystem/git tools.
    pub execution_context: Option<ExecutionContext>,
    /// Optional state for token ledger writes.
    pub state: Option<AppState>,
    /// Optional user ID for token ledger attribution.
    pub user_id: Option<UserId>,
}

/// Phase 2 strategy: use tools to research a single document.
pub struct DocumenterResearchStrategy {
    config: DocumenterResearchConfig,
}

impl DocumenterResearchStrategy {
    pub fn new(config: DocumenterResearchConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ExecutionStrategy for DocumenterResearchStrategy {
    fn system_prompt(&self) -> &str {
        &self.config.system_prompt
    }

    fn tools(&self) -> Vec<Tool> {
        self.config.tools.clone()
    }

    fn model_id(&self) -> &str {
        &self.config.model_id
    }

    fn max_rounds(&self) -> u32 {
        15
    }

    fn context_budget(&self) -> usize {
        480_000
    }

    fn streaming(&self) -> bool {
        false
    }

    fn temperature(&self) -> f32 {
        0.2
    }

    async fn build_messages(&self, input: &str) -> Result<Vec<Message>, HubError> {
        Ok(vec![Message::user(input)])
    }

    async fn execute_tool(&self, name: &str, input: &Value) -> Value {
        match &self.config.execution_context {
            Some(ctx) => {
                info!(tool = %name, "Documenter research tool call");
                execution_tools::execute_execution_tool(
                    name,
                    input,
                    ctx,
                    Some(&self.config.tool_names),
                )
                .await
            }
            None => {
                info!(tool = %name, "Documenter research tool call (context-free)");
                execution_tools::execute_context_free_tool(
                    name,
                    input,
                    Some(&self.config.tool_names),
                )
                .await
            }
        }
    }

    fn state(&self) -> Option<&AppState> {
        self.config.state.as_ref()
    }

    fn user_id(&self) -> Option<Uuid> {
        self.config.user_id.map(|u| u.0)
    }
}
