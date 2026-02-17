//! Chat service: message validation, history retrieval, and history clearing.

use uuid::Uuid;

use crate::db::traits::ServerRepo;
use crate::db::ChatMessageRow;
use crate::types::UserId;

use super::error::ServiceError;

#[cfg(test)]
mod tests;

// ============================================================================
// Service functions
// ============================================================================

/// Validate a chat message (non-empty, within length limit).
pub fn validate_message(message: &str, max_length: usize) -> Result<(), ServiceError> {
    if message.trim().is_empty() || message.len() > max_length {
        return Err(ServiceError::validation(
            "Message must be non-empty and within the maximum length",
        ));
    }
    Ok(())
}

/// Store a user chat message in the database.
pub async fn store_message(
    repo: &dyn ServerRepo,
    user_id: UserId,
    message_id: Uuid,
    content: String,
) -> Result<(), ServiceError> {
    repo.insert_chat_message(user_id, message_id, "user".to_string(), content)
        .await?;
    Ok(())
}

pub async fn get_chat_history(
    repo: &dyn ServerRepo,
    user_id: UserId,
    limit: u32,
    offset: u32,
) -> Result<Vec<ChatMessageRow>, ServiceError> {
    Ok(repo.get_chat_history(user_id, limit, offset).await?)
}

pub async fn clear_chat_history(
    repo: &dyn ServerRepo,
    user_id: UserId,
) -> Result<(), ServiceError> {
    repo.clear_chat_history(user_id).await?;
    Ok(())
}
