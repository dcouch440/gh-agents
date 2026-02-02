//! ExecutionStrategy trait — parameterizes the execution engine.
//!
//! Each strategy defines how to build messages, which tools to use,
//! how to execute tool calls, and what to do after completion.

use async_trait::async_trait;
use serde_json::Value;

use crate::llm::{Message, Tool, TokenUsage};

use super::error::HubError;

/// A strategy that parameterizes the execution engine loop.
///
/// The engine handles the LLM call cycle; the strategy decides
/// system prompt, tools, message construction, tool execution,
/// and post-processing.
#[async_trait]
pub trait ExecutionStrategy: Send + Sync {
    /// System prompt for the LLM.
    fn system_prompt(&self) -> &str;

    /// Tools available for the LLM to call.
    fn tools(&self) -> Vec<Tool>;

    /// Model identifier (e.g. "claude-sonnet-4-20250514").
    fn model_id(&self) -> &str;

    /// Maximum number of tool-use rounds before stopping.
    fn max_rounds(&self) -> u32;

    /// Maximum total character count across all messages.
    fn context_budget(&self) -> usize;

    /// Whether to stream tokens to the sink.
    fn streaming(&self) -> bool;

    /// Sampling temperature.
    fn temperature(&self) -> f32;

    /// Build the initial message list (history + current input).
    async fn build_messages(&self, input: &str) -> Result<Vec<Message>, HubError>;

    /// Execute a tool call. Returns the tool result as JSON.
    async fn execute_tool(&self, name: &str, input: &Value) -> Value;

    /// Post-processing after the final LLM response.
    async fn on_complete(&self, response: &str, usage: &TokenUsage) -> Result<(), HubError>;
}
