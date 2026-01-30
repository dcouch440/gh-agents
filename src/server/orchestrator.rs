//! Orchestrator consumer that processes chat messages via LLM streaming
//!
//! Reads messages from the orchestrator channel, builds conversation context
//! from chat history, calls the LLM with streaming, and pipes tokens back
//! to the SSE response stream.

use futures::StreamExt;
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::llm::{
    AnthropicClient, LLMProvider, LLMRequest, Message,
    StreamChunk as LLMStreamChunk,
};

use super::state::{AppState, OrchestratorMessage, StreamChunk};

const SYSTEM_PROMPT: &str = "You are nexor, an AI assistant for software engineering. \
    You help users plan, build, and manage software projects. \
    Be concise and technical. Use markdown formatting when helpful.";

/// Spawn the orchestrator consumer as a background task.
///
/// Consumes messages from `orchestrator_rx`, calls the LLM with streaming,
/// and pipes token chunks back through the AppState response streams.
pub fn spawn_orchestrator(
    state: AppState,
    orchestrator_rx: mpsc::Receiver<OrchestratorMessage>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(run_orchestrator(state, orchestrator_rx))
}

async fn run_orchestrator(
    state: AppState,
    mut orchestrator_rx: mpsc::Receiver<OrchestratorMessage>,
) {
    let provider = match AnthropicClient::from_env() {
        Ok(p) => {
            info!(
                "Orchestrator started with model: {}",
                p.model_id().to_string()
            );
            p
        }
        Err(e) => {
            error!("Failed to initialize LLM provider: {}. Chat will not work. Set ANTHROPIC_API_KEY.", e);
            // Drain messages and send errors so clients don't hang
            while let Some(msg) = orchestrator_rx.recv().await {
                state
                    .send_stream_chunk(
                        msg.id,
                        StreamChunk::Error("LLM provider not configured. Set ANTHROPIC_API_KEY.".into()),
                    )
                    .await;
                state.remove_response_stream(msg.id).await;
            }
            return;
        }
    };

    while let Some(msg) = orchestrator_rx.recv().await {
        let state = state.clone();
        let provider = provider.clone();
        // Process each message concurrently so one slow response doesn't block the queue
        tokio::spawn(async move {
            if let Err(e) = handle_message(&state, &provider, msg).await {
                warn!("Orchestrator message handling failed: {}", e);
            }
        });
    }

    info!("Orchestrator consumer shutting down (channel closed)");
}

