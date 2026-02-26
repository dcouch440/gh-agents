//! Step service: create, read, update, delete workflow steps.

use serde::Deserialize;
use uuid::Uuid;

use crate::db::traits::{SessionRepo, WorkflowRepo};
use crate::db::WorkflowStepRow;

use super::error::ServiceError;
use super::workflows::verify_workflow_ownership;

/// Shared payload fields for creating or updating a workflow step.
///
/// All 20 optional fields that are common to both create and update operations.
/// Derives `Deserialize` + `utoipa::ToSchema` so it can be embedded in API
/// request types via `#[serde(flatten)]`.
#[derive(Default, Deserialize, utoipa::ToSchema)]
pub struct StepPayload {
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

/// Input for creating a new workflow step.
pub struct CreateStepInput {
    pub workflow_id: Uuid,
    pub user_id: Uuid,
    pub payload: StepPayload,
}

/// Input for updating an existing workflow step.
pub struct UpdateStepInput {
    pub workflow_id: Uuid,
    pub step_id: Uuid,
    pub user_id: Uuid,
    pub payload: StepPayload,
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

    let p = input.payload;
    let execution_mode = p.execution_mode.unwrap_or_else(|| "workforce".to_string());

    // Fetch existing steps (used for input constraint + ref_id generation)
    let existing_steps = repo.list_steps(input.workflow_id).await?;

    // Enforce single-input constraint
    if execution_mode == "input" && existing_steps.iter().any(|s| s.execution_mode == "input") {
        return Err(ServiceError::validation(
            "Workflow can have at most one input step",
        ));
    }

    // Generate stable ref_id: "{execution_mode}-{next_number}"
    let ref_id = generate_ref_id(&existing_steps, &execution_mode);

    let (resolved_agent_id, resolved_schema_id, resolved_reasoning) = resolve_step_defaults(
        &execution_mode,
        p.agent_id,
        p.output_schema_id,
        p.reasoning_trace,
    );

    let description = p.description.unwrap_or_default();

