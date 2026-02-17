//! Agent context service: manage document linkage for agents.

use uuid::Uuid;

use crate::db::traits::ServerRepo;
use crate::db::DocumentRow;

use super::error::ServiceError;

#[cfg(test)]
mod tests;

// ============================================================================
// Input types
// ============================================================================

pub struct SetAgentContextInput {
    pub agent_id: Uuid,
    pub document_ids: Vec<String>,
}

// ============================================================================
// Service functions
// ============================================================================

pub async fn get_agent_context(
    repo: &dyn ServerRepo,
    agent_id: Uuid,
) -> Result<Vec<DocumentRow>, ServiceError> {
    Ok(repo.get_agent_context(agent_id).await?)
}

pub async fn set_agent_context(
    repo: &dyn ServerRepo,
    input: SetAgentContextInput,
) -> Result<Vec<DocumentRow>, ServiceError> {
    let document_ids: Result<Vec<Uuid>, _> = input
        .document_ids
        .iter()
        .map(|s| Uuid::parse_str(s))
        .collect();
    let document_ids = document_ids
        .map_err(|_| ServiceError::validation("Invalid document ID format, expected UUID"))?;

    repo.set_agent_context(input.agent_id, document_ids).await?;
    Ok(repo.get_agent_context(input.agent_id).await?)
}
