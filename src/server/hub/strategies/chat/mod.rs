//! ChatStrategy — replaces the orchestrator's `handle_message` execution loop.
//!
//! Handles interactive chat sessions: loads history, streams tokens, saves
//! messages, auto-names sessions, and triggers compaction.

use async_trait::async_trait;
use serde_json::Value;
use tracing::error;
use uuid::Uuid;

use crate::llm::{Message, Role, TokenUsage, Tool};
use crate::server::state::AppState;
use crate::server::tools;
use crate::server::ws::events::{WorkflowEvent, WorkflowEventKind};
use crate::types::UserId;

use super::super::error::HubError;
use super::super::strategy::ExecutionStrategy;

/// Configuration for a chat execution.
pub struct ChatConfig {
    pub system_prompt: String,
    pub tool_names: Vec<String>,
    pub model_id: String,
    pub max_rounds: u32,
    pub context_budget: usize,
    pub temperature: f32,
    pub max_history: u32,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            system_prompt: String::new(),
            tool_names: vec![],
            model_id: String::new(),
            max_rounds: 10,
            context_budget: 480_000,
            temperature: crate::constants::DEFAULT_TEMPERATURE,
            max_history: 50,
        }
    }
}

/// Optional context for step-scoped chat sessions.
///
/// When present, `execute_tool` routes step-specific tools to the
/// appropriate dispatcher (e.g., documenter tools) instead of
/// generic server tools.
pub struct StepChatContext {
    pub workflow_id: Uuid,
    pub step_id: Uuid,
    pub execution_mode: String,
}

/// Strategy for interactive chat sessions.
///
/// Loads session history, executes server tools (agent management, docs, etc.),
/// and handles post-processing (save message, auto-name, compaction).
pub struct ChatStrategy {
    config: ChatConfig,
    state: AppState,
    user_id: UserId,
    session_id: Option<Uuid>,
    message_id: Uuid,
    step_context: Option<StepChatContext>,
}

impl ChatStrategy {
    pub fn new(
        config: ChatConfig,
        state: AppState,
        user_id: UserId,
        session_id: Option<Uuid>,
        message_id: Uuid,
    ) -> Self {
        Self {
            config,
            state,
            user_id,
            session_id,
            message_id,
            step_context: None,
        }
    }

    /// Create a ChatStrategy with step context for step-scoped chat sessions.
    pub fn with_step_context(
        config: ChatConfig,
        state: AppState,
        user_id: UserId,
        session_id: Option<Uuid>,
        message_id: Uuid,
        step_context: StepChatContext,
    ) -> Self {
        Self {
            config,
            state,
            user_id,
            session_id,
            message_id,
            step_context: Some(step_context),
        }
    }

    /// Broadcast a workflow event when a documenter tool mutates data.
    ///
    /// Only emits if the step context is present and the tool result
    /// indicates success (no `"error"` key in the JSON).
    fn broadcast_documenter_event(&self, name: &str, input: &Value, result: &Value) {
        let Some(ref ctx) = self.step_context else {
            return;
        };

        // Skip if the tool returned an error
        if result.get("error").is_some() {
            return;
        }

        let kind = match name {
            "create_doc_def" => {
                let doc_def_id = result["id"]
                    .as_str()
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .unwrap_or_else(Uuid::new_v4);
                let doc_name = result["name"].as_str().unwrap_or("Untitled").to_string();
                WorkflowEventKind::DocDefCreated {
                    step_id: ctx.step_id,
                    doc_def_id,
                    name: doc_name,
                }
            }
            "update_doc_def" => {
                let doc_def_id = result["id"]
                    .as_str()
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .or_else(|| {
                        input["doc_def_id"]
                            .as_str()
                            .and_then(|s| Uuid::parse_str(s).ok())
                    })
                    .unwrap_or_else(Uuid::new_v4);
                let doc_name = result["name"].as_str().unwrap_or("Untitled").to_string();
                WorkflowEventKind::DocDefUpdated {
                    step_id: ctx.step_id,
                    doc_def_id,
                    name: doc_name,
                }
            }
            "delete_doc_def" => {
                let doc_def_id = input["doc_def_id"]
                    .as_str()
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .unwrap_or_else(Uuid::new_v4);
                WorkflowEventKind::DocDefDeleted {
                    step_id: ctx.step_id,
                    doc_def_id,
                }
            }
            "update_config" => WorkflowEventKind::StepConfigUpdated {
                step_id: ctx.step_id,
            },
            _ => return,
        };

        self.state.broadcast_workflow(WorkflowEvent {
            run_id: None,
            workflow_id: ctx.workflow_id,
            user_id: Some(self.user_id.0),
            kind,
        });
    }
}

