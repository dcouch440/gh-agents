//! Board API handlers — submit and load saved elements.

use axum::extract::{Json, Path, State};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::api::workflows::types::{step_response, WorkflowStepResponse};
use crate::server::api::AppError;
use crate::server::auth::AuthUser;
use crate::server::hub::board_serializer::{CanvasSnapshot, ExcalidrawElement, FilteredChangeset};
use crate::server::services::board;
use crate::server::services::dispatch::{self, DispatchInput};
use crate::server::state::AppState;
use crate::types::UserId;

#[derive(Deserialize)]
pub struct BoardSubmitRequest {
    /// Raw Excalidraw elements — kept as `Value` so we can both parse and
    /// persist the original JSON without needing Serialize on ExcalidrawElement.
    pub elements: serde_json::Value,
}

#[derive(Serialize)]
pub struct PhaseZeroResponse {
    pub created_steps: Vec<PhaseZeroStep>,
    pub created_edges: Vec<ElementEdgePair>,
    pub deleted_steps: Vec<String>,
    pub deleted_edges: Vec<String>,
    pub rewired_edges: Vec<String>,
    pub moved_steps: Vec<String>,
    pub updated_steps: Vec<PhaseZeroStep>,
}

/// A step created or updated by Phase 0, with the Excalidraw element ID
/// that produced it. Contains the full step data for frontend selective sync.
#[derive(Serialize)]
pub struct PhaseZeroStep {
    pub element_id: String,
    #[serde(flatten)]
    pub step: WorkflowStepResponse,
}

#[derive(Serialize)]
pub struct ElementEdgePair {
    pub element_id: String,
    pub edge_id: Uuid,
}

#[derive(Serialize)]
pub struct BoardDispatchInfo {
    pub execution_id: Uuid,
    pub session_id: Uuid,
    pub step_id: Uuid,
    pub instruction: String,
}

#[derive(Serialize)]
pub struct BoardSubmitResponse {
    pub is_first_submit: bool,
    pub changeset: FilteredChangeset,
    pub snapshot: CanvasSnapshot,
    pub phase_zero: PhaseZeroResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dispatch: Option<BoardDispatchInfo>,
}

pub async fn submit_board(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(workflow_id): Path<Uuid>,
    Json(request): Json<BoardSubmitRequest>,
) -> Result<Json<BoardSubmitResponse>, AppError> {
    // Preserve raw JSON for DB persistence
    let elements_json = request.elements.to_string();

    // Parse into typed elements
    let elements: Vec<ExcalidrawElement> = serde_json::from_value(request.elements)
        .map_err(|e| AppError::bad_request(format!("Invalid elements: {e}")))?;

    let result = board::submit_board(
        state.repos().workflows.as_ref(),
        state.repos().sessions.as_ref(),
        board::BoardSubmitInput {
            workflow_id,
            user_id: auth.user_id.0,
            elements,
            elements_json,
        },
    )
    .await?;

    // Try to dispatch to the board dispatcher (best-effort, fire-and-forget)
    let dispatch = if result.changeset.should_dispatch {
        try_dispatch_board(
            &state,
            workflow_id,
            auth.user_id,
            &result.changeset,
            &result.phase_zero,
            &result.snapshot,
        )
        .await
    } else {
        None
    };

    let phase_zero = PhaseZeroResponse {
        created_steps: result
            .phase_zero
            .created_steps
            .into_iter()
            .map(|(element_id, row)| PhaseZeroStep {
                element_id,
                step: step_response(row),
            })
            .collect(),
        created_edges: result
            .phase_zero
            .created_edges
            .into_iter()
            .map(|(element_id, edge_id)| ElementEdgePair {
                element_id,
                edge_id,
            })
            .collect(),
        deleted_steps: result.phase_zero.deleted_steps,
        deleted_edges: result.phase_zero.deleted_edges,
        rewired_edges: result.phase_zero.rewired_edges,
        moved_steps: result.phase_zero.moved_steps,
        updated_steps: result
            .phase_zero
            .updated_steps
            .into_iter()
            .map(|(element_id, row)| PhaseZeroStep {
                element_id,
                step: step_response(row),
            })
            .collect(),
    };

    let response_body = BoardSubmitResponse {
        is_first_submit: result.is_first_submit,
        changeset: result.changeset,
        snapshot: result.snapshot,
        phase_zero,
        dispatch,
    };

    // Persist the response for debug panel rehydration on page refresh
    if let Ok(response_json) = serde_json::to_string(&response_body) {
        let _ = state
            .repos()
            .workflows
            .update_canvas_snapshot_response(workflow_id, response_json)
            .await;
    }

    Ok(Json(response_body))
}

/// Try to dispatch meaningful changes to the board dispatcher agent.
///
/// Best-effort: returns `None` on any error. Never fails the board submit.
async fn try_dispatch_board(
    state: &AppState,
    workflow_id: Uuid,
    user_id: UserId,
    changeset: &FilteredChangeset,
    phase_zero: &board::PhaseZeroResult,
    snapshot: &CanvasSnapshot,
) -> Option<BoardDispatchInfo> {
    // Format the instruction from the changeset
    let instruction =
        board::instruction::format_board_instruction(changeset, phase_zero, snapshot)?;

    // Find the manager step to use as the dispatch anchor
    let steps = state.repos().workflows.list_steps(workflow_id).await.ok()?;
    let manager_step = steps.iter().find(|s| s.execution_mode == "manager")?;

    // Dispatch via the shared dispatch service
    let output = dispatch::dispatch_to_builder(
        state,
        DispatchInput {
            step_id: manager_step.id,
            workflow_id,
            user_id,
            instruction: instruction.clone(),
            execution_mode: "board_dispatch".to_string(),
        },
    )
    .await;

    Some(BoardDispatchInfo {
        execution_id: output.execution_id,
        session_id: output.session_id,
        step_id: manager_step.id,
        instruction,
    })
}

// ── GET board elements ───────────────────────────────────────────────────

#[derive(Serialize)]
pub struct BoardElementsResponse {
    /// Raw Excalidraw elements JSON, or null if no snapshot exists.
    pub elements: Option<serde_json::Value>,
    /// The last board submit response, for debug panel rehydration on refresh.
    pub last_submit: Option<serde_json::Value>,
}

/// Return the last saved Excalidraw elements for a workflow.
///
/// If the workflow has never been submitted, returns `{ "elements": null }`.
pub async fn get_board_elements(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(workflow_id): Path<Uuid>,
) -> Result<Json<BoardElementsResponse>, AppError> {
    let row = state
        .repos()
        .workflows
        .get_canvas_snapshot(workflow_id)
        .await?;

    let (elements, last_submit) = match row {
        Some(r) => {
            let val: serde_json::Value = serde_json::from_str(&r.elements_json)
                .map_err(|e| AppError::Internal(format!("Bad stored elements: {e}")))?;
            let last = r
                .last_response_json
                .and_then(|json| serde_json::from_str(&json).ok());
            (Some(val), last)
        }
        None => (None, None),
    };

    Ok(Json(BoardElementsResponse {
        elements,
        last_submit,
    }))
}

mod tests;
