//! Workflow step CRUD handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::server::api::AppError;
use crate::server::auth as auth_utils;
use crate::server::services::steps;
use crate::server::state::AppState;

use super::types::{
    step_response, CreateStepRequest, StepQuestionStateEntry, TogglePinRequest, UpdateStepRequest,
    WorkflowNoteEntry, WorkflowStepPath, WorkflowStepResponse,
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
    let row = steps::create_step(
        state.repos().workflows.as_ref(),
        steps::CreateStepInput {
            workflow_id: wid,
            user_id: auth.user_id.0,
            payload: req.payload,
        },
    )
    .await?;
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
    let rows = steps::list_steps(state.repos().workflows.as_ref(), auth.user_id.0, wid).await?;
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
    let row = steps::get_step(state.repos().workflows.as_ref(), auth.user_id.0, p.0, p.1).await?;
    Ok(Json(step_response(row)))
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
    let row = steps::update_step(
        state.repos().workflows.as_ref(),
        steps::UpdateStepInput {
            workflow_id: p.wid,
            step_id: p.sid,
            user_id: auth.user_id.0,
            payload: req.payload,
        },
    )
    .await?;
    Ok(Json(step_response(row)))
}

/// GET /api/workflows/:wid/steps/:sid/config — unified config readback
pub async fn get_step_config(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(p): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Verify ownership and step membership via service
    let _step = steps::get_step(state.repos().workflows.as_ref(), auth.user_id.0, p.0, p.1).await?;

    let config = serde_json::json!({
        "archetype": serde_json::Value::Null,
    });

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
    let deleted_session_id = steps::delete_step(
        state.repos().workflows.as_ref(),
        state.repos().sessions.as_ref(),
        auth.user_id.0,
        p.wid,
        p.sid,
    )
    .await?;

    // Broadcast session deletion if a chat session was cleaned up
    if let Some(session_id) = deleted_session_id {
        state.broadcast_session(crate::server::ws::events::SessionEvent {
            session_id,
            user_id: Some(auth.user_id.0),
            kind: crate::server::ws::events::SessionEventKind::Deleted,
        });
    }

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/workflows/:id/notes — all assistant notes for a workflow
pub async fn get_workflow_notes(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(workflow_id): Path<Uuid>,
) -> Result<Json<Vec<WorkflowNoteEntry>>, AppError> {
    let repo = &state.repos().workflows;
    // Verify ownership via service
    crate::server::services::workflows::verify_workflow_ownership(
        repo.as_ref(),
        auth.user_id.0,
        workflow_id,
    )
    .await?;
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

/// POST /api/workflows/:wid/steps/:sid/pin — toggle step pin state
#[utoipa::path(
    post,
    path = "/api/workflows/{wid}/steps/{sid}/pin",
    tag = "Workflow Steps",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID")
    ),
    request_body = TogglePinRequest,
    responses(
        (status = 200, description = "Pin state updated", body = WorkflowStepResponse),
        (status = 400, description = "Step not eligible for pinning"),
        (status = 404, description = "Not found")
    )
)]
pub async fn toggle_step_pin(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(p): Path<WorkflowStepPath>,
    Json(req): Json<TogglePinRequest>,
) -> Result<Json<WorkflowStepResponse>, AppError> {
    let repo = state.repos().workflows.as_ref();

    // Verify ownership
    let step = steps::get_step(repo, auth.user_id.0, p.wid, p.sid).await?;

    // Only single, context, and input modes are pin-eligible in v1
    let pin_eligible = matches!(step.execution_mode.as_str(), "single" | "context" | "input");
    if !pin_eligible {
        return Err(AppError::BadRequest(format!(
            "Execution mode '{}' is not eligible for pinning. Only single, context, and input steps can be pinned.",
            step.execution_mode
        )));
    }

    // For single steps: require a prior execution output before pinning
    if req.pinned && step.execution_mode == "single" {
        let has_envelope = state
            .repos()
            .content_versions
            .get_latest_envelope_for_step(p.sid)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .is_some();
        if !has_envelope {
            return Err(AppError::BadRequest(
                "Cannot pin a single step that has never been executed. Run the step first."
                    .to_string(),
            ));
        }
    }

    // Toggle pin
    repo.set_step_pinned(p.sid, req.pinned)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Broadcast event
    state.broadcast_workflow(crate::server::ws::events::WorkflowEvent {
        run_id: None,
        workflow_id: p.wid,
        user_id: Some(auth.user_id.0),
        kind: crate::server::ws::events::WorkflowEventKind::StepPinChanged {
            step_id: p.sid,
            pinned: req.pinned,
        },
    });

    // Return updated step
    let updated = repo
        .get_step(p.sid)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or(AppError::not_found("Step"))?;
    Ok(Json(step_response(updated)))
}

/// GET /api/workflows/:id/question-states — question state for all steps in a workflow
pub async fn list_question_states(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(workflow_id): Path<Uuid>,
) -> Result<Json<Vec<StepQuestionStateEntry>>, AppError> {
    let repo = &state.repos().workflows;

    // Verify ownership
    crate::server::services::workflows::verify_workflow_ownership(
        repo.as_ref(),
        auth.user_id.0,
        workflow_id,
    )
    .await?;

    // Get all step IDs for this workflow
    let steps = repo
        .list_steps(workflow_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let step_ids: Vec<Uuid> = steps.iter().map(|s| s.id).collect();

    // Batch-load question states
    let states = repo
        .get_step_question_states(&step_ids)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let entries: Vec<StepQuestionStateEntry> = states
        .into_iter()
        .map(|qs| StepQuestionStateEntry {
            step_id: qs.step_id.to_string(),
            status_text: qs.status_text,
            question_text: qs.question_text,
            updated_at: qs.updated_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(entries))
}
