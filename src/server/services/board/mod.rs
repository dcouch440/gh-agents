//! Board submit service — orchestrates the classify → diff → filter → execute pipeline.
//!
//! On each board submit:
//! 1. Classify raw Excalidraw elements → `CanvasSnapshot`
//! 2. Diff against previous snapshot → `CanvasChangeset`
//! 3. Filter noise → `FilteredChangeset` (three tiers)
//! 4. Persist snapshot for next diff
//! 5. Execute Phase 0 structural changes (agentless DB writes)

use uuid::Uuid;

use crate::db::traits::{SessionRepo, WorkflowRepo};
use crate::db::CanvasSnapshotRow;
use crate::server::hub::board_serializer::{
    self, CanvasSnapshot, ExcalidrawElement, FilterConfig, FilteredChangeset,
};
use crate::server::services::ownership;
use crate::server::services::ServiceError;

pub use executor::PhaseZeroResult;

/// Input for a board submit request.
pub struct BoardSubmitInput {
    pub workflow_id: Uuid,
    pub user_id: Uuid,
    pub elements: Vec<ExcalidrawElement>,
    /// Raw JSON string of the elements array (for persistence without re-serialization).
    pub elements_json: String,
}

/// Result of a board submit — the classified snapshot, filtered changeset,
/// and Phase 0 structural execution result.
#[derive(Debug)]
pub struct BoardSubmitResult {
    /// Whether this is the first submit for this workflow (no previous snapshot).
    pub is_first_submit: bool,
    /// The current board state after classification.
    pub snapshot: CanvasSnapshot,
    /// The filtered changeset (diff against previous snapshot, run through noise filters).
    pub changeset: FilteredChangeset,
    /// Phase 0 structural execution result (nodes/edges created, deleted, rewired, moved).
    pub phase_zero: PhaseZeroResult,
}

/// Process a board submit: classify, diff, filter, persist, and execute Phase 0.
pub async fn submit_board(
    repo: &dyn WorkflowRepo,
    session_repo: &dyn SessionRepo,
    input: BoardSubmitInput,
) -> Result<BoardSubmitResult, ServiceError> {
    // 1. Verify workflow ownership
    let workflow = repo
        .get_workflow(input.workflow_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Workflow"))?;
    ownership::check_direct_owner(workflow.user_id, input.user_id, "Workflow")?;

    // 2. Classify raw Excalidraw elements → CanvasSnapshot
    let current_snapshot = board_serializer::classify_board(&input.elements);

    // 3. Load previous snapshot (if any)
    let previous_row = repo.get_canvas_snapshot(input.workflow_id).await?;
    let is_first_submit = previous_row.is_none();

    let previous_snapshot = match &previous_row {
        Some(row) => serde_json::from_str::<CanvasSnapshot>(&row.snapshot_json)
            .map_err(|e| ServiceError::Internal(e.into()))?,
        None => CanvasSnapshot {
            nodes: vec![],
            edges: vec![],
            global_notes: vec![],
        },
    };

    // 4. Diff previous vs current → CanvasChangeset
    let changeset = board_serializer::diff_snapshots(&previous_snapshot, &current_snapshot);

    // 5. Filter changeset → FilteredChangeset
    let filtered = board_serializer::filter_changeset(
        &changeset,
        &current_snapshot.edges,
        if is_first_submit {
            None
        } else {
            Some(&previous_snapshot)
        },
        &FilterConfig::default(),
    );

    // 6. Persist current snapshot for next diff
    let snapshot_json =
        serde_json::to_string(&current_snapshot).map_err(|e| ServiceError::Internal(e.into()))?;

    repo.upsert_canvas_snapshot(CanvasSnapshotRow {
        workflow_id: input.workflow_id,
        snapshot_json,
        elements_json: input.elements_json,
        last_response_json: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    })
    .await?;

    // 7. Execute Phase 0 structural changes (agentless DB writes)
    let phase_zero = executor::execute_phase_zero(
        repo,
        session_repo,
        input.workflow_id,
        input.user_id,
        &filtered,
    )
    .await?;

    Ok(BoardSubmitResult {
        is_first_submit,
        snapshot: current_snapshot,
        changeset: filtered,
        phase_zero,
    })
}

pub mod executor;
pub mod instruction;

mod tests;
