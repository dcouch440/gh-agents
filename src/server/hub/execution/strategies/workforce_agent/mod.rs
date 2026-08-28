//! WorkforceAgentStrategy — executes a single roster agent within a workforce.
//!
//! Each roster agent in a workforce step gets its own strategy instance with
//! capability-resolved tools. Supports 3-way tool dispatch: container (if
//! available) → local execution context → context-free tools.

mod tests;

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::info;
use uuid::Uuid;

use crate::execution::diagnostics::DiagnosticsEngine;
use crate::execution::{ContainerHandle, ExecutionContext};
use crate::llm::{ContentBlock, Message, TokenUsage, Tool};
use crate::server::hub::error::HubError;
use crate::server::hub::strategies;
use crate::server::hub::strategy::ExecutionStrategy;
use crate::server::state::AppState;
use crate::server::tools::execution as execution_tools;
use crate::types::UserId;

/// Configuration for a single workforce agent execution.
pub struct WorkforceAgentConfig {
    /// System prompt with agent identity, mission context, and previous outputs.
    pub system_prompt: String,
    /// Model to use.
    pub model_id: String,
    /// LLM temperature.
    pub temperature: f32,
    /// Maximum output tokens for one call.
    pub max_tokens: u32,
    /// Reasoning effort, for providers that support it.
    pub effort: Option<crate::llm::ReasoningEffort>,
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
    /// Diagnostics engine for enriched run_command feedback (per-agent, stateful).
    pub diagnostics: Option<Arc<tokio::sync::Mutex<DiagnosticsEngine>>>,
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

    fn max_tokens(&self) -> u32 {
        self.config.max_tokens
    }

    fn effort(&self) -> Option<crate::llm::ReasoningEffort> {
        self.config.effort
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

        // The allow-list is the only thing that makes `read_only` real, and
        // both intercepts below bypass the cascade that normally checks it.
        // Check once, here, before any branch.
        if !crate::server::tools::shared::is_tool_allowed(name, Some(&self.config.tool_names)) {
            return crate::server::tools::shared::tool_not_allowed_error(name);
        }

        // File-tool intercept: dispatch normally, then feed the write back into
        // diagnostics so the passdown manifest and loop detector still see it.
        if matches!(name, "write_file" | "edit_file") {
            let result = execution_tools::dispatch_tool_cascade(
                name,
                input,
                self.config.container_handle.as_ref(),
                self.config.execution_context.as_ref(),
                Some(&self.config.tool_names),
                self.config.state.as_ref(),
                self.config.user_id,
            )
            .await;

            if let (Some(diag), Some(path)) = (&self.config.diagnostics, input["path"].as_str()) {
                if result.get("error").is_none() {
                    // write_file reports `overwrote`; edit_file only ever
                    // touches a file that already exists.
                    let change_type =
                        if name == "edit_file" || result["overwrote"].as_bool().unwrap_or(false) {
                            crate::execution::diagnostics::types::ChangeType::Modified
                        } else {
                            crate::execution::diagnostics::types::ChangeType::Created
                        };
                    let size = result["bytes"].as_u64().unwrap_or(0);
                    let status = diag.lock().await.record_file_write(
                        std::path::PathBuf::from(path),
                        change_type,
                        size,
                    );
                    if status.should_render() {
                        let mut result = result;
                        result["loop_warning"] = Value::String(status.render());
                        return result;
                    }
                }
            }
            return result;
        }

        // Diagnostics intercept: enrich run_command with pre-checks,
        // filesystem observation, and structured feedback.
        if name == "run_command" {
            if let (Some(diag), Some(handle)) =
                (&self.config.diagnostics, &self.config.container_handle)
            {
                let command = match input["command"].as_str() {
                    Some(c) => crate::execution::diagnostics::html_unescape(c),
                    None => {
                        return json!({ "error": "Missing required parameter: command" });
                    }
                };
                let mut engine = diag.lock().await;
                return match engine.execute(&command, handle).await {
                    // A bare string reaches the model verbatim; wrapping the
                    // rendered envelope in an object would deliver it as JSON
                    // with escaped newlines and undo the formatting.
                    Ok(rendered) => Value::String(rendered),
                    Err(e) => json!({ "error": e.to_string() }),
                };
            }
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
