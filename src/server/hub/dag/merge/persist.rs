//! Overlay persistence — write clean overlay diffs to JuiceFS.
//!
//! Both functions are synchronous (`std::fs` via WorkspaceManager).
//! Callers should wrap in `tokio::task::spawn_blocking` to avoid
//! blocking the async runtime on FUSE I/O.

use std::collections::HashMap;
use std::path::PathBuf;

use tracing::{info, warn};
use uuid::Uuid;

use crate::server::services::workspace::{WorkspaceError, WorkspaceManager};

use super::denylist;
use super::types::{OverlayChange, StepOverlay};
use super::MergeOutcome;

/// Persist a single step's overlay to JuiceFS. Applies denylist first.
///
/// Returns the count of files written + deleted.
pub(crate) fn persist_step_overlay(
    workspace: &WorkspaceManager,
    workflow_id: Uuid,
    run_id: Uuid,
    overlay: &mut StepOverlay,
) -> Result<usize, WorkspaceError> {
    let removed = denylist::filter_overlay(overlay);
    if removed > 0 {
        info!(
            step_id = %overlay.step_id,
            removed,
            remaining = overlay.diff.len(),
            "Denylist filtered overlay entries"
        );
    }

    let mut count = 0;
    for (path, change) in &overlay.diff {
        match change {
            OverlayChange::Created(bytes) | OverlayChange::Modified(bytes) => {
                workspace.write_file(workflow_id, run_id, path, bytes)?;
                count += 1;
            }
            OverlayChange::Deleted => {
                if workspace.delete_file(workflow_id, run_id, path)? {
                    count += 1;
                }
            }
        }
    }

    info!(
        step_id = %overlay.step_id,
        files = count,
        "Overlay persisted to workspace"
    );
    Ok(count)
}

/// Persist merge outcomes (from parallel merge) to JuiceFS.
///
/// Returns the count of files written + deleted.
pub(crate) fn persist_merge_outcomes(
    workspace: &WorkspaceManager,
    workflow_id: Uuid,
    run_id: Uuid,
    outcomes: &HashMap<PathBuf, MergeOutcome>,
) -> Result<usize, WorkspaceError> {
    let mut count = 0;
    for (path, outcome) in outcomes {
        match outcome {
            MergeOutcome::Write(bytes) => {
                workspace.write_file(workflow_id, run_id, path, bytes)?;
                count += 1;
            }
            MergeOutcome::Delete => {
                if workspace.delete_file(workflow_id, run_id, path)? {
                    count += 1;
                }
            }
        }
    }

    if count > 0 {
        info!(files = count, "Merge outcomes persisted to workspace");
    }
    Ok(count)
}
