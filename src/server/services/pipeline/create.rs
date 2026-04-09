//! Pipeline creation: child workflow for a parent step.

use uuid::Uuid;

use crate::db::traits::{CreateWorkflowInput, WorkflowRepo};
use crate::server::services::ServiceError;

use super::types::{PipelineContext, PipelineCreated};

/// Create a child workflow (pipeline) for a parent step.
///
/// If the parent step already has a `child_workflow_id`, returns the existing
/// pipeline. Otherwise creates a new workflow and links it to the parent step.
pub async fn create_pipeline(
    repo: &dyn WorkflowRepo,
    ctx: &PipelineContext,
    user_id: Uuid,
) -> Result<PipelineCreated, ServiceError> {
    let step = repo
        .get_step(ctx.parent_step_id)
        .await
        .map_err(ServiceError::Internal)?
        .ok_or_else(|| ServiceError::not_found("Parent step"))?;

    // Return existing pipeline if already linked
    if let Some(pipeline_id) = step.child_workflow_id {
        return Ok(PipelineCreated { pipeline_id });
    }

    let step_name = step.name.clone().unwrap_or_else(|| "Pipeline".to_string());

    let child_workflow = repo
        .create_workflow(CreateWorkflowInput {
            user_id,
            name: format!("{} (child)", step_name),
            description: String::new(),
            container_enabled: false,
            target_repo_url: None,
            target_branch: None,
            vpn_enabled: false,
        })
        .await
        .map_err(ServiceError::Internal)?;

    // Link child workflow to parent step.
    // If the link fails, delete the child workflow to prevent orphans.
    let mut updated_step = step;
    updated_step.child_workflow_id = Some(child_workflow.id);
    if let Err(e) = repo.update_step(updated_step).await {
        tracing::warn!(
            child_workflow_id = %child_workflow.id,
            parent_step_id = %ctx.parent_step_id,
            error = %e,
            "Failed to link child workflow to parent step — deleting orphan"
        );
        let _ = repo.delete_workflow(child_workflow.id).await;
        return Err(ServiceError::Internal(e));
    }

    Ok(PipelineCreated {
        pipeline_id: child_workflow.id,
    })
}
