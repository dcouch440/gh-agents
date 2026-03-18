//! Parallel workspace merge for DAG execution.
//!
//! When parallel steps write to the workspace via OverlayFS, their diffs
//! are merged before the next batch. Auto-merges non-conflicting changes,
//! uses diff3 for multi-step modifications, and resolves conflict hunks
//! via Grok one-shot LLM calls with file-type-aware context extraction.

use std::collections::HashMap;
use std::path::PathBuf;

use tracing::{info, warn};

use crate::constants;

pub(crate) mod classify;
pub(crate) mod context;
pub(crate) mod denylist;
pub(crate) mod diff3;
pub(crate) mod persist;
pub(crate) mod resolve;
pub mod types;
pub(crate) mod verify;

mod tests;

use classify::detect_file_type;
use context::extract_context;
use diff3::{n_way_merge, reassemble, three_way_merge};
use types::{
    FileClassification, FileType, MergeAction, MergeFileDetail, MergeReport, MergeResult, StepInfo,
    StepOverlay,
};

/// Result from an async file merge task.
struct AsyncMergeResult {
    path: PathBuf,
    outcome: MergeOutcome,
    detail: MergeFileDetail,
    tokens: u64,
    hunks_resolved: usize,
}

/// Merge parallel step overlays into a unified workspace state.
///
/// Called after a parallel batch completes in `execute_level_parallel`.
/// Returns the merged file contents to persist to JuiceFS and a report
/// for observability.
///
/// Uses a two-pass approach: sync auto-accepts first, then LLM-needing
/// merges run concurrently via JoinSet.
///
/// On failure, returns `Err` — the caller should fall back to
/// `fallback_last_write_wins`.
pub async fn merge_parallel_overlays(
    overlays: &[StepOverlay],
    base_files: &HashMap<PathBuf, Vec<u8>>,
) -> Result<(HashMap<PathBuf, MergeOutcome>, MergeReport), String> {
    if overlays.len() < 2 {
        return Err("merge_parallel_overlays requires 2+ overlays".to_string());
    }

    let classifications = classify::classify_overlays(overlays, base_files);
    let step_infos = build_step_info_map(overlays);

    let mut outcomes: HashMap<PathBuf, MergeOutcome> = HashMap::new();
    let mut report = MergeReport::default();

    // ── Pass 1: Sync — handle auto-accept cases, collect LLM work ────────
    let mut join_set = tokio::task::JoinSet::new();

    for (path, classification) in classifications {
        let file_type = detect_file_type(&path);

        match classification {
            // Auto-accept cases (no LLM needed)
            FileClassification::NewFileSingle { content, .. }
            | FileClassification::ModifiedSingle { content, .. }
            | FileClassification::BinarySingle { content, .. } => {
                outcomes.insert(path.clone(), MergeOutcome::Write(content));
                report.auto_merged += 1;
                report.file_details.push(MergeFileDetail {
                    path,
                    action: MergeAction::AutoAccepted,
                });
            }

            FileClassification::DeletedSingle => {
                outcomes.insert(path.clone(), MergeOutcome::Delete);
                report.auto_merged += 1;
                report.file_details.push(MergeFileDetail {
                    path,
                    action: MergeAction::Deleted,
                });
            }

            FileClassification::BinaryMulti { versions } => {
                let winner = pick_last_write(&versions, &step_infos);
                outcomes.insert(path.clone(), MergeOutcome::Write(winner));
                report.fallback_used += 1;
                report.file_details.push(MergeFileDetail {
                    path,
                    action: MergeAction::Fallback {
                        reason: "Binary file — last-write-wins".to_string(),
                    },
                });
            }

            // LLM-needing cases — spawn concurrently
            FileClassification::ModifiedMulti { versions } => {
                let base_owned = base_files.clone();
                let infos = step_infos.clone();
                let p = path.clone();
                let ft = file_type.clone();
                join_set.spawn(async move {
                    handle_modified_multi(&p, &ft, &versions, &base_owned, &infos).await
                });
            }

            FileClassification::NewFileMulti { versions } => {
                let infos = step_infos.clone();
                let p = path.clone();
                let ft = file_type.clone();
                join_set.spawn(async move { handle_new_new(&p, &ft, &versions, &infos).await });
            }

            FileClassification::DeletedConflict {
                modifier_step_id,
                modified_content,
            } => {
                let deleter_info = find_deleter_step(overlays, &path, modifier_step_id);
                let modifier_info = step_infos
                    .get(&modifier_step_id)
                    .cloned()
                    .unwrap_or_else(|| unknown_step_info(modifier_step_id));
                let p = path.clone();
                join_set.spawn(async move {
                    handle_delete_conflict(p, deleter_info, modifier_info, modified_content).await
                });
            }
        }
    }

    // ── Pass 2: Collect concurrent LLM results ───────────────────────────
    while let Some(join_result) = join_set.join_next().await {
        match join_result {
            Ok(result) => {
                report.total_tokens += result.tokens;
                if result.hunks_resolved > 0 {
                    report.llm_resolved += 1;
                    report.conflict_hunks += result.hunks_resolved;
                } else {
                    report.auto_merged += 1;
                }
                report.file_details.push(result.detail);
                outcomes.insert(result.path, result.outcome);
            }
            Err(e) => {
                warn!(error = %e, "Merge task panicked");
            }
        }
    }

    info!(
        auto_merged = report.auto_merged,
        llm_resolved = report.llm_resolved,
        conflict_hunks = report.conflict_hunks,
        fallback = report.fallback_used,
        "Workspace merge completed"
    );

    Ok((outcomes, report))
}

