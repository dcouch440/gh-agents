//! Overlay persistence — persists OverlayFS diffs to JuiceFS and coordinates
//! three-way merge for parallel step overlays.

use tracing::{info, warn};
use uuid::Uuid;

use super::super::dag_state::DagExecutionState;
use super::super::DagContext;

/// Persist a sequential step's overlay to JuiceFS via `spawn_blocking`.
pub(super) async fn persist_step_overlay_if_present(
    dag: &DagContext<'_>,
    dag_state: &mut DagExecutionState,
) {
    let Some(mut overlay) = dag_state.step_overlay.take() else {
        return;
    };
    let Some(workspace) = dag.state.workspace() else {
        return;
    };
    let Some(cc) = dag.ctx.container_config.as_ref() else {
        return;
    };
    let (Some(wf_id), Some(run_id)) = (cc.workflow_id, cc.run_id) else {
        return;
    };

    let ws = workspace.clone();
    let step_id = overlay.step_id;
    let result = tokio::task::spawn_blocking(move || {
        super::super::merge::persist::persist_step_overlay(&ws, wf_id, run_id, &mut overlay)
    })
    .await;

    match result {
        Ok(Ok(count)) => {
            info!(step_id = %step_id, files = count, "Sequential overlay persisted");
        }
        Ok(Err(e)) => {
            warn!(step_id = %step_id, error = %e, "Failed to persist overlay");
        }
        Err(e) => {
            warn!(step_id = %step_id, error = %e, "Overlay persist task panicked");
        }
    }
}

/// Capture a pinned step's workspace files to the durable pinned location.
///
/// Called after overlay persistence so the manifest and files are available.
pub(super) async fn capture_pinned_step_files(dag: &DagContext<'_>, step_id: Uuid) {
    let Some(workspace) = dag.state.workspace() else {
        return;
    };
    let Some(cc) = dag.ctx.container_config.as_ref() else {
        return;
    };
    let (Some(wf_id), Some(run_id)) = (cc.workflow_id, cc.run_id) else {
        return;
    };

    let ws = workspace.clone();
    match tokio::task::spawn_blocking(move || ws.capture_pinned_files(wf_id, run_id, step_id)).await
    {
        Ok(Ok(n)) if n > 0 => {
            info!(step_id = %step_id, files = n, "Captured pinned step files");
        }
        Ok(Err(e)) => {
            warn!(step_id = %step_id, error = %e, "Failed to capture pinned step files");
        }
        _ => {}
    }
}

/// Merge parallel overlays and persist results to JuiceFS.
///
/// For a single overlay: auto-accept and persist directly.
/// For 2+ overlays: apply denylist, lazy-load base files for three-way merge,
/// call `merge_parallel_overlays`, persist outcomes.
pub(super) async fn merge_and_persist_overlays(
    dag: &DagContext<'_>,
    overlays: &mut Vec<super::super::merge::types::StepOverlay>,
) {
    let Some(workspace) = dag.state.workspace() else {
        return;
    };
    let Some(cc) = dag.ctx.container_config.as_ref() else {
        return;
    };
    let (Some(wf_id), Some(run_id)) = (cc.workflow_id, cc.run_id) else {
        return;
    };

    // Apply denylist to all overlays
    for ov in overlays.iter_mut() {
        super::super::merge::denylist::filter_overlay(ov);
    }

    if overlays.len() == 1 {
        // Single parallel step — auto-accept, persist directly
        let ws = workspace.clone();
        let mut ov = overlays.remove(0);
        let _ = tokio::task::spawn_blocking(move || {
            super::super::merge::persist::persist_step_overlay(&ws, wf_id, run_id, &mut ov)
        })
        .await;
        return;
    }

    // Multi-step: find files modified by 2+ steps, read their base content
    let paths_needing_base = find_multi_modified_paths(overlays);
    let base_files = if paths_needing_base.is_empty() {
        std::collections::HashMap::new()
    } else {
        let ws = workspace.clone();
        let paths = paths_needing_base;
        tokio::task::spawn_blocking(move || {
            ws.read_base_files(wf_id, run_id, &paths)
                .unwrap_or_default()
        })
        .await
        .unwrap_or_default()
    };

    // Merge
    match super::super::merge::merge_parallel_overlays(overlays, &base_files).await {
        Ok((outcomes, report)) => {
            info!(
                auto = report.auto_merged,
                llm = report.llm_resolved,
                "Parallel overlay merge completed"
            );
            let ws = workspace.clone();
            let _ = tokio::task::spawn_blocking(move || {
                super::super::merge::persist::persist_merge_outcomes(&ws, wf_id, run_id, &outcomes)
            })
            .await;
        }
        Err(e) => {
            warn!(error = %e, "Overlay merge failed, using last-write-wins");
            let outcomes = super::super::merge::fallback_last_write_wins(overlays);
            let ws = workspace.clone();
            let _ = tokio::task::spawn_blocking(move || {
                super::super::merge::persist::persist_merge_outcomes(&ws, wf_id, run_id, &outcomes)
            })
            .await;
        }
    }
}

/// Find paths modified by 2+ overlays (need base content for three-way merge).
fn find_multi_modified_paths(
    overlays: &[super::super::merge::types::StepOverlay],
) -> std::collections::HashSet<std::path::PathBuf> {
    use super::super::merge::types::OverlayChange;
    let mut seen: std::collections::HashMap<std::path::PathBuf, usize> =
        std::collections::HashMap::new();
    for ov in overlays {
        for (path, change) in &ov.diff {
            if matches!(change, OverlayChange::Modified(_)) {
                *seen.entry(path.clone()).or_default() += 1;
            }
        }
    }
    seen.into_iter()
        .filter(|(_, count)| *count >= 2)
        .map(|(path, _)| path)
        .collect()
}
