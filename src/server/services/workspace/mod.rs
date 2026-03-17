//! Workspace manager for JuiceFS-backed workflow workspaces.
//!
//! Manages directory lifecycle on the JuiceFS mount point. Each workflow run
//! gets an isolated directory at `{mount}/workflows/{workflow_id}/runs/{run_id}/`.
//!
//! This service does not manage containers or mounts — it only handles
//! directory CRUD on an already-mounted filesystem.

use std::path::{Path, PathBuf};

use thiserror::Error;
use uuid::Uuid;

#[cfg(test)]
mod tests;

// ── Errors ──────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("workspace mount point does not exist: {0}")]
    MountPointMissing(PathBuf),

    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

// ── WorkspaceManager ────────────────────────────────────────────────────────

/// Manages per-run workspace directories on a JuiceFS mount.
///
/// Clone is cheap — inner state is just a `PathBuf`.
#[derive(Debug, Clone)]
pub struct WorkspaceManager {
    /// Root of the JuiceFS mount (e.g. `/tmp/nexor-jfs`).
    mount_point: PathBuf,
}

impl WorkspaceManager {
    /// Create a new workspace manager rooted at `mount_point`.
    ///
    /// Returns an error if the mount point directory does not exist.
    pub fn new(mount_point: impl Into<PathBuf>) -> Result<Self, WorkspaceError> {
        let mount_point = mount_point.into();
        if !mount_point.exists() {
            return Err(WorkspaceError::MountPointMissing(mount_point));
        }
        Ok(Self { mount_point })
    }

    /// Try to create a workspace manager from an environment variable.
    ///
    /// Returns `None` if the env var is not set or the path doesn't exist.
    /// Logs a warning on path-not-found so startup isn't silent about it.
    pub fn from_env(env_var: &str) -> Option<Self> {
        let path = std::env::var(env_var).ok()?;
        match Self::new(&path) {
            Ok(mgr) => {
                tracing::info!("Workspace manager initialized at {}", path);
                Some(mgr)
            }
            Err(e) => {
                tracing::warn!("Workspace manager not available: {e}");
                None
            }
        }
    }

    /// The root mount point path.
    pub fn mount_point(&self) -> &Path {
        &self.mount_point
    }

    // ── Path helpers ────────────────────────────────────────────────────

    /// Path to a workflow's directory: `{mount}/workflows/{workflow_id}`.
    pub fn workflow_path(&self, workflow_id: Uuid) -> PathBuf {
        self.mount_point
            .join(crate::constants::WORKSPACE_PREFIX)
            .join(workflow_id.to_string())
    }

    /// Path to a specific run's workspace: `{mount}/workflows/{wf_id}/runs/{run_id}`.
    pub fn run_workspace_path(&self, workflow_id: Uuid, run_id: Uuid) -> PathBuf {
        self.workflow_path(workflow_id)
            .join("runs")
            .join(run_id.to_string())
    }

    // ── CRUD ────────────────────────────────────────────────────────────

    /// Create the run workspace directory (and parents). Returns the path.
    pub fn create_run_workspace(
        &self,
        workflow_id: Uuid,
        run_id: Uuid,
    ) -> Result<PathBuf, WorkspaceError> {
        let path = self.run_workspace_path(workflow_id, run_id);
        std::fs::create_dir_all(&path).map_err(|source| WorkspaceError::Io {
            path: path.clone(),
            source,
        })?;
        Ok(path)
    }

    /// Remove the run workspace directory and all contents.
    ///
    /// Returns `Ok(false)` if the directory didn't exist.
    pub fn destroy_run_workspace(
        &self,
        workflow_id: Uuid,
        run_id: Uuid,
    ) -> Result<bool, WorkspaceError> {
        let path = self.run_workspace_path(workflow_id, run_id);
        if !path.exists() {
            return Ok(false);
        }
        std::fs::remove_dir_all(&path).map_err(|source| WorkspaceError::Io {
            path: path.clone(),
            source,
        })?;
        Ok(true)
    }

    /// Check if a run workspace exists.
    pub fn workspace_exists(&self, workflow_id: Uuid, run_id: Uuid) -> bool {
        self.run_workspace_path(workflow_id, run_id).exists()
    }

    /// List files in a run workspace, optionally filtered by a prefix subdirectory.
    ///
    /// Returns paths relative to the run workspace root.
    pub fn list_files(
        &self,
        workflow_id: Uuid,
        run_id: Uuid,
        prefix: Option<&str>,
    ) -> Result<Vec<PathBuf>, WorkspaceError> {
        let base = self.run_workspace_path(workflow_id, run_id);
        let search_dir = match prefix {
            Some(p) => base.join(p),
            None => base.clone(),
        };

        if !search_dir.exists() {
            return Ok(Vec::new());
        }

        let mut files = Vec::new();
        collect_files_recursive(&search_dir, &base, &mut files)?;
        files.sort();
        Ok(files)
    }
}

/// Recursively collect file paths relative to `base`.
fn collect_files_recursive(
    dir: &Path,
    base: &Path,
    out: &mut Vec<PathBuf>,
) -> Result<(), WorkspaceError> {
    let entries = std::fs::read_dir(dir).map_err(|source| WorkspaceError::Io {
        path: dir.to_path_buf(),
        source,
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| WorkspaceError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();

        if path.is_dir() {
            collect_files_recursive(&path, base, out)?;
        } else if let Ok(rel) = path.strip_prefix(base) {
            out.push(rel.to_path_buf());
        }
    }
    Ok(())
}
