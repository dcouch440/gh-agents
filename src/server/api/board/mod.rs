//! Board API handlers — submit and load saved elements.

use axum::extract::{Json, Path, State};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::api::workflows::types::{step_response, WorkflowStepResponse};
use crate::server::api::AppError;
use crate::server::auth::AuthUser;
use crate::server::hub::board_serializer::{CanvasSnapshot, ExcalidrawElement, FilteredChangeset};
use crate::server::services::board;
use crate::server::services::dispatch;
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
    pub rewired_edges: Vec<ElementEdgePair>,
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
    pub from_step_id: Uuid,
    pub to_step_id: Uuid,
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
    pub dispatches: Vec<BoardDispatchInfo>,
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

    // Dispatch meaningful changes directly to per-node builders (agentless fan-out)
    let dispatches = if result.changeset.should_dispatch {
        dispatch_board_changes(
            &state,
            workflow_id,
            auth.user_id,
            &result.changeset,
            &result.phase_zero,
            &result.snapshot,
        )
        .await
    } else {
        vec![]
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
            .map(|(element_id, edge_row)| ElementEdgePair {
                element_id,
                edge_id: edge_row.id,
                from_step_id: edge_row.from_step_id,
                to_step_id: edge_row.to_step_id,
            })
            .collect(),
        deleted_steps: result.phase_zero.deleted_steps,
        deleted_edges: result.phase_zero.deleted_edges,
        rewired_edges: result
            .phase_zero
            .rewired_edges
            .into_iter()
            .map(|(element_id, edge_row)| ElementEdgePair {
                element_id,
                edge_id: edge_row.id,
                from_step_id: edge_row.from_step_id,
                to_step_id: edge_row.to_step_id,
            })
            .collect(),
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
        dispatches,
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

/// Dispatch meaningful changes to per-node builders via the sequential design pipeline.
///
/// Phase 0 has already created the topology — nodes, edges, positions are in the DB.
/// This function builds per-node instructions from the changeset, returns dispatch info
/// immediately for the HTTP response, and spawns a background task that runs each
/// builder + designer in topological order — threading handoff context between steps.
async fn dispatch_board_changes(
    state: &AppState,
    workflow_id: Uuid,
    user_id: UserId,
    changeset: &FilteredChangeset,
    phase_zero: &board::PhaseZeroResult,
    snapshot: &CanvasSnapshot,
) -> Vec<BoardDispatchInfo> {
    let instructions =
        board::instruction::build_per_node_instructions(changeset, phase_zero, snapshot);

    if instructions.is_empty() {
        return vec![];
    }

    // Build dispatch info for the HTTP response (returned immediately)
    let dispatches: Vec<BoardDispatchInfo> = instructions
        .iter()
        .map(|i| {
            // Generate deterministic execution IDs so the frontend can track them
            let execution_id = Uuid::new_v4();
            let session_id = Uuid::new_v5(
                &Uuid::NAMESPACE_OID,
                format!("builder:{}:{}", workflow_id, i.step_id).as_bytes(),
            );
            BoardDispatchInfo {
                execution_id,
                session_id,
                step_id: i.step_id,
                instruction: i.instruction.clone(),
            }
        })
        .collect();

    // Load steps and edges for the sequential pipeline
    let steps = state
        .repos()
        .workflows
        .list_steps(workflow_id)
        .await
        .unwrap_or_default();
    let edges = state
        .repos()
        .workflows
        .list_edges(workflow_id)
        .await
        .unwrap_or_default();

    // Spawn the sequential pipeline in the background
    let bg_state = state.clone();
    tokio::spawn(async move {
        dispatch::sequential::run_sequential_design_pipeline(
            bg_state,
            workflow_id,
            user_id,
            instructions,
            steps,
            edges,
        )
        .await;
    });

    dispatches
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
