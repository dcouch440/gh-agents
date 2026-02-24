//! Board submit API handler.

use axum::extract::{Json, Path, State};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    pub created_steps: Vec<ElementStepPair>,
    pub created_edges: Vec<ElementEdgePair>,
    pub deleted_steps: Vec<String>,
    pub deleted_edges: Vec<String>,
    pub rewired_edges: Vec<String>,
    pub moved_steps: Vec<String>,
    pub updated_steps: Vec<ElementStepPair>,
}

#[derive(Serialize)]
pub struct ElementStepPair {
    pub element_id: String,
    pub step_id: Uuid,
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
            .map(|(element_id, step_id, _ref_id)| ElementStepPair {
                element_id,
                step_id,
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
            .map(|(element_id, step_id, _ref_id)| ElementStepPair {
                element_id,
                step_id,
            })
            .collect(),
    };

    Ok(Json(BoardSubmitResponse {
        is_first_submit: result.is_first_submit,
        changeset: result.changeset,
        snapshot: result.snapshot,
        phase_zero,
        dispatch,
    }))
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

mod tests;
