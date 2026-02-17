//! Tool router service: create, read, update, delete tool routers and manage
//! their tool assignments.

use uuid::Uuid;

use crate::db::traits::ToolRouterRepo;
use crate::db::{ToolRouterRow, ToolRow};

use super::error::ServiceError;
use super::validation;

/// Verify the caller owns this tool router.
async fn verify_ownership(
    repo: &dyn ToolRouterRepo,
    user_id: Uuid,
    router_id: Uuid,
) -> Result<ToolRouterRow, ServiceError> {
    let router = repo
        .get_tool_router(router_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Tool router"))?;
    if router.user_id != user_id {
        return Err(ServiceError::not_found("Tool router"));
    }
    Ok(router)
}

/// Create a new tool router owned by the given user.
pub async fn create_tool_router(
    repo: &dyn ToolRouterRepo,
    user_id: Uuid,
    name: &str,
    description: Option<String>,
    system_prompt: &str,
    model_id: &str,
) -> Result<ToolRouterRow, ServiceError> {
    validation::validate_name(name, "name")?;
    let row = repo
        .create_tool_router(user_id, name, description, system_prompt, model_id)
        .await?;
    Ok(row)
}

/// Get a single tool router by ID, verifying ownership.
pub async fn get_tool_router(
    repo: &dyn ToolRouterRepo,
    user_id: Uuid,
    router_id: Uuid,
) -> Result<ToolRouterRow, ServiceError> {
    verify_ownership(repo, user_id, router_id).await
}

/// List tool routers for a user.
pub async fn list_tool_routers(
    repo: &dyn ToolRouterRepo,
    user_id: Uuid,
) -> Result<Vec<ToolRouterRow>, ServiceError> {
    let rows = repo.list_tool_routers(user_id).await?;
    Ok(rows)
}

/// Update an existing tool router (partial update).
pub async fn update_tool_router(
    repo: &dyn ToolRouterRepo,
    user_id: Uuid,
    router_id: Uuid,
    name: Option<String>,
    description: Option<String>,
    system_prompt: Option<String>,
    model_id: Option<String>,
    is_active: Option<bool>,
) -> Result<ToolRouterRow, ServiceError> {
    verify_ownership(repo, user_id, router_id).await?;

    if let Some(ref n) = name {
        validation::validate_name(n, "name")?;
    }

    let row = repo
        .update_tool_router(
            router_id,
            name,
            description,
            system_prompt,
            model_id,
            is_active,
        )
        .await?;
    Ok(row)
}

/// Delete a tool router by ID, verifying ownership.
pub async fn delete_tool_router(
    repo: &dyn ToolRouterRepo,
    user_id: Uuid,
    router_id: Uuid,
) -> Result<(), ServiceError> {
    verify_ownership(repo, user_id, router_id).await?;
    repo.delete_tool_router(router_id).await?;
    Ok(())
}

/// Get all tools assigned to a tool router, verifying ownership.
pub async fn get_router_tools(
    repo: &dyn ToolRouterRepo,
    user_id: Uuid,
    router_id: Uuid,
) -> Result<Vec<ToolRow>, ServiceError> {
    verify_ownership(repo, user_id, router_id).await?;
    let tools = repo.get_router_tools(router_id).await?;
    Ok(tools)
}

/// Set the full tool list for a router, verifying ownership.
pub async fn set_router_tools(
    repo: &dyn ToolRouterRepo,
    user_id: Uuid,
    router_id: Uuid,
    tool_ids: &[Uuid],
) -> Result<(), ServiceError> {
    verify_ownership(repo, user_id, router_id).await?;
    repo.set_router_tools(router_id, tool_ids).await?;
    Ok(())
}

mod tests;
