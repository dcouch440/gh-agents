//! Agent service: create, read, update, delete agents.

use uuid::Uuid;

use crate::db::traits::AgentRepo;
use crate::db::AgentRow;
use crate::types::UserId;

use super::error::ServiceError;
use super::validation;

/// Input for creating a new agent.
pub struct CreateAgentInput {
    pub user_id: Uuid,
    pub name: String,
    pub system_prompt: Option<String>,
    pub persona_style: Option<String>,
    pub model_provider: Option<String>,
    pub model_id: String,
    pub model_max_tokens: Option<i32>,
    pub model_temperature: Option<f32>,
    pub output_schema_id: Option<Uuid>,
}

/// Input for updating an existing agent.
pub struct UpdateAgentInput {
    pub name: Option<String>,
    pub system_prompt: Option<String>,
    pub persona_style: Option<String>,
    pub model_provider: Option<String>,
    pub model_id: Option<String>,
    pub model_max_tokens: Option<i32>,
    pub model_temperature: Option<f32>,
    pub output_schema_id: Option<Uuid>,
}

/// Verify the caller owns this agent. System agents (user_id = NULL) are
/// accessible to all users.
async fn verify_ownership(
    repo: &dyn AgentRepo,
    user_id: Uuid,
    agent_id: Uuid,
) -> Result<AgentRow, ServiceError> {
    let agent = repo
        .get_persisted_agent(agent_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Agent"))?;
    if agent.user_id.is_some() && agent.user_id != Some(user_id) {
        return Err(ServiceError::not_found("Agent"));
    }
    Ok(agent)
}

/// Create a new agent.
pub async fn create_agent(
    repo: &dyn AgentRepo,
    input: CreateAgentInput,
) -> Result<AgentRow, ServiceError> {
    validation::validate_required(&input.model_id, "model_id")?;
    if input.name.len() > crate::constants::MAX_TITLE_LENGTH {
        return Err(ServiceError::validation("Name exceeds maximum length"));
    }
    if let Some(ref prompt) = input.system_prompt {
        validation::validate_prompt(prompt)?;
    }

    let row = AgentRow {
        id: Uuid::new_v4(),
        user_id: Some(input.user_id),
        tier: None,
        name: input.name.trim().to_string(),
        system_prompt: input.system_prompt.unwrap_or_default(),
        persona_style: Some(input.persona_style.unwrap_or_else(|| "casual".to_string())),
        model_provider: input
            .model_provider
            .unwrap_or_else(|| "anthropic".to_string()),
        model_id: input.model_id.trim().to_string(),
        model_max_tokens: input.model_max_tokens.unwrap_or(4096),
        model_temperature: input.model_temperature.unwrap_or(0.7),
        status: Some("idle".to_string()),
        output_schema_id: input.output_schema_id,
        version: 1,
        default_reasoning_trace: None,
        is_system: false,
    };

    repo.upsert_agent(row.clone()).await?;
    Ok(row)
}

/// Get a single agent by ID, verifying ownership.
pub async fn get_agent(
    repo: &dyn AgentRepo,
    user_id: Uuid,
    agent_id: Uuid,
) -> Result<AgentRow, ServiceError> {
    verify_ownership(repo, user_id, agent_id).await
}

/// List agents for a user.
pub async fn list_agents(
    repo: &dyn AgentRepo,
    user_id: UserId,
) -> Result<Vec<AgentRow>, ServiceError> {
    let rows = repo.list_persisted_agents(user_id).await?;
    Ok(rows)
}

/// Update an existing agent (partial update).
pub async fn update_agent(
    repo: &dyn AgentRepo,
    user_id: Uuid,
    agent_id: Uuid,
    input: UpdateAgentInput,
) -> Result<AgentRow, ServiceError> {
    let existing = verify_ownership(repo, user_id, agent_id).await?;

    let updated = AgentRow {
        id: existing.id,
        user_id: Some(user_id),
        tier: None,
        name: input.name.unwrap_or(existing.name),
        system_prompt: input.system_prompt.unwrap_or(existing.system_prompt),
        persona_style: input
            .persona_style
            .map(Some)
            .unwrap_or(existing.persona_style),
        model_provider: input.model_provider.unwrap_or(existing.model_provider),
        model_id: input.model_id.unwrap_or(existing.model_id),
        model_max_tokens: input.model_max_tokens.unwrap_or(existing.model_max_tokens),
        model_temperature: input
            .model_temperature
            .unwrap_or(existing.model_temperature),
        status: existing.status,
        output_schema_id: input.output_schema_id.or(existing.output_schema_id),
        version: existing.version,
        default_reasoning_trace: existing.default_reasoning_trace,
        is_system: existing.is_system,
    };

    repo.upsert_agent(updated.clone()).await?;
    Ok(updated)
}

/// Delete an agent by ID, verifying ownership.
pub async fn delete_agent(
    repo: &dyn AgentRepo,
    user_id: Uuid,
    agent_id: Uuid,
) -> Result<(), ServiceError> {
    let agent = verify_ownership(repo, user_id, agent_id).await?;

    if agent.is_system {
        return Err(ServiceError::validation("Cannot delete system agents"));
    }

    repo.delete_persisted_agent(agent_id).await?;
    Ok(())
}

mod tests;
