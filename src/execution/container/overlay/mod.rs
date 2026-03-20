//! OverlayFS setup and diff extraction for agent containers.
//!
//! Each agent container mounts an OverlayFS with JuiceFS as the read-only
//! lower layer and a local writable upper layer. After the step completes,
//! the upper directory is walked to extract a `StepOverlay` representing
//! all files created, modified, or deleted by the agent.

mod tests;

use std::collections::HashSet;
use std::path::PathBuf;

use tracing::{info, warn};
use uuid::Uuid;

use crate::constants;
use crate::server::hub::dag::merge::types::{OverlayChange, OverlayDiff, StepOverlay};

use super::ContainerError;
use super::ContainerHandle;

// ── Setup ─────────────────────────────────────────────────────────────────

/// Set up OverlayFS inside an agent container.
///
/// Expects the JuiceFS volume already mounted at `/workspace-base` (read-only).
/// Creates upper + work directories, then mounts an overlay at `/workspace`.
/// The `volatile` option skips fsync for performance (safe since containers
/// are ephemeral).
pub async fn setup_overlay(handle: &ContainerHandle) -> Result<(), ContainerError> {
    // The upper and work dirs must live on a real filesystem, not on Docker's
    // own OverlayFS root (nested overlay-on-overlay is not supported). Mount a
    // tmpfs at /tmp/overlay first, then create upper + work inside it.
    let script = format!(
        "mkdir -p /tmp/overlay && \
         mount -t tmpfs tmpfs /tmp/overlay && \
         mkdir -p {} {} {} && \
         mount -t overlay overlay \
         -o lowerdir={},upperdir={},workdir={} \
         {}",
        constants::OVERLAY_UPPER_DIR,
        constants::OVERLAY_WORK_DIR,
        constants::OVERLAY_MERGED_DIR,
        constants::OVERLAY_LOWER_DIR,
        constants::OVERLAY_UPPER_DIR,
        constants::OVERLAY_WORK_DIR,
        constants::OVERLAY_MERGED_DIR,
    );

    tracing::info!(script = %script, "Running overlay setup script");

    let result = handle.exec_shell(&script).await?;
    if !result.success {
        tracing::error!(
            exit_code = result.exit_code,
            stderr = %result.stderr,
            stdout = %result.stdout,
            script = %script,
            "OverlayFS setup script failed"
        );
        return Err(ContainerError::CreationFailed(format!(
            "OverlayFS mount failed (exit {}): {}",
            result.exit_code, result.stderr
        )));
    }

    Ok(())
}

// ── Diff Extraction ───────────────────────────────────────────────────────