/// Fallback: accept the version from the step with the highest display_order.
pub fn fallback_last_write_wins(overlays: &[StepOverlay]) -> HashMap<PathBuf, MergeOutcome> {
    let mut all_paths: HashMap<PathBuf, (i32, MergeOutcome)> = HashMap::new();

    for overlay in overlays {
        for (path, change) in &overlay.diff {
            let dominated = all_paths
                .get(path)
                .map_or(true, |(order, _)| overlay.display_order > *order);

            if dominated {
                let outcome = match change {
                    types::OverlayChange::Created(c) | types::OverlayChange::Modified(c) => {
                        MergeOutcome::Write(c.clone())
                    }
                    types::OverlayChange::Deleted => MergeOutcome::Delete,
                };
                all_paths.insert(path.clone(), (overlay.display_order, outcome));
            }
        }
    }

    all_paths.into_iter().map(|(k, (_, v))| (k, v)).collect()
}

/// What to do with a file after merge.
#[derive(Debug)]
pub enum MergeOutcome {
    Write(Vec<u8>),
    Delete,
}

// ── Internal Handlers (return AsyncMergeResult for concurrent collection) ────

async fn handle_modified_multi(
    path: &PathBuf,
    file_type: &FileType,
    versions: &[(uuid::Uuid, Vec<u8>)],
    base_files: &HashMap<PathBuf, Vec<u8>>,
    step_infos: &HashMap<uuid::Uuid, StepInfo>,
) -> AsyncMergeResult {
    let base_bytes = match base_files.get(path) {
        Some(b) => b,
        None => {
            warn!(path = %path.display(), "No base file for modified_multi — using first version");
            return AsyncMergeResult {
                path: path.clone(),
                outcome: MergeOutcome::Write(versions[0].1.clone()),
                detail: MergeFileDetail {
                    path: path.clone(),
                    action: MergeAction::Fallback {
                        reason: "No base file found".to_string(),
                    },
                },
                tokens: 0,
                hunks_resolved: 0,
            };
        }
    };

    // Check file size limit
    if base_bytes.len() > constants::MERGE_MAX_FILE_SIZE {
        warn!(path = %path.display(), size = base_bytes.len(), "File too large for merge");
        return AsyncMergeResult {
            path: path.clone(),
            outcome: MergeOutcome::Write(pick_last_write(versions, step_infos)),
            detail: MergeFileDetail {
                path: path.clone(),
                action: MergeAction::Fallback {
                    reason: format!("File too large: {} bytes", base_bytes.len()),
                },
            },
            tokens: 0,
            hunks_resolved: 0,
        };
    }

    // Convert to UTF-8 strings
    let base_str = match std::str::from_utf8(base_bytes) {
        Ok(s) => s,
        Err(_) => {
            return AsyncMergeResult {
                path: path.clone(),
                outcome: MergeOutcome::Write(pick_last_write(versions, step_infos)),
                detail: MergeFileDetail {
                    path: path.clone(),
                    action: MergeAction::Fallback {
                        reason: "Non-UTF8 file".to_string(),
                    },
                },
                tokens: 0,
                hunks_resolved: 0,
            };
        }
    };

    let version_strs: Vec<(&uuid::Uuid, &str)> = versions
        .iter()
        .filter_map(|(id, bytes)| std::str::from_utf8(bytes).ok().map(|s| (id, s)))
        .collect();

    if version_strs.len() < 2 {
        return AsyncMergeResult {
            path: path.clone(),
            outcome: MergeOutcome::Write(versions[0].1.clone()),
            detail: MergeFileDetail {
                path: path.clone(),
                action: MergeAction::AutoAccepted,
            },
            tokens: 0,
            hunks_resolved: 0,
        };
    }

    // Sort by display_order for determinism
    let mut sorted_versions: Vec<_> = version_strs
        .iter()
        .map(|(id, s)| {
            let order = step_infos.get(id).map(|si| si.display_order).unwrap_or(0);
            (order, **id, *s)
        })
        .collect();
    sorted_versions.sort_by_key(|(order, _, _)| *order);

    // Try N-way merge
    let str_refs: Vec<&str> = sorted_versions.iter().map(|(_, _, s)| *s).collect();
    match n_way_merge(base_str, &str_refs) {
        MergeResult::Clean(merged) => AsyncMergeResult {
            path: path.clone(),
            outcome: MergeOutcome::Write(merged.into_bytes()),
            detail: MergeFileDetail {
                path: path.clone(),
                action: MergeAction::CleanMerge,
            },
            tokens: 0,
            hunks_resolved: 0,
        },
        MergeResult::Conflicts { conflicted, hunks } => {
            let step_a_id = sorted_versions[0].1;
            let step_b_id = sorted_versions[1].1;
            let step_a = step_infos
                .get(&step_a_id)
                .cloned()
                .unwrap_or_else(|| unknown_step_info(step_a_id));
            let step_b = step_infos
                .get(&step_b_id)
                .cloned()
                .unwrap_or_else(|| unknown_step_info(step_b_id));

            let mut resolved_hunks = Vec::new();
            let hunk_count = hunks.len();
            let mut hunk_tokens: u64 = 0;

            // Hunks within a single file are sequential (each depends on context)
            for hunk in &hunks {
                let context = extract_context(
                    base_str,
                    hunk,
                    file_type,
                    &path.to_string_lossy(),
                    sorted_versions[0].2,
                    sorted_versions[1].2,
                );
                match resolve::resolve_hunk(hunk, &context, &step_a, &step_b).await {
                    Ok((resolved, tokens)) => {
                        resolved_hunks.push(resolved);
                        hunk_tokens += tokens;
                    }
                    Err(e) => {
                        warn!(
                            path = %path.display(),
                            error = %e,
                            "Hunk resolution failed — using version A"
                        );
                        resolved_hunks.push(hunk.version_a_lines.clone());
                    }
                }
            }

            let final_content = reassemble(&conflicted, &resolved_hunks);
            AsyncMergeResult {
                path: path.clone(),
                outcome: MergeOutcome::Write(final_content.into_bytes()),
                detail: MergeFileDetail {
                    path: path.clone(),
                    action: MergeAction::LlmResolved { hunks: hunk_count },
                },
                tokens: hunk_tokens,
                hunks_resolved: hunk_count,
            }
        }
    }
}

