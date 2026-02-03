//! Orchestrator consumer that processes chat messages via direct LLM calls
//! with Anthropic native tool use.
//!
//! Reads messages from the orchestrator channel, calls the LLM with agent
//! management tools, executes any tool calls, and streams text responses
//! back through the AppState SSE streams.

use std::sync::Arc;

use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::llm::{AnthropicClient, LLMProvider, RateLimitedProvider, RetryingProvider};

use crate::agents::{AgentCommand, AgentResponse, CommunicationStyle, FileContent, OutputFormat, RoleContext, RoleId, TaskAssignment, TaskConstraints, TaskContext, TriggerEvent};

use super::state::{AppState, OrchestratorMessage, StreamChunk};
use super::ws::{AgentUpdate, FeedUpdate, PipelineUpdate, TaskUpdate};


/// Spawn a background task that drains agent responses from the dispatcher
/// and stores them in `state.task_results` for retrieval by `get_task_result`.
pub fn spawn_response_consumer(state: AppState) -> Option<tokio::task::JoinHandle<()>> {
    let dispatcher = state.dispatcher.clone()?;

    Some(tokio::spawn(async move {
        // Extract the response receiver so we can await it without holding the dispatcher lock.
        // This prevents deadlocks when tool handlers (e.g. create_agents) need to lock the dispatcher.
        let mut response_rx = {
            let mut d = dispatcher.lock().await;
            match d.take_response_rx() {
                Some(rx) => rx,
                None => {
                    tracing::error!("Response receiver already taken");
                    return;
                }
            }
        };

        loop {
            let response = response_rx.recv().await;
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
                                content: format!("Approval needed: {} — {}", request.action, request.details),
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
                        AgentResponse::ContextRequest { agent_id, request } => {
                            info!("Context request from agent {:?}: {:?}", agent_id, request);

                            // Auto-resolve file requests
                            let mut resolved_files = Vec::new();
                            for path in &request.files_needed {
                                if let Ok(content) = tokio::fs::read_to_string(path).await {
                                    let truncated = if content.len() > crate::constants::TRUNCATE_CONTEXT_FILE {
                                        content[..crate::constants::TRUNCATE_CONTEXT_FILE].to_string()
                                    } else {
                                        content
                                    };
                                    resolved_files.push(FileContent {
                                        path: path.clone(),
                                        content: truncated,
                                    });
                                }
                            }

                            // Send context back to the agent
                            if !resolved_files.is_empty() {
                                if let Some(disp) = &state.dispatcher {
                                    let disp = disp.lock().await;
                                    let _ = disp
                                        .send_to_agent(
                                            agent_id,
                                            AgentCommand::ProvideContext(crate::agents::ContextResponse {
                                                task_id: request.task_id,
                                                files: resolved_files,
                                                answers: vec![],
                                                true_context: None,
                                            }),
                                        )
                                        .await;
                                }
                            }

                            // Broadcast to feed for UI visibility
                            state.broadcast_feed(FeedUpdate {
                                id: request.task_id,
                                agent_id: agent_id.0.to_string(),
                                content: format!("Context request — questions: {:?}, files: {:?}", request.questions, request.files_needed),
                                item_type: "context_request".into(),
                                timestamp: chrono::Utc::now(),
                                user_id: None,
                            });
                            state.broadcast_agent(AgentUpdate {
                                id: agent_id.0.to_string(),
                                status: "waiting_for_context".into(),
                                current_task: Some(request.task_id),
                                user_id: None,
                            });
                        }
                        _ => {}
                    }

                    // Check for pipeline auto-advance on completion/failure
                    // Tuple: (run_id, stage_number, output, succeeded, input_tokens, output_tokens, duration_ms)
                    let pipeline_advance = match &resp {
                        AgentResponse::TaskCompleted { result, .. } => {
                            let mgr = state.pipeline_manager.read().await;
                            mgr.lookup_run_by_task(result.task_id)
                                .map(|(run_id, stage_number)| (run_id, stage_number, result.output.clone(), true, result.input_tokens, result.output_tokens, result.duration_ms))
                        }
                        AgentResponse::TaskFailed { result, .. } => {
                            let mgr = state.pipeline_manager.read().await;
                            mgr.lookup_run_by_task(result.task_id)
                                .map(|(run_id, stage_number)| (run_id, stage_number, String::new(), false, result.input_tokens, result.output_tokens, result.duration_ms))
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
                        debug!("Response consumer received {:?} for task {}", std::mem::discriminant(&resp), id);
                        state.task_results.write().await.insert(id, resp);
                    }

                    // Pipeline auto-advance (delegated to hub::advance_pipeline)
                    if let Some((run_id, completed_stage_number, prev_output, succeeded, stage_input_tokens, stage_output_tokens, stage_duration_ms)) = pipeline_advance {
                        let stage_output = if succeeded && !prev_output.is_empty() {
                            Some(prev_output.clone())
                        } else {
                            None
                        };

                        match super::hub::advance_pipeline(
                            &state,
                            run_id,
                            completed_stage_number as i32,
                            stage_output,
                            succeeded,
                            stage_input_tokens as i64,
                            stage_output_tokens as i64,
                            stage_duration_ms as i64,
                        )
                        .await
                        {
                            Ok(super::hub::PipelineAdvanceAction::NextStage { stage_number, stage_name }) => {
                                info!("Pipeline {} advanced to stage {} ({})", run_id, stage_number, stage_name);
                                // Dispatch the next stage via the old agent system
                                // (full hub dispatch will replace this in a future phase)
                                let initial_task = {
                                    let mgr = state.pipeline_manager.read().await;
                                    mgr.get_run_initial_task(run_id).unwrap_or_default().to_string()
                                };
                                let stage_outputs = {
                                    let mgr = state.pipeline_manager.read().await;
                                    mgr.get_stage_outputs(run_id).cloned().unwrap_or_default()
                                };
                                let next_stage_info = {
                                    let mgr = state.pipeline_manager.read().await;
                                    mgr.get_run_pipeline_id(run_id).and_then(|pid| {
                                        mgr.get_pipeline(&pid).and_then(|p| {
                                            p.stages.get(stage_number as usize).map(|s| {
                                                (s.agent_id.clone(), s.role.clone())
                                            })
                                        })
                                    })
                                };
                                let pipeline_id_opt = {
                                    let mgr = state.pipeline_manager.read().await;
                                    mgr.get_run_pipeline_id(run_id)
                                };

                                // Try to render stage prompt
                                let rendered_prompt = if let Some(pid) = pipeline_id_opt {
                                    match state.repo.list_pipeline_stages(pid.0).await {
                                        Ok(db_stages) => {
                                            if let Some(db_stage) = db_stages.into_iter().find(|s| s.stage_number == stage_number) {
                                                let doc_repo_ref = state.doc_repo.as_deref();
                                                Some(super::api::render_stage(doc_repo_ref, &db_stage, &stage_outputs).await)
                                            } else {
                                                None
                                            }
                                        }
                                        Err(e) => { warn!("Failed to load pipeline stages: {}", e); None }
                                    }
                                } else { None };

                                let description = rendered_prompt.unwrap_or_else(|| format!("{}\n\nPrevious stage output:\n{}", initial_task, prev_output));
                                let rendered_prompt_copy = description.clone();
                                let mut context_reading: Vec<FileContent> = Vec::new();
                                let mut context_docs: Vec<crate::db::DocumentRow> = Vec::new();

                                let (resolved_agent_id, role_str) = if let Some((agent_id, role)) = next_stage_info {
                                    if let Some(ref aid) = agent_id {
                                        if let Ok(docs) = state.repo.get_agent_context(aid.0).await {
                                            for doc in &docs {
                                                context_reading.push(FileContent {
                                                    path: format!("context:{}", doc.ref_tag.as_deref().filter(|s| !s.is_empty()).unwrap_or(&doc.title)),
                                                    content: doc.content.clone(),
                                                });
                                            }
                                            context_docs = docs;
                                        }
                                    }
                                    (agent_id, role.unwrap_or_else(|| "worker".to_string()))
                                } else {
                                    (None, "worker".to_string())
                                };

                                let role_id = RoleId::new(&role_str);
                                let assignment = TaskAssignment {
                                    task_id: Uuid::new_v4(),
                                    title: format!("Pipeline stage {}: {}", stage_number, initial_task),
                                    description,
                                    context: TaskContext {
                                        required_reading: context_reading,
                                        files: vec![],
                                        history: vec![],
                                        conventions: String::new(),
                                        role_context: RoleContext {
                                            system_prompt: format!("You are a {} working on: {}", role_str, initial_task),
                                            style: CommunicationStyle::Technical,
                                            output_format: OutputFormat::CodeAndReport,
                                        },
                                        chat_messages: vec![],
                                        execution_context: Some(crate::execution::ExecutionContext::new(std::env::current_dir().unwrap_or_default())),
                                        tool_rows: vec![],
                                        router_mode: false,
                                        cluster_routing: None,
                                        context_docs,
                                        distiller_mode: crate::agents::DistillerMode::Blocking,
                                    },
                                    constraints: TaskConstraints::default(),
                                    timeout: std::time::Duration::from_secs(crate::constants::DEFAULT_TIMEOUT_SECS),
                                    role_id,
                                };
                                let new_task_id = assignment.task_id;
                                {
                                    let mut mgr = state.pipeline_manager.write().await;
                                    mgr.record_stage_task(run_id, stage_number as u32, new_task_id);
                                }
                                // Persist stage execution
                                let stage_exec = crate::db::StageExecutionRow {
                                    id: uuid::Uuid::new_v4(),
                                    run_id,
                                    stage_number,
                                    stage_name: stage_name.clone(),
                                    agent_id: resolved_agent_id.as_ref().map(|a| a.0),
                                    status: "running".to_string(),
                                    rendered_prompt: Some(rendered_prompt_copy),
                                    output: None, structured_output: None, user_input: None,
                                    input_tokens: 0, output_tokens: 0,
                                    started_at: chrono::Utc::now(), completed_at: None, duration_ms: 0,
                                    stage_member_id: None, pipeline_id: None,
                                };
                                let _ = state.repo.create_stage_execution(&stage_exec).await;
                                if let Some(ae_repo) = &state.agent_execution_repo {
                                    if let Some(aid) = &resolved_agent_id {
                                        let rendered = stage_exec.rendered_prompt.as_deref().unwrap_or("");
                                        let _ = ae_repo.create_agent_execution(stage_exec.id, aid.0, None, false, None, &initial_task, rendered, None, None, None).await;
                                    }
                                }

                                if let Some(agent_id) = &resolved_agent_id {
                                    if let Some(disp) = &state.dispatcher {
                                        let disp = disp.lock().await;
                                        if let Err(e) = disp.send_to_agent(agent_id, AgentCommand::AssignTask(Box::new(assignment))).await {
                                            error!("Pipeline auto-advance failed: {}", e);
                                            let mut mgr = state.pipeline_manager.write().await;
                                            let _ = mgr.fail_run(run_id, &e.to_string());
                                        } else {
                                            state.broadcast_feed(FeedUpdate {
                                                id: run_id, agent_id: "pipeline".into(),
                                                content: format!("Pipeline advanced to stage {}", stage_number),
                                                item_type: "pipeline_progress".into(),
                                                timestamp: chrono::Utc::now(), user_id: None,
                                            });
                                            let pipeline_id = {
                                                let mgr = state.pipeline_manager.read().await;
                                                mgr.get_run_pipeline_id(run_id).map(|p| p.0).unwrap_or(run_id)
                                            };
                                            state.broadcast_pipeline(PipelineUpdate {
                                                run_id, pipeline_id,
                                                event: "stage_started".into(),
                                                stage_number: Some(stage_number),
                                                stage_name: Some(stage_name),
                                                agent_id: resolved_agent_id.as_ref().map(|a| a.0.to_string()),
                                                output: None, input_tokens: None, output_tokens: None,
                                                duration_ms: None, user_input: None,
                                                timestamp: chrono::Utc::now(), user_id: None,
                                            });
                                        }
                                    }
                                } else {
                                    let reason = format!("Pipeline stage {} has no agent_id", stage_number);
                                    error!("{}", reason);
                                    let mut mgr = state.pipeline_manager.write().await;
                                    let _ = mgr.fail_run(run_id, &reason);
                                }
                            }
                            Ok(super::hub::PipelineAdvanceAction::AwaitingApproval { stage_number, stage_name }) => {
                                info!("Pipeline {} awaiting approval at stage {} ({})", run_id, stage_number, stage_name);
                            }
                            Ok(super::hub::PipelineAdvanceAction::Completed) => {
                                info!("Pipeline {} completed", run_id);
                            }
                            Ok(super::hub::PipelineAdvanceAction::Failed { reason }) => {
                                warn!("Pipeline {} failed: {}", run_id, reason);
                            }
                            Err(e) => {
                                error!("Pipeline advance error for run {}: {}", run_id, e);
                            }
                        }
                    }
                    // Event-driven triggers
                    if let Some(event) = trigger_event {
                        let triggers = {
                            let mgr = state.schedule_manager.read().await;
                            mgr.get_triggers_for_event(event).into_iter().cloned().collect::<Vec<_>>()
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
                                        execution_context: Some(crate::execution::ExecutionContext::new(std::env::current_dir().unwrap_or_default())),
                                        tool_rows: vec![],
                                        router_mode: false,
                                        cluster_routing: None,
                                        context_docs: vec![],
                                        distiller_mode: crate::agents::DistillerMode::Off,
                                    },
                                    constraints: TaskConstraints::default(),
                                    timeout: std::time::Duration::from_secs(crate::constants::DEFAULT_TIMEOUT_SECS),
                                    role_id,
                                };

                                let disp = disp.lock().await;
                                if let Err(e) = disp.send_to_agent(&trigger.agent_id, AgentCommand::AssignTask(Box::new(assignment))).await {
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
                mgr.get_due_schedules(chrono::Utc::now()).into_iter().cloned().collect::<Vec<_>>()
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
                        execution_context: Some(crate::execution::ExecutionContext::new(std::env::current_dir().unwrap_or_default())),
                        tool_rows: vec![],
                        router_mode: false,
                        cluster_routing: None,
                        context_docs: vec![],
                        distiller_mode: crate::agents::DistillerMode::Off,
                    },
                    constraints: TaskConstraints::default(),
                    timeout: std::time::Duration::from_secs(crate::constants::DEFAULT_TIMEOUT_SECS),
                    role_id,
                };

                let disp = dispatcher.lock().await;
                if let Err(e) = disp.send_to_agent(&schedule.agent_id, AgentCommand::AssignTask(Box::new(assignment))).await {
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

                // Mark as run in memory
                let now = chrono::Utc::now();
                {
                    let mut mgr = state.schedule_manager.write().await;
                    mgr.mark_run(schedule.id, now);
                }
            }
        }
    }))
}

