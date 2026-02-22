//! Step chat session lifecycle handlers.
//!
//! Provides find-or-create semantics for step-scoped chat sessions,
//! message clearing, and debug prompt inspection.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use uuid::Uuid;

use crate::server::api::sessions::SessionResponse;
use crate::server::api::AppError;
use crate::server::auth as auth_utils;
use crate::server::state::AppState;
use crate::server::ws::events::{SessionEvent, SessionEventKind};

/// Response for the step chat debug endpoint.
#[derive(Debug, Serialize)]
pub struct StepChatDebugResponse {
    pub system_prompt: String,
    pub messages: Vec<DebugMessage>,
}

/// A single message in the debug view.
#[derive(Debug, Serialize)]
pub struct DebugMessage {
    pub role: String,
    pub content: String,
}

/// Path parameters for step chat endpoints.
#[derive(serde::Deserialize)]
pub struct StepChatPath {
    pub wid: Uuid,
    pub sid: Uuid,
}

/// GET /api/workflows/:wid/steps/:sid/chat/session
///
/// Returns the existing step chat session, or 404 if none exists.
#[utoipa::path(
    get,
    path = "/api/workflows/{wid}/steps/{sid}/chat/session",
    tag = "Step Chat",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID"),
    ),
    responses(
        (status = 200, description = "Step chat session", body = SessionResponse),
        (status = 404, description = "No session exists for this step")
    )
)]
pub async fn get_step_session(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(p): Path<StepChatPath>,
) -> Result<Json<SessionResponse>, AppError> {
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(p.wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }

    let session = state
        .repos()
        .sessions
        .find_session_by_step_id(p.sid)
        .await?
        .ok_or(AppError::not_found("Step session"))?;

    Ok(Json(SessionResponse {
        id: session.id,
        mode_id: session.mode_id,
        agent_id: session.agent_id,
        draft_config: session.draft_config,
        title: session.title,
        created_at: session.created_at,
        updated_at: session.updated_at,
    }))
}

/// POST /api/workflows/:wid/steps/:sid/chat/session
///
/// Find-or-create: returns existing step session or creates a new one.
#[utoipa::path(
    post,
    path = "/api/workflows/{wid}/steps/{sid}/chat/session",
    tag = "Step Chat",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID"),
    ),
    responses(
        (status = 200, description = "Step chat session", body = SessionResponse),
        (status = 404, description = "Workflow or step not found")
    )
)]
pub async fn get_or_create_step_session(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(p): Path<StepChatPath>,
) -> Result<Json<SessionResponse>, AppError> {
    // Verify workflow + step ownership
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(p.wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }
    let step = repo
        .get_step(p.sid)
        .await?
        .ok_or(AppError::not_found("Step"))?;
    if step.workflow_id != p.wid {
        return Err(AppError::not_found("Step"));
    }

    // Try to find existing session
    if let Some(session) = state
        .repos()
        .sessions
        .find_session_by_step_id(p.sid)
        .await?
    {
        return Ok(Json(SessionResponse {
            id: session.id,
            mode_id: session.mode_id,
            agent_id: session.agent_id,
            draft_config: session.draft_config,
            title: session.title,
            created_at: session.created_at,
            updated_at: session.updated_at,
        }));
    }

    // Create new session with step context in draft_config
    let session_id = Uuid::new_v4();
    let step_name = step.name.as_deref().unwrap_or("Step");
    let title = format!("{} Chat", step_name);
    let draft_config = serde_json::json!({
        "step_id": p.sid.to_string(),
        "workflow_id": p.wid.to_string(),
    });

    state
        .repos()
        .sessions
        .create_session(
            auth.user_id,
            session_id,
            "step_chat",
            &title,
            None,
            Some(draft_config),
        )
        .await?;

    let session = state
        .repos()
        .sessions
        .get_session(session_id)
        .await?
        .ok_or(AppError::Internal(
            "Session not found after creation".into(),
        ))?;

    state.broadcast_session(SessionEvent {
        session_id,
        user_id: Some(auth.user_id.0),
        kind: SessionEventKind::Created {
            title: session.title.clone(),
            mode_id: session.mode_id.clone(),
        },
    });

    Ok(Json(SessionResponse {
        id: session.id,
        mode_id: session.mode_id,
        agent_id: session.agent_id,
        draft_config: session.draft_config,
        title: session.title,
        created_at: session.created_at,
        updated_at: session.updated_at,
    }))
}