/// Extract the overlay diff from a container's upper directory.
///
/// Walks `/tmp/overlay/upper` to detect created, modified, and deleted files.
/// Whiteout files (`.wh.{name}` or char device 0:0) indicate deletions.
/// Files present in `base_file_paths` are classified as `Modified`; others
/// as `Created`.
///
/// Respects `OVERLAY_MAX_FILES` and `OVERLAY_MAX_TOTAL_BYTES` safety limits.
pub async fn extract_overlay_diff(
    handle: &ContainerHandle,
    step_id: Uuid,
    step_name: String,
    step_description: String,
    display_order: i32,
    base_file_paths: &HashSet<PathBuf>,
) -> Result<StepOverlay, ContainerError> {
    // Phase 1: Inventory the upper directory
    let inventory_cmd = format!(
        "find {} -mindepth 1 -printf '%P\\t%y\\t%s\\n' 2>/dev/null || true",
        constants::OVERLAY_UPPER_DIR
    );
    let inventory = handle.exec_shell(&inventory_cmd).await?;

    let entries = parse_inventory(&inventory.stdout);
    let mut diff = OverlayDiff::new();
    let mut total_bytes: usize = 0;
    let mut file_count: usize = 0;

    // Phase 2: Process each entry
    for entry in entries {
        // Safety limits
        if file_count >= constants::OVERLAY_MAX_FILES {
            warn!(
                step_id = %step_id,
                max = constants::OVERLAY_MAX_FILES,
                "Overlay diff extraction hit file limit, returning partial diff"
            );
            break;
        }
        if total_bytes >= constants::OVERLAY_MAX_TOTAL_BYTES {
            warn!(
                step_id = %step_id,
                max = constants::OVERLAY_MAX_TOTAL_BYTES,
                "Overlay diff extraction hit byte limit, returning partial diff"
            );
            break;
        }

        match entry.file_type {
            // Whiteout character device = deletion
            'c' => {
                diff.insert(entry.path, OverlayChange::Deleted);
                file_count += 1;
            }
            // Directory — skip (implicit in file paths)
            'd' => {
                // Check for opaque dir marker inside
                // (.wh..wh..opq is filtered out by inventory, dirs themselves are structural)
                continue;
            }
            // Regular file
            'f' => {
                let file_name = entry
                    .path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");

                // Whiteout file = deletion marker
                if is_whiteout_file(file_name) {
                    let real_name = strip_whiteout_prefix(file_name);
                    let real_path = entry
                        .path
                        .parent()
                        .map(|p| p.join(real_name))
                        .unwrap_or_else(|| PathBuf::from(real_name));
                    diff.insert(real_path, OverlayChange::Deleted);
                    file_count += 1;
                    continue;
                }

                // Opaque directory marker — skip
                if file_name == ".wh..wh..opq" {
                    continue;
                }

                // Read file content (raw bytes to preserve binary data)
                let abs_path = format!("{}/{}", constants::OVERLAY_UPPER_DIR, entry.path.display());
                let content_result = handle.exec_raw(&["cat", &abs_path]).await;
                match content_result {
                    Ok((bytes, true, _)) => {
                        total_bytes += bytes.len();

                        let change = if base_file_paths.contains(&entry.path) {
                            OverlayChange::Modified(bytes)
                        } else {
                            OverlayChange::Created(bytes)
                        };
                        diff.insert(entry.path, change);
                        file_count += 1;
                    }
                    Ok((_, false, exit_code)) => {
                        warn!(
                            path = %entry.path.display(),
                            exit_code,
                            "Failed to read overlay file, skipping"
                        );
                    }
                    Err(e) => {
                        warn!(
                            path = %entry.path.display(),
                            error = %e,
                            "Error reading overlay file, skipping"
                        );
                    }
                }
            }
            // Symlink, socket, etc. — skip
            _ => continue,
        }
    }

    info!(
        step_id = %step_id,
        files = file_count,
        bytes = total_bytes,
        "Overlay diff extracted"
    );

    Ok(StepOverlay {
        step_id,
        step_name,
        step_description,
        display_order,
        diff,
    })
}

// ── Inventory Parsing ─────────────────────────────────────────────────────

/// A parsed entry from the `find -printf` inventory.
#[derive(Debug)]
struct InventoryEntry {
    path: PathBuf,
    file_type: char,
    #[allow(dead_code)]
    size: u64,
}

/// Parse the tab-delimited output of `find -printf '%P\t%y\t%s\n'`.
fn parse_inventory(output: &str) -> Vec<InventoryEntry> {
    output.lines().filter_map(parse_inventory_line).collect()
}

/// Parse a single inventory line: `relative_path\tfile_type\tsize`.
fn parse_inventory_line(line: &str) -> Option<InventoryEntry> {
    let parts: Vec<&str> = line.splitn(3, '\t').collect();
    if parts.len() < 3 {
        return None;
    }

    let path_str = parts[0].trim();
    if path_str.is_empty() {
        return None;
    }

    let file_type = parts[1].chars().next()?;
    let size = parts[2].trim().parse::<u64>().unwrap_or(0);

    Some(InventoryEntry {
        path: PathBuf::from(path_str),
        file_type,
        size,
    })
}

// ── Whiteout Helpers ──────────────────────────────────────────────────────

/// Check if a filename is an OverlayFS whiteout marker.
///
/// Whiteout files are named `.wh.{original_name}` and indicate that the
/// original file was deleted in the overlay. The special marker
/// `.wh..wh..opq` is NOT a whiteout — it marks an opaque directory.
fn is_whiteout_file(name: &str) -> bool {
    name.starts_with(".wh.") && name != ".wh..wh..opq"
}

/// Strip the `.wh.` prefix from a whiteout filename to get the original name.
fn strip_whiteout_prefix(name: &str) -> &str {
    name.strip_prefix(".wh.").unwrap_or(name)
}
