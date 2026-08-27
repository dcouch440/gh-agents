//! Workspace digest — one-line spatial awareness summary.
//!
//! Appended to every command response so agents know the workspace state
//! without running `ls` to re-orient.

use std::path::PathBuf;

/// Compact workspace summary.
#[derive(Debug, Clone)]
pub struct WorkspaceDigest {
    /// Total regular files in the upper directory.
    pub file_count: usize,
    /// Change since last command: +N or -N.
    pub file_delta: i32,
    /// Total directories.
    pub dir_count: usize,
    /// Total size of regular files.
    pub total_size: u64,
    /// Most recently modified file (by mtime).
    pub last_modified: Option<PathBuf>,
}

impl WorkspaceDigest {
    /// Render as a single line for the LLM.
    pub fn render(&self) -> String {
        let delta = if self.file_delta > 0 {
            format!(" (+{})", self.file_delta)
        } else if self.file_delta < 0 {
            format!(" ({})", self.file_delta)
        } else {
            String::new()
        };

        // `last_modified` is deliberately not rendered — the envelope's
        // `changes:` block already names the paths that just changed.
        format!(
            "{} files{}, {} dirs | {} total",
            self.file_count,
            delta,
            self.dir_count,
            format_size(self.total_size),
        )
    }
}

/// Human-readable size formatting. Shared with the agent passdown manifest.
pub fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{}KB", bytes / 1024)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_positive_delta() {
        let d = WorkspaceDigest {
            file_count: 14,
            file_delta: 2,
            dir_count: 3,
            total_size: 12288,
            last_modified: Some(PathBuf::from("reports/analysis.md")),
        };
        let rendered = d.render();
        assert!(rendered.contains("14 files (+2)"));
        assert!(rendered.contains("3 dirs"));
        assert!(rendered.contains("12KB"));
        // `last:` is intentionally not rendered — `changes:` already names paths.
        assert!(!rendered.contains("last:"));
    }

    #[test]
    fn render_negative_delta() {
        let d = WorkspaceDigest {
            file_count: 5,
            file_delta: -1,
            dir_count: 2,
            total_size: 500,
            last_modified: None,
        };
        let rendered = d.render();
        assert!(rendered.contains("5 files (-1)"));
        assert!(!rendered.contains("last:"));
    }

    #[test]
    fn render_zero_delta() {
        let d = WorkspaceDigest {
            file_count: 10,
            file_delta: 0,
            dir_count: 1,
            total_size: 2_097_152,
            last_modified: Some(PathBuf::from("main.py")),
        };
        let rendered = d.render();
        assert!(rendered.contains("10 files,"));
        assert!(rendered.contains("2.0MB"));
    }
}
