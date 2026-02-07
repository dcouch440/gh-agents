//! DagStepStrategy — replaces the DAG executor's `execute_step` react loop.
//!
//! Handles a single workflow step execution: builds the prompt with variable
//! resolution, appends schema enforcement, executes execution tools (file ops,
//! git, etc.), and records results to agent_executions.

use async_trait::async_trait;
use serde_json::Value;
use tracing::info;
use uuid::Uuid;

use crate::agents::execution_tools;
use crate::db::{AgentRow, WorkflowStepRow};
use crate::execution::{ContainerHandle, ExecutionContext};
use crate::llm::{Message, TokenUsage, Tool};
use crate::server::state::AppState;

use super::super::error::HubError;
use super::super::strategy::ExecutionStrategy;

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
        false
    }

    fn temperature(&self) -> f32 {
        self.config.temperature // Use mode-resolved temperature
    }

    async fn build_messages(&self, _input: &str) -> Result<Vec<Message>, HubError> {
        // DAG steps use the pre-composed user prompt, not the raw input
        Ok(vec![Message::user(&self.config.user_prompt)])
    }

    async fn execute_tool(&self, name: &str, input: &Value) -> Value {
        // Container mode: route through docker exec
        if let Some(ref handle) = self.config.container_handle {
            info!(
                agent = %self.config.agent.name,
                tool = %name,
                container = %handle.container_name(),
                "DAG step tool call (container)"
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
                info!(
                    agent = %self.config.agent.name,
                    tool = %name,
                    "DAG step tool call"
                );
                execution_tools::execute_execution_tool(
                    name,
                    input,
                    exec_ctx,
                    Some(&self.config.tool_names),
                )
                .await
            }
            None => {
                serde_json::json!({ "error": "No execution context available for tool calls" })
            }
        }
    }

    async fn on_complete(&self, response: &str, usage: &TokenUsage) -> Result<(), HubError> {
        // Record token usage to ledger
        let tl_repo = &self.state.repos().token_ledger;
        let cost = super::compute_cost(
            &self.config.agent.model_id,
            usage.input_tokens as i64,
            usage.output_tokens as i64,
        );
        let _ = tl_repo
            .insert_ledger_entry(
                self.config.user_id,
                Some(self.config.agent_execution_id),
                &self.config.agent.model_id,
                usage.input_tokens as i64,
                usage.output_tokens as i64,
                cost,
            )
            .await;

        // Update agent_execution with final status
        let ae_repo = &self.state.repos().agent_executions;
        let structured = parse_structured_output(response);
        let _ = ae_repo
            .update_agent_execution_status(
                self.config.agent_execution_id,
                "completed",
                Some(response.to_string()),
                structured,
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
pub(crate) fn parse_structured_output(content: &str) -> Option<Value> {
    let trimmed = content.trim();

    // Try direct parse
    if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
        if v.is_object() || v.is_array() {
            return Some(v);
        }
    }

    // Try extracting from ```json code fence
    if let Some(start) = trimmed.find("```json") {
        if let Some(end) = trimmed[start + 7..].find("```") {
            let json_str = trimmed[start + 7..start + 7 + end].trim();
            if let Ok(v) = serde_json::from_str::<Value>(json_str) {
                return Some(v);
            }
        }
    }

    // Try extracting from ``` code fence
    if let Some(start) = trimmed.find("```") {
        if let Some(end) = trimmed[start + 3..].find("```") {
            let json_str = trimmed[start + 3..start + 3 + end].trim();
            if let Ok(v) = serde_json::from_str::<Value>(json_str) {
                return Some(v);
            }
        }
    }

    // Try finding { ... }
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            if let Ok(v) = serde_json::from_str::<Value>(&trimmed[start..=end]) {
                return Some(v);
            }
        }
    }

    None
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
