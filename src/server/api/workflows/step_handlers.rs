//! Workflow step CRUD handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::server::api::AppError;
use crate::server::auth as auth_utils;
use crate::server::state::AppState;

use super::types::{
    step_response, CreateStepRequest, UpdateStepRequest, WorkflowNoteEntry, WorkflowStepPath,
    WorkflowStepResponse,
};

/// POST /api/workflows/:id/steps
#[utoipa::path(
    post,
    path = "/api/workflows/{id}/steps",
    tag = "Workflow Steps",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    request_body = CreateStepRequest,
    responses(
        (status = 201, description = "Step created", body = WorkflowStepResponse),
        (status = 404, description = "Workflow not found")
    )
)]
pub async fn create_workflow_step(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(wid): Path<Uuid>,
    Json(req): Json<CreateStepRequest>,
) -> Result<(StatusCode, Json<WorkflowStepResponse>), AppError> {
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }

    // Auto-wire entry/documenter: resolve agent, schema, and reasoning
    let execution_mode = req.execution_mode.unwrap_or_else(|| "single".to_string());

    // Enforce single-input constraint: only one input step per workflow
    if execution_mode == "input" {
        let existing_steps = repo.list_steps(wid).await?;
        if existing_steps.iter().any(|s| s.execution_mode == "input") {
            return Err(AppError::bad_request(
                "Workflow can have at most one input step",
            ));
        }
    }

    let (resolved_agent_id, resolved_schema_id, resolved_reasoning): (Option<Uuid>, _, _) =
        if execution_mode == "context" || execution_mode == "input" {
            (None, None, false)
        } else if execution_mode == "documenter" {
            let proto = state
                .repos()
                .protocols
                .get_protocol_by_type("documenter")
                .await
                .ok()
                .flatten();
            match proto {
                Some(p) => (
                    None, // documenter steps are agent-less
                    p.output_schema_id,
                    true, // documenter always reasons
                ),
                None => (None, None, false),
            }
        } else {
            (
                Some(req.agent_id.unwrap_or(crate::constants::DEFAULT_AGENT_ID)),
                req.output_schema_id,
                req.reasoning_trace.unwrap_or(false),
            )
        };

    let description = req.description.unwrap_or_default();

    let step = crate::db::WorkflowStepRow {
        id: Uuid::new_v4(),
        workflow_id: wid,
        agent_id: resolved_agent_id,
        execution_mode,
        agent_execution_mode: None, // NULL = inherit from workflow
        for_each_ref: req.for_each_ref,
        prompt_template_id: req.prompt_template_id,
        prompt_template: req.prompt_template.unwrap_or_default(),
        output_schema_id: resolved_schema_id,
        output_variable_name: req.output_variable_name,
        interactive_agent_id: req.interactive_agent_id,
        for_each_label_field: req.for_each_label_field,
        room_id: None,
        routing_mode: None,
        routing_field: None,
        display_order: req.display_order.unwrap_or(0),
        version: 1,
        reasoning_trace: resolved_reasoning,
        verification_agent_ids: req
            .verification_agent_ids
            .map(|ids| serde_json::to_value(ids).unwrap()),
        position_x: req.position_x,
        position_y: req.position_y,
        width: req.width,
        height: req.height,
        name: req.name,
        system_prompt_suffix: req.system_prompt_suffix,
        visible: true,
        description,
        board_context_cache: String::new(),
        board_context_updated_at: None,
        goal_summary: String::new(),
        goal_summary_updated_at: None,
        sub_workflow_template_id: req.sub_workflow_template_id,
        child_workflow_id: None,
        is_designer_step: false,
    };
    let row = repo.create_step(step).await?;
    Ok((StatusCode::CREATED, Json(step_response(row))))
}

/// GET /api/workflows/:id/steps
#[utoipa::path(
    get,
    path = "/api/workflows/{id}/steps",
    tag = "Workflow Steps",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Workflow ID")),
    responses(
        (status = 200, description = "List of workflow steps", body = Vec<WorkflowStepResponse>),
        (status = 404, description = "Not found")
    )
)]
pub async fn list_workflow_steps(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(wid): Path<Uuid>,
) -> Result<Json<Vec<WorkflowStepResponse>>, AppError> {
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    let rows = repo.list_steps(wid).await?;
    Ok(Json(rows.into_iter().map(step_response).collect()))
}

