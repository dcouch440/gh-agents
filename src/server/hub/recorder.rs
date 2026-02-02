//! Unified DB writes for all execution paths.
//!
//! Centralizes chat message persistence, agent execution tracking,
//! execution message logging, token ledger writes, and stage updates.

use anyhow::Context;
use uuid::Uuid;

use crate::db::traits::{AgentExecutionRepo, ServerRepo, TokenLedgerRepo};
use crate::server::hub::error::HubError;
use crate::types::UserId;

/// Handles all database writes during execution.
///
/// Wraps the various repository traits so execution code doesn't need
/// to know which repo handles which table.
pub struct ExecutionRecorder<'a> {
    repo: &'a dyn ServerRepo,
    agent_execution_repo: Option<&'a dyn AgentExecutionRepo>,
    token_ledger_repo: Option<&'a dyn TokenLedgerRepo>,
}

impl<'a> ExecutionRecorder<'a> {
    pub fn new(
        repo: &'a dyn ServerRepo,
        agent_execution_repo: Option<&'a dyn AgentExecutionRepo>,
        token_ledger_repo: Option<&'a dyn TokenLedgerRepo>,
    ) -> Self {
        Self {
            repo,
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
                self.repo
                    .insert_session_message(user_id, sid, message_id, role.to_string(), content.to_string())
                    .await
                    .context("failed to insert session message")?;
            }
            None => {
                self.repo
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
        stage_execution_id: Uuid,
        agent_id: Uuid,
        workflow_step_id: Option<Uuid>,
        is_interactive: bool,
        parent_agent_execution_id: Option<Uuid>,
        system_prompt: &str,
        user_prompt: &str,
        selected_mode_id: Option<Uuid>,
    ) -> Result<Uuid, HubError> {
        let repo = self
            .agent_execution_repo
            .ok_or_else(|| anyhow::anyhow!("agent_execution_repo not configured"))?;
        let row = repo
            .create_agent_execution(
                stage_execution_id,
                agent_id,
                workflow_step_id,
                is_interactive,
                parent_agent_execution_id,
                system_prompt,
                user_prompt,
                selected_mode_id,
            )
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
        repo.update_agent_execution_status(
            execution_id,
            status,
            output,
            structured_output,
        )
        .await
        .context("failed to update agent execution status")?;
        Ok(())
    }

    /// Record an execution message (tool call, result, text) within a DAG step.
    pub async fn record_execution_message(
        &self,
        agent_execution_id: Uuid,
        role: &str,
        content: &str,
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
        repo.insert_ledger_entry(user_id, agent_execution_id, model_id, input_tokens, output_tokens, cost_usd)
            .await
            .context("failed to insert token ledger entry")?;
        Ok(())
    }

    /// Update a stage execution row.
    pub async fn record_stage_update(
        &self,
        exec: &crate::db::StageExecutionRow,
    ) -> Result<(), HubError> {
        self.repo
            .update_stage_execution(exec)
            .await
            .context("failed to update stage execution")?;
        Ok(())
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::traits::{MockAgentExecutionRepo, MockServerRepo, MockTokenLedgerRepo};

    #[tokio::test]
    async fn record_chat_message_global() {
        let mut mock = MockServerRepo::new();
        mock.expect_insert_chat_message()
            .withf(|_uid, _id, role, content| role == "user" && content == "hello")
            .returning(|_, _, _, _| Ok(()));

        let recorder = ExecutionRecorder::new(&mock, None, None);
        recorder
            .record_chat_message(UserId::new(), None, Uuid::new_v4(), "user", "hello")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn record_chat_message_session() {
        let mut mock = MockServerRepo::new();
        mock.expect_insert_session_message()
            .withf(|_uid, _sid, _id, role, content| role == "assistant" && content == "hi there")
            .returning(|_, _, _, _, _| Ok(()));

        let session_id = Uuid::new_v4();
        let recorder = ExecutionRecorder::new(&mock, None, None);
        recorder
            .record_chat_message(UserId::new(), Some(session_id), Uuid::new_v4(), "assistant", "hi there")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn record_tokens_without_repo_fails() {
        let mock = MockServerRepo::new();
        let recorder = ExecutionRecorder::new(&mock, None, None);
        let result = recorder
            .record_tokens(Uuid::new_v4(), Some(Uuid::new_v4()), "claude-3", 100, 50, 0.01)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn record_tokens_with_repo() {
        let mock = MockServerRepo::new();
        let mut tl_mock = MockTokenLedgerRepo::new();
        tl_mock
            .expect_insert_ledger_entry()
            .returning(|uid, aeid, _model, inp, out, cost| {
                Ok(crate::db::TokenLedgerRow {
                    id: Uuid::new_v4(),
                    user_id: uid,
                    agent_execution_id: aeid,
                    model_id: "claude-3".to_string(),
                    input_tokens: inp,
                    output_tokens: out,
                    cost_usd: cost,
                    created_at: chrono::Utc::now(),
                })
            });

        let recorder = ExecutionRecorder::new(&mock, None, Some(&tl_mock));
        recorder
            .record_tokens(Uuid::new_v4(), Some(Uuid::new_v4()), "claude-3", 100, 50, 0.01)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn record_agent_execution_without_repo_fails() {
        let mock = MockServerRepo::new();
        let recorder = ExecutionRecorder::new(&mock, None, None);
        let result = recorder
            .record_agent_execution(Uuid::new_v4(), Uuid::new_v4(), None, false, None, "system prompt", "user prompt", None)
            .await;
        assert!(result.is_err());
    }
}