async fn handle_new_new(
    path: &PathBuf,
    file_type: &FileType,
    versions: &[(uuid::Uuid, Vec<u8>)],
    step_infos: &HashMap<uuid::Uuid, StepInfo>,
) -> AsyncMergeResult {
    if versions.len() < 2 {
        return AsyncMergeResult {
            path: path.clone(),
            outcome: MergeOutcome::Write(versions[0].1.clone()),
            detail: MergeFileDetail {
                path: path.clone(),
                action: MergeAction::AutoAccepted,
            },
            tokens: 0,
            hunks_resolved: 0,
        };
    }

    let strs: Vec<(&uuid::Uuid, &str)> = versions
        .iter()
        .filter_map(|(id, bytes)| std::str::from_utf8(bytes).ok().map(|s| (id, s)))
        .collect();

    if strs.len() < 2 {
        return AsyncMergeResult {
            path: path.clone(),
            outcome: MergeOutcome::Write(versions[0].1.clone()),
            detail: MergeFileDetail {
                path: path.clone(),
                action: MergeAction::AutoAccepted,
            },
            tokens: 0,
            hunks_resolved: 0,
        };
    }

    // Try diff3 with empty string as base
    match three_way_merge("", strs[0].1, strs[1].1) {
        MergeResult::Clean(merged) => AsyncMergeResult {
            path: path.clone(),
            outcome: MergeOutcome::Write(merged.into_bytes()),
            detail: MergeFileDetail {
                path: path.clone(),
                action: MergeAction::CleanMerge,
            },
            tokens: 0,
            hunks_resolved: 0,
        },
        MergeResult::Conflicts { .. } => {
            let step_a = step_infos
                .get(strs[0].0)
                .cloned()
                .unwrap_or_else(|| unknown_step_info(*strs[0].0));
            let step_b = step_infos
                .get(strs[1].0)
                .cloned()
                .unwrap_or_else(|| unknown_step_info(*strs[1].0));

            match resolve::resolve_new_new(
                &path.to_string_lossy(),
                file_type,
                &step_a,
                strs[0].1,
                &step_b,
                strs[1].1,
            )
            .await
            {
                Ok((merged, tokens)) => AsyncMergeResult {
                    path: path.clone(),
                    outcome: MergeOutcome::Write(merged.into_bytes()),
                    detail: MergeFileDetail {
                        path: path.clone(),
                        action: MergeAction::LlmResolved { hunks: 1 },
                    },
                    tokens,
                    hunks_resolved: 1,
                },
                Err(e) => {
                    warn!(path = %path.display(), error = %e, "New-new merge failed");
                    AsyncMergeResult {
                        path: path.clone(),
                        outcome: MergeOutcome::Write(versions[0].1.clone()),
                        detail: MergeFileDetail {
                            path: path.clone(),
                            action: MergeAction::Fallback {
                                reason: format!("LLM failed: {e}"),
                            },
                        },
                        tokens: 0,
                        hunks_resolved: 0,
                    }
                }
            }
        }
    }
}

