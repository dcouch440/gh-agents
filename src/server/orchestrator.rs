//! Orchestrator consumer that processes chat messages via direct LLM calls
//! with Anthropic native tool use.
//!
//! Reads messages from the orchestrator channel, calls the LLM with agent
//! management tools, executes any tool calls, and streams text responses
//! back through the AppState SSE streams.

use std::sync::Arc;

use futures::StreamExt;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::llm::{
    AnthropicClient, ContentBlock, LLMProvider, LLMRequest, Message, Role,
    StreamAccumulator, StopReason,
    StreamChunk as LLMStreamChunk,
};

use crate::agents::AgentResponse;

use super::state::{AppState, OrchestratorMessage, StreamChunk};
use super::tools;
use super::ws::{AgentUpdate, FeedUpdate, TaskUpdate};

const SYSTEM_PROMPT: &str = "You are nexor, an AI assistant for software engineering. \
    You help users plan, build, and manage software projects. \
    You can create and manage AI agents to help with tasks. \
    Be concise and technical. Use markdown formatting when helpful.";

/// Spawn a background task that drains agent responses from the dispatcher
/// and stores them in `state.task_results` for retrieval by `get_task_result`.
pub fn spawn_response_consumer(state: AppState) -> Option<tokio::task::JoinHandle<()>> {
    let dispatcher = state.dispatcher.clone()?;
    Some(tokio::spawn(async move {
        loop {
            let response = {
                let mut d = dispatcher.lock().await;
                d.recv_response().await
            };
            match response {
                Some(resp) => {
                    let task_id = match &resp {
                        AgentResponse::TaskStarted { task_id, .. } => Some(*task_id),
                        AgentResponse::TaskCompleted { result, .. } => Some(result.task_id),
                        AgentResponse::TaskFailed { result, .. } => Some(result.task_id),
                        AgentResponse::ProgressUpdate { update, .. } => Some(update.task_id),
                        AgentResponse::ContextRequest { request, .. } => Some(request.task_id),
                        AgentResponse::ApprovalRequest { request, .. } => Some(request.task_id),
                        AgentResponse::ShutdownComplete { .. } => None,
                    };
                    // Broadcast to UI channels
                    match &resp {
                        AgentResponse::TaskStarted { agent_id, task_id } => {
                            state.broadcast_task(TaskUpdate {
                                id: *task_id,
                                status: "in_progress".into(),
                                progress: None,
                                assigned_agent: Some(agent_id.0.to_string()),
                                user_id: None,
                            });
                            state.broadcast_agent(AgentUpdate {
                                id: agent_id.0.to_string(),
                                status: "working".into(),
                                current_task: Some(*task_id),
                                user_id: None,
                            });
                        }
                        AgentResponse::TaskCompleted { agent_id, result } => {
                            state.broadcast_task(TaskUpdate {
                                id: result.task_id,
                                status: "completed".into(),
                                progress: Some(1.0),
                                assigned_agent: Some(agent_id.0.to_string()),
                                user_id: None,
                            });
                            state.broadcast_agent(AgentUpdate {
                                id: agent_id.0.to_string(),
                                status: "idle".into(),
                                current_task: None,
                                user_id: None,
                            });
                        }
                        AgentResponse::TaskFailed { agent_id, result } => {
                            state.broadcast_task(TaskUpdate {
                                id: result.task_id,
                                status: "failed".into(),
                                progress: None,
                                assigned_agent: Some(agent_id.0.to_string()),
                                user_id: None,
                            });
                            state.broadcast_agent(AgentUpdate {
                                id: agent_id.0.to_string(),
                                status: "idle".into(),
                                current_task: None,
                                user_id: None,
                            });
                        }
                        AgentResponse::ProgressUpdate { agent_id, update } => {
                            state.broadcast_feed(FeedUpdate {
                                id: update.task_id,
                                agent_id: agent_id.0.to_string(),
                                content: update.message.clone(),
                                item_type: "progress".into(),
                                timestamp: chrono::Utc::now(),
                                user_id: None,
                            });
                            if let Some(pct) = update.progress_percent {
                                state.broadcast_task(TaskUpdate {
                                    id: update.task_id,
                                    status: "in_progress".into(),
                                    progress: Some(pct as f32 / 100.0),
                                    assigned_agent: Some(agent_id.0.to_string()),
                                    user_id: None,
                                });
                            }
                        }
                        AgentResponse::ApprovalRequest { agent_id, request } => {
                            state.broadcast_feed(FeedUpdate {
                                id: request.task_id,
                                agent_id: agent_id.0.to_string(),
                                content: format!(
                                    "Approval needed: {} — {}",
                                    request.action, request.details
                                ),
                                item_type: "approval_request".into(),
                                timestamp: chrono::Utc::now(),
                                user_id: None,
                            });
                            state.broadcast_agent(AgentUpdate {
                                id: agent_id.0.to_string(),
                                status: "waiting_for_approval".into(),
                                current_task: Some(request.task_id),
                                user_id: None,
                            });
                        }
                        _ => {}
                    }

                    if let Some(id) = task_id {
                        debug!("Response consumer received {:?} for task {}",
                            std::mem::discriminant(&resp), id);
                        state.task_results.write().await.insert(id, resp);
                    }
                }
                None => {
                    info!("Response consumer shutting down (channel closed)");
                    break;
                }
            }
        }
    }))
}

