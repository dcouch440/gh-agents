//! WorkforceAgentStrategy — executes a single roster agent within a workforce.
//!
//! Each roster agent in a workforce step gets its own strategy instance with
//! capability-resolved tools. Supports 3-way tool dispatch: container (if
//! available) → local execution context → context-free tools.

mod tests;

use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::info;
use uuid::Uuid;

use crate::agents::execution_tools;
use crate::execution::{ContainerHandle, ExecutionContext};
use crate::llm::{Message, TokenUsage, Tool};
use crate::server::hub::error::HubError;
use crate::server::hub::strategies;
use crate::server::hub::strategy::ExecutionStrategy;
use crate::server::state::AppState;
use crate::types::UserId;

/// Configuration for a single workforce agent execution.
pub struct WorkforceAgentConfig {
    /// System prompt with agent identity, mission context, and previous outputs.
    pub system_prompt: String,
    /// Model to use.
    pub model_id: String,
    /// LLM temperature.
    pub temperature: f32,
    /// Maximum execution rounds.
    pub max_rounds: u32,
    /// Maximum context size in characters.
    pub context_budget: usize,
    /// Resolved tools from capability resolution.
    pub tools: Vec<Tool>,
    /// Allow-list for tool execution filtering.
    pub tool_names: Vec<String>,
    /// Optional execution context for filesystem/git tools.
    pub execution_context: Option<ExecutionContext>,
    /// Optional container handle for containerized execution.
    pub container_handle: Option<ContainerHandle>,
    /// Optional state for token ledger writes.
    pub state: Option<AppState>,
    /// Optional user ID for token ledger attribution.
    pub user_id: Option<UserId>,
    /// Agent execution ID for message persistence and on_complete updates.
    pub agent_execution_id: Option<Uuid>,
}

/// Strategy for executing a single agent within a workforce roster.
///
/// Supports 3-way tool dispatch (container → local → context-free) and
/// persists execution results via `on_complete`.
pub struct WorkforceAgentStrategy {
    config: WorkforceAgentConfig,
}

impl WorkforceAgentStrategy {
    pub fn new(config: WorkforceAgentConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ExecutionStrategy for WorkforceAgentStrategy {
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
        self.config.max_rounds
    }

    fn context_budget(&self) -> usize {
        self.config.context_budget
    }

    fn streaming(&self) -> bool {
        true
    }

    fn temperature(&self) -> f32 {
        self.config.temperature
    }

    async fn build_messages(&self, input: &str) -> Result<Vec<Message>, HubError> {
        Ok(vec![Message::user(input)])
    }

    async fn execute_tool(&self, name: &str, input: &Value) -> Value {
        // Stateful tools that need DB access (AppState + optionally UserId).
        // These are intercepted before the container/local cascade because they
        // operate on the knowledge base, not the filesystem.
        match name {
            "read_document" => {
                if let Some(ref state) = self.config.state {
                    return crate::server::tools::documents::execute_read_document(input, state)
                        .await;
                }
                return json!({ "error": "Document reading not available in this context" });
            }
            "create_doc" => {
                if let (Some(ref state), Some(user_id)) = (&self.config.state, self.config.user_id)
                {
                    return crate::server::tools::documents::execute_create_doc(
                        input, state, user_id,
                    )
                    .await;
                }
                return json!({ "error": "Document creation not available in this context" });
            }
            "update_doc" => {
                if let Some(ref state) = self.config.state {
                    return crate::server::tools::documents::execute_update_doc(input, state).await;
                }
                return json!({ "error": "Document update not available in this context" });
            }
            "search_docs" => {
                if let (Some(ref state), Some(user_id)) = (&self.config.state, self.config.user_id)
                {
                    return crate::server::tools::documents::execute_search_docs(
                        input, state, user_id,
                    )
                    .await;
                }
                return json!({ "error": "Document search not available in this context" });
            }
            _ => {}
        }

        // Container mode: route through docker exec
        if let Some(ref handle) = self.config.container_handle {
            info!(
                tool = %name,
                container = %handle.container_name(),
                "Workforce agent tool call (container)"
            );
            return execution_tools::execute_tool_in_container(
                name,
                input,
                handle,
                Some(&self.config.tool_names),
            )
            .await;
        }

        // Local mode: use host execution context
        match &self.config.execution_context {
            Some(exec_ctx) => {
                info!(tool = %name, "Workforce agent tool call");
                execution_tools::execute_execution_tool(
                    name,
                    input,
                    exec_ctx,
                    Some(&self.config.tool_names),
                )
                .await
            }
            None => {
                // No local execution context — try context-free tools
                info!(tool = %name, "Workforce agent tool call (context-free)");
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

    fn agent_execution_id(&self) -> Option<Uuid> {
        self.config.agent_execution_id
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
