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

use crate::agents::{
    AgentCommand, AgentResponse, CommunicationStyle, FileContent, OutputFormat, RoleContext, RoleId,
    TaskAssignment, TaskConstraints, TaskContext, TriggerEvent,
};

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

                    // Check for pipeline auto-advance on completion/failure
                    let pipeline_advance = match &resp {
                        AgentResponse::TaskCompleted { result, .. } => {
                            let mgr = state.pipeline_manager.read().await;
                            mgr.lookup_run_by_task(result.task_id)
                                .map(|(run_id, _)| (run_id, result.output.clone(), true))
                        }
                        AgentResponse::TaskFailed { result, .. } => {
                            let mgr = state.pipeline_manager.read().await;
                            mgr.lookup_run_by_task(result.task_id)
                                .map(|(run_id, _)| (run_id, String::new(), false))
                        }
                        _ => None,
                    };

                    // Extract trigger event before resp is moved
                    let trigger_event = match &resp {
                        AgentResponse::TaskCompleted { .. } => Some(TriggerEvent::TaskCompleted),
                        AgentResponse::TaskFailed { .. } => Some(TriggerEvent::TaskFailed),
                        _ => None,
                    };

                    if let Some(id) = task_id {
                        debug!("Response consumer received {:?} for task {}",
                            std::mem::discriminant(&resp), id);
                        state.task_results.write().await.insert(id, resp);
                    }

                    // Pipeline auto-advance
                    if let Some((run_id, prev_output, succeeded)) = pipeline_advance {
                        if !succeeded {
                            let mut mgr = state.pipeline_manager.write().await;
                            mgr.fail_run(run_id);
                            state.broadcast_feed(FeedUpdate {
                                id: run_id,
                                agent_id: "pipeline".into(),
                                content: "Pipeline failed due to stage failure".into(),
                                item_type: "pipeline_failed".into(),
                                timestamp: chrono::Utc::now(),
                                user_id: None,
                            });
                        } else {
                            // Try to advance to next stage
                            let next_stage = {
                                let mut mgr = state.pipeline_manager.write().await;
                                mgr.advance_stage(run_id).ok().flatten()
                            };

                            if let Some(next_stage) = next_stage {
                                if next_stage.approval_required {
                                    let mut mgr = state.pipeline_manager.write().await;
                                    mgr.set_waiting_for_approval(run_id);
                                    state.broadcast_feed(FeedUpdate {
                                        id: run_id,
                                        agent_id: "pipeline".into(),
                                        content: format!("Pipeline waiting for approval at stage {}", next_stage.stage_number),
                                        item_type: "pipeline_approval".into(),
                                        timestamp: chrono::Utc::now(),
                                        user_id: None,
                                    });
                                    // TODO: integrate with approval gate
                                } else {
                                    // Auto-assign next stage
                                    let initial_task = {
                                        let mgr = state.pipeline_manager.read().await;
                                        mgr.get_run_initial_task(run_id)
                                            .unwrap_or_default()
                                            .to_string()
                                    };

                                    let description = format!(
                                        "{}\n\nPrevious stage output:\n{}",
                                        initial_task, prev_output
                                    );

                                    let role_str = next_stage.role.as_deref().unwrap_or("worker");
                                    let role_id = RoleId::new(role_str);

                                    let (system_prompt, style, output_format, required_reading) =
                                        if let Some(rm) = &state.role_manager {
                                            if let Some(role) = rm.get_role(&role_id) {
                                                let vars = std::collections::HashMap::new();
                                                let ctx = rm.build_context_for_role(role, &vars).await;
                                                let prompt = ctx.build_system_prompt();
                                                let s = role.style;
                                                let fmt = role.output_format.clone();
                                                let files: Vec<FileContent> = ctx
                                                    .loaded_files
                                                    .into_iter()
                                                    .map(|f| FileContent {
                                                        path: f.path,
                                                        content: f.content,
                                                    })
                                                    .collect();
                                                (prompt, s, fmt, files)
                                            } else {
                                                (
                                                    format!("You are a {} working on: {}", role_str, initial_task),
                                                    CommunicationStyle::Technical,
                                                    OutputFormat::CodeAndReport,
                                                    vec![],
                                                )
                                            }
                                        } else {
                                            (
                                                format!("You are a {} working on: {}", role_str, initial_task),
                                                CommunicationStyle::Technical,
                                                OutputFormat::CodeAndReport,
                                                vec![],
                                            )
                                        };

                                    let project_root = std::env::current_dir().unwrap_or_default();
                                    let execution_context = Some(
                                        crate::execution::ExecutionContext::new(project_root),
                                    );

                                    let assignment = TaskAssignment {
                                        task_id: Uuid::new_v4(),
                                        title: format!(
                                            "Pipeline stage {}: {}",
                                            next_stage.stage_number, initial_task
                                        ),
                                        description,
                                        context: TaskContext {
                                            required_reading,
                                            files: vec![],
                                            history: vec![],
                                            conventions: String::new(),
                                            role_context: RoleContext {
                                                system_prompt,
                                                style,
                                                output_format,
                                            },
                                            chat_messages: vec![],
                                            execution_context,
                                        },
                                        constraints: TaskConstraints::default(),
                                        timeout: std::time::Duration::from_secs(300),
                                        role_id,
                                    };

                                    let new_task_id = assignment.task_id;

                                    // Record in pipeline manager
                                    {
                                        let mut mgr = state.pipeline_manager.write().await;
                                        mgr.record_stage_task(
                                            run_id,
                                            next_stage.stage_number,
                                            new_task_id,
                                        );
                                    }

                                    if let Some(disp) = &state.dispatcher {
                                        let disp = disp.lock().await;
                                        if let Err(e) = disp
                                            .send_to_agent(
                                                &next_stage.agent_id,
                                                AgentCommand::AssignTask(assignment),
                                            )
                                            .await
                                        {
                                            error!("Pipeline auto-advance failed: {}", e);
                                            let mut mgr = state.pipeline_manager.write().await;
                                            mgr.fail_run(run_id);
                                        } else {
                                            state.broadcast_feed(FeedUpdate {
                                                id: run_id,
                                                agent_id: "pipeline".into(),
                                                content: format!(
                                                    "Pipeline advanced to stage {}",
                                                    next_stage.stage_number
                                                ),
                                                item_type: "pipeline_progress".into(),
                                                timestamp: chrono::Utc::now(),
                                                user_id: None,
                                            });
                                        }
                                    }
                                }
                            } else {
                                // Pipeline completed
                                state.broadcast_feed(FeedUpdate {
                                    id: run_id,
                                    agent_id: "pipeline".into(),
                                    content: "Pipeline completed successfully".into(),
                                    item_type: "pipeline_completed".into(),
                                    timestamp: chrono::Utc::now(),
                                    user_id: None,
                                });
                            }
                        }
                    }
                    // Event-driven triggers
                    if let Some(event) = trigger_event {
                        let triggers = {
                            let mgr = state.schedule_manager.read().await;
                            mgr.get_triggers_for_event(event)
                                .into_iter()
                                .cloned()
                                .collect::<Vec<_>>()
                        };

                        for trigger in triggers {
                            if let Some(disp) = &state.dispatcher {
                                let role_str = trigger.role.as_deref().unwrap_or("worker");
                                let role_id = RoleId::new(role_str);

                                let assignment = TaskAssignment {
                                    task_id: Uuid::new_v4(),
                                    title: trigger.task_title.clone(),
                                    description: trigger.task_description.clone(),
                                    context: TaskContext {
                                        required_reading: vec![],
                                        files: vec![],
                                        history: vec![],
                                        conventions: String::new(),
                                        role_context: RoleContext {
                                            system_prompt: format!("You are a {} triggered by event: {}", role_str, event.as_str()),
                                            style: CommunicationStyle::Technical,
                                            output_format: OutputFormat::CodeAndReport,
                                        },
                                        chat_messages: vec![],
                                        execution_context: Some(crate::execution::ExecutionContext::new(
                                            std::env::current_dir().unwrap_or_default(),
                                        )),
                                    },
                                    constraints: TaskConstraints::default(),
                                    timeout: std::time::Duration::from_secs(300),
                                    role_id,
                                };

                                let disp = disp.lock().await;
                                if let Err(e) = disp.send_to_agent(&trigger.agent_id, AgentCommand::AssignTask(assignment)).await {
                                    error!("Trigger {} failed to assign task: {}", trigger.name, e);
                                } else {
                                    state.broadcast_feed(FeedUpdate {
                                        id: Uuid::new_v4(),
                                        agent_id: "trigger".into(),
                                        content: format!("Trigger '{}' fired on {}", trigger.name, event.as_str()),
                                        item_type: "trigger_fired".into(),
                                        timestamp: chrono::Utc::now(),
                                        user_id: None,
                                    });
                                }
                            }
                        }
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

/// Spawn a background task that checks for due schedules every 60 seconds
/// and assigns tasks to agents.
pub fn spawn_schedule_runner(state: AppState) -> Option<tokio::task::JoinHandle<()>> {
    let dispatcher = state.dispatcher.clone()?;
    Some(tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;

            let due_schedules = {
                let mgr = state.schedule_manager.read().await;
                mgr.get_due_schedules(chrono::Utc::now())
                    .into_iter()
                    .cloned()
                    .collect::<Vec<_>>()
            };

            for schedule in due_schedules {
                let role_str = schedule.role.as_deref().unwrap_or("worker");
                let role_id = RoleId::new(role_str);

                let assignment = TaskAssignment {
                    task_id: Uuid::new_v4(),
                    title: schedule.task_title.clone(),
                    description: schedule.task_description.clone(),
                    context: TaskContext {
                        required_reading: vec![],
                        files: vec![],
                        history: vec![],
                        conventions: String::new(),
                        role_context: RoleContext {
                            system_prompt: format!("You are a {} running on schedule: {}", role_str, schedule.name),
                            style: CommunicationStyle::Technical,
                            output_format: OutputFormat::CodeAndReport,
                        },
                        chat_messages: vec![],
                        execution_context: Some(crate::execution::ExecutionContext::new(
                            std::env::current_dir().unwrap_or_default(),
                        )),
                    },
                    constraints: TaskConstraints::default(),
                    timeout: std::time::Duration::from_secs(300),
                    role_id,
                };

                let disp = dispatcher.lock().await;
                if let Err(e) = disp.send_to_agent(&schedule.agent_id, AgentCommand::AssignTask(assignment)).await {
                    error!("Schedule {} failed to assign task: {}", schedule.name, e);
                } else {
                    info!("Schedule '{}' fired, assigned task to agent {}", schedule.name, schedule.agent_id.0);
                    state.broadcast_feed(FeedUpdate {
                        id: Uuid::new_v4(),
                        agent_id: "scheduler".into(),
                        content: format!("Schedule '{}' fired", schedule.name),
                        item_type: "schedule_fired".into(),
                        timestamp: chrono::Utc::now(),
                        user_id: None,
                    });
                }
                drop(disp);

                // Mark as run and persist
                let now = chrono::Utc::now();
                {
                    let mut mgr = state.schedule_manager.write().await;
                    mgr.mark_run(schedule.id, now);
                }
                if let Err(e) = state.repo.update_schedule_last_run(schedule.id.0, now).await {
                    error!("Failed to persist schedule last_run_at: {}", e);
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
                    // Don't remove immediately — the SSE client may not have connected yet.
    // Schedule cleanup after a delay to allow the client to replay the buffer.
    let cleanup_state = state.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(120)).await;
        cleanup_state.remove_response_stream(message_id).await;
    });
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
                // Don't remove immediately — the SSE client may not have connected yet.
    // Schedule cleanup after a delay to allow the client to replay the buffer.
    let cleanup_state = state.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(120)).await;
        cleanup_state.remove_response_stream(message_id).await;
    });
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

    // Don't remove immediately — the SSE client may not have connected yet.
    // Schedule cleanup after a delay to allow the client to replay the buffer.
    let cleanup_state = state.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(120)).await;
        cleanup_state.remove_response_stream(message_id).await;
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::traits::ServerRepo;
    use crate::db::{ChatMessageRow, PipelineRow, PipelineStageRow, ScheduleRow, TriggerRow};
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
        async fn list_pipelines(&self, _user_id: UserId) -> anyhow::Result<Vec<PipelineRow>> { Ok(vec![]) }
        async fn upsert_pipeline(&self, _user_id: UserId, _pipeline: PipelineRow) -> anyhow::Result<()> { Ok(()) }
        async fn delete_pipeline(&self, _pipeline_id: Uuid) -> anyhow::Result<()> { Ok(()) }
        async fn list_pipeline_stages(&self, _pipeline_id: Uuid) -> anyhow::Result<Vec<PipelineStageRow>> { Ok(vec![]) }
        async fn upsert_pipeline_stage(&self, _stage: PipelineStageRow) -> anyhow::Result<()> { Ok(()) }
        async fn list_schedules(&self, _user_id: UserId) -> anyhow::Result<Vec<ScheduleRow>> { Ok(vec![]) }
        async fn upsert_schedule(&self, _user_id: UserId, _schedule: ScheduleRow) -> anyhow::Result<()> { Ok(()) }
        async fn delete_schedule(&self, _schedule_id: Uuid) -> anyhow::Result<()> { Ok(()) }
        async fn update_schedule_last_run(&self, _schedule_id: Uuid, _last_run_at: DateTime<Utc>) -> anyhow::Result<()> { Ok(()) }
        async fn list_triggers(&self, _user_id: UserId) -> anyhow::Result<Vec<TriggerRow>> { Ok(vec![]) }
        async fn upsert_trigger(&self, _user_id: UserId, _trigger: TriggerRow) -> anyhow::Result<()> { Ok(()) }
        async fn delete_trigger(&self, _trigger_id: Uuid) -> anyhow::Result<()> { Ok(()) }
    }

    #[tokio::test]
    async fn orchestrator_sends_error_when_no_api_key() {
        let saved = std::env::var("ANTHROPIC_API_KEY").ok();
        std::env::remove_var("ANTHROPIC_API_KEY");

        let repo: Arc<dyn ServerRepo> = Arc::new(TestRepo::new());
        let (state, orchestrator_rx) = AppState::with_repo(None, repo, None, AppConfig::default());

        let msg_id = Uuid::new_v4();
        let (_buf, mut rx, _done) = state.get_response_stream(msg_id).await;

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
