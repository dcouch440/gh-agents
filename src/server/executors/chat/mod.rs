//! Chat consumer that processes chat messages via the unified hub.
//!
//! Reads messages from the chat channel, calls run_chat() which uses
//! the ExecutionEngine, and streams responses back through SSE.

use std::sync::Arc;

use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::llm::{AnthropicClient, LLMProvider, RateLimitedProvider, RetryingProvider};
use crate::server::hub::HubError;
use crate::server::state::{AppState, ConsumerMessage, StreamChunk};

/// Spawn the chat consumer as a background task.
pub fn spawn_chat_consumer(
    state: AppState,
    chat_rx: mpsc::Receiver<ConsumerMessage>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(run_chat_consumer(state, chat_rx))
}

async fn run_chat_consumer(state: AppState, mut chat_rx: mpsc::Receiver<ConsumerMessage>) {
    let provider: Arc<dyn LLMProvider + Send + Sync> = match AnthropicClient::from_env() {
        Ok(p) => {
            info!(
                "Chat consumer started with model: {}",
                p.model_id().to_string()
            );
            Arc::new(RetryingProvider::with_defaults(
                RateLimitedProvider::with_defaults(p),
            ))
        }
        Err(e) => {
            error!(
                "Failed to initialize LLM provider: {}. Chat will not work. Set ANTHROPIC_API_KEY.",
                e
            );
            while let Some(msg) = chat_rx.recv().await {
                state.send_stream_chunk(
                    msg.id,
                    StreamChunk::Error(
                        "LLM provider not configured. Set ANTHROPIC_API_KEY.".into(),
                    ),
                );
                let cleanup_state = state.clone();
                let mid = msg.id;
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(120)).await;
                    cleanup_state.remove_response_stream(mid);
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
                state.send_stream_chunk(
                    message_id,
                    StreamChunk::Error(format!("Chat error: {}", e)),
                );
                let cleanup_state = state.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(120)).await;
                    cleanup_state.remove_response_stream(message_id);
                });
            }
        });
    }

    info!("Chat consumer shutting down (channel closed)");
}

async fn handle_message(
    state: &AppState,
    provider: Arc<dyn LLMProvider + Send + Sync>,
    msg: ConsumerMessage,
) -> anyhow::Result<()> {
    let message_id = msg.id;
    let cancel_token = state.register_cancellation(message_id);

    // Check if this is a step-scoped session — route to run_step_chat
    if let Some(session_id) = msg.session_id {
        if let Ok(Some(session)) = state.repo().get_session(session_id).await {
            if let Some(ref dc) = session.draft_config {
                if let (Some(step_id_str), Some(wf_id_str)) = (
                    dc.get("step_id").and_then(|v| v.as_str()),
                    dc.get("workflow_id").and_then(|v| v.as_str()),
                ) {
                    if let (Ok(step_id), Ok(workflow_id)) = (
                        uuid::Uuid::parse_str(step_id_str),
                        uuid::Uuid::parse_str(wf_id_str),
                    ) {
                        match crate::server::hub::run_step_chat(
                            state,
                            provider,
                            session_id,
                            workflow_id,
                            step_id,
                            message_id,
                            &msg.content,
                            msg.user_id,
                            Some(&cancel_token),
                        )
                        .await
                        {
                            Ok(_) => {}
                            Err(e) => {
                                handle_chat_error(state, message_id, e);
                            }
                        }
                        state.remove_cancellation(message_id);
                        crate::server::hub::schedule_stream_cleanup(state, message_id);
                        return Ok(());
                    }
                }
            }
        }
    }

    // Existing agent chat path
    let agent_id = msg.agent_id;

    match agent_id {
        Some(aid) => match crate::server::hub::run_chat(
            state,
            provider,
            aid,
            msg.session_id,
            message_id,
            &msg.content,
            msg.user_id,
            Some(&cancel_token),
        )
        .await
        {
            Ok(_) => {}
            Err(e) => {
                handle_chat_error(state, message_id, e);
            }
        },
        None => {
            warn!(
                "No agent_id and no default agent for message {}",
                message_id
            );
            state.send_stream_chunk(message_id, StreamChunk::Error("No agent configured".into()));
            state.send_stream_chunk(message_id, StreamChunk::Done);
        }
    }

    state.remove_cancellation(message_id);
    crate::server::hub::schedule_stream_cleanup(state, message_id);
    Ok(())
}

/// Handle errors from chat execution, with special treatment for cancellation.
fn handle_chat_error(state: &AppState, message_id: uuid::Uuid, error: HubError) {
    match error {
        HubError::Cancelled => {
            info!("Chat message {} cancelled by user", message_id);
            state.send_stream_chunk(message_id, StreamChunk::Error("Generation stopped".into()));
            state.send_stream_chunk(message_id, StreamChunk::Done);
        }
        e => {
            warn!("Chat error for {}: {}", message_id, e);
            state.send_stream_chunk(message_id, StreamChunk::Error(format!("{}", e)));
            state.send_stream_chunk(message_id, StreamChunk::Done);
        }
    }
}

#[cfg(test)]
mod tests;
