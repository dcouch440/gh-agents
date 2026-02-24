//! Board submit API handler.

use axum::extract::{Json, Path, State};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::api::AppError;
use crate::server::auth::AuthUser;
use crate::server::hub::board_serializer::{
    CanvasSnapshot, ExcalidrawElement, FilteredChangeset,
};
use crate::server::services::board;
use crate::server::state::AppState;

#[derive(Deserialize)]
pub struct BoardSubmitRequest {
    /// Raw Excalidraw elements — kept as `Value` so we can both parse and
    /// persist the original JSON without needing Serialize on ExcalidrawElement.
    pub elements: serde_json::Value,
}

#[derive(Serialize)]
pub struct BoardSubmitResponse {
    pub is_first_submit: bool,
    pub changeset: FilteredChangeset,
    pub snapshot: CanvasSnapshot,
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
        board::BoardSubmitInput {
            workflow_id,
            user_id: auth.user_id.0,
            elements,
            elements_json,
        },
    )
    .await?;

    Ok(Json(BoardSubmitResponse {
        is_first_submit: result.is_first_submit,
        changeset: result.changeset,
        snapshot: result.snapshot,
    }))
}

mod tests;