#[async_trait]
impl ExecutionStrategy for ChatStrategy {
    fn system_prompt(&self) -> &str {
        &self.config.system_prompt
    }

    fn tools(&self) -> Vec<Tool> {
        if let Some(ref ctx) = self.step_context {
            return resolve_step_tools(&ctx.execution_mode);
        }
        tools::filtered_tools(&self.config.tool_names)
    }

    fn model_id(&self) -> &str {
        &self.config.model_id
    }

    fn max_rounds(&self) -> u32 {
        self.config.max_rounds
    }

    fn context_budget(&self) -> usize {
        self.config.context_budget
    }

    fn streaming(&self) -> bool {
        true
    }

    fn temperature(&self) -> f32 {
        self.config.temperature
    }

    async fn build_messages(&self, input: &str) -> Result<Vec<Message>, HubError> {
        let mut messages = Vec::new();

        // Load session history if applicable
        if let Some(session_id) = self.session_id {
            let history = self
                .state
                .repo()
                .get_session_history(session_id, self.config.max_history)
                .await
                .map_err(|e| anyhow::anyhow!("failed to load session history: {}", e))?;

            for row in &history {
                let msg = match row.role.as_str() {
                    "user" => Message::user(&row.content),
                    "assistant" => Message::assistant(&row.content),
                    _ => continue,
                };
                messages.push(msg);
            }

            // Inject prior context from session summary via distiller
            if let Ok(Some(session)) = self.state.repo().get_session(session_id).await {
                if !session.summary.is_empty() {
                    if let Some(targeted) =
                        tools::haiku_extract_context(&session.summary, input).await
                    {
                        if !targeted.contains("No prior context needed") {
                            messages
                                .insert(0, Message::user(format!("[Prior context] {}", targeted)));
                            messages.insert(
                                1,
                                Message::assistant("Understood, I have the relevant context."),
                            );
                        }
                    }
                }
            }
        }

        // Ensure the current message is included
        if !messages
            .iter()
            .any(|m| m.role == Role::User && m.text() == input)
        {
            messages.push(Message::user(input));
        }
        if messages.is_empty() {
            messages.push(Message::user(input));
        }

        Ok(messages)
    }

    async fn execute_tool(&self, name: &str, input: &Value) -> Value {
        if let Some(ref ctx) = self.step_context {
            if let Some(value) = dispatch_step_tool(name, input, &self.state, ctx).await {
                self.broadcast_documenter_event(name, input, &value);
                return value;
            }
        }
        tools::execute_tool(name, input, &self.state, self.user_id, self.session_id).await
    }

    fn state(&self) -> Option<&AppState> {
        Some(&self.state)
    }

    fn user_id(&self) -> Option<Uuid> {
        Some(self.user_id.0)
    }

