//! Agent execution service: list, get, approve, and mark exemplary.

use uuid::Uuid;

use crate::db::traits::AgentExecutionRepo;
use crate::db::{AgentExecutionRow, ExecutionMessageRow};

use super::error::ServiceError;

#[cfg(test)]
mod tests;

// ============================================================================
// Types
// ============================================================================

/// Result of approving an execution — handler uses this to decide whether to
/// resume the paused DAG.
pub struct ApprovalResult {
    pub execution: AgentExecutionRow,
    /// If `Some`, all interactive reviews for this step are done and the DAG
    /// should be resumed from this step.
    pub resume_step_id: Option<Uuid>,
}

// ============================================================================
// Service functions
// ============================================================================

pub async fn list_agent_executions(
    repo: &dyn AgentExecutionRepo,
    user_id: Uuid,
    status: Option<String>,
) -> Result<Vec<AgentExecutionRow>, ServiceError> {
    Ok(repo.list_agent_executions(user_id, status).await?)
}

pub async fn get_agent_execution(
    repo: &dyn AgentExecutionRepo,
    id: Uuid,
) -> Result<AgentExecutionRow, ServiceError> {
    repo.get_agent_execution(id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Agent execution"))
}

pub async fn list_execution_messages(
    repo: &dyn AgentExecutionRepo,
    execution_id: Uuid,
) -> Result<Vec<ExecutionMessageRow>, ServiceError> {
    // Verify execution exists
    repo.get_agent_execution(execution_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Agent execution"))?;
    Ok(repo.list_execution_messages(execution_id).await?)
}

pub async fn set_exemplary(
    repo: &dyn AgentExecutionRepo,
    id: Uuid,
    is_exemplary: bool,
) -> Result<AgentExecutionRow, ServiceError> {
    repo.get_agent_execution(id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Agent execution"))?;
    Ok(repo.set_execution_exemplary(id, is_exemplary).await?)
}

/// Approve an interactive execution. Returns the updated row plus an optional
/// step ID if all interactive reviews for the step are now complete (caller
/// should resume the DAG).
pub async fn approve_execution(
    repo: &dyn AgentExecutionRepo,
    id: Uuid,
    structured_output: Option<serde_json::Value>,
) -> Result<ApprovalResult, ServiceError> {
    let ae = repo
        .get_agent_execution(id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Agent execution"))?;

    if !ae.is_interactive || ae.status != "awaiting_user" {
        return Err(ServiceError::validation(
            "Execution is not interactive or not awaiting user approval",
        ));
    }

    let updated = repo
        .update_agent_execution_status(id, "completed", ae.output.clone(), structured_output)
        .await?;

    // Check if all interactive reviews for this step are done
    let resume_step_id = if let Some(step_id) = ae.workflow_step_id {
        let all_approved = match repo.list_interactive_executions_for_step(step_id).await {
            Ok(interactive_execs) => interactive_execs
                .iter()
                .all(|iae| iae.status == "completed" || iae.id == id),
            Err(_) => false,
        };
        if all_approved {
            Some(step_id)
        } else {
            None
        }
    } else {
        None
    };

    Ok(ApprovalResult {
        execution: updated,
        resume_step_id,
    })
}
