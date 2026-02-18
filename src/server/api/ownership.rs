//! Shared ownership and authorization helpers for API handlers.

use uuid::Uuid;

use super::AppError;
use crate::db::traits::AgentRepo;
use crate::db::AgentRow;
use crate::server::auth::AuthUser;

/// Verify the authenticated user owns this agent.
///
/// Returns the agent row on success, or 404 on mismatch (avoids confirming
/// resource existence to unauthorized users).
pub async fn verify_agent_ownership(
    repo: &dyn AgentRepo,
    auth: &AuthUser,
    agent_id: Uuid,
) -> Result<AgentRow, AppError> {
    let agent = repo
        .get_persisted_agent(agent_id)
        .await?
        .ok_or(AppError::not_found("Agent"))?;
    // System agents (user_id = NULL) are accessible to all users.
    if agent.user_id.is_some() && agent.user_id != Some(auth.user_id.0) {
        return Err(AppError::not_found("Agent"));
    }
    Ok(agent)
}

/// Verify the authenticated user is an admin.
///
/// Returns 404 on non-admin (avoids confirming endpoint existence to
/// regular users).
pub fn require_admin(auth: &AuthUser) -> Result<(), AppError> {
    if !auth.claims.is_admin {
        return Err(AppError::not_found("Resource"));
    }
    Ok(())
}
