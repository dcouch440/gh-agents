//! WorkforceAgentStrategy — executes a single roster agent within a workforce.
//!
//! Each roster agent in a workforce step gets its own strategy instance with
//! capability-resolved tools. Supports 3-way tool dispatch: container (if
//! available) → local execution context → context-free tools.

mod tests;

use async_trait::async_trait;
use serde_json::Value;
use tracing::info;
use uuid::Uuid;

use crate::execution::{ContainerHandle, ExecutionContext};
use crate::llm::{ContentBlock, Message, TokenUsage, Tool};
use crate::server::hub::error::HubError;
use crate::server::hub::strategies;
use crate::server::hub::strategy::ExecutionStrategy;
use crate::server::state::AppState;
use crate::server::tools::execution as execution_tools;
use crate::server::tools::system_store;
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
    /// Base64-encoded PNG of pen strokes for vision-capable LLMs.
    pub stroke_image: Option<String>,
    /// Workflow ID for store tool scoping.
    pub workflow_id: Option<Uuid>,
    /// Step ID for store tool produced_by tracking.
    pub step_id: Option<Uuid>,
    /// Agent name for store tool produced_by_agent tracking.
    pub agent_name: Option<String>,
    /// Workflow run ID for scoping store artifacts to the current execution.
    pub workflow_run_id: Option<Uuid>,
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
        if let Some(ref image_data) = self.config.stroke_image {
            let blocks = vec![
                ContentBlock::Text {
                    text: input.to_string(),
                },
                ContentBlock::image_png_base64(image_data.clone()),
            ];
            Ok(vec![Message::user_with_blocks(blocks)])
        } else {
            Ok(vec![Message::user(input)])
        }
    }

    async fn execute_tool(&self, name: &str, input: &Value) -> Value {
        info!(tool = %name, "Workforce agent tool call");

        // Intercept store tools before the cascade (need workflow context)
        if matches!(name, "store_read_file" | "store_write_file") {
            if let (Some(state), Some(wf_id)) =
                (self.config.state.as_ref(), self.config.workflow_id)
            {
                if let Some(s3) = state.s3() {
                    let repo = state.repos().system_files.as_ref();
                    return match name {
                        "store_read_file" => {
                            system_store::execute_store_read_file(input, s3, repo, wf_id).await
                        }
                        "store_write_file" => {
                            system_store::execute_store_write_file(
                                input,
                                s3,
                                repo,
                                wf_id,
                                self.config.step_id.unwrap_or_default(),
                                self.config.agent_name.as_deref(),
                                self.config.workflow_run_id,
                            )
                            .await
                        }
                        _ => unreachable!(),
                    };
                }
            }
            return serde_json::json!({"error": "store not available"});
        }

        execution_tools::dispatch_tool_cascade(
            name,
            input,
            self.config.container_handle.as_ref(),
            self.config.execution_context.as_ref(),
            Some(&self.config.tool_names),
            self.config.state.as_ref(),
            self.config.user_id,
        )
        .await
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
