//! DagStepStrategy — replaces the DAG executor's `execute_step` react loop.
//!
//! Handles a single workflow step execution: builds the prompt with variable
//! resolution, appends schema enforcement, executes execution tools (file ops,
//! git, etc.), and records results to agent_executions.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tracing::info;
use uuid::Uuid;

use crate::agents::execution_tools;
use crate::db::{AgentRow, WorkflowStepRow};
use crate::execution::ExecutionContext;
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
    /// Execution context for file/git/test tool calls.
    pub execution_context: Option<ExecutionContext>,
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
        self.config.agent.model_temperature
    }

    async fn build_messages(&self, _input: &str) -> Result<Vec<Message>, HubError> {
        // DAG steps use the pre-composed user prompt, not the raw input
        Ok(vec![Message::user(&self.config.user_prompt)])
    }

    async fn execute_tool(&self, name: &str, input: &Value) -> Value {
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
        if let Some(tl_repo) = &self.state.token_ledger_repo {
            let cost = compute_cost(
                &self.config.agent.model_id,
                usage.input_tokens as i64,
                usage.output_tokens as i64,
            );
            let _ = tl_repo
                .insert_ledger_entry(
                    self.config.user_id,
                    self.config.agent_execution_id,
                    &self.config.agent.model_id,
                    usage.input_tokens as i64,
                    usage.output_tokens as i64,
                    cost,
                )
                .await;
        }

        // Update agent_execution with final status
        if let Some(ae_repo) = &self.state.agent_execution_repo {
            let structured = parse_structured_output(response);
            let cost = compute_cost(
                &self.config.agent.model_id,
                usage.input_tokens as i64,
                usage.output_tokens as i64,
            );
            let _ = ae_repo
                .update_agent_execution_status(
                    self.config.agent_execution_id,
                    "completed",
                    Some(response.to_string()),
                    structured,
                    usage.input_tokens as i64,
                    usage.output_tokens as i64,
                    cost,
                )
                .await;
        }

        Ok(())
    }
}

/// Approximate cost computation per model.
pub fn compute_cost(model_id: &str, input_tokens: i64, output_tokens: i64) -> f32 {
    let (input_rate, output_rate) = if model_id.contains("opus") {
        (15.0_f32, 75.0_f32)
    } else if model_id.contains("sonnet") {
        (3.0, 15.0)
    } else if model_id.contains("haiku") {
        (0.25, 1.25)
    } else if model_id.contains("gpt-4o") {
        (2.5, 10.0)
    } else if model_id.contains("gpt-4") {
        (30.0, 60.0)
    } else {
        (1.0, 3.0)
    };

    let input_cost = (input_tokens as f32 / 1_000_000.0) * input_rate;
    let output_cost = (output_tokens as f32 / 1_000_000.0) * output_rate;
    input_cost + output_cost
}

/// Try to parse JSON from the LLM's final response.
fn parse_structured_output(content: &str) -> Option<Value> {
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
mod tests {
    use super::*;

    #[test]
    fn compute_cost_sonnet() {
        let cost = compute_cost("claude-sonnet-4-20250514", 1_000_000, 500_000);
        // 3.0 + 7.5 = 10.5
        assert!((cost - 10.5).abs() < 0.01);
    }

    #[test]
    fn compute_cost_haiku() {
        let cost = compute_cost("claude-3-haiku", 1_000_000, 1_000_000);
        // 0.25 + 1.25 = 1.50
        assert!((cost - 1.50).abs() < 0.01);
    }

    #[test]
    fn compute_cost_opus() {
        let cost = compute_cost("claude-3-opus", 100_000, 50_000);
        // 1.5 + 3.75 = 5.25
        assert!((cost - 5.25).abs() < 0.01);
    }

    #[test]
    fn parse_structured_output_direct_json() {
        let result = parse_structured_output(r#"{"key": "value"}"#);
        assert!(result.is_some());
        assert_eq!(result.unwrap()["key"], "value");
    }

    #[test]
    fn parse_structured_output_code_fence() {
        let input = "Here is the result:\n```json\n{\"key\": \"value\"}\n```";
        let result = parse_structured_output(input);
        assert!(result.is_some());
        assert_eq!(result.unwrap()["key"], "value");
    }

    #[test]
    fn parse_structured_output_embedded_json() {
        let input = "The answer is {\"key\": \"value\"} as shown.";
        let result = parse_structured_output(input);
        assert!(result.is_some());
    }

    #[test]
    fn parse_structured_output_plain_text() {
        let result = parse_structured_output("Just plain text, no JSON here.");
        assert!(result.is_none());
    }

    #[test]
    fn parse_structured_output_array() {
        let result = parse_structured_output(r#"[{"a": 1}, {"a": 2}]"#);
        assert!(result.is_some());
        assert!(result.unwrap().is_array());
    }
}
