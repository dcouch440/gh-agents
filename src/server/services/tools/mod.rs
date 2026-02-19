//! Tool service: CRUD for system-wide tools and agent-tool assignments.

use uuid::Uuid;

use crate::db::traits::{AgentRepo, ToolRepo};
use crate::db::ToolRow;

use super::error::ServiceError;
use super::validation;

#[cfg(test)]
mod tests;

// ============================================================================
// Input types
// ============================================================================

pub struct CreateToolInput {
    pub is_admin: bool,
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub parameters: Option<serde_json::Value>,
}

pub struct UpdateToolInput {
    pub is_admin: bool,
    pub tool_id: Uuid,
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub parameters: Option<serde_json::Value>,
}

pub struct SetAgentToolsInput {
    pub user_id: Uuid,
    pub agent_id: Uuid,
    pub tool_ids: Vec<String>,
}

// ============================================================================
// Helpers
// ============================================================================

fn require_admin(is_admin: bool) -> Result<(), ServiceError> {
    if !is_admin {
        return Err(ServiceError::not_found("Resource"));
    }
    Ok(())
}

async fn verify_agent_ownership(
    agent_repo: &dyn AgentRepo,
    user_id: Uuid,
    agent_id: Uuid,
) -> Result<(), ServiceError> {
    let agent = agent_repo
        .get_persisted_agent(agent_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Agent"))?;
    super::ownership::check_system_passthrough(agent.user_id, user_id, "Agent")?;
    Ok(())
}

// ============================================================================
// Service functions
// ============================================================================

pub async fn list_tools(repo: &dyn ToolRepo) -> Result<Vec<ToolRow>, ServiceError> {
    Ok(repo.list_tools().await?)
}

pub async fn get_tool(repo: &dyn ToolRepo, tool_id: Uuid) -> Result<ToolRow, ServiceError> {
    repo.get_tool(tool_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Tool"))
}

pub async fn create_tool(
    repo: &dyn ToolRepo,
    input: CreateToolInput,
) -> Result<ToolRow, ServiceError> {
    require_admin(input.is_admin)?;

    let name = input.name.trim().to_string();
    validation::validate_required(&name, "name")?;

    let display_name = input.display_name.unwrap_or_else(|| name.clone());

    let row = ToolRow {
        id: Uuid::new_v4(),
        name,
        display_name,
        description: input.description.unwrap_or_default(),
        parameters: input.parameters.unwrap_or_else(|| serde_json::json!({})),
        created_at: chrono::Utc::now(),
        version: 1,
    };

    repo.upsert_tool(row.clone()).await?;
    Ok(row)
}

pub async fn update_tool(
    repo: &dyn ToolRepo,
    input: UpdateToolInput,
) -> Result<ToolRow, ServiceError> {
    require_admin(input.is_admin)?;

    let existing = repo
        .get_tool(input.tool_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Tool"))?;

    let updated = ToolRow {
        id: existing.id,
        name: input.name.unwrap_or(existing.name),
        display_name: input.display_name.unwrap_or(existing.display_name),
        description: input.description.unwrap_or(existing.description),
        parameters: input.parameters.unwrap_or(existing.parameters),
        created_at: existing.created_at,
        version: existing.version,
    };

    repo.upsert_tool(updated.clone()).await?;
    Ok(updated)
}

pub async fn delete_tool(
    repo: &dyn ToolRepo,
    is_admin: bool,
    tool_id: Uuid,
) -> Result<(), ServiceError> {
    require_admin(is_admin)?;
    repo.delete_tool(tool_id).await?;
    Ok(())
}

pub async fn get_agent_tools(
    tool_repo: &dyn ToolRepo,
    agent_repo: &dyn AgentRepo,
    user_id: Uuid,
    agent_id: Uuid,
) -> Result<Vec<ToolRow>, ServiceError> {
    verify_agent_ownership(agent_repo, user_id, agent_id).await?;
    Ok(tool_repo.get_agent_tools(agent_id).await?)
}

pub async fn set_agent_tools(
    tool_repo: &dyn ToolRepo,
    agent_repo: &dyn AgentRepo,
    input: SetAgentToolsInput,
) -> Result<Vec<ToolRow>, ServiceError> {
    verify_agent_ownership(agent_repo, input.user_id, input.agent_id).await?;

    let tool_ids: Result<Vec<Uuid>, _> =
        input.tool_ids.iter().map(|s| Uuid::parse_str(s)).collect();
    let tool_ids =
        tool_ids.map_err(|_| ServiceError::validation("Invalid tool ID format, expected UUID"))?;

    tool_repo.set_agent_tools(input.agent_id, tool_ids).await?;
    Ok(tool_repo.get_agent_tools(input.agent_id).await?)
}
