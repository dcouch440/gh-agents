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
        }
    }
}

#[async_trait]
impl ExecutionStrategy for ChatStrategy {
    fn system_prompt(&self) -> &str {
        &self.config.system_prompt
    }

    fn tools(&self) -> Vec<Tool> {
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
        tools::execute_tool(name, input, &self.state, self.user_id, self.session_id).await
    }

    async fn on_complete(&self, response: &str, usage: &TokenUsage) -> Result<(), HubError> {
        // Record token usage to ledger
        if let Some(tl_repo) = self.state.token_ledger_repo() {
            let cost = super::compute_cost(
                &self.config.model_id,
                usage.input_tokens as i64,
                usage.output_tokens as i64,
            );
            let _ = tl_repo
                .insert_ledger_entry(
                    self.user_id.0,
                    None,
                    &self.config.model_id,
                    usage.input_tokens as i64,
                    usage.output_tokens as i64,
                    cost,
                )
                .await;
        }

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

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
