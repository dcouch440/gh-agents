//! DagStepStrategy — replaces the DAG executor's `execute_step` react loop.
//!
//! Handles a single workflow step execution: builds the prompt with variable
//! resolution, appends schema enforcement, executes execution tools (file ops,
//! git, etc.), and records results to agent_executions.

use async_trait::async_trait;
use serde_json::Value;
use tracing::info;
use uuid::Uuid;

use crate::db::{AgentRow, WorkflowStepRow};
use crate::execution::{ContainerHandle, ExecutionContext};
use crate::llm::{ContentBlock, Message, TokenUsage, Tool};
use crate::server::state::AppState;
use crate::server::tools::execution as execution_tools;
use crate::types::UserId;

use super::super::strategy::ExecutionStrategy;
use crate::server::hub::error::HubError;

/// Configuration for a DAG step execution.
pub struct DagStepConfig {
    /// The agent executing this step.
    pub agent: AgentRow,
    /// The workflow step definition.
    pub step: WorkflowStepRow,
    /// Composed system prompt (agent prompt + schema enforcement if applicable).
    pub system_prompt: String,
    /// The composed user prompt (template rendered with variables).
    pub user_prompt: String,
    /// Tool definitions resolved from agent's tool assignments.
    pub tools: Vec<Tool>,
    /// Tool name allow-list for execution_tools.
    pub tool_names: Vec<String>,
    /// Temperature for sampling (from mode resolution or agent default).
    pub temperature: f32,
    /// Maximum output tokens for one call.
    pub max_tokens: u32,
    /// Reasoning effort, for providers that support it.
    pub effort: Option<crate::llm::ReasoningEffort>,
    /// Execution context for file/git/test tool calls (local mode).
    pub execution_context: Option<ExecutionContext>,
    /// Container handle for containerized execution (overrides local context).
    pub container_handle: Option<ContainerHandle>,
    /// Pipeline run ID for broadcasting.
    pub run_id: Uuid,
    /// User ID for token ledger.
    pub user_id: Uuid,
    /// Agent execution ID (created before calling the engine).
    pub agent_execution_id: Uuid,
    /// Base64-encoded PNG of pen strokes for vision-capable LLMs.
    pub board_context_image: Option<String>,
}

/// Strategy for DAG workflow step execution.
///
/// Uses the agent's model/temperature, execution tools (not server tools),
/// and records per-round token usage.
pub struct DagStepStrategy {
    config: DagStepConfig,
    state: AppState,
}

impl DagStepStrategy {
    pub fn new(config: DagStepConfig, state: AppState) -> Self {
        Self { config, state }
    }

    /// Get the agent execution ID for recording results.
    pub fn agent_execution_id(&self) -> Uuid {
        self.config.agent_execution_id
    }

    /// Get the step for post-processing (output variable name, etc.).
    pub fn step(&self) -> &WorkflowStepRow {
        &self.config.step
    }

    /// Get the agent for metadata.
    pub fn agent(&self) -> &AgentRow {
        &self.config.agent
    }
}

#[async_trait]
impl ExecutionStrategy for DagStepStrategy {
    fn system_prompt(&self) -> &str {
        &self.config.system_prompt
    }

    fn tools(&self) -> Vec<Tool> {
        self.config.tools.clone()
    }

    fn model_id(&self) -> &str {
        &self.config.agent.model_id
    }

    fn max_rounds(&self) -> u32 {
        15
    }

    fn context_budget(&self) -> usize {
        480_000
    }

    fn streaming(&self) -> bool {
        true
    }

    fn temperature(&self) -> f32 {
        self.config.temperature // Use mode-resolved temperature
    }

    fn max_tokens(&self) -> u32 {
        self.config.max_tokens
    }

    fn effort(&self) -> Option<crate::llm::ReasoningEffort> {
        self.config.effort
    }

    async fn build_messages(&self, _input: &str) -> Result<Vec<Message>, HubError> {
        // DAG steps use the pre-composed user prompt, not the raw input.
        // When a stroke image is available, send as multimodal (text + image).
        if let Some(ref image_data) = self.config.board_context_image {
            let blocks = vec![
                ContentBlock::Text {
                    text: self.config.user_prompt.clone(),
                },
                ContentBlock::image_png_base64(image_data.clone()),
            ];
            Ok(vec![Message::user_with_blocks(blocks)])
        } else {
            Ok(vec![Message::user(&self.config.user_prompt)])
        }
    }

    async fn execute_tool(&self, name: &str, input: &Value) -> Value {
        info!(agent = %self.config.agent.name, tool = %name, "DAG step tool call");
        execution_tools::dispatch_tool_cascade(
            name,
            input,
            self.config.container_handle.as_ref(),
            self.config.execution_context.as_ref(),
            Some(&self.config.tool_names),
            Some(&self.state),
            Some(UserId(self.config.user_id)),
        )
        .await
    }

    fn state(&self) -> Option<&AppState> {
        Some(&self.state)
    }

    fn user_id(&self) -> Option<Uuid> {
        Some(self.config.user_id)
    }

    fn agent_execution_id(&self) -> Option<Uuid> {
        Some(self.config.agent_execution_id)
    }

    async fn on_complete(&self, response: &str, usage: &TokenUsage) -> Result<(), HubError> {
        super::complete_agent_execution(
            Some(&self.state),
            Some(self.config.user_id),
            Some(self.config.agent_execution_id),
            &self.config.agent.model_id,
            response,
            usage,
            true,
        )
        .await;
        Ok(())
    }
}

impl DagStepStrategy {
    /// Parse structured output from raw LLM response (public for dag.rs).
    pub fn parse_output(content: &str) -> Option<Value> {
        parse_structured_output(content)
    }
}

/// Re-export compute_cost from the parent module for backward compatibility.
pub use super::compute_cost;

/// Try to parse JSON from the LLM's final response.
///
/// Delegates to the shared [`json_utils::parse_structured_output`] utility.
pub(crate) fn parse_structured_output(content: &str) -> Option<Value> {
    crate::server::hub::protocols::json_utils::parse_structured_output(content)
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