async fn handle_delete_conflict(
    path: PathBuf,
    deleter_info: StepInfo,
    modifier_info: StepInfo,
    modified_content: Vec<u8>,
) -> AsyncMergeResult {
    let (keep, tokens) = resolve::resolve_delete_modify(
        &path.to_string_lossy(),
        &deleter_info,
        &modifier_info,
        "(modified)",
    )
    .await;

    AsyncMergeResult {
        outcome: if keep {
            MergeOutcome::Write(modified_content)
        } else {
            MergeOutcome::Delete
        },
        detail: MergeFileDetail {
            path: path.clone(),
            action: MergeAction::LlmResolved { hunks: 1 },
        },
        path,
        tokens,
        hunks_resolved: 1,
    }
}

// ── Utilities ────────────────────────────────────────────────────────────────

fn build_step_info_map(overlays: &[StepOverlay]) -> HashMap<uuid::Uuid, StepInfo> {
    overlays
        .iter()
        .map(|o| {
            (
                o.step_id,
                StepInfo {
                    step_id: o.step_id,
                    name: o.step_name.clone(),
                    description: o.step_description.clone(),
                    display_order: o.display_order,
                },
            )
        })
        .collect()
}

fn pick_last_write(
    versions: &[(uuid::Uuid, Vec<u8>)],
    step_infos: &HashMap<uuid::Uuid, StepInfo>,
) -> Vec<u8> {
    versions
        .iter()
        .max_by_key(|(id, _)| step_infos.get(id).map(|si| si.display_order).unwrap_or(0))
        .map(|(_, content)| content.clone())
        .unwrap_or_default()
}

fn find_deleter_step(
    overlays: &[StepOverlay],
    path: &PathBuf,
    modifier_step_id: uuid::Uuid,
) -> StepInfo {
    for overlay in overlays {
        if overlay.step_id == modifier_step_id {
            continue;
        }
        if let Some(types::OverlayChange::Deleted) = overlay.diff.get(path) {
            return StepInfo {
                step_id: overlay.step_id,
                name: overlay.step_name.clone(),
                description: overlay.step_description.clone(),
                display_order: overlay.display_order,
            };
        }
    }
    unknown_step_info(uuid::Uuid::nil())
}

fn unknown_step_info(step_id: uuid::Uuid) -> StepInfo {
    StepInfo {
        step_id,
        name: "Unknown".to_string(),
        description: "Unknown step".to_string(),
        display_order: 0,
    }
}
