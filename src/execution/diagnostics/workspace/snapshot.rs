//! Upper directory snapshot — lightweight metadata capture for per-command diffs.
//!
//! Uses the same `find -printf` technique as `extract_overlay_diff()` in
//! `src/execution/container/overlay/mod.rs`, but adds `%T@` for mtime
//! tracking and skips file content reads entirely.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::constants::OVERLAY_MERGED_DIR;
use crate::execution::ContainerHandle;

/// Metadata-only snapshot of the overlay upper directory.
#[derive(Debug, Clone)]
pub struct UpperDirSnapshot {
    pub entries: HashMap<PathBuf, FileMetadata>,
}

/// Per-file metadata captured from `find -printf`.
#[derive(Debug, Clone)]
pub struct FileMetadata {
    /// File type: 'f' (regular), 'd' (directory), 'c' (whiteout/char device).
    pub file_type: char,
    /// Size in bytes.
    pub size: u64,
    /// Modification time as epoch seconds (from `%T@`).
    pub mtime: f64,
}

impl UpperDirSnapshot {
    pub fn empty() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Count of regular files (type 'f') in the snapshot.
    pub fn file_count(&self) -> usize {
        self.entries.values().filter(|m| m.file_type == 'f').count()
    }

    /// Count of directories (type 'd') in the snapshot.
    pub fn dir_count(&self) -> usize {
        self.entries.values().filter(|m| m.file_type == 'd').count()
    }

    /// Total size of all regular files.
    pub fn total_size(&self) -> u64 {
        self.entries
            .values()
            .filter(|m| m.file_type == 'f')
            .map(|m| m.size)
            .sum()
    }
}

/// Capture a metadata-only snapshot of the overlay upper directory.
///
/// Graceful: returns an empty snapshot on failure (never blocks the agent).
/// Typical cost: ~50ms for 100 files, ~200ms for 1000.
///
/// Walks the merged view (`/workspace`) so it sees both inherited files
/// from prior steps (lower layer) and new files (upper layer).
/// Depth-limited to 4 levels to keep it fast.
pub async fn capture_snapshot(handle: &ContainerHandle) -> UpperDirSnapshot {
    let cmd = format!(
        "find {} -maxdepth 4 -mindepth 1 -printf '%P\\t%y\\t%s\\t%T@\\n' 2>/dev/null || true",
        OVERLAY_MERGED_DIR
    );
    match handle.exec_shell(&cmd).await {
        Ok(result) => parse_snapshot(&result.stdout),
        Err(e) => {
            tracing::warn!(
                container = %handle.container_name(),
                error = %e,
                "Snapshot capture failed — returning empty snapshot"
            );
            UpperDirSnapshot::empty()
        }
    }
}

/// Parse tab-delimited `find -printf '%P\t%y\t%s\t%T@\n'` output.
pub fn parse_snapshot(output: &str) -> UpperDirSnapshot {
    let entries = output
        .lines()
        .filter_map(parse_snapshot_line)
        .collect::<HashMap<_, _>>();
    UpperDirSnapshot { entries }
}

/// Parse a single snapshot line: `relative_path\tfile_type\tsize\tmtime`.
fn parse_snapshot_line(line: &str) -> Option<(PathBuf, FileMetadata)> {
    let parts: Vec<&str> = line.splitn(4, '\t').collect();
    if parts.len() < 4 {
        return None;
    }

    let path_str = parts[0].trim();
    if path_str.is_empty() {
        return None;
    }

    let file_type = parts[1].chars().next()?;
    let size = parts[2].trim().parse::<u64>().unwrap_or(0);
    let mtime = parts[3].trim().parse::<f64>().unwrap_or(0.0);

    Some((
        PathBuf::from(path_str),
        FileMetadata {
            file_type,
            size,
            mtime,
        },
    ))
}