/// Spawn the orchestrator consumer as a background task.
pub fn spawn_orchestrator(state: AppState, orchestrator_rx: mpsc::Receiver<OrchestratorMessage>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(run_orchestrator(state, orchestrator_rx))
}

async fn run_orchestrator(state: AppState, mut orchestrator_rx: mpsc::Receiver<OrchestratorMessage>) {
    let provider: Arc<dyn LLMProvider + Send + Sync> = match AnthropicClient::from_env() {
        Ok(p) => {
            info!("Orchestrator started with model: {}", p.model_id().to_string());
            Arc::new(RetryingProvider::with_defaults(RateLimitedProvider::with_defaults(p)))
        }
        Err(e) => {
            error!("Failed to initialize LLM provider: {}. Chat will not work. Set ANTHROPIC_API_KEY.", e);
            while let Some(msg) = orchestrator_rx.recv().await {
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

    while let Some(msg) = orchestrator_rx.recv().await {
        let state = state.clone();
        let provider = Arc::clone(&provider);
        let message_id = msg.id;
        tokio::spawn(async move {
            if let Err(e) = handle_message(&state, provider, msg).await {
                warn!("Orchestrator message handling failed: {}", e);
                state.send_stream_chunk(message_id, StreamChunk::Error(format!("Orchestrator error: {}", e))).await;
                let cleanup_state = state.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(120)).await;
                    cleanup_state.remove_response_stream(message_id).await;
                });
            }
        });
    }

    info!("Orchestrator consumer shutting down (channel closed)");
}

