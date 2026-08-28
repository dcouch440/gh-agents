//! ExecutionStrategy trait — parameterizes the execution engine.
//!
//! Each strategy defines how to build messages, which tools to use,
//! how to execute tool calls, and what to do after completion.
//!
//! The default `on_complete` logs token usage to the ledger via
//! `strategies::log_token_usage`. Strategies that need custom
//! post-processing override `on_complete` and call `log_token_usage`
//! directly for the shared ledger logic.

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::llm::{Message, TokenUsage, Tool};
use crate::server::state::AppState;

use super::strategies;
use crate::server::hub::error::HubError;

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

    /// Maximum output tokens for one call.
    ///
    /// Defaulted so existing strategies keep their current behaviour; override
    /// to honour a per-agent limit.
    fn max_tokens(&self) -> u32 {
        crate::constants::DEFAULT_MAX_TOKENS_WORKER
    }

    /// Reasoning effort, for providers that support it.
    ///
    /// `None` omits the parameter so the provider applies its own default.
    fn effort(&self) -> Option<crate::llm::ReasoningEffort> {
        None
    }

    /// App state for token ledger and DB access. Return `None` to skip ledger logging.
    fn state(&self) -> Option<&AppState> {
        None
    }

    /// User ID for token attribution. Return `None` to skip ledger logging.
    fn user_id(&self) -> Option<Uuid> {
        None
    }

    /// Agent execution ID for ledger correlation. Most strategies return `None`.
    fn agent_execution_id(&self) -> Option<Uuid> {
        None
    }

    /// Build the initial message list (history + current input).
    async fn build_messages(&self, input: &str) -> Result<Vec<Message>, HubError>;

    /// Execute a tool call. Returns the tool result as JSON.
    async fn execute_tool(&self, name: &str, input: &Value) -> Value;

    /// Whether the engine should stop after the current tool-use round.
    ///
    /// Strategies that have terminal tools (e.g. `complete_task` in dispatch)
    /// return `true` once their terminal tool has been called. The engine
    /// checks this after executing all tools in a round and breaks the loop
    /// instead of sending another LLM request.
    fn should_stop(&self) -> bool {
        false
    }

    /// If this strategy requires a terminal tool before completion,
    /// return its name. The engine re-prompts the LLM on premature EndTurn.
    fn requires_terminal_tool(&self) -> Option<&str> {
        None
    }

    /// Post-processing after the final LLM response.
    ///
    /// The default implementation logs token usage to the ledger.
    /// Override to add custom behavior (agent execution updates, message saving, etc.)
    /// and call [`strategies::log_token_usage`] for the shared ledger logic.
    async fn on_complete(&self, _response: &str, usage: &TokenUsage) -> Result<(), HubError> {
        if let (Some(state), Some(uid)) = (self.state(), self.user_id()) {
            strategies::log_token_usage(
                state,
                uid,
                self.agent_execution_id(),
                self.model_id(),
                usage,
            )
            .await;
        }
        Ok(())
    }
}
