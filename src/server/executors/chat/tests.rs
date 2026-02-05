//! Tests for chat consumer

use super::*;
use crate::db::traits::ServerRepo;
use crate::db::{ChatMessageRow, SessionRow};
use crate::server::state::test_helpers::default_mock_repos;
use crate::types::{AppConfig, UserId};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

/// Minimal in-memory repo for orchestrator tests
struct TestRepo {
    messages: std::sync::Mutex<Vec<ChatMessageRow>>,
}

impl TestRepo {
    fn new() -> Self {
        Self {
            messages: std::sync::Mutex::new(vec![]),
        }
    }
}

#[async_trait::async_trait]
impl ServerRepo for TestRepo {
    async fn health_check(&self) -> bool {
        true
    }
    async fn list_tasks(
        &self,
        _user_id: UserId,
        _: Option<String>,
        _: Option<u32>,
    ) -> anyhow::Result<Vec<crate::types::Task>> {
        Ok(vec![])
    }
    async fn get_task_by_uuid(
        &self,
        _user_id: UserId,
        _: Uuid,
    ) -> anyhow::Result<Option<crate::types::Task>> {
        Ok(None)
    }
    async fn insert_task(&self, _user_id: UserId, _: crate::types::Task) -> anyhow::Result<()> {
        Ok(())
    }
    async fn insert_chat_message(
        &self,
        _user_id: UserId,
        id: Uuid,
        role: String,
        content: String,
    ) -> anyhow::Result<()> {
        self.messages.lock().unwrap().push(ChatMessageRow {
            id,
            role,
            content,
            timestamp: Utc::now(),
        });
        Ok(())
    }
    async fn get_chat_history(
        &self,
        _user_id: UserId,
        limit: u32,
        offset: u32,
    ) -> anyhow::Result<Vec<ChatMessageRow>> {
        let msgs = self.messages.lock().unwrap();
        Ok(msgs
            .iter()
            .skip(offset as usize)
            .take(limit as usize)
            .cloned()
            .collect())
    }
    async fn clear_chat_history(&self, _user_id: UserId) -> anyhow::Result<()> {
        Ok(())
    }
    async fn has_password(&self) -> anyhow::Result<bool> {
        Ok(false)
    }
    async fn set_password(&self, _: String) -> anyhow::Result<()> {
        Ok(())
    }
    async fn get_password(&self) -> anyhow::Result<Option<String>> {
        Ok(None)
    }
    async fn list_persisted_agents(
        &self,
        _user_id: UserId,
    ) -> anyhow::Result<Vec<crate::db::AgentRow>> {
        Ok(vec![])
    }
    async fn get_persisted_agent(
        &self,
        _agent_id: Uuid,
    ) -> anyhow::Result<Option<crate::db::AgentRow>> {
        Ok(None)
    }
    async fn upsert_agent(
        &self,
        _user_id: UserId,
        _agent: crate::db::AgentRow,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn delete_persisted_agent(&self, _agent_id: Uuid) -> anyhow::Result<()> {
        Ok(())
    }
    async fn list_tools(&self, _user_id: UserId) -> anyhow::Result<Vec<crate::db::ToolRow>> {
        Ok(vec![])
    }
    async fn get_tool(&self, _tool_id: Uuid) -> anyhow::Result<Option<crate::db::ToolRow>> {
        Ok(None)
    }
    async fn upsert_tool(&self, _user_id: UserId, _tool: crate::db::ToolRow) -> anyhow::Result<()> {
        Ok(())
    }
    async fn delete_tool(&self, _tool_id: Uuid) -> anyhow::Result<()> {
        Ok(())
    }
    async fn get_agent_tools(&self, _agent_id: Uuid) -> anyhow::Result<Vec<crate::db::ToolRow>> {
        Ok(vec![])
    }
    async fn seed_builtin_tools(&self, _user_id: UserId) -> anyhow::Result<()> {
        Ok(())
    }
    async fn set_agent_tools(&self, _agent_id: Uuid, _tool_ids: Vec<Uuid>) -> anyhow::Result<()> {
        Ok(())
    }
    async fn get_agent_context(
        &self,
        _agent_id: Uuid,
    ) -> anyhow::Result<Vec<crate::db::DocumentRow>> {
        Ok(vec![])
    }
    async fn set_agent_context(
        &self,
        _agent_id: Uuid,
        _document_ids: Vec<Uuid>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn create_session(
        &self,
        _user_id: UserId,
        _session_id: Uuid,
        _mode_id: &str,
        _title: &str,
        _agent_id: Option<Uuid>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn list_sessions(&self, _user_id: UserId) -> anyhow::Result<Vec<SessionRow>> {
        Ok(vec![])
    }
    async fn get_session(&self, _session_id: Uuid) -> anyhow::Result<Option<SessionRow>> {
        Ok(None)
    }
    async fn delete_session(&self, _session_id: Uuid) -> anyhow::Result<()> {
        Ok(())
    }
    async fn insert_session_message(
        &self,
        _user_id: UserId,
        _session_id: Uuid,
        _id: Uuid,
        _role: String,
        _content: String,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn get_session_history(
        &self,
        _session_id: Uuid,
        _limit: u32,
    ) -> anyhow::Result<Vec<ChatMessageRow>> {
        Ok(vec![])
    }
    async fn update_session_title(&self, _session_id: Uuid, _title: &str) -> anyhow::Result<()> {
        Ok(())
    }
    async fn update_session_summary(
        &self,
        _session_id: Uuid,
        _summary: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn count_session_messages(&self, _session_id: Uuid) -> anyhow::Result<u32> {
        Ok(0)
    }
    async fn get_agent_modes(
        &self,
        _agent_id: Uuid,
    ) -> anyhow::Result<Vec<crate::db::AgentModeRow>> {
        Ok(vec![])
    }
    async fn create_agent_mode(&self, _mode: &crate::db::AgentModeRow) -> anyhow::Result<()> {
        Ok(())
    }
    async fn delete_agent_mode(&self, _mode_id: Uuid) -> anyhow::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn chat_consumer_sends_error_when_no_api_key() {
    let saved = std::env::var(crate::constants::ENV_ANTHROPIC_API_KEY).ok();
    std::env::remove_var(crate::constants::ENV_ANTHROPIC_API_KEY);

    let repo: Arc<dyn ServerRepo> = Arc::new(TestRepo::new());
    let repos = default_mock_repos();
    let (state, chat_rx) = AppState::with_repo(None, repo, repos, AppConfig::default());

    let msg_id = Uuid::new_v4();
    let (_buf, mut rx, _done) = state.get_response_stream(msg_id);

    state
        .chat_tx()
        .send(ConsumerMessage {
            id: msg_id,
            user_id: UserId::new(),
            session_id: None,
            agent_id: None,
            content: "Hello".into(),
            timestamp: Utc::now(),
        })
        .await
        .unwrap();

    let _handle = spawn_chat_consumer(state, chat_rx);

    let chunk = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
        .await
        .expect("timeout waiting for chunk")
        .expect("channel closed");

    assert!(matches!(chunk, StreamChunk::Error(_)));

    if let Some(key) = saved {
        std::env::set_var(crate::constants::ENV_ANTHROPIC_API_KEY, key);
    }
}