/// DELETE /api/workflows/:wid/steps/:sid/chat/messages
///
/// Clear all messages for a step's chat session. Session is preserved.
#[utoipa::path(
    delete,
    path = "/api/workflows/{wid}/steps/{sid}/chat/messages",
    tag = "Step Chat",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID"),
    ),
    responses(
        (status = 204, description = "Messages cleared"),
        (status = 404, description = "Step session not found")
    )
)]
pub async fn clear_step_messages(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(p): Path<StepChatPath>,
) -> Result<StatusCode, AppError> {
    // Verify workflow ownership
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(p.wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }

    let session = state
        .repos()
        .sessions
        .find_session_by_step_id(p.sid)
        .await?
        .ok_or(AppError::not_found("Step session"))?;

    state
        .repos()
        .sessions
        .clear_session_messages(session.id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/workflows/:wid/steps/:sid/chat/debug
///
/// Returns the resolved system prompt and message history for debugging.
/// Useful for inspecting exactly what the LLM receives.
pub async fn get_step_chat_debug(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(p): Path<StepChatPath>,
) -> Result<Json<StepChatDebugResponse>, AppError> {
    // Verify workflow ownership
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(p.wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }

    // Load step to get execution_mode
    let step = repo
        .get_step(p.sid)
        .await?
        .ok_or(AppError::not_found("Step"))?;
    if step.workflow_id != p.wid {
        return Err(AppError::not_found("Step"));
    }

    // Build the resolved system prompt (same logic used at chat time)
    let system_prompt =
        crate::server::hub::build_step_system_prompt(&state, p.wid, p.sid, &step.execution_mode)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to build system prompt: {e}")))?;

    // Load session history if a session exists
    let messages = if let Some(session) = state
        .repos()
        .sessions
        .find_session_by_step_id(p.sid)
        .await?
    {
        let history = state
            .repos()
            .sessions
            .get_session_history(session.id, 50)
            .await?;
        history
            .into_iter()
            .map(|m| DebugMessage {
                role: m.role,
                content: m.content,
            })
            .collect()
    } else {
        vec![]
    };

    Ok(Json(StepChatDebugResponse {
        system_prompt,
        messages,
    }))
}

/// API response for dispatch trace history (mirrors frontend DispatchTraceResponse).
#[derive(Debug, Serialize)]
pub struct DispatchTraceApiResponse {
    pub execution_id: String,
    pub step_id: String,
    pub workflow_id: String,
    pub status: String,
    pub instruction: String,
    pub trace: serde_json::Value,
    pub result: Option<String>,
}

/// GET /api/workflows/:wid/steps/:sid/dispatch/history
///
/// Returns the latest dispatch execution trace for a step from agent_executions.
/// Includes the full serialized trace (tokens, tool calls, errors) for
/// frontend hydration on page refresh.
pub async fn get_step_dispatch_history(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(p): Path<StepChatPath>,
) -> Result<Json<DispatchTraceApiResponse>, AppError> {
    // Verify workflow ownership
    let repo = &state.repos().workflows;
    let wf = repo
        .get_workflow(p.wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if wf.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }

    // Query agent_executions for latest dispatch execution on this step
    let ae = state
        .repos()
        .agent_executions
        .get_latest_dispatch_execution_for_step(p.sid)
        .await?
        .ok_or(AppError::not_found("Dispatch execution"))?;

    let trace = ae.trace.unwrap_or(serde_json::json!([]));

    Ok(Json(DispatchTraceApiResponse {
        execution_id: ae.id.to_string(),
        step_id: p.sid.to_string(),
        workflow_id: p.wid.to_string(),
        status: ae.status,
        instruction: ae.input,
        trace,
        result: ae.output,
    }))
}
