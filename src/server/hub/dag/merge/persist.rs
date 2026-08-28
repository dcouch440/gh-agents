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

/// Where a superseded version is kept: `.nexor/superseded/{step_id}/{path}`.
///
/// Sits beside `.nexor/step-manifests/`, out of the agents' way but inside the
/// run workspace, so it survives in the run download.
fn superseded_path(step_id: Uuid, path: &std::path::Path) -> PathBuf {
    PathBuf::from(".nexor/superseded")
        .join(step_id.to_string())
        .join(path)
}

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
    let mut superseded = 0;
    for (path, change) in &overlay.diff {
        match change {
            OverlayChange::Created(bytes) => {
                workspace.write_file(workflow_id, run_id, path, bytes)?;
                count += 1;
            }
            OverlayChange::Modified(bytes) => {
                // `Modified` means the path existed in base before this step
                // ran — the step is replacing someone else's file. This loop
                // used to be a silent last-write-wins, which is how run
                // dd27d008's Visual Direction deliverable ceased to exist when
                // a later step wrote the same filename.
                if !path.starts_with(".nexor") {
                    if let Some(prior) = workspace.read_file(workflow_id, run_id, path)? {
                        if prior != *bytes {
                            let snap = superseded_path(overlay.step_id, path);
                            workspace.write_file(workflow_id, run_id, &snap, &prior)?;
                            superseded += 1;
                            warn!(
                                step_id = %overlay.step_id,
                                path = %path.display(),
                                snapshot = %snap.display(),
                                prior_bytes = prior.len(),
                                new_bytes = bytes.len(),
                                "Step replaced an upstream file; prior version preserved"
                            );
                        }
                    }
                }
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

    // Write overlay manifest — tracks which files this step produced.
    // Used by the pin system to capture/preload step files across runs.
    let manifest_path =
        std::path::PathBuf::from(format!(".nexor/step-manifests/{}.json", overlay.step_id));
    let file_paths: Vec<String> = overlay
        .diff
        .keys()
        .map(|p| p.display().to_string())
        .collect();
    if let Ok(json) = serde_json::to_vec(&file_paths) {
        let _ = workspace.write_file(workflow_id, run_id, &manifest_path, &json);
    }

    info!(
        step_id = %overlay.step_id,
        files = count,
        superseded,
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
