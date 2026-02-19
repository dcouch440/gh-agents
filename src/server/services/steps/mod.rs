//! Step service: create, read, update, delete workflow steps.

use uuid::Uuid;

use crate::db::traits::{SessionRepo, WorkflowRepo};
use crate::db::WorkflowStepRow;

use super::error::ServiceError;
use super::workflows::verify_workflow_ownership;

/// Input for creating a new workflow step.
pub struct CreateStepInput {
    pub workflow_id: Uuid,
    pub user_id: Uuid,
    pub agent_id: Option<Uuid>,
    pub execution_mode: Option<String>,
    pub for_each_ref: Option<String>,
    pub prompt_template_id: Option<Uuid>,
    pub prompt_template: Option<String>,
    pub output_schema_id: Option<Uuid>,
    pub output_variable_name: Option<String>,
    pub interactive_agent_id: Option<Uuid>,
    pub for_each_label_field: Option<String>,
    pub display_order: Option<i32>,
    pub reasoning_trace: Option<bool>,
    pub verification_agent_ids: Option<Vec<Uuid>>,
    pub position_x: Option<f64>,
    pub position_y: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub name: Option<String>,
    pub system_prompt_suffix: Option<String>,
    pub description: Option<String>,
    pub sub_workflow_template_id: Option<Uuid>,
}

/// Input for updating an existing workflow step.
pub struct UpdateStepInput {
    pub workflow_id: Uuid,
    pub step_id: Uuid,
    pub user_id: Uuid,
    pub agent_id: Option<Uuid>,
    pub execution_mode: Option<String>,
    pub for_each_ref: Option<String>,
    pub prompt_template_id: Option<Uuid>,
    pub prompt_template: Option<String>,
    pub output_schema_id: Option<Uuid>,
    pub output_variable_name: Option<String>,
    pub interactive_agent_id: Option<Uuid>,
    pub for_each_label_field: Option<String>,
    pub display_order: Option<i32>,
    pub reasoning_trace: Option<bool>,
    pub verification_agent_ids: Option<Vec<Uuid>>,
    pub position_x: Option<f64>,
    pub position_y: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub name: Option<String>,
    pub system_prompt_suffix: Option<String>,
    pub description: Option<String>,
    pub sub_workflow_template_id: Option<Uuid>,
}

/// Resolve agent_id, output_schema_id, and reasoning_trace based on execution_mode.
/// Context and input steps are agentless.
fn resolve_step_defaults(
    execution_mode: &str,
    agent_id: Option<Uuid>,
    output_schema_id: Option<Uuid>,
    reasoning_trace: Option<bool>,
) -> (Option<Uuid>, Option<Uuid>, bool) {
    if execution_mode == "context" || execution_mode == "input" {
        (None, None, false)
    } else {
        (
            Some(agent_id.unwrap_or(crate::constants::DEFAULT_AGENT_ID)),
            output_schema_id,
            reasoning_trace.unwrap_or(false),
        )
    }
}

/// Create a new workflow step.
pub async fn create_step(
    repo: &dyn WorkflowRepo,
    input: CreateStepInput,
) -> Result<WorkflowStepRow, ServiceError> {
    verify_workflow_ownership(repo, input.user_id, input.workflow_id).await?;

    let execution_mode = input.execution_mode.unwrap_or_else(|| "single".to_string());

    // Enforce single-input constraint
    if execution_mode == "input" {
        let existing_steps = repo.list_steps(input.workflow_id).await?;
        if existing_steps.iter().any(|s| s.execution_mode == "input") {
            return Err(ServiceError::validation(
                "Workflow can have at most one input step",
            ));
        }
    }

    let (resolved_agent_id, resolved_schema_id, resolved_reasoning) = resolve_step_defaults(
        &execution_mode,
        input.agent_id,
        input.output_schema_id,
        input.reasoning_trace,
    );

    let description = input.description.unwrap_or_default();

    let step = WorkflowStepRow {
        id: Uuid::new_v4(),
        workflow_id: input.workflow_id,
        agent_id: resolved_agent_id,
        execution_mode,
        agent_execution_mode: None,
        for_each_ref: input.for_each_ref,
        prompt_template_id: input.prompt_template_id,
        prompt_template: input.prompt_template.unwrap_or_default(),
        output_schema_id: resolved_schema_id,
        output_variable_name: input.output_variable_name,
        interactive_agent_id: input.interactive_agent_id,
        for_each_label_field: input.for_each_label_field,
        room_id: None,
        routing_mode: None,
        routing_field: None,
        display_order: input.display_order.unwrap_or(0),
        version: 1,
        reasoning_trace: resolved_reasoning,
        verification_agent_ids: input
            .verification_agent_ids
            .map(|ids| serde_json::to_value(ids).unwrap()),
        position_x: input.position_x,
        position_y: input.position_y,
        width: input.width,
        height: input.height,
        name: input.name,
        system_prompt_suffix: input.system_prompt_suffix,
        visible: true,
        description,
        board_context_cache: String::new(),
        board_context_updated_at: None,
        goal_summary: String::new(),
        goal_summary_updated_at: None,
        sub_workflow_template_id: input.sub_workflow_template_id,
        child_workflow_id: None,
        is_designer_step: false,
        pinned: false,
        run_results_summary: String::new(),
    };
    let row = repo.create_step(step).await?;
    Ok(row)
}

