//! InteractiveChatStrategy — handles LLM responses in interactive review sessions.
//!
//! When a user sends a message to an interactive agent execution (review queue),
//! this strategy loads the full conversation from `execution_messages`, calls the
//! LLM with streaming, and records the assistant response back to the DB.

use async_trait::async_trait;
use serde_json::Value;
use tracing::{error, info};
use uuid::Uuid;

use crate::db::AgentRow;
use crate::llm::{Message, TokenUsage, Tool};
use crate::server::state::AppState;
use crate::server::tools;

use super::super::error::HubError;
use super::super::strategy::ExecutionStrategy;

/// Configuration for an interactive chat execution.
pub struct InteractiveChatConfig {
    /// The interactive agent handling the review.
    pub agent: AgentRow,
    /// The agent_execution this conversation belongs to.
    pub agent_execution_id: Uuid,
    /// System prompt (from the original execution or mode-resolved).
    pub system_prompt: String,
    /// Tool definitions resolved from mode or agent defaults.
    pub tools: Vec<Tool>,
    /// Tool name allow-list.
    pub tool_names: Vec<String>,
    /// Sampling temperature (from mode resolution or agent default).
    pub temperature: f32,
    /// User ID for token ledger tracking.
    pub user_id: Uuid,
}

/// Strategy for interactive review chat sessions.
///
/// Loads conversation history from `execution_messages`, streams tokens to the
/// client, and records the assistant response. Status stays `awaiting_user`
/// until the user explicitly approves.
pub struct InteractiveChatStrategy {
    config: InteractiveChatConfig,
    state: AppState,
}

impl InteractiveChatStrategy {
    pub fn new(config: InteractiveChatConfig, state: AppState) -> Self {
        Self { config, state }
    }
}

#[async_trait]
impl ExecutionStrategy for InteractiveChatStrategy {
    fn system_prompt(&self) -> &str {
        &self.config.system_prompt
    }

    fn tools(&self) -> Vec<Tool> {
        tools::filtered_tools(&self.config.tool_names)
    }

    fn model_id(&self) -> &str {
        &self.config.agent.model_id
    }

    fn max_rounds(&self) -> u32 {
        10
    }

    fn context_budget(&self) -> usize {
        480_000
    }

    fn streaming(&self) -> bool {
        true
    }

    fn temperature(&self) -> f32 {
        self.config.temperature
    }

    async fn build_messages(&self, input: &str) -> Result<Vec<Message>, HubError> {
        let ae_repo =
            self.state.agent_execution_repo.as_ref().ok_or_else(|| {
                HubError::Internal(anyhow::anyhow!("agent_execution_repo missing"))
            })?;

        let rows = ae_repo
            .list_execution_messages(self.config.agent_execution_id)
            .await
            .map_err(|e| HubError::Internal(anyhow::anyhow!("failed to load messages: {}", e)))?;

        let mut messages = Vec::new();
        for row in &rows {
            match row.role.as_str() {
                "user" => messages.push(Message::user(&row.content)),
                "assistant" => messages.push(Message::assistant(&row.content)),
                _ => continue, // Skip system messages
            }
        }

        // Ensure the current message is included (it was just recorded by the handler)
        if !messages
            .iter()
            .any(|m| m.role == crate::llm::Role::User && m.text() == input)
        {
            messages.push(Message::user(input));
        }

        Ok(messages)
    }

    async fn execute_tool(&self, name: &str, input: &Value) -> Value {
        // Interactive agents use server tools (read-only operations like search, docs)
        tools::execute_tool(
            name,
            input,
            &self.state,
            crate::types::UserId(self.config.user_id),
            None,
        )
        .await
    }

    async fn on_complete(&self, response: &str, usage: &TokenUsage) -> Result<(), HubError> {
        // Record token usage to ledger
        if let Some(tl_repo) = &self.state.token_ledger_repo {
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
        }

        // Record assistant response as execution message
        if let Some(ae_repo) = &self.state.agent_execution_repo {
            if let Err(e) = ae_repo
                .create_execution_message(
                    self.config.agent_execution_id,
                    "assistant",
                    response,
                    None,
                    usage.input_tokens as i64,
                    usage.output_tokens as i64,
                )
                .await
            {
                error!("Failed to record interactive assistant message: {}", e);
            }
        }

        // Update agent_execution output (but NOT status — keep awaiting_user)
        if let Some(ae_repo) = &self.state.agent_execution_repo {
            let _ = ae_repo
                .update_agent_execution_status(
                    self.config.agent_execution_id,
                    "awaiting_user", // Preserve status
                    Some(response.to_string()),
                    super::super::strategies::dag_step::parse_structured_output(response),
                )
                .await;
        }

        info!(
            agent_execution_id = %self.config.agent_execution_id,
            input_tokens = usage.input_tokens,
            output_tokens = usage.output_tokens,
            "Interactive chat response complete"
        );

        Ok(())
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
