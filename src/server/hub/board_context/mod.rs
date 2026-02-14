//! Board context — ambient node awareness via Haiku-distilled summaries.
//!
//! Renders the workflow board and distills per-node perspective summaries
//! so each node's agent sounds like an informed colleague. Cached in DB
//! with stale-on-write, refresh-on-read pattern.

pub mod distiller;
pub mod renderer;
pub mod refresh;

mod tests;

use crate::server::state::AppState;
use uuid::Uuid;

/// Spawn a background task to refresh board context for all nodes in a workflow.
///
/// Non-blocking — fires and forgets. Errors are logged, not propagated.
pub fn spawn_board_refresh(state: AppState, workflow_id: Uuid) {
    tokio::spawn(async move {
        if let Err(e) = refresh::refresh_board_context(&state, workflow_id).await {
            tracing::error!("Background board context refresh failed: {e}");
        }
    });
}

/// Spawn a background task to refresh a single node's goal summary.
///
/// Non-blocking — fires and forgets. Errors are logged, not propagated.
pub fn spawn_goal_refresh(state: AppState, step_id: Uuid, session_id: Uuid, workflow_id: Uuid) {
    tokio::spawn(async move {
        if let Err(e) =
            refresh::refresh_node_goal(&state, step_id, session_id, workflow_id).await
        {
            tracing::error!("Background goal refresh failed for step {step_id}: {e}");
        }
    });
}
