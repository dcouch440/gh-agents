use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::db::{AgentGuidanceRow, AgentRow, DocumentRow, ToolRow};
use crate::types::UserId;

// ============================================================================
// Agent Repository
// ============================================================================

/// Database operations for agent persistence, context, and guidance.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AgentRepo: Send + Sync {
    /// List all agents for a user (includes system agents where user_id IS NULL).
    async fn list_persisted_agents(&self, user_id: UserId) -> Result<Vec<AgentRow>>;

    /// Insert or update an agent definition.
    async fn upsert_agent(&self, agent: AgentRow) -> Result<()>;

    /// Get a single agent by ID.
    async fn get_persisted_agent(&self, agent_id: Uuid) -> Result<Option<AgentRow>>;

    /// Get multiple agents by their IDs in a single query.
    async fn get_agents_by_ids(&self, agent_ids: &[Uuid]) -> Result<Vec<AgentRow>>;

    /// Delete an agent by ID.
    async fn delete_persisted_agent(&self, agent_id: Uuid) -> Result<()>;

    /// Get all context documents assigned to an agent.
    async fn get_agent_context(&self, agent_id: Uuid) -> Result<Vec<DocumentRow>>;

    /// Set the full context document list for an agent (replaces existing).
    async fn set_agent_context(&self, agent_id: Uuid, document_ids: Vec<Uuid>) -> Result<()>;

    /// Load active guidances for an agent, optionally filtered by step.
    async fn get_agent_guidances(
        &self,
        agent_id: Uuid,
        step_id: Option<Uuid>,
    ) -> Result<Vec<AgentGuidanceRow>>;
}

// ============================================================================
// Tool Repository
// ============================================================================

/// Database operations for tool persistence and agent-tool linkage.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ToolRepo: Send + Sync {
    /// List all tools (system-wide).
    async fn list_tools(&self) -> Result<Vec<ToolRow>>;

    /// Get a tool by ID.
    async fn get_tool(&self, tool_id: Uuid) -> Result<Option<ToolRow>>;

    /// Insert or update a tool (system-wide).
    async fn upsert_tool(&self, tool: ToolRow) -> Result<()>;

    /// Delete a tool by ID.
    async fn delete_tool(&self, tool_id: Uuid) -> Result<()>;

    /// Get all tools assigned to an agent.
    async fn get_agent_tools(&self, agent_id: Uuid) -> Result<Vec<ToolRow>>;

    /// Get tools for multiple agents in a single query.
    /// Returns `(agent_id, ToolRow)` pairs; the caller groups by agent.
    async fn get_tools_for_agents(&self, agent_ids: &[Uuid]) -> Result<Vec<(Uuid, ToolRow)>>;

    /// Set the full tool list for an agent (replaces existing).
    async fn set_agent_tools(&self, agent_id: Uuid, tool_ids: Vec<Uuid>) -> Result<()>;

    /// Seed the built-in execution tools (system-wide). Idempotent.
    async fn seed_builtin_tools(&self) -> Result<()>;
}
