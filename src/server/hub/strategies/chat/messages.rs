//! Message building for chat sessions.

use uuid::Uuid;

use crate::llm::{Message, Role};
use crate::server::state::AppState;
use crate::server::tools;

use super::super::super::error::HubError;

/// Build the initial message list for a chat session.
///
/// Loads session history, optionally injects prior context from the session
/// summary via a distiller LLM call, and ensures the current user input is
/// included.
pub(super) async fn build_chat_messages(
    state: &AppState,
    session_id: Option<Uuid>,
    max_history: u32,
    input: &str,
) -> Result<Vec<Message>, HubError> {
    let mut messages = Vec::new();

    // Load session history if applicable
    if let Some(session_id) = session_id {
        let history = state
            .repo()
            .get_session_history(session_id, max_history)
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
        if let Ok(Some(session)) = state.repo().get_session(session_id).await {
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
