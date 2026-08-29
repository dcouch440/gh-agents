//! Unified DB writes for all execution paths.
//!
//! Centralizes chat message persistence, agent execution tracking,
//! execution message logging, token ledger writes, and stage updates.

use anyhow::Context;
use uuid::Uuid;

use crate::db::traits::{
    AgentExecutionRepo, ChatMessageRepo, CreateAgentExecutionInput, SessionRepo, TokenLedgerRepo,
};
use crate::server::hub::error::HubError;
use crate::types::UserId;

/// Handles all database writes during execution.
///
/// Wraps the various repository traits so execution code doesn't need
/// to know which repo handles which table.
pub struct ExecutionRecorder<'a> {
    session_repo: &'a dyn SessionRepo,
    chat_message_repo: &'a dyn ChatMessageRepo,
    agent_execution_repo: Option<&'a dyn AgentExecutionRepo>,
    token_ledger_repo: Option<&'a dyn TokenLedgerRepo>,
}

impl<'a> ExecutionRecorder<'a> {
    pub fn new(
        session_repo: &'a dyn SessionRepo,
        chat_message_repo: &'a dyn ChatMessageRepo,
        agent_execution_repo: Option<&'a dyn AgentExecutionRepo>,
        token_ledger_repo: Option<&'a dyn TokenLedgerRepo>,
    ) -> Self {
        Self {
            session_repo,
            chat_message_repo,
            agent_execution_repo,
            token_ledger_repo,
        }
    }

    /// Record a chat message (global or session-scoped).
    pub async fn record_chat_message(
        &self,
        user_id: UserId,
        session_id: Option<Uuid>,
        message_id: Uuid,
        role: &str,
        content: &str,
    ) -> Result<(), HubError> {
        match session_id {
            Some(sid) => {
                self.session_repo
                    .insert_session_message(
                        user_id,
                        sid,
                        message_id,
                        role.to_string(),
                        content.to_string(),
                    )
                    .await
                    .context("failed to insert session message")?;
            }
            None => {
                self.chat_message_repo
                    .insert_chat_message(user_id, message_id, role.to_string(), content.to_string())
                    .await
                    .context("failed to insert chat message")?;
            }
        }
        Ok(())
    }

    /// Create an agent execution record for a DAG step.
    pub async fn record_agent_execution(
        &self,
        input: CreateAgentExecutionInput,
    ) -> Result<Uuid, HubError> {
        let repo = self
            .agent_execution_repo
            .ok_or_else(|| anyhow::anyhow!("agent_execution_repo not configured"))?;
        let row = repo
            .create_agent_execution(input)
            .await
            .context("failed to create agent execution")?;
        Ok(row.id)
    }

    /// Update an agent execution's final status.
    pub async fn update_agent_execution(
        &self,
        execution_id: Uuid,
        status: &str,
        output: Option<String>,
        structured_output: Option<serde_json::Value>,
    ) -> Result<(), HubError> {
        let repo = self
            .agent_execution_repo
            .ok_or_else(|| anyhow::anyhow!("agent_execution_repo not configured"))?;
        repo.update_agent_execution_status(execution_id, status, output, structured_output)
            .await
            .context("failed to update agent execution status")?;
        Ok(())
    }

    /// Record an execution message (tool call, result, text) within a DAG step.
    #[allow(clippy::too_many_arguments)]
    pub async fn record_execution_message(
        &self,
        agent_execution_id: Uuid,
        role: &str,
        content: &str,
        reasoning: Option<String>,
        tool_call_id: Option<String>,
        input_tokens: i64,
        output_tokens: i64,
    ) -> Result<(), HubError> {
        let repo = self
            .agent_execution_repo
            .ok_or_else(|| anyhow::anyhow!("agent_execution_repo not configured"))?;
        repo.create_execution_message(
            agent_execution_id,
            role,
            content,
            reasoning,
            tool_call_id,
            input_tokens,
            output_tokens,
        )
        .await
        .context("failed to create execution message")?;
        Ok(())
    }

    /// Write a token ledger entry for cost tracking.
    pub async fn record_tokens(
        &self,
        user_id: Uuid,
        agent_execution_id: Option<Uuid>,
        model_id: &str,
        input_tokens: i64,
        output_tokens: i64,
        cost_usd: f32,
    ) -> Result<(), HubError> {
        let repo = self
            .token_ledger_repo
            .ok_or_else(|| anyhow::anyhow!("token_ledger_repo not configured"))?;
        repo.insert_ledger_entry(
            user_id,
            agent_execution_id,
            model_id,
            input_tokens,
            output_tokens,
            cost_usd,
        )
        .await
        .context("failed to insert token ledger entry")?;
        Ok(())
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