/// Verify workflow ownership and step membership.
/// Returns `Ok(())` if the caller owns the workflow and the step belongs to it.
pub async fn verify_step_access(
    repo: &dyn WorkflowRepo,
    user_id: Uuid,
    workflow_id: Uuid,
    step_id: Uuid,
) -> Result<(), ServiceError> {
    verify_workflow_ownership(repo, user_id, workflow_id).await?;
    let step = repo
        .get_step(step_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Step"))?;
    if step.workflow_id != workflow_id {
        return Err(ServiceError::not_found("Step"));
    }
    Ok(())
}

/// Get a workflow step, verifying workflow ownership and step membership.
pub async fn get_step(
    repo: &dyn WorkflowRepo,
    user_id: Uuid,
    workflow_id: Uuid,
    step_id: Uuid,
) -> Result<WorkflowStepRow, ServiceError> {
    verify_workflow_ownership(repo, user_id, workflow_id).await?;
    let step = repo
        .get_step(step_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Step"))?;
    if step.workflow_id != workflow_id {
        return Err(ServiceError::not_found("Step"));
    }
    Ok(step)
}

/// List all steps for a workflow, verifying ownership.
pub async fn list_steps(
    repo: &dyn WorkflowRepo,
    user_id: Uuid,
    workflow_id: Uuid,
) -> Result<Vec<WorkflowStepRow>, ServiceError> {
    verify_workflow_ownership(repo, user_id, workflow_id).await?;
    let rows = repo.list_steps(workflow_id).await?;
    Ok(rows)
}

/// Update a workflow step (partial update).
pub async fn update_step(
    repo: &dyn WorkflowRepo,
    input: UpdateStepInput,
) -> Result<WorkflowStepRow, ServiceError> {
    verify_workflow_ownership(repo, input.user_id, input.workflow_id).await?;

    let existing = repo
        .get_step(input.step_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Step"))?;
    if existing.workflow_id != input.workflow_id {
        return Err(ServiceError::not_found("Step"));
    }

    let execution_mode = input.execution_mode.unwrap_or(existing.execution_mode);
    let agent_id = if execution_mode == "context" || execution_mode == "input" {
        None
    } else {
        input.agent_id.or(existing.agent_id)
    };

    let step = WorkflowStepRow {
        id: input.step_id,
        workflow_id: input.workflow_id,
        agent_id,
        execution_mode,
        agent_execution_mode: existing.agent_execution_mode,
        for_each_ref: input.for_each_ref.or(existing.for_each_ref),
        prompt_template_id: input.prompt_template_id.or(existing.prompt_template_id),
        prompt_template: input.prompt_template.unwrap_or(existing.prompt_template),
        output_schema_id: input.output_schema_id.or(existing.output_schema_id),
        output_variable_name: input.output_variable_name.or(existing.output_variable_name),
        interactive_agent_id: input.interactive_agent_id.or(existing.interactive_agent_id),
        for_each_label_field: input.for_each_label_field.or(existing.for_each_label_field),
        room_id: existing.room_id,
        routing_mode: existing.routing_mode,
        routing_field: existing.routing_field,
        display_order: input.display_order.unwrap_or(existing.display_order),
        version: existing.version,
        reasoning_trace: input.reasoning_trace.unwrap_or(existing.reasoning_trace),
        verification_agent_ids: input
            .verification_agent_ids
            .map(|ids| serde_json::to_value(ids).unwrap())
            .or(existing.verification_agent_ids),
        position_x: input.position_x.or(existing.position_x),
        position_y: input.position_y.or(existing.position_y),
        width: input.width.or(existing.width),
        height: input.height.or(existing.height),
        name: input.name.or(existing.name),
        system_prompt_suffix: input.system_prompt_suffix.or(existing.system_prompt_suffix),
        visible: existing.visible,
        description: input.description.unwrap_or(existing.description),
        board_context_cache: existing.board_context_cache,
        board_context_updated_at: existing.board_context_updated_at,
        goal_summary: existing.goal_summary,
        goal_summary_updated_at: existing.goal_summary_updated_at,
        sub_workflow_template_id: input
            .sub_workflow_template_id
            .or(existing.sub_workflow_template_id),
        child_workflow_id: existing.child_workflow_id,
        is_designer_step: existing.is_designer_step,
        pinned: existing.pinned,
        run_results_summary: existing.run_results_summary.clone(),
    };
    let row = repo.update_step(step).await?;
    Ok(row)
}

/// Delete a workflow step, verifying ownership.
///
/// Returns `Some((session_id, user_id))` if a chat session was cleaned up
/// (the caller is responsible for broadcasting the session deletion event).
pub async fn delete_step(
    repo: &dyn WorkflowRepo,
    session_repo: &dyn SessionRepo,
    user_id: Uuid,
    workflow_id: Uuid,
    step_id: Uuid,
) -> Result<Option<Uuid>, ServiceError> {
    verify_workflow_ownership(repo, user_id, workflow_id).await?;

    let existing = repo
        .get_step(step_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Step"))?;
    if existing.workflow_id != workflow_id {
        return Err(ServiceError::not_found("Step"));
    }

    // Clean up any associated chat session
    let deleted_session_id =
        if let Ok(Some(session)) = session_repo.find_session_by_step_id(step_id).await {
            let _ = session_repo.delete_session(session.id).await;
            Some(session.id)
        } else {
            None
        };

    repo.delete_step(step_id).await?;
    Ok(deleted_session_id)
}

mod tests;
