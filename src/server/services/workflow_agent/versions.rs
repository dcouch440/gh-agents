//! Workflow version management — save, list, and restore checkpoints.
//!
//! Uses `WorkflowSnapshot` (same as templates) for full state capture.
//! Restores atomically via `restore_workflow_from_snapshot()`.

use tracing::info;
use uuid::Uuid;

use crate::db::WorkflowVersionRow;
use crate::server::hub::dag::templates::{
    capture_workflow_snapshot, restore::restore_workflow_from_snapshot, WorkflowSnapshot,
};
use crate::server::services::ServiceError;
use crate::server::state::AppState;

#[cfg(test)]
#[path = "versions_tests.rs"]
mod tests;

// ── Public API ─────────────────────────────────────────────────────────────

/// Save a version checkpoint of the current workflow state.
///
/// Captures the full `WorkflowSnapshot` and stores it as JSONB.
pub(crate) async fn save_version(
    workflow_id: Uuid,
    user_id: Uuid,
    label: Option<String>,
    source: &str,
    state: &AppState,
) -> Result<WorkflowVersionRow, ServiceError> {
    let snapshot = capture_workflow_snapshot(state, workflow_id)
        .await
        .map_err(ServiceError::Internal)?;

    let snapshot_json = serde_json::to_value(&snapshot).map_err(|e| {
        ServiceError::Internal(anyhow::anyhow!("Failed to serialize snapshot: {e}"))
    })?;

    let version_number = state
        .repos()
        .workflows
        .get_latest_version_number(workflow_id)
        .await
        .map_err(ServiceError::Internal)?
        + 1;

    let row = WorkflowVersionRow {
        id: Uuid::new_v4(),
        workflow_id,
        version_number,
        label,
        source: source.to_string(),
        snapshot: snapshot_json,
        created_by: user_id,
        created_at: chrono::Utc::now(),
    };

    let created = state
        .repos()
        .workflows
        .create_workflow_version(row)
        .await
        .map_err(ServiceError::Internal)?;

    info!(
        workflow_id = %workflow_id,
        version = created.version_number,
        source = source,
        "Saved workflow version"
    );

    Ok(created)
}

/// List all versions for a workflow, newest first.
pub(crate) async fn list_versions(
    workflow_id: Uuid,
    state: &AppState,
) -> Result<Vec<WorkflowVersionRow>, ServiceError> {
    state
        .repos()
        .workflows
        .list_workflow_versions(workflow_id)
        .await
        .map_err(ServiceError::Internal)
}

/// Restore a workflow to a previous version checkpoint.
///
/// 1. Auto-saves current state as "auto_pre_revert"
/// 2. Restores the target version's snapshot atomically
/// 3. Archives the current workflow agent session and creates a new one
/// 4. Re-projects the board repo to match restored state
/// 5. Broadcasts update event
///
/// Returns the auto-checkpoint (so user can undo the revert).
pub(crate) async fn restore_version(
    workflow_id: Uuid,
    version_id: Uuid,
    user_id: Uuid,
    state: &AppState,
) -> Result<WorkflowVersionRow, ServiceError> {
    // 1. Auto-checkpoint current state
    let auto_checkpoint =
        save_version(workflow_id, user_id, None, "auto_pre_revert", state).await?;

    // 2. Load target version
    let target = state
        .repos()
        .workflows
        .get_workflow_version(version_id)
        .await
        .map_err(ServiceError::Internal)?
        .ok_or_else(|| ServiceError::not_found("Version"))?;

    if target.workflow_id != workflow_id {
        return Err(ServiceError::not_found("Version"));
    }

    // 3. Deserialize and restore snapshot atomically
    let snapshot: WorkflowSnapshot = serde_json::from_value(target.snapshot).map_err(|e| {
        ServiceError::Internal(anyhow::anyhow!("Failed to deserialize snapshot: {e}"))
    })?;

    let pool = state
        .db()
        .ok_or_else(|| ServiceError::Internal(anyhow::anyhow!("Database not available")))?;

    restore_workflow_from_snapshot(pool, workflow_id, &snapshot)
        .await
        .map_err(ServiceError::Internal)?;

    // 4. Hide conversation messages after the checkpoint timestamp
    //    Same session persists — messages before the checkpoint remain visible,
    //    messages after are soft-deleted (hidden_at set). No data duplication.
    if let Ok(Some(session)) = state
        .repos()
        .sessions
        .find_workflow_agent_session(workflow_id)
        .await
    {
        let hidden = state
            .repos()
            .sessions
            .hide_messages_after(session.id, target.created_at)
            .await
            .unwrap_or(0);

        info!(
            workflow_id = %workflow_id,
            session_id = %session.id,
            messages_hidden = hidden,
            "Hid conversation messages after checkpoint"
        );
    }

    // 5. Re-project board repo
    let base_dir = super::resolve_base_dir(state, workflow_id);
    let wf_repo = &*state.repos().workflows;
    let _ = super::project::project_to_repo(&base_dir, workflow_id, wf_repo).await;

    // 6. Broadcast
    let label = target.label.as_deref().unwrap_or("unnamed");
    info!(
        workflow_id = %workflow_id,
        version = target.version_number,
        label = label,
        "Restored workflow to version"
    );

    use crate::server::ws::events::{WorkflowEvent, WorkflowEventKind};
    state.broadcast_workflow(WorkflowEvent {
        run_id: None,
        workflow_id,
        user_id: Some(user_id),
        kind: WorkflowEventKind::StepConfigUpdated {
            step_id: workflow_id,
        },
    });

    Ok(auto_checkpoint)
}