/// Spawn the orchestrator consumer as a background task.
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
    let provider: Arc<dyn LLMProvider + Send + Sync> = match AnthropicClient::from_env() {
        Ok(p) => {
            info!(
                "Orchestrator started with model: {}",
                p.model_id().to_string()
            );
            Arc::new(p)
        }
        Err(e) => {
            error!("Failed to initialize LLM provider: {}. Chat will not work. Set ANTHROPIC_API_KEY.", e);
            while let Some(msg) = orchestrator_rx.recv().await {
                state
                    .send_stream_chunk(
                        msg.id,
                        StreamChunk::Error(
                            "LLM provider not configured. Set ANTHROPIC_API_KEY.".into(),
                        ),
                    )
                    .await;
                state.remove_response_stream(msg.id).await;
            }
            return;
        }
    };

    while let Some(msg) = orchestrator_rx.recv().await {
        let state = state.clone();
        let provider = Arc::clone(&provider);
        tokio::spawn(async move {
            if let Err(e) = handle_message(&state, provider, msg).await {
                warn!("Orchestrator message handling failed: {}", e);
            }
        });
    }

    info!("Orchestrator consumer shutting down (channel closed)");
}

async fn handle_message(
    state: &AppState,
    provider: Arc<dyn LLMProvider + Send + Sync>,
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

    // Build LLM messages from chat history
    let mut messages: Vec<Message> = history
        .iter()
        .map(|row| match row.role.as_str() {
            "assistant" => Message::assistant(&row.content),
            _ => Message::user(&row.content),
        })
        .collect();

    // Ensure the current message is included
    if !messages
        .iter()
        .any(|m| m.role == Role::User && m.text() == msg.content)
    {
        messages.push(Message::user(&msg.content));
    }
    if messages.is_empty() {
        messages.push(Message::user(&msg.content));
    }

    let model_id = provider.model_id().to_string();
    let tool_defs = tools::agent_tools();

    // Multi-turn tool use loop
    let mut accumulated_response = String::new();
    let max_tool_rounds = 10;

    for round in 0..max_tool_rounds {
        debug!("Tool use round {} for message {}", round, message_id);

        let request = LLMRequest {
            model: model_id.clone(),
            system: Some(SYSTEM_PROMPT.to_string()),
            messages: messages.clone(),
            max_tokens: 4096,
            stream: true,
            tools: tool_defs.clone(),
            ..Default::default()
        };

        // Stream the response
        let mut stream = provider
            .send_message_stream(request)
            .await
            .map_err(|e| anyhow::anyhow!("LLM stream error: {}", e))?;

        let mut accumulator = StreamAccumulator::new();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(ref chunk @ LLMStreamChunk::ContentDelta { ref text, .. }) => {
                    accumulated_response.push_str(text);
                    state
                        .send_stream_chunk(message_id, StreamChunk::Token(text.clone()))
                        .await;
                    accumulator.apply(chunk);
                }
                Ok(ref chunk) => {
                    accumulator.apply(chunk);
                }
                Err(e) => {
                    error!("Stream error for message {}: {}", message_id, e);
                    state
                        .send_stream_chunk(
                            message_id,
                            StreamChunk::Error(format!("Stream error: {}", e)),
                        )
                        .await;
                    state.remove_response_stream(message_id).await;
                    return Ok(());
                }
            }
        }

        let response = match accumulator.build() {
            Some(r) => r,
            None => {
                error!("Incomplete LLM response for message {}", message_id);
                state
                    .send_stream_chunk(
                        message_id,
                        StreamChunk::Error("Incomplete response from LLM".into()),
                    )
                    .await;
                state.remove_response_stream(message_id).await;
                return Ok(());
            }
        };

        // Check if we need to execute tools
        if response.stop_reason == StopReason::ToolUse {
            // Add assistant message with all content blocks (text + tool_use)
            messages.push(Message::assistant_with_blocks(response.content_blocks.clone()));

            // Execute each tool call and collect results
            let mut tool_results = Vec::new();
            for block in &response.content_blocks {
                if let ContentBlock::ToolUse { id, name, input } = block {
                    debug!("Executing tool: {} (id: {})", name, id);
                    let result = tools::execute_tool(name, input, state, user_id).await;
                    let result_str = serde_json::to_string_pretty(&result)
                        .unwrap_or_else(|_| result.to_string());

                    tool_results.push(ContentBlock::ToolResult {
                        tool_use_id: id.clone(),
                        content: result_str,
                    });

                    // Stream tool execution info to user
                    let tool_info = format!("\n\n*Executed tool `{}`*\n\n", name);
                    accumulated_response.push_str(&tool_info);
                    state
                        .send_stream_chunk(message_id, StreamChunk::Token(tool_info))
                        .await;
                }
            }

            // Add tool results as a single user message with content blocks
            messages.push(Message::tool_results(tool_results));

            // Continue the loop for the next LLM call
            continue;
        }

        // EndTurn or MaxTokens — we're done
        break;
    }

    // Send done signal
    state
        .send_stream_chunk(message_id, StreamChunk::Done)
        .await;

    // Save the assistant response to the database
    if !accumulated_response.is_empty() {
        let response_id = Uuid::new_v4();
        if let Err(e) = state
            .repo
            .insert_chat_message(user_id, response_id, "assistant".into(), accumulated_response)
            .await
        {
            error!("Failed to save assistant message: {}", e);
        }
    }

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
        async fn list_persisted_agents(&self, _user_id: UserId) -> anyhow::Result<Vec<crate::db::AgentRow>> { Ok(vec![]) }
        async fn upsert_agent(&self, _user_id: UserId, _agent: crate::db::AgentRow) -> anyhow::Result<()> { Ok(()) }
        async fn delete_persisted_agent(&self, _agent_id: Uuid) -> anyhow::Result<()> { Ok(()) }
        async fn list_persisted_clusters(&self, _user_id: UserId) -> anyhow::Result<Vec<crate::db::ClusterRow>> { Ok(vec![]) }
        async fn upsert_cluster(&self, _user_id: UserId, _cluster: crate::db::ClusterRow) -> anyhow::Result<()> { Ok(()) }
        async fn delete_cluster(&self, _cluster_id: Uuid) -> anyhow::Result<()> { Ok(()) }
        async fn list_cluster_members(&self, _cluster_id: Uuid) -> anyhow::Result<Vec<Uuid>> { Ok(vec![]) }
        async fn add_cluster_member(&self, _cluster_id: Uuid, _agent_id: Uuid) -> anyhow::Result<()> { Ok(()) }
        async fn remove_cluster_member(&self, _cluster_id: Uuid, _agent_id: Uuid) -> anyhow::Result<()> { Ok(()) }
    }

    #[tokio::test]
    async fn orchestrator_sends_error_when_no_api_key() {
        let saved = std::env::var("ANTHROPIC_API_KEY").ok();
        std::env::remove_var("ANTHROPIC_API_KEY");

        let repo: Arc<dyn ServerRepo> = Arc::new(TestRepo::new());
        let (state, orchestrator_rx) = AppState::with_repo(None, repo, None, AppConfig::default());

        let msg_id = Uuid::new_v4();
        let mut rx = state.get_response_stream(msg_id).await;

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

        let _handle = spawn_orchestrator(state, orchestrator_rx);

        let chunk = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
            .await
            .expect("timeout waiting for chunk")
            .expect("channel closed");

        assert!(matches!(chunk, StreamChunk::Error(_)));

        if let Some(key) = saved {
            std::env::set_var("ANTHROPIC_API_KEY", key);
        }
    }
}