async fn handle_message(state: &AppState, provider: Arc<dyn LLMProvider + Send + Sync>, msg: OrchestratorMessage) -> anyhow::Result<()> {
    let message_id = msg.id;
    let agent_id = msg.agent_id.or(state.default_agent_id);

    match agent_id {
        Some(aid) => {
            match super::hub::run_chat(state, provider, aid, msg.session_id, message_id, &msg.content, msg.user_id, None).await {
                Ok(_) => {}
                Err(e) => {
                    warn!("Chat error for {}: {}", message_id, e);
                    state.send_stream_chunk(message_id, StreamChunk::Error(format!("{}", e))).await;
                    state.send_stream_chunk(message_id, StreamChunk::Done).await;
                }
            }
        }
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
    async fn orchestrator_sends_error_when_no_api_key() {
        let saved = std::env::var(crate::constants::ENV_ANTHROPIC_API_KEY).ok();
        std::env::remove_var(crate::constants::ENV_ANTHROPIC_API_KEY);

        let repo: Arc<dyn ServerRepo> = Arc::new(TestRepo::new());
        let (state, orchestrator_rx) = AppState::with_repo(None, repo, AppConfig::default());

        let msg_id = Uuid::new_v4();
        let (_buf, mut rx, _done) = state.get_response_stream(msg_id).await;

        state
            .orchestrator_tx
            .send(OrchestratorMessage {
                id: msg_id,
                user_id: UserId::new(),
                session_id: None,
                agent_id: None,
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
            std::env::set_var(crate::constants::ENV_ANTHROPIC_API_KEY, key);
        }
    }
}
