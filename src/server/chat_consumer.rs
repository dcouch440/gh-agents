//! Chat consumer that processes chat messages via the unified hub.
//!
//! Reads messages from the chat channel, calls run_chat() which uses
//! the ExecutionEngine, and streams responses back through SSE.

use std::sync::Arc;

use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::llm::{AnthropicClient, LLMProvider, RateLimitedProvider, RetryingProvider};

use super::state::{AppState, ConsumerMessage, StreamChunk};

/// Spawn the chat consumer as a background task.
pub fn spawn_chat_consumer(state: AppState, chat_rx: mpsc::Receiver<ConsumerMessage>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(run_chat_consumer(state, chat_rx))
}

async fn run_chat_consumer(state: AppState, mut chat_rx: mpsc::Receiver<ConsumerMessage>) {
    let provider: Arc<dyn LLMProvider + Send + Sync> = match AnthropicClient::from_env() {
        Ok(p) => {
            info!("Chat consumer started with model: {}", p.model_id().to_string());
            Arc::new(RetryingProvider::with_defaults(RateLimitedProvider::with_defaults(p)))
        }
        Err(e) => {
            error!("Failed to initialize LLM provider: {}. Chat will not work. Set ANTHROPIC_API_KEY.", e);
            while let Some(msg) = chat_rx.recv().await {
                state.send_stream_chunk(msg.id, StreamChunk::Error("LLM provider not configured. Set ANTHROPIC_API_KEY.".into())).await;
                let cleanup_state = state.clone();
                let mid = msg.id;
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(120)).await;
                    cleanup_state.remove_response_stream(mid).await;
                });
            }
            return;
        }
    };

    while let Some(msg) = chat_rx.recv().await {
        let state = state.clone();
        let provider = Arc::clone(&provider);
        let message_id = msg.id;
        tokio::spawn(async move {
            if let Err(e) = handle_message(&state, provider, msg).await {
                warn!("Chat message handling failed: {}", e);
                state.send_stream_chunk(message_id, StreamChunk::Error(format!("Chat error: {}", e))).await;
                let cleanup_state = state.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(120)).await;
                    cleanup_state.remove_response_stream(message_id).await;
                });
            }
        });
    }

    info!("Chat consumer shutting down (channel closed)");
}