/// GET /api/workflows/:wid/steps/:sid
#[utoipa::path(
    get,
    path = "/api/workflows/{wid}/steps/{sid}",
    tag = "Workflow Steps",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID")
    ),
    responses(
        (status = 200, description = "Workflow step found", body = WorkflowStepResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_workflow_step(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(p): Path<(Uuid, Uuid)>,
) -> Result<Json<WorkflowStepResponse>, AppError> {
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(p.0)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    let step = repo
        .get_step(p.1)
        .await?
        .ok_or(AppError::not_found("Step"))?;
    if step.workflow_id != p.0 {
        return Err(AppError::not_found("Step"));
    }
    Ok(Json(step_response(step)))
}

/// PUT /api/workflows/:wid/steps/:sid
#[utoipa::path(
    put,
    path = "/api/workflows/{wid}/steps/{sid}",
    tag = "Workflow Steps",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID")
    ),
    request_body = UpdateStepRequest,
    responses(
        (status = 200, description = "Updated workflow step", body = WorkflowStepResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_workflow_step(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(p): Path<WorkflowStepPath>,
    Json(req): Json<UpdateStepRequest>,
) -> Result<Json<WorkflowStepResponse>, AppError> {
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(p.wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    let existing = repo
        .get_step(p.sid)
        .await?
        .ok_or(AppError::not_found("Step"))?;
    if existing.workflow_id != p.wid {
        return Err(AppError::not_found("Step"));
    }
    let execution_mode = req.execution_mode.unwrap_or(existing.execution_mode);
    let agent_id = if execution_mode == "context" || execution_mode == "input" {
        None
    } else {
        req.agent_id.or(existing.agent_id)
    };
    let step = crate::db::WorkflowStepRow {
        id: p.sid,
        workflow_id: p.wid,
        agent_id,
        execution_mode,
        agent_execution_mode: existing.agent_execution_mode, // Preserve existing value
        for_each_ref: req.for_each_ref.or(existing.for_each_ref),
        prompt_template_id: req.prompt_template_id.or(existing.prompt_template_id),
        prompt_template: req.prompt_template.unwrap_or(existing.prompt_template),
        output_schema_id: req.output_schema_id.or(existing.output_schema_id),
        output_variable_name: req.output_variable_name.or(existing.output_variable_name),
        interactive_agent_id: req.interactive_agent_id.or(existing.interactive_agent_id),
        for_each_label_field: req.for_each_label_field.or(existing.for_each_label_field),
        room_id: existing.room_id,
        routing_mode: existing.routing_mode,
        routing_field: existing.routing_field,
        display_order: req.display_order.unwrap_or(existing.display_order),
        version: existing.version,
        reasoning_trace: req.reasoning_trace.unwrap_or(existing.reasoning_trace),
        verification_agent_ids: req
            .verification_agent_ids
            .map(|ids| serde_json::to_value(ids).unwrap())
            .or(existing.verification_agent_ids),
        position_x: req.position_x.or(existing.position_x),
        position_y: req.position_y.or(existing.position_y),
        width: req.width.or(existing.width),
        height: req.height.or(existing.height),
        name: req.name.or(existing.name),
        system_prompt_suffix: req.system_prompt_suffix.or(existing.system_prompt_suffix),
        visible: existing.visible,
        description: req.description.unwrap_or(existing.description),
        board_context_cache: existing.board_context_cache,
        board_context_updated_at: existing.board_context_updated_at,
        goal_summary: existing.goal_summary,
        goal_summary_updated_at: existing.goal_summary_updated_at,
        sub_workflow_template_id: req
            .sub_workflow_template_id
            .or(existing.sub_workflow_template_id),
        child_workflow_id: existing.child_workflow_id,
        is_designer_step: existing.is_designer_step,
    };
    let row = repo.update_step(step).await?;
    Ok(Json(step_response(row)))
}

/// GET /api/workflows/:wid/steps/:sid/config — unified config readback
pub async fn get_step_config(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(p): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(p.0)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    let step = repo
        .get_step(p.1)
        .await?
        .ok_or(AppError::not_found("Step"))?;
    if step.workflow_id != p.0 {
        return Err(AppError::not_found("Step"));
    }

    let config = match step.execution_mode.as_str() {
        "documenter" => {
            let doc_defs = repo.list_document_defs(p.1).await?;
            let documents: Vec<serde_json::Value> = doc_defs
                .into_iter()
                .map(|d| {
                    serde_json::json!({
                        "id": d.id.to_string(),
                        "name": d.name,
                        "description": d.description,
                        "target_length": d.target_length,
                    })
                })
                .collect();
            serde_json::json!({
                "archetype": "documenter",
                "documents": documents,
            })
        }
        _ => serde_json::json!({
            "archetype": serde_json::Value::Null,
        }),
    };

    Ok(Json(config))
}

/// DELETE /api/workflows/:wid/steps/:sid
#[utoipa::path(
    delete,
    path = "/api/workflows/{wid}/steps/{sid}",
    tag = "Workflow Steps",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID")
    ),
    responses(
        (status = 204, description = "Deleted successfully"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_workflow_step(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(p): Path<WorkflowStepPath>,
) -> Result<StatusCode, AppError> {
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(p.wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    let existing = repo
        .get_step(p.sid)
        .await?
        .ok_or(AppError::not_found("Step"))?;
    if existing.workflow_id != p.wid {
        return Err(AppError::not_found("Step"));
    }
    // Clean up any associated chat session
    if let Ok(Some(session)) = state.repo().find_session_by_step_id(p.sid).await {
        let _ = state.repo().delete_session(session.id).await;
        state.broadcast_session(crate::server::ws::events::SessionEvent {
            session_id: session.id,
            user_id: Some(auth.user_id.0),
            kind: crate::server::ws::events::SessionEventKind::Deleted,
        });
    }

    repo.delete_step(p.sid).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/workflows/:id/notes — all assistant notes for a workflow
pub async fn get_workflow_notes(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(workflow_id): Path<Uuid>,
) -> Result<Json<Vec<WorkflowNoteEntry>>, AppError> {
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(workflow_id)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    let notes = repo
        .get_all_assistant_notes_for_workflow(workflow_id)
        .await?;
    let entries: Vec<WorkflowNoteEntry> = notes
        .into_iter()
        .map(|(step_id, _name, _mode, content)| WorkflowNoteEntry {
            step_id: step_id.to_string(),
            content,
        })
        .collect();
    Ok(Json(entries))
}