async fn handle_message(
    state: &AppState,
    provider: &AnthropicClient,
    msg: OrchestratorMessage,
) -> anyhow::Result<()> {
    let message_id = msg.id;
    let user_id = msg.user_id;

    // Load chat history for conversation context
    let history = state
        .repo
        .get_chat_history(user_id, 50, 0)
        .await
        .unwrap_or_default();

    // Build messages from history (excluding the current message which was already saved)
    let mut messages: Vec<Message> = history
        .iter()
        .map(|row| match row.role.as_str() {
            "assistant" => Message::assistant(&row.content),
            _ => Message::user(&row.content),
        })
        .collect();

    // If the current message isn't in history yet (race condition), ensure it's included
    if !messages.iter().any(|m| {
        m.role == crate::llm::Role::User && m.content == msg.content
    }) {
        messages.push(Message::user(&msg.content));
    }

    // Ensure we don't send an empty messages array
    if messages.is_empty() {
        messages.push(Message::user(&msg.content));
    }

    let request = LLMRequest::new(provider.model_id(), messages)
        .with_system(SYSTEM_PROMPT)
        .with_streaming();

    // Stream the LLM response
    let mut accumulated = String::new();

    match provider.send_message_stream(request).await {
        Ok(mut stream) => {
            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(LLMStreamChunk::ContentDelta { text, .. }) => {
                        accumulated.push_str(&text);
                        state
                            .send_stream_chunk(message_id, StreamChunk::Token(text))
                            .await;
                    }
                    Ok(LLMStreamChunk::MessageStop) => {
                        break;
                    }
                    Ok(_) => {
                        // MessageStart, ContentBlockStart/Stop, MessageDelta, Ping — skip
                    }
                    Err(e) => {
                        error!("LLM stream error for message {}: {}", message_id, e);
                        state
                            .send_stream_chunk(
                                message_id,
                                StreamChunk::Error(format!("LLM error: {}", e)),
                            )
                            .await;
                        break;
                    }
                }
            }

            // Send done signal
            state
                .send_stream_chunk(message_id, StreamChunk::Done)
                .await;

            // Save the assistant response to the database
            if !accumulated.is_empty() {
                let response_id = Uuid::new_v4();
                if let Err(e) = state
                    .repo
                    .insert_chat_message(user_id, response_id, "assistant".into(), accumulated)
                    .await
                {
                    error!("Failed to save assistant message: {}", e);
                }
            }
        }
        Err(e) => {
            error!("Failed to start LLM stream for message {}: {}", message_id, e);
            state
                .send_stream_chunk(
                    message_id,
                    StreamChunk::Error(format!("Failed to reach LLM: {}", e)),
                )
                .await;
        }
    }

    // Cleanup the response stream
    state.remove_response_stream(message_id).await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::traits::ServerRepo;
    use crate::db::ChatMessageRow;
    use crate::types::{AppConfig, UserId};
    use chrono::Utc;
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
        async fn health_check(&self) -> bool { true }
        async fn list_tasks(&self, _user_id: UserId, _: Option<String>, _: Option<u32>) -> anyhow::Result<Vec<crate::types::Task>> { Ok(vec![]) }
        async fn get_task_by_uuid(&self, _user_id: UserId, _: Uuid) -> anyhow::Result<Option<crate::types::Task>> { Ok(None) }
        async fn insert_task(&self, _user_id: UserId, _: crate::types::Task) -> anyhow::Result<()> { Ok(()) }
        async fn insert_chat_message(&self, _user_id: UserId, id: Uuid, role: String, content: String) -> anyhow::Result<()> {
            self.messages.lock().unwrap().push(ChatMessageRow {
                id, role, content, timestamp: Utc::now(),
            });
            Ok(())
        }
        async fn get_chat_history(&self, _user_id: UserId, limit: u32, offset: u32) -> anyhow::Result<Vec<ChatMessageRow>> {
            let msgs = self.messages.lock().unwrap();
            Ok(msgs.iter().skip(offset as usize).take(limit as usize).cloned().collect())
        }
        async fn clear_chat_history(&self, _user_id: UserId) -> anyhow::Result<()> { Ok(()) }
        async fn has_password(&self) -> anyhow::Result<bool> { Ok(false) }
        async fn set_password(&self, _: String) -> anyhow::Result<()> { Ok(()) }
        async fn get_password(&self) -> anyhow::Result<Option<String>> { Ok(None) }
    }

    #[tokio::test]
    async fn orchestrator_sends_error_when_no_api_key() {
        // Ensure ANTHROPIC_API_KEY is not set for this test
        let saved = std::env::var("ANTHROPIC_API_KEY").ok();
        std::env::remove_var("ANTHROPIC_API_KEY");

        let repo: Arc<dyn ServerRepo> = Arc::new(TestRepo::new());
        let (state, orchestrator_rx) = AppState::with_repo(None, repo, None, AppConfig::default());

        let msg_id = Uuid::new_v4();

        // Subscribe to the response stream before spawning
        let mut rx = state.get_response_stream(msg_id).await;

        // Send a message
        state
            .orchestrator_tx
            .send(OrchestratorMessage {
                id: msg_id,
                user_id: UserId::new(),
                content: "Hello".into(),
                timestamp: Utc::now(),
            })
            .await
            .unwrap();

        // Spawn orchestrator — without API key it will send errors and drain
        let _handle = spawn_orchestrator(state, orchestrator_rx);

        // Should receive an error chunk
        let chunk = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
            .await
            .expect("timeout waiting for chunk")
            .expect("channel closed");

        assert!(matches!(chunk, StreamChunk::Error(_)));

        // Restore env var
        if let Some(key) = saved {
            std::env::set_var("ANTHROPIC_API_KEY", key);
        }
    }
}
