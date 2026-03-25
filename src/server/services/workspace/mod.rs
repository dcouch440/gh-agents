//! Workspace manager for JuiceFS-backed workflow workspaces.
//!
//! Manages directory lifecycle on the JuiceFS mount point. Each workflow run
//! gets an isolated directory at `{mount}/workflows/{workflow_id}/runs/{run_id}/`.
//!
//! This service does not manage containers or mounts — it only handles
//! directory CRUD on an already-mounted filesystem.

use std::collections::{HashMap, HashSet};
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

    /// Path for a system node agent's config repository:
    /// `{mount}/workflows/{wf_id}/system_node/{step_id}`.
    ///
    /// Lives outside `runs/` so it survives run cleanup. Persists across
    /// dispatches — the agent sees its previous config on re-runs.
    pub fn system_node_path(&self, workflow_id: Uuid, step_id: Uuid) -> PathBuf {
        self.workflow_path(workflow_id)
            .join("system_node")
            .join(step_id.to_string())
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

    // ── Read / Write / Delete ────────────────────────────────────────

    /// Write a file to the run workspace. Creates parent directories as needed.
    pub fn write_file(
        &self,
        workflow_id: Uuid,
        run_id: Uuid,
        relative_path: &Path,
        content: &[u8],
    ) -> Result<(), WorkspaceError> {
        let full_path = self
            .run_workspace_path(workflow_id, run_id)
            .join(relative_path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| WorkspaceError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        std::fs::write(&full_path, content).map_err(|source| WorkspaceError::Io {
            path: full_path,
            source,
        })
    }

    /// Delete a file from the run workspace. Returns `Ok(false)` if the file didn't exist.
    pub fn delete_file(
        &self,
        workflow_id: Uuid,
        run_id: Uuid,
        relative_path: &Path,
    ) -> Result<bool, WorkspaceError> {
        let full_path = self
            .run_workspace_path(workflow_id, run_id)
            .join(relative_path);
        if !full_path.exists() {
            return Ok(false);
        }
        std::fs::remove_file(&full_path).map_err(|source| WorkspaceError::Io {
            path: full_path,
            source,
        })?;
        Ok(true)
    }

    /// Read a file's content from the run workspace. Returns `None` if the file doesn't exist.
    pub fn read_file(
        &self,
        workflow_id: Uuid,
        run_id: Uuid,
        relative_path: &Path,
    ) -> Result<Option<Vec<u8>>, WorkspaceError> {
        let full_path = self
            .run_workspace_path(workflow_id, run_id)
            .join(relative_path);
        if !full_path.exists() {
            return Ok(None);
        }
        let bytes = std::fs::read(&full_path).map_err(|source| WorkspaceError::Io {
            path: full_path,
            source,
        })?;
        Ok(Some(bytes))
    }

    // ── Pinning ────────────────────────────────────────────────────────

    /// Path for pinned step files: `{mount}/workflows/{wf_id}/pinned/{step_id}/`.
    ///
    /// Lives outside `runs/` so it survives run cleanup.
    pub fn pinned_step_path(&self, workflow_id: Uuid, step_id: Uuid) -> PathBuf {
        self.workflow_path(workflow_id)
            .join("pinned")
            .join(step_id.to_string())
    }

    /// Capture a pinned step's workspace files from a run.
    ///
    /// Reads the overlay manifest (`.nexor/step-manifests/{step_id}.json`) to
    /// identify which files this step produced, then copies them to the pinned
    /// location. Returns the count of files captured.
    pub fn capture_pinned_files(
        &self,
        workflow_id: Uuid,
        run_id: Uuid,
        step_id: Uuid,
    ) -> Result<usize, WorkspaceError> {
        let manifest_path = PathBuf::from(format!(".nexor/step-manifests/{}.json", step_id));
        let manifest_bytes = match self.read_file(workflow_id, run_id, &manifest_path)? {
            Some(b) => b,
            None => return Ok(0), // No manifest — step produced no files
        };
        let file_paths: Vec<String> = serde_json::from_slice(&manifest_bytes).unwrap_or_default();

        let pinned_dir = self.pinned_step_path(workflow_id, step_id);
        // Clear previous pinned files if any
        if pinned_dir.exists() {
            std::fs::remove_dir_all(&pinned_dir).map_err(|source| WorkspaceError::Io {
                path: pinned_dir.clone(),
                source,
            })?;
        }

        let run_root = self.run_workspace_path(workflow_id, run_id);
        let mut count = 0;
        for rel in &file_paths {
            let src = run_root.join(rel);
            if !src.exists() {
                continue;
            }
            let dst = pinned_dir.join(rel);
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent).map_err(|source| WorkspaceError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            std::fs::copy(&src, &dst).map_err(|source| WorkspaceError::Io {
                path: src.clone(),
                source,
            })?;
            count += 1;
        }
        Ok(count)
    }

    /// Pre-load pinned step files into a run workspace.
    ///
    /// Copies all files from `pinned/{step_id}/` to `runs/{run_id}/`.
    /// Returns the count of files pre-loaded.
    pub fn preload_pinned_files(
        &self,
        workflow_id: Uuid,
        run_id: Uuid,
        step_id: Uuid,
    ) -> Result<usize, WorkspaceError> {
        let pinned_dir = self.pinned_step_path(workflow_id, step_id);
        if !pinned_dir.exists() {
            return Ok(0);
        }

        let mut files = Vec::new();
        collect_files_recursive(&pinned_dir, &pinned_dir, &mut files)?;

        let mut count = 0;
        for rel in &files {
            let src = pinned_dir.join(rel);
            let content = std::fs::read(&src).map_err(|source| WorkspaceError::Io {
                path: src.clone(),
                source,
            })?;
            self.write_file(workflow_id, run_id, rel, &content)?;
            count += 1;
        }
        Ok(count)
    }

    /// Remove pinned step files (on unpin).
    pub fn remove_pinned_files(
        &self,
        workflow_id: Uuid,
        step_id: Uuid,
    ) -> Result<(), WorkspaceError> {
        let pinned_dir = self.pinned_step_path(workflow_id, step_id);
        if pinned_dir.exists() {
            std::fs::remove_dir_all(&pinned_dir).map_err(|source| WorkspaceError::Io {
                path: pinned_dir,
                source,
            })?;
        }
        Ok(())
    }

    // ── Merge support ────────────────────────────────────────────────

    /// Read base file contents for specific paths (used for three-way merge).
    ///
    /// Only reads files in `paths_needed` — not the entire workspace.
    /// Skips files that don't exist (they may have been deleted).
    pub fn read_base_files(
        &self,
        workflow_id: Uuid,
        run_id: Uuid,
        paths_needed: &HashSet<PathBuf>,
    ) -> Result<HashMap<PathBuf, Vec<u8>>, WorkspaceError> {
        let mut result = HashMap::with_capacity(paths_needed.len());
        for path in paths_needed {
            if let Some(content) = self.read_file(workflow_id, run_id, path)? {
                result.insert(path.clone(), content);
            }
        }
        Ok(result)
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
