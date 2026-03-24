//! Standalone Designer service — query endpoint for Designer output.
//!
//! The `get_latest_design` query endpoint remains functional for viewing
//! past design runs.

use uuid::Uuid;

use crate::db::traits::WorkflowRepo;
use crate::db::{AgentDesignerOutputRow, AgentDesignerRunRow};

use super::ServiceError;

/// Latest design for a step (if any).
#[derive(Debug, Clone)]
pub struct LatestDesign {
    pub run: AgentDesignerRunRow,
    pub outputs: Vec<AgentDesignerOutputRow>,
}

// ── get_latest_design ───────────────────────────────────────────────────────

/// Get the most recent Designer output for a step (across all executions).
pub async fn get_latest_design(
    repo: &dyn WorkflowRepo,
    workflow_id: Uuid,
    step_id: Uuid,
    user_id: Uuid,
) -> Result<Option<LatestDesign>, ServiceError> {
    // Verify ownership
    let workflow = repo
        .get_workflow(workflow_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Workflow"))?;
    if workflow.user_id != user_id {
        return Err(ServiceError::not_found("Workflow"));
    }

    // Check step belongs to workflow
    let step = repo
        .get_step(step_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Step"))?;
    if step.workflow_id != workflow_id {
        return Err(ServiceError::not_found("Step"));
    }

    let run = repo.get_latest_designer_run_for_step(step_id).await?;
    let Some(run) = run else {
        return Ok(None);
    };

    let outputs = repo.list_designer_outputs(run.id).await?;

    Ok(Some(LatestDesign { run, outputs }))
}

mod tests;