    async fn on_complete(&self, response: &str, usage: &TokenUsage) -> Result<(), HubError> {
        super::log_token_usage(
            &self.state,
            self.user_id.0,
            None,
            &self.config.model_id,
            usage,
        )
        .await;

        // Save assistant response
        if !response.is_empty() {
            let response_id = Uuid::new_v4();
            let save_result = if let Some(session_id) = self.session_id {
                self.state
                    .repo()
                    .insert_session_message(
                        self.user_id,
                        session_id,
                        response_id,
                        "assistant".to_string(),
                        response.to_string(),
                    )
                    .await
            } else {
                self.state
                    .repo()
                    .insert_chat_message(
                        self.user_id,
                        response_id,
                        "assistant".to_string(),
                        response.to_string(),
                    )
                    .await
            };
            if let Err(e) = save_result {
                error!("Failed to save assistant message: {}", e);
            }

            // Auto-name session after first exchange
            if let Some(session_id) = self.session_id {
                let state = self.state.clone();
                let user_msg = self.message_id; // used as correlation, not content
                let _ = user_msg; // the actual input text isn't stored on strategy
                                  // Spawn auto-naming in background
                let input_preview = response[..response.len().min(500)].to_string();
                tokio::spawn(async move {
                    if let Ok(Some(session)) = state.repo().get_session(session_id).await {
                        if session.title.starts_with("New ") {
                            if let Some(title) = tools::haiku_summarize_title(&format!(
                                "Conversation opener: {}",
                                input_preview
                            ))
                            .await
                            {
                                let _ = state.repo().update_session_title(session_id, &title).await;
                                state.broadcast_session(crate::server::ws::events::SessionEvent {
                                    session_id,
                                    user_id: None,
                                    kind: crate::server::ws::events::SessionEventKind::Updated {
                                        title: Some(title),
                                        mode_id: None,
                                    },
                                });
                            }
                        }
                    }
                });
            }

            // Spawn background compaction if session has many messages
            if let Some(session_id) = self.session_id {
                let state = self.state.clone();
                tokio::spawn(async move {
                    let count = state
                        .repo()
                        .count_session_messages(session_id)
                        .await
                        .unwrap_or(0);
                    if count > crate::constants::SUMMARIZE_THRESHOLD as u32 {
                        let history = state
                            .repo()
                            .get_session_history(session_id, count)
                            .await
                            .unwrap_or_default();
                        let older_messages: Vec<_> = history
                            .iter()
                            .take(
                                (count as usize)
                                    .saturating_sub(crate::constants::SUMMARIZE_KEEP_RECENT),
                            )
                            .collect();
                        if !older_messages.is_empty() {
                            let conversation_text = older_messages
                                .iter()
                                .map(|m| format!("{}: {}", m.role, m.content))
                                .collect::<Vec<_>>()
                                .join("\n");
                            if let Some(summary) = tools::haiku_summarize(&conversation_text).await
                            {
                                let _ = state
                                    .repo()
                                    .update_session_summary(session_id, &summary)
                                    .await;
                            }
                        }
                    }
                });
            }
        }

        Ok(())
    }
}

// ── Step tool helpers ────────────────────────────────────────────────────────

/// Resolve tool definitions by step execution mode.
fn resolve_step_tools(execution_mode: &str) -> Vec<Tool> {
    let tool_names: &[&str] = match execution_mode {
        "documenter" => &[
            "create_doc_def",
            "update_doc_def",
            "delete_doc_def",
            "update_config",
            "think",
        ],
        _ => &["think"],
    };
    tool_names
        .iter()
        .filter_map(|name| crate::tools::registry::get_tool_definition(name))
        .collect()
}

/// Try to dispatch a tool call to a step-specific handler.
/// Returns `Some(result)` if handled, `None` to fall through to generic tools.
async fn dispatch_step_tool(
    name: &str,
    input: &Value,
    state: &AppState,
    ctx: &StepChatContext,
) -> Option<Value> {
    match ctx.execution_mode.as_str() {
        "documenter" => {
            const DOCUMENTER_TOOLS: &[&str] = &[
                "create_doc_def",
                "update_doc_def",
                "delete_doc_def",
                "update_config",
            ];
            if DOCUMENTER_TOOLS.contains(&name) {
                let tool_ctx = crate::server::tools::documenter::DocumenterToolContext {
                    workflow_id: ctx.workflow_id,
                    step_id: ctx.step_id,
                };
                let result = crate::server::tools::documenter::execute_documenter_tool(
                    name,
                    input,
                    state.repos().workflows.as_ref(),
                    &tool_ctx,
                )
                .await;
                return Some(result);
            }
            None
        }
        _ => None,
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