async fn handle_message(state: &AppState, provider: Arc<dyn LLMProvider + Send + Sync>, msg: ConsumerMessage) -> anyhow::Result<()> {
    let message_id = msg.id;
    let agent_id = msg.agent_id.or(state.default_agent_id);

    match agent_id {
        Some(aid) => match super::hub::run_chat(state, provider, aid, msg.session_id, message_id, &msg.content, msg.user_id, None).await {
            Ok(_) => {}
            Err(e) => {
                warn!("Chat error for {}: {}", message_id, e);
                state.send_stream_chunk(message_id, StreamChunk::Error(format!("{}", e))).await;
                state.send_stream_chunk(message_id, StreamChunk::Done).await;
            }
        },
        None => {
            warn!("No agent_id and no default agent configured for message {}", message_id);
            state.send_stream_chunk(message_id, StreamChunk::Error("No agent configured".into())).await;
            state.send_stream_chunk(message_id, StreamChunk::Done).await;
        }
    }

    super::hub::schedule_stream_cleanup(state, message_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::traits::ServerRepo;
    use crate::db::{ChatMessageRow, PipelineRow, PipelineStageRow, SessionRow};
    use crate::types::{AppConfig, UserId};
    use chrono::{DateTime, Utc};
    use std::sync::Arc;

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
        async fn list_tasks(&self, _user_id: UserId, _: Option<String>, _: Option<u32>) -> anyhow::Result<Vec<crate::types::Task>> {
            Ok(vec![])
        }
        async fn get_task_by_uuid(&self, _user_id: UserId, _: Uuid) -> anyhow::Result<Option<crate::types::Task>> {
            Ok(None)
        }
        async fn insert_task(&self, _user_id: UserId, _: crate::types::Task) -> anyhow::Result<()> {
            Ok(())
        }
        async fn insert_chat_message(&self, _user_id: UserId, id: Uuid, role: String, content: String) -> anyhow::Result<()> {
            self.messages.lock().unwrap().push(ChatMessageRow {
                id,
                role,
                content,
                timestamp: Utc::now(),
            });
            Ok(())
        }
        async fn get_chat_history(&self, _user_id: UserId, limit: u32, offset: u32) -> anyhow::Result<Vec<ChatMessageRow>> {
            let msgs = self.messages.lock().unwrap();
            Ok(msgs.iter().skip(offset as usize).take(limit as usize).cloned().collect())
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
        async fn list_persisted_agents(&self, _user_id: UserId) -> anyhow::Result<Vec<crate::db::AgentRow>> {
            Ok(vec![])
        }
        async fn get_persisted_agent(&self, _agent_id: Uuid) -> anyhow::Result<Option<crate::db::AgentRow>> {
            Ok(None)
        }
        async fn upsert_agent(&self, _user_id: UserId, _agent: crate::db::AgentRow) -> anyhow::Result<()> {
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
        async fn get_agent_context(&self, _agent_id: Uuid) -> anyhow::Result<Vec<crate::db::DocumentRow>> {
            Ok(vec![])
        }
        async fn set_agent_context(&self, _agent_id: Uuid, _document_ids: Vec<Uuid>) -> anyhow::Result<()> {
            Ok(())
        }
        async fn list_pipelines(&self, _user_id: UserId) -> anyhow::Result<Vec<PipelineRow>> {
            Ok(vec![])
        }
        async fn upsert_pipeline(&self, _user_id: UserId, _pipeline: PipelineRow) -> anyhow::Result<()> {
            Ok(())
        }
        async fn delete_pipeline(&self, _pipeline_id: Uuid) -> anyhow::Result<()> {
            Ok(())
        }
        async fn list_pipeline_stages(&self, _pipeline_id: Uuid) -> anyhow::Result<Vec<PipelineStageRow>> {
            Ok(vec![])
        }
        async fn upsert_pipeline_stage(&self, _stage: PipelineStageRow) -> anyhow::Result<()> {
            Ok(())
        }
        async fn create_session(&self, _user_id: UserId, _session_id: Uuid, _mode_id: &str, _title: &str, _agent_id: Option<Uuid>) -> anyhow::Result<()> {
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
        async fn insert_session_message(&self, _user_id: UserId, _session_id: Uuid, _id: Uuid, _role: String, _content: String) -> anyhow::Result<()> {
            Ok(())
        }
        async fn get_session_history(&self, _session_id: Uuid, _limit: u32) -> anyhow::Result<Vec<ChatMessageRow>> {
            Ok(vec![])
        }
        async fn update_session_title(&self, _session_id: Uuid, _title: &str) -> anyhow::Result<()> {
            Ok(())
        }
        async fn update_session_summary(&self, _session_id: Uuid, _summary: &str) -> anyhow::Result<()> {
            Ok(())
        }
        async fn count_session_messages(&self, _session_id: Uuid) -> anyhow::Result<u32> {
            Ok(0)
        }
        async fn create_pipeline_run(&self, _run: &crate::db::PipelineRunRow) -> anyhow::Result<()> {
            Ok(())
        }
        async fn update_pipeline_run(&self, _run: &crate::db::PipelineRunRow) -> anyhow::Result<()> {
            Ok(())
        }
        async fn get_pipeline_run(&self, _run_id: Uuid) -> anyhow::Result<Option<crate::db::PipelineRunRow>> {
            Ok(None)
        }
        async fn list_pipeline_runs(&self, _pipeline_id: Uuid) -> anyhow::Result<Vec<crate::db::PipelineRunRow>> {
            Ok(vec![])
        }
        async fn create_stage_execution(&self, _exec: &crate::db::StageExecutionRow) -> anyhow::Result<()> {
            Ok(())
        }
        async fn update_stage_execution(&self, _exec: &crate::db::StageExecutionRow) -> anyhow::Result<()> {
            Ok(())
        }
        async fn list_stage_executions(&self, _run_id: Uuid) -> anyhow::Result<Vec<crate::db::StageExecutionRow>> {
            Ok(vec![])
        }
        async fn get_agent_modes(&self, _agent_id: Uuid) -> anyhow::Result<Vec<crate::db::AgentModeRow>> {
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
        let (state, chat_rx) = AppState::with_repo(None, repo, AppConfig::default());

        let msg_id = Uuid::new_v4();
        let (_buf, mut rx, _done) = state.get_response_stream(msg_id).await;

        state
            .chat_tx
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
}