    let step = WorkflowStepRow {
        id: Uuid::new_v4(),
        workflow_id: input.workflow_id,
        agent_id: resolved_agent_id,
        execution_mode,
        agent_execution_mode: None,
        for_each_ref: p.for_each_ref,
        prompt_template_id: p.prompt_template_id,
        prompt_template: p.prompt_template.unwrap_or_default(),
        output_schema_id: resolved_schema_id,
        output_variable_name: p.output_variable_name,
        interactive_agent_id: p.interactive_agent_id,
        for_each_label_field: p.for_each_label_field,
        room_id: None,
        routing_mode: None,
        routing_field: None,
        display_order: p.display_order.unwrap_or(0),
        version: 1,
        reasoning_trace: resolved_reasoning,
        verification_agent_ids: p
            .verification_agent_ids
            .map(|ids| serde_json::to_value(ids).unwrap()),
        position_x: p.position_x,
        position_y: p.position_y,
        width: p.width,
        height: p.height,
        name: p.name,
        system_prompt_suffix: p.system_prompt_suffix,
        visible: true,
        description,
        board_context_cache: String::new(),
        board_context_updated_at: None,
        goal_summary: String::new(),
        goal_summary_updated_at: None,
        sub_workflow_template_id: p.sub_workflow_template_id,
        child_workflow_id: None,
        ref_id: Some(ref_id),
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

    let p = input.payload;
    let execution_mode = p.execution_mode.unwrap_or(existing.execution_mode);
    let agent_id = if execution_mode == "context" || execution_mode == "input" {
        None
    } else {
        p.agent_id.or(existing.agent_id)
    };

    let step = WorkflowStepRow {
        id: input.step_id,
        workflow_id: input.workflow_id,
        agent_id,
        execution_mode,
        agent_execution_mode: existing.agent_execution_mode,
        for_each_ref: p.for_each_ref.or(existing.for_each_ref),
        prompt_template_id: p.prompt_template_id.or(existing.prompt_template_id),
        prompt_template: p.prompt_template.unwrap_or(existing.prompt_template),
        output_schema_id: p.output_schema_id.or(existing.output_schema_id),
        output_variable_name: p.output_variable_name.or(existing.output_variable_name),
        interactive_agent_id: p.interactive_agent_id.or(existing.interactive_agent_id),
        for_each_label_field: p.for_each_label_field.or(existing.for_each_label_field),
        room_id: existing.room_id,
        routing_mode: existing.routing_mode,
        routing_field: existing.routing_field,
        display_order: p.display_order.unwrap_or(existing.display_order),
        version: existing.version,
        reasoning_trace: p.reasoning_trace.unwrap_or(existing.reasoning_trace),
        verification_agent_ids: p
            .verification_agent_ids
            .map(|ids| serde_json::to_value(ids).unwrap())
            .or(existing.verification_agent_ids),
        position_x: p.position_x.or(existing.position_x),
        position_y: p.position_y.or(existing.position_y),
        width: p.width.or(existing.width),
        height: p.height.or(existing.height),
        name: p.name.or(existing.name),
        system_prompt_suffix: p.system_prompt_suffix.or(existing.system_prompt_suffix),
        visible: existing.visible,
        description: p.description.unwrap_or(existing.description),
        board_context_cache: existing.board_context_cache,
        board_context_updated_at: existing.board_context_updated_at,
        goal_summary: existing.goal_summary,
        goal_summary_updated_at: existing.goal_summary_updated_at,
        sub_workflow_template_id: p
            .sub_workflow_template_id
            .or(existing.sub_workflow_template_id),
        child_workflow_id: existing.child_workflow_id,
        ref_id: existing.ref_id.clone(),
        pinned: existing.pinned,
        run_results_summary: existing.run_results_summary.clone(),
    };
    let row = repo.update_step(step).await?;
    Ok(row)
}

/// Sessions cleaned up by [`delete_step`].
///
/// Callers are responsible for broadcasting `SessionEventKind::Deleted`
/// for any `Some` session IDs returned here.
pub struct DeleteStepResult {
    /// L3 assistant chat session that was deleted (if any).
    pub deleted_chat_session_id: Option<Uuid>,
    /// L4 builder session that was deleted (if any).
    pub deleted_builder_session_id: Option<Uuid>,
}

/// Delete a workflow step, verifying ownership.
///
/// Cleans up associated sessions:
/// - L3 assistant chat session (deleted)
/// - L4 builder session (deleted)
/// - L2 manager builder session messages (cleared, session kept)
pub async fn delete_step(
    repo: &dyn WorkflowRepo,
    session_repo: &dyn SessionRepo,
    user_id: Uuid,
    workflow_id: Uuid,
    step_id: Uuid,
) -> Result<DeleteStepResult, ServiceError> {
    verify_workflow_ownership(repo, user_id, workflow_id).await?;

    let existing = repo
        .get_step(step_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Step"))?;
    if existing.workflow_id != workflow_id {
        return Err(ServiceError::not_found("Step"));
    }

    // Clean up L3 assistant chat session
    let deleted_chat_session_id =
        if let Ok(Some(session)) = session_repo.find_session_by_step_id(step_id).await {
            let _ = session_repo.delete_session(session.id).await;
            Some(session.id)
        } else {
            None
        };

    // Clean up L4 builder session (one per step, orphaned after deletion)
    let deleted_builder_session_id =
        if let Ok(Some(session)) = session_repo.find_builder_session_by_step_id(step_id).await {
            let _ = session_repo.delete_session(session.id).await;
            Some(session.id)
        } else {
            None
        };

    // Clear stale messages from L2 manager builder session (one per workflow).
    // The session shell is kept — next dispatch repopulates via fresh board_state.
    if let Ok(Some(manager_session)) = session_repo.find_manager_builder_session(workflow_id).await
    {
        let _ = session_repo
            .clear_session_messages(manager_session.id)
            .await;
    }

    repo.delete_step(step_id).await?;
    Ok(DeleteStepResult {
        deleted_chat_session_id,
        deleted_builder_session_id,
    })
}

/// Generate a stable ref_id like "workforce-1" for a new step.
///
/// Scans existing steps in the workflow for the highest number with the same
/// execution_mode prefix and increments it.
pub(crate) fn generate_ref_id(existing_steps: &[WorkflowStepRow], execution_mode: &str) -> String {
    let prefix = format!("{}-", execution_mode);
    let max_num = existing_steps
        .iter()
        .filter_map(|s| s.ref_id.as_deref())
        .filter_map(|r| r.strip_prefix(&prefix))
        .filter_map(|n| n.parse::<u32>().ok())
        .max()
        .unwrap_or(0);
    format!("{}{}", prefix, max_num + 1)
}

mod tests;
