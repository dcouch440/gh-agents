//! Pipeline creation: child workflow + optional Designer step.

use uuid::Uuid;

use crate::db::traits::{CreateWorkflowInput, WorkflowRepo};
use crate::db::WorkflowStepRow;
use crate::server::services::ServiceError;

use super::types::{PipelineContext, PipelineCreated};

/// Create a child workflow (pipeline) for a parent step.
///
/// If the parent step already has a `child_workflow_id`, returns the existing
/// pipeline. Otherwise creates a new workflow, links it to the parent step,
/// and optionally creates the auto-managed Designer step.
pub async fn create_pipeline(
    repo: &dyn WorkflowRepo,
    ctx: &PipelineContext,
    user_id: Uuid,
    include_designer: bool,
) -> Result<PipelineCreated, ServiceError> {
    let step = repo
        .get_step(ctx.parent_step_id)
        .await
        .map_err(|e| ServiceError::Internal(e.into()))?
        .ok_or_else(|| ServiceError::not_found("Parent step"))?;

    // Return existing pipeline if already linked
    if let Some(pipeline_id) = step.child_workflow_id {
        let designer_step_id = find_designer_step(repo, pipeline_id).await?;
        return Ok(PipelineCreated {
            pipeline_id,
            designer_step_id,
        });
    }

    let step_name = step
        .name
        .clone()
        .unwrap_or_else(|| "Pipeline".to_string());

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
        .map_err(|e| ServiceError::Internal(e.into()))?;

    // Link child workflow to parent step
    let mut updated_step = step;
    updated_step.child_workflow_id = Some(child_workflow.id);
    repo.update_step(updated_step)
        .await
        .map_err(|e| ServiceError::Internal(e.into()))?;

    // Optionally create Designer step
    let designer_step_id = if include_designer {
        let designer = create_designer_step(repo, child_workflow.id).await?;
        Some(designer.id)
    } else {
        None
    };

    Ok(PipelineCreated {
        pipeline_id: child_workflow.id,
        designer_step_id,
    })
}

/// Create the auto-managed Designer step in a pipeline.
async fn create_designer_step(
    repo: &dyn WorkflowRepo,
    pipeline_id: Uuid,
) -> Result<WorkflowStepRow, ServiceError> {
    let step = WorkflowStepRow {
        id: Uuid::new_v4(),
        workflow_id: pipeline_id,
        agent_id: None,
        execution_mode: "single".to_string(),
        agent_execution_mode: None,
        for_each_ref: None,
        prompt_template_id: None,
        prompt_template: String::new(),
        output_schema_id: None,
        output_variable_name: Some("designer_output".to_string()),
        interactive_agent_id: None,
        for_each_label_field: None,
        room_id: None,
        routing_mode: None,
        routing_field: None,
        display_order: 0,
        version: 1,
        reasoning_trace: false,
        verification_agent_ids: None,
        position_x: Some(0.0),
        position_y: Some(0.0),
        width: None,
        height: None,
        name: Some("Designer".to_string()),
        system_prompt_suffix: None,
        visible: true,
        description: "Auto-managed Designer step".to_string(),
        board_context_cache: String::new(),
        board_context_updated_at: None,
        goal_summary: String::new(),
        goal_summary_updated_at: None,
        sub_workflow_template_id: None,
        child_workflow_id: None,
        is_designer_step: true,
        pinned: false,
        run_results_summary: String::new(),
    };

    repo.create_step(step)
        .await
        .map_err(|e| ServiceError::Internal(e.into()))
}

/// Find the Designer step in a pipeline (if it exists).
async fn find_designer_step(
    repo: &dyn WorkflowRepo,
    pipeline_id: Uuid,
) -> Result<Option<Uuid>, ServiceError> {
    let steps = repo
        .list_steps(pipeline_id)
        .await
        .map_err(|e| ServiceError::Internal(e.into()))?;

    Ok(steps.iter().find(|s| s.is_designer_step).map(|s| s.id))
}
