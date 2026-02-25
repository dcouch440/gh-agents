//! Post-processing after chat completion: token logging, message saving,
//! session auto-naming, compaction, and belief extraction.

use tracing::error;
use uuid::Uuid;

use crate::llm::TokenUsage;
use crate::server::state::AppState;
use crate::server::tools;
use crate::types::UserId;

use super::config::StepChatContext;
use crate::server::hub::error::HubError;

/// Post-processing after the final LLM response.
///
/// Logs token usage, saves the assistant message, spawns background tasks
/// for session auto-naming and compaction, and triggers belief extraction
/// for step-scoped chats.
pub(super) async fn on_chat_complete(
    state: &AppState,
    user_id: UserId,
    session_id: Option<Uuid>,
    step_context: Option<&StepChatContext>,
    model_id: &str,
    response: &str,
    usage: &TokenUsage,
) -> Result<(), HubError> {
    super::super::log_token_usage(state, user_id.0, None, model_id, usage).await;

    // Save assistant response
    if !response.is_empty() {
        let response_id = Uuid::new_v4();
        let save_result = if let Some(session_id) = session_id {
            state
                .repos()
                .sessions
                .insert_session_message(
                    user_id,
                    session_id,
                    response_id,
                    "assistant".to_string(),
                    response.to_string(),
                )
                .await
        } else {
            state
                .repos()
                .chat_messages
                .insert_chat_message(
                    user_id,
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
        if let Some(session_id) = session_id {
            let input_preview = response[..response.len().min(500)].to_string();
            spawn_auto_naming(state.clone(), session_id, input_preview);
        }

        // Spawn background compaction if session has many messages
        if let Some(session_id) = session_id {
            spawn_compaction(state.clone(), session_id);
        }
    }

    // Spawn background belief extraction + question extraction for step-scoped chats
    if let (Some(ctx), Some(session_id)) = (step_context, session_id) {
        crate::server::hub::chat_beliefs::spawn_chat_belief_extraction(
            state.clone(),
            ctx.workflow_id,
            ctx.step_id,
            session_id,
        );
        crate::server::hub::question_extraction::spawn_question_extraction(
            state.clone(),
            ctx.workflow_id,
            ctx.step_id,
            session_id,
        );
    }

    Ok(())
}

/// Spawn a background task to auto-name the session if it still has the
/// default "New ..." title.
fn spawn_auto_naming(state: AppState, session_id: Uuid, input_preview: String) {
    tokio::spawn(async move {
        if let Ok(Some(session)) = state.repos().sessions.get_session(session_id).await {
            if session.title.starts_with("New ") {
                if let Some(title) =
                    tools::haiku_summarize_title(&format!("Conversation opener: {}", input_preview))
                        .await
                {
                    let _ = state
                        .repos()
                        .sessions
                        .update_session_title(session_id, &title)
                        .await;
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

/// Spawn a background task to summarize and compact older messages when the
/// session exceeds the configured threshold.
fn spawn_compaction(state: AppState, session_id: Uuid) {
    tokio::spawn(async move {
        let count = state
            .repos()
            .sessions
            .count_session_messages(session_id)
            .await
            .unwrap_or(0);
        if count > crate::constants::SUMMARIZE_THRESHOLD as u32 {
            let history = state
                .repos()
                .sessions
                .get_session_history(session_id, count)
                .await
                .unwrap_or_default();
            let older_messages: Vec<_> = history
                .iter()
                .take((count as usize).saturating_sub(crate::constants::SUMMARIZE_KEEP_RECENT))
                .collect();
            if !older_messages.is_empty() {
                let conversation_text = older_messages
                    .iter()
                    .map(|m| format!("{}: {}", m.role, m.content))
                    .collect::<Vec<_>>()
                    .join("\n");
                if let Some(summary) = tools::haiku_summarize(&conversation_text).await {
                    let _ = state
                        .repos()
                        .sessions
                        .update_session_summary(session_id, &summary)
                        .await;
                }
            }
        }
    });
}
