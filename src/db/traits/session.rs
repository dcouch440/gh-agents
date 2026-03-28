use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::db::{ChatMessageRow, SessionRow};
use crate::types::{User, UserId};

// ============================================================================
// Session Repository
// ============================================================================

/// Database operations for chat sessions and session-scoped messages.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SessionRepo: Send + Sync {
    /// Create a new chat session.
    async fn create_session(
        &self,
        user_id: UserId,
        session_id: Uuid,
        mode_id: &str,
        title: &str,
        agent_id: Option<Uuid>,
        draft_config: Option<serde_json::Value>,
    ) -> Result<()>;

    /// List sessions for a user.
    async fn list_sessions(&self, user_id: UserId) -> Result<Vec<SessionRow>>;

    /// Get a session by ID.
    async fn get_session(&self, session_id: Uuid) -> Result<Option<SessionRow>>;

    /// Delete a session and its messages.
    async fn delete_session(&self, session_id: Uuid) -> Result<()>;

    /// Insert a chat message scoped to a session.
    async fn insert_session_message(
        &self,
        user_id: UserId,
        session_id: Uuid,
        id: Uuid,
        role: String,
        content: String,
    ) -> Result<()>;

    /// Insert an agent-sourced message into a session.
    async fn insert_agent_message(
        &self,
        user_id: UserId,
        session_id: Uuid,
        id: Uuid,
        role: String,
        content: String,
        source_type: String,
    ) -> Result<()>;

    /// Get chat history for a session.
    async fn get_session_history(
        &self,
        session_id: Uuid,
        limit: u32,
    ) -> Result<Vec<ChatMessageRow>>;

    /// Update the title for a session.
    async fn update_session_title(&self, session_id: Uuid, title: &str) -> Result<()>;

    /// Update the summary for a session.
    async fn update_session_summary(&self, session_id: Uuid, summary: &str) -> Result<()>;

    /// Count messages in a session.
    async fn count_session_messages(&self, session_id: Uuid) -> Result<u32>;

    /// Update draft_config for a session.
    async fn update_session_draft_config(
        &self,
        session_id: Uuid,
        draft_config: Option<serde_json::Value>,
    ) -> Result<()>;

    /// Clear all messages for a session.
    async fn clear_session_messages(&self, session_id: Uuid) -> Result<()>;

    /// Find a chat session linked to a workflow step via draft_config.
    /// Excludes builder sessions (L4) — only returns assistant sessions.
    async fn find_session_by_step_id(&self, step_id: Uuid) -> Result<Option<SessionRow>>;

    /// Find the L4 builder session for a workflow step.
    async fn find_builder_session_by_step_id(&self, step_id: Uuid) -> Result<Option<SessionRow>>;

    /// Find a dispatch session for a step by role (e.g., "builder", "system_agent").
    async fn find_session_by_step_id_and_role(
        &self,
        step_id: Uuid,
        role: &str,
    ) -> Result<Option<SessionRow>>;

    /// Find the L2 manager builder session for a workflow.
    async fn find_manager_builder_session(&self, workflow_id: Uuid) -> Result<Option<SessionRow>>;

    /// Find the workflow agent session for a workflow.
    async fn find_workflow_agent_session(&self, workflow_id: Uuid) -> Result<Option<SessionRow>>;

    /// Batch-check which steps have received initial instructions.
    /// Returns the set of step_ids that have been instructed.
    async fn check_initial_instructions_sent(
        &self,
        step_ids: &[Uuid],
    ) -> Result<std::collections::HashSet<Uuid>>;

    /// Link an agent to a session (and clear draft_config).
    async fn link_session_agent(&self, session_id: Uuid, agent_id: Uuid) -> Result<()>;
}

// ============================================================================
// Chat Message Repository
// ============================================================================

/// Database operations for global (non-session) chat messages.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ChatMessageRepo: Send + Sync {
    /// Insert a chat message.
    async fn insert_chat_message(
        &self,
        user_id: UserId,
        id: Uuid,
        role: String,
        content: String,
    ) -> Result<()>;

    /// Get chat history with pagination.
    async fn get_chat_history(
        &self,
        user_id: UserId,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<ChatMessageRow>>;

    /// Clear all chat history.
    async fn clear_chat_history(&self, user_id: UserId) -> Result<()>;
}

// ============================================================================
// Auth Config Repository
// ============================================================================

/// Database operations for authentication configuration and health checks.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AuthConfigRepo: Send + Sync {
    /// Check database connectivity (returns true if alive).
    async fn health_check(&self) -> bool;

    /// Check if a password has been configured.
    async fn has_password(&self) -> Result<bool>;

    /// Store the password hash.
    async fn set_password(&self, password_hash: String) -> Result<()>;

    /// Get the stored password hash.
    async fn get_password(&self) -> Result<Option<String>>;
}

// ============================================================================
// User Repository
// ============================================================================

/// Database operations for user management.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait UserRepo: Send + Sync {
    /// Create a new user with email and password.
    async fn create_user(&self, email: &str, password_hash: &str) -> Result<User>;
    /// Get a user by email.
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>>;
    /// Get a user by ID.
    async fn get_user_by_id(&self, id: UserId) -> Result<Option<User>>;
    /// Get a user by GitHub ID.
    async fn get_user_by_github_id(&self, github_id: i64) -> Result<Option<User>>;
    /// Link GitHub account to existing user.
    async fn link_github(
        &self,
        user_id: UserId,
        github_id: i64,
        github_login: &str,
        token_encrypted: &str,
    ) -> Result<()>;
    /// Create a new user from GitHub OAuth.
    async fn create_github_user(
        &self,
        email: &str,
        github_id: i64,
        github_login: &str,
        token_encrypted: &str,
    ) -> Result<User>;
}
