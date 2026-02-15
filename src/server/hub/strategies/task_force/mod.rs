//! TaskForceAgentStrategy — executes a single roster agent within a task force.
//!
//! Each roster agent in a task force step gets its own strategy instance with
//! capability-resolved tools. Supports 3-way tool dispatch: container (if
//! available) → local execution context → context-free tools.

mod tests;

use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::info;
use uuid::Uuid;

use crate::agents::execution_tools;
use crate::execution::{ContainerHandle, ExecutionContext};
use crate::llm::{Message, Tool};
use crate::server::hub::error::HubError;
use crate::server::hub::strategy::ExecutionStrategy;
use crate::server::state::AppState;
use crate::types::UserId;

/// Configuration for a single task force agent execution.
pub struct TaskForceAgentConfig {
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
}

/// Strategy for executing a single agent within a task force roster.
///
/// Supports 3-way tool dispatch (container → local → context-free) and
/// logs token usage via the default `on_complete` implementation.
pub struct TaskForceAgentStrategy {
    config: TaskForceAgentConfig,
}

impl TaskForceAgentStrategy {
    pub fn new(config: TaskForceAgentConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ExecutionStrategy for TaskForceAgentStrategy {
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
        false
    }

    fn temperature(&self) -> f32 {
        self.config.temperature
    }

    async fn build_messages(&self, input: &str) -> Result<Vec<Message>, HubError> {
        Ok(vec![Message::user(input)])
    }

    async fn execute_tool(&self, name: &str, input: &Value) -> Value {
        // Stateful tools that need DB access (AppState)
        if name == "read_document" {
            if let Some(ref state) = self.config.state {
                return crate::server::tools::documents::execute_read_document(input, state)
                    .await;
            }
            return json!({ "error": "Document reading not available in this context" });
        }

        // Container mode: route through docker exec
        if let Some(ref handle) = self.config.container_handle {
            info!(
                tool = %name,
                container = %handle.container_name(),
                "Task force agent tool call (container)"
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
                info!(tool = %name, "Task force agent tool call");
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
                info!(tool = %name, "Task force agent tool call (context-free)");
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
