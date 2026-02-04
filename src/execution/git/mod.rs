//! Git operations with audit logging

use crate::execution::ExecutionContext;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitError {
    #[error("not a git repository: {path}")]
    NotARepo { path: PathBuf },

    #[error("git command failed: {command}\n{stderr}")]
    CommandFailed { command: String, stderr: String },

    #[error("failed to execute git: {0}")]
    ExecutionError(#[from] std::io::Error),

    #[error("failed to parse git output: {reason}")]
    ParseError { reason: String },

    #[error("operation not allowed: {reason}")]
    NotAllowed { reason: String },
}

#[derive(Debug, Clone, Default)]
pub struct GitStatus {
    /// Files with staged changes
    pub staged: Vec<FileChange>,
    /// Files with unstaged changes
    pub unstaged: Vec<FileChange>,
    /// Untracked files
    pub untracked: Vec<PathBuf>,
    /// Current branch name (None if detached HEAD)
    pub branch: Option<String>,
    /// Whether there are uncommitted changes
    pub is_dirty: bool,
}

#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: PathBuf,
    pub change_type: ChangeType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Unknown,
}

/// Information about a newly created branch
#[derive(Debug, Clone)]
pub struct BranchInfo {
    /// Name of the created branch
    pub name: String,
    /// Branch this was created from (parent branch)
    pub parent_branch: Option<String>,
    /// Commit SHA at branch creation
    pub base_commit: String,
}

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct DiffOptions {
    /// Only show diff for these paths
    pub paths: Vec<String>,
    /// Show staged changes (--cached)
    pub staged: bool,
    /// Compare against specific commit
    pub base_commit: Option<String>,
    /// Context lines around changes
    pub context_lines: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct PushResult {
    pub remote: String,
    pub branch: String,
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct PushOptions {
    pub remote: String,
    pub branch: Option<String>,
    pub set_upstream: bool,
    pub force: bool,
}

impl Default for PushOptions {
    fn default() -> Self {
        Self {
            remote: "origin".to_string(),
            branch: None,
            set_upstream: false,
            force: false,
        }
    }
}

// ============================================================================
// Merge Operations (Ticket 7.6)
// ============================================================================

#[derive(Debug, Clone)]
pub struct FetchResult {
    /// Remote that was fetched from
    pub remote: String,
    /// Refs that were updated
    pub updated_refs: Vec<String>,
    /// Whether any refs were updated
    pub had_updates: bool,
}

#[derive(Debug, Clone)]
pub enum MergeResult {
    /// Merge completed successfully
    Success {
        /// Commit hash of the merge commit (if not fast-forward)
        merge_commit: Option<String>,
        /// Whether it was a fast-forward merge
        fast_forward: bool,
    },
    /// Merge has conflicts that need resolution
    Conflict {
        /// Files with conflicts
        conflicting_files: Vec<PathBuf>,
    },
    /// Merge failed for other reasons
    Failed { reason: String },
}

impl MergeResult {
    pub fn is_success(&self) -> bool {
        matches!(self, MergeResult::Success { .. })
    }

    pub fn has_conflicts(&self) -> bool {
        matches!(self, MergeResult::Conflict { .. })
    }
}

#[derive(Debug, Clone)]
pub struct ConflictInfo {
    /// Path to the conflicting file
    pub path: PathBuf,
    /// Conflict regions in the file
    pub regions: Vec<ConflictRegion>,
}

#[derive(Debug, Clone)]
pub struct ConflictRegion {
    /// Line number where conflict starts (1-indexed)
    pub start_line: usize,
    /// Line number where conflict ends (1-indexed)
    pub end_line: usize,
    /// Content from "ours" (current branch)
    pub ours: String,
    /// Content from "theirs" (merging branch)
    pub theirs: String,
    /// Content from common ancestor (if 3-way merge)
    pub base: Option<String>,
}

/// Strategy for resolving a conflict
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Accept our version (current branch)
    Ours,
    /// Accept their version (merging branch)
    Theirs,
}

pub struct GitOps {
    ctx: ExecutionContext,
}

impl GitOps {
    pub fn new(ctx: ExecutionContext) -> Self {
        Self { ctx }
    }

    /// Get the current git status
    pub fn status(&self) -> Result<GitStatus, GitError> {
        self.ensure_git_repo()?;

        // Get porcelain status for parsing
        let output = self.run_git(&["status", "--porcelain=v1", "-b"])?;

        let mut status = GitStatus::default();

        for line in output.lines() {
            if line.starts_with("## ") {
                // Branch line: ## main...origin/main
                status.branch = self.parse_branch_line(line);
            } else if line.len() >= 3 {
                // File status line: XY filename
                let index_status = line.chars().next().unwrap_or(' ');
                let work_status = line.chars().nth(1).unwrap_or(' ');
                let path = PathBuf::from(line[3..].trim());

                // Staged changes (index)
                if index_status != ' ' && index_status != '?' {
                    status.staged.push(FileChange {
                        path: path.clone(),
                        change_type: Self::parse_change_type(index_status),
                    });
                }

                // Unstaged changes (working tree)
                if work_status != ' ' && work_status != '?' {
                    status.unstaged.push(FileChange {
                        path: path.clone(),
                        change_type: Self::parse_change_type(work_status),
                    });
                }

                // Untracked
                if index_status == '?' && work_status == '?' {
                    status.untracked.push(path);
                }
            }
        }

        status.is_dirty = !status.staged.is_empty()
            || !status.unstaged.is_empty()
            || !status.untracked.is_empty();

        tracing::debug!(
            branch = ?status.branch,
            staged = status.staged.len(),
            unstaged = status.unstaged.len(),
            untracked = status.untracked.len(),
            "Git status retrieved"
        );

        Ok(status)
    }

    fn parse_branch_line(&self, line: &str) -> Option<String> {
        // ## main...origin/main or ## HEAD (no branch)
        let branch_part = line.strip_prefix("## ")?;

        if branch_part.starts_with("HEAD") {
            return None; // Detached HEAD
        }

        // Split on ... to get local branch
        let branch = branch_part.split("...").next()?;
        Some(branch.to_string())
    }

    fn parse_change_type(c: char) -> ChangeType {
        match c {
            'A' => ChangeType::Added,
            'M' => ChangeType::Modified,
            'D' => ChangeType::Deleted,
            'R' => ChangeType::Renamed,
            'C' => ChangeType::Copied,
            _ => ChangeType::Unknown,
        }
    }

    /// Create a new branch from current HEAD
    pub fn create_branch(&self, name: &str) -> Result<BranchInfo, GitError> {
        self.ensure_git_repo()?;
        self.validate_branch_name(name)?;

        // Capture parent branch before creating new one
        let parent_branch = self.current_branch()?;
        let base_commit = self.run_git(&["rev-parse", "HEAD"])?.trim().to_string();

        self.run_git(&["branch", name])?;

        tracing::info!(
            branch = %name,
            parent = ?parent_branch,
            "Created branch"
        );

        Ok(BranchInfo {
            name: name.to_string(),
            parent_branch,
            base_commit,
        })
    }

    /// Create and checkout a new branch, returning info about parent branch
    pub fn create_and_checkout_branch(&self, name: &str) -> Result<BranchInfo, GitError> {
        self.ensure_git_repo()?;
        self.validate_branch_name(name)?;

        // Capture parent branch before creating new one
        let parent_branch = self.current_branch()?;
        let base_commit = self.run_git(&["rev-parse", "HEAD"])?.trim().to_string();

        self.run_git(&["checkout", "-b", name])?;

        tracing::info!(
            branch = %name,
            parent = ?parent_branch,
            "Created and checked out branch"
        );

        Ok(BranchInfo {
            name: name.to_string(),
            parent_branch,
            base_commit,
        })
    }

    /// Checkout an existing branch
    pub fn checkout_branch(&self, name: &str) -> Result<(), GitError> {
        self.ensure_git_repo()?;
        self.validate_branch_name(name)?;

        self.run_git(&["checkout", name])?;

        tracing::info!(branch = %name, "Checked out branch");
        Ok(())
    }

    /// Get the current branch name
    pub fn current_branch(&self) -> Result<Option<String>, GitError> {
        self.ensure_git_repo()?;

        let output = self.run_git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
        let branch = output.trim();

        if branch == "HEAD" {
            Ok(None) // Detached HEAD
        } else {
            Ok(Some(branch.to_string()))
        }
    }

    /// List all local branches
    pub fn list_branches(&self) -> Result<Vec<String>, GitError> {
        self.ensure_git_repo()?;

        let output = self.run_git(&["branch", "--list", "--format=%(refname:short)"])?;

        Ok(output
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect())
    }

    /// Delete a branch (only if merged)
    pub fn delete_branch(&self, name: &str) -> Result<(), GitError> {
        self.ensure_git_repo()?;
        self.validate_branch_name(name)?;

        // Don't allow deleting current branch
        if self.current_branch()?.as_deref() == Some(name) {
            return Err(GitError::NotAllowed {
                reason: "Cannot delete current branch".to_string(),
            });
        }

        // Use -d (not -D) to only delete if merged
        self.run_git(&["branch", "-d", name])?;

        tracing::info!(branch = %name, "Deleted branch");
        Ok(())
    }

    /// Stage files for commit
    pub fn add_files(&self, paths: &[&str]) -> Result<(), GitError> {
        self.ensure_git_repo()?;

        if paths.is_empty() {
            return Err(GitError::NotAllowed {
                reason: "No files specified to add".to_string(),
            });
        }

        let mut args = vec!["add", "--"];
        args.extend(paths);

        self.run_git(&args)?;

        tracing::debug!(files = ?paths, "Staged files");
        Ok(())
    }

    /// Stage all changes (modified and new files)
    pub fn add_all(&self) -> Result<(), GitError> {
        self.ensure_git_repo()?;

        self.run_git(&["add", "-A"])?;

        tracing::debug!("Staged all changes");
        Ok(())
    }

    /// Create a commit with the staged changes
    pub fn commit(&self, message: &str) -> Result<CommitInfo, GitError> {
        self.ensure_git_repo()?;
        self.validate_commit_message(message)?;

        // Check if there's anything to commit
        let status = self.status()?;
        if status.staged.is_empty() {
            return Err(GitError::NotAllowed {
                reason: "Nothing staged to commit".to_string(),
            });
        }

        self.run_git(&["commit", "-m", message])?;

        // Get the commit info
        let hash = self.run_git(&["rev-parse", "HEAD"])?.trim().to_string();
        let short_hash = self
            .run_git(&["rev-parse", "--short", "HEAD"])?
            .trim()
            .to_string();

        let info = CommitInfo {
            hash,
            short_hash,
            message: message.to_string(),
        };

        tracing::info!(
            commit = %info.short_hash,
            message = %message,
            "Created commit"
        );

        Ok(info)
    }

    /// Amend the last commit (use with caution)
    pub fn amend_commit(&self, message: Option<&str>) -> Result<CommitInfo, GitError> {
        self.ensure_git_repo()?;

        let args = match message {
            Some(msg) => {
                self.validate_commit_message(msg)?;
                vec!["commit", "--amend", "-m", msg]
            }
            None => vec!["commit", "--amend", "--no-edit"],
        };

        self.run_git(&args)?;

        let hash = self.run_git(&["rev-parse", "HEAD"])?.trim().to_string();
        let short_hash = self
            .run_git(&["rev-parse", "--short", "HEAD"])?
            .trim()
            .to_string();
        let final_message = self
            .run_git(&["log", "-1", "--format=%s"])?
            .trim()
            .to_string();

        let info = CommitInfo {
            hash,
            short_hash,
            message: final_message,
        };

        tracing::info!(commit = %info.short_hash, "Amended commit");
        Ok(info)
    }

    /// Get diff of unstaged changes
    pub fn diff(&self) -> Result<String, GitError> {
        self.diff_with_options(DiffOptions::default())
    }

    /// Get diff of staged changes
    pub fn diff_staged(&self) -> Result<String, GitError> {
        self.diff_with_options(DiffOptions {
            staged: true,
            ..Default::default()
        })
    }

    /// Get diff with custom options
    pub fn diff_with_options(&self, options: DiffOptions) -> Result<String, GitError> {
        self.ensure_git_repo()?;

        let mut args = vec!["diff"];

        if options.staged {
            args.push("--cached");
        }

        if let Some(ref base) = options.base_commit {
            args.push(base);
        }

        args.push("--");

        for path in &options.paths {
            args.push(path);
        }

        self.run_git(&args)
    }

    /// Get diff for a specific commit
    pub fn diff_commit(&self, commit: &str) -> Result<String, GitError> {
        self.ensure_git_repo()?;

        // Show changes introduced by this commit
        self.run_git(&["show", "--format=", commit])
    }

    /// Get list of files changed between two refs
    pub fn diff_files(&self, base: &str, head: &str) -> Result<Vec<String>, GitError> {
        self.ensure_git_repo()?;

        let output = self.run_git(&["diff", "--name-only", base, head])?;

        Ok(output
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect())
    }

    /// Get a summary of changes (stat)
    pub fn diff_stat(&self) -> Result<String, GitError> {
        self.ensure_git_repo()?;
        self.run_git(&["diff", "--stat"])
    }

    /// Push current branch to origin
    pub fn push(&self) -> Result<PushResult, GitError> {
        self.push_with_options(PushOptions::default())
    }

    /// Push with custom options
    pub fn push_with_options(&self, options: PushOptions) -> Result<PushResult, GitError> {
        self.ensure_git_repo()?;

        // Safety: never allow force push
        if options.force {
            return Err(GitError::NotAllowed {
                reason: "Force push is not allowed".to_string(),
            });
        }

        let current_branch = self.current_branch()?.ok_or_else(|| GitError::NotAllowed {
            reason: "Cannot push in detached HEAD state".to_string(),
        })?;

        let branch = options.branch.unwrap_or_else(|| current_branch.clone());

        let mut args = vec!["push"];

        if options.set_upstream {
            args.push("-u");
        }

        args.push(&options.remote);
        args.push(&branch);

        match self.run_git(&args) {
            Ok(output) => {
                tracing::info!(
                    remote = %options.remote,
                    branch = %branch,
                    "Pushed successfully"
                );

                Ok(PushResult {
                    remote: options.remote,
                    branch,
                    success: true,
                    message: output,
                })
            }
            Err(GitError::CommandFailed { stderr, .. }) => {
                // Parse common push failures
                if stderr.contains("Authentication failed")
                    || stderr.contains("could not read Username")
                {
                    Err(GitError::NotAllowed {
                        reason: "Authentication required for push".to_string(),
                    })
                } else if stderr.contains("rejected") {
                    Err(GitError::NotAllowed {
                        reason: "Push rejected - remote has changes. Pull first.".to_string(),
                    })
                } else {
                    Err(GitError::CommandFailed {
                        command: format!("git push {} {}", options.remote, branch),
                        stderr,
                    })
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Pull changes from remote
    pub fn pull(&self) -> Result<String, GitError> {
        self.ensure_git_repo()?;
        self.run_git(&["pull"])
    }

    /// Fetch from remote without merging
    pub fn fetch(&self, remote: &str) -> Result<(), GitError> {
        self.ensure_git_repo()?;
        self.run_git(&["fetch", remote])?;
        tracing::debug!(remote = %remote, "Fetched");
        Ok(())
    }

    // ========================================================================
    // Merge Operations (Ticket 7.6)
    // ========================================================================

    /// Fetch from a remote with result info
    pub fn fetch_remote(&self, remote: &str) -> Result<FetchResult, GitError> {
        self.ensure_git_repo()?;

        let output = self.run_git(&["fetch", remote, "--prune"])?;

        let updated_refs: Vec<String> = output
            .lines()
            .filter(|l| l.contains("->"))
            .map(|l| l.trim().to_string())
            .collect();

        tracing::info!(
            remote = %remote,
            updated = updated_refs.len(),
            "Fetched from remote"
        );

        Ok(FetchResult {
            remote: remote.to_string(),
            updated_refs: updated_refs.clone(),
            had_updates: !updated_refs.is_empty(),
        })
    }

    /// Fetch a specific refspec from remote
    pub fn fetch_refspec(&self, remote: &str, refspec: &str) -> Result<FetchResult, GitError> {
        self.ensure_git_repo()?;

        self.run_git(&["fetch", remote, refspec])?;

        tracing::info!(
            remote = %remote,
            refspec = %refspec,
            "Fetched refspec"
        );

        Ok(FetchResult {
            remote: remote.to_string(),
            updated_refs: vec![refspec.to_string()],
            had_updates: true,
        })
    }

    /// Fetch a PR branch from GitHub (uses refs/pull/{number}/head format)
    pub fn fetch_pr(&self, remote: &str, pr_number: u32) -> Result<String, GitError> {
        self.ensure_git_repo()?;

        let local_branch = format!("pr-{}", pr_number);
        let refspec = format!("refs/pull/{}/head:{}", pr_number, local_branch);

        self.run_git(&["fetch", remote, &refspec])?;

        tracing::info!(
            remote = %remote,
            pr_number = pr_number,
            local_branch = %local_branch,
            "Fetched PR branch"
        );

        Ok(local_branch)
    }

    /// Attempt to merge a branch into the current branch
    pub fn merge(&self, branch: &str) -> Result<MergeResult, GitError> {
        self.ensure_git_repo()?;

        // Try to merge - capture both stdout and stderr
        let output = Command::new("git")
            .args(["merge", branch, "--no-edit"])
            .current_dir(&self.ctx.project_root)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            // Check if fast-forward
            let fast_forward = stdout.contains("Fast-forward");
            let merge_commit = if fast_forward {
                None
            } else {
                Some(self.run_git(&["rev-parse", "HEAD"])?.trim().to_string())
            };

            tracing::info!(
                branch = %branch,
                fast_forward = fast_forward,
                "Merge completed"
            );

            Ok(MergeResult::Success {
                merge_commit,
                fast_forward,
            })
        } else {
            // Check for conflicts (message can be in stdout or stderr)
            let combined = format!("{}{}", stdout, stderr);
            if combined.contains("CONFLICT") || combined.contains("Automatic merge failed") {
                // Get list of conflicting files
                let conflicting_files = self.get_conflicting_files()?;

                tracing::warn!(
                    branch = %branch,
                    conflicts = conflicting_files.len(),
                    "Merge has conflicts"
                );

                Ok(MergeResult::Conflict { conflicting_files })
            } else {
                Ok(MergeResult::Failed { reason: combined })
            }
        }
    }

    /// Get list of files with merge conflicts
    pub fn get_conflicting_files(&self) -> Result<Vec<PathBuf>, GitError> {
        self.ensure_git_repo()?;

        let output = self.run_git(&["diff", "--name-only", "--diff-filter=U"])?;

        Ok(output
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| PathBuf::from(l.trim()))
            .collect())
    }

    /// Check if we're in a merge state
    pub fn is_merging(&self) -> Result<bool, GitError> {
        self.ensure_git_repo()?;

        let merge_head = self.ctx.project_root.join(".git/MERGE_HEAD");
        Ok(merge_head.exists())
    }

    /// Pull from remote (fetch + merge)
    pub fn pull_from(&self, remote: &str, branch: &str) -> Result<MergeResult, GitError> {
        self.ensure_git_repo()?;

        // Fetch first
        self.fetch_remote(remote)?;

        // Then merge the remote branch
        let remote_branch = format!("{}/{}", remote, branch);
        self.merge(&remote_branch)
    }

    /// Get detailed conflict information for a file
    pub fn get_conflict_info(&self, path: &Path) -> Result<ConflictInfo, GitError> {
        self.ensure_git_repo()?;

        let full_path = self.ctx.project_root.join(path);
        let content = std::fs::read_to_string(&full_path).map_err(GitError::ExecutionError)?;

        let regions = self.parse_conflict_markers(&content)?;

        Ok(ConflictInfo {
            path: path.to_path_buf(),
            regions,
        })
    }

    /// Parse conflict markers from file content
    fn parse_conflict_markers(&self, content: &str) -> Result<Vec<ConflictRegion>, GitError> {
        let mut regions = Vec::new();
        let mut current_region: Option<ConflictRegionBuilder> = None;
        let lines: Vec<&str> = content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let line_num = i + 1; // 1-indexed

            if line.starts_with("<<<<<<<") {
                current_region = Some(ConflictRegionBuilder {
                    start_line: line_num,
                    ours_lines: Vec::new(),
                    theirs_lines: Vec::new(),
                    base_lines: None,
                    in_section: ConflictSection::Ours,
                });
            } else if line.starts_with("|||||||") {
                // 3-way merge: base section
                if let Some(ref mut region) = current_region {
                    region.base_lines = Some(Vec::new());
                    region.in_section = ConflictSection::Base;
                }
            } else if line.starts_with("=======") {
                if let Some(ref mut region) = current_region {
                    region.in_section = ConflictSection::Theirs;
                }
            } else if line.starts_with(">>>>>>>") {
                if let Some(region) = current_region.take() {
                    regions.push(ConflictRegion {
                        start_line: region.start_line,
                        end_line: line_num,
                        ours: region.ours_lines.join("\n"),
                        theirs: region.theirs_lines.join("\n"),
                        base: region.base_lines.map(|l| l.join("\n")),
                    });
                }
            } else if let Some(ref mut region) = current_region {
                match region.in_section {
                    ConflictSection::Ours => region.ours_lines.push((*line).to_string()),
                    ConflictSection::Base => {
                        if let Some(ref mut base) = region.base_lines {
                            base.push((*line).to_string());
                        }
                    }
                    ConflictSection::Theirs => region.theirs_lines.push((*line).to_string()),
                }
            }
        }

        Ok(regions)
    }

    /// Resolve a conflict by accepting one side
    pub fn resolve_conflict(
        &self,
        path: &Path,
        resolution: ConflictResolution,
    ) -> Result<(), GitError> {
        self.ensure_git_repo()?;

        let strategy = match resolution {
            ConflictResolution::Ours => "--ours",
            ConflictResolution::Theirs => "--theirs",
        };

        let path_str = path.to_string_lossy();
        self.run_git(&["checkout", strategy, "--", &path_str])?;
        self.mark_resolved(path)?;

        tracing::info!(
            path = %path_str,
            resolution = ?resolution,
            "Resolved conflict"
        );

        Ok(())
    }

    /// Resolve all conflicts by accepting one side
    pub fn resolve_all_conflicts(&self, resolution: ConflictResolution) -> Result<u32, GitError> {
        self.ensure_git_repo()?;

        let conflicts = self.get_conflicting_files()?;
        let count = conflicts.len() as u32;

        for path in conflicts {
            self.resolve_conflict(&path, resolution)?;
        }

        tracing::info!(
            count = count,
            resolution = ?resolution,
            "Resolved all conflicts"
        );

        Ok(count)
    }

    /// Resolve a conflict with custom content
    pub fn resolve_conflict_manual(&self, path: &Path, content: &str) -> Result<(), GitError> {
        self.ensure_git_repo()?;

        let full_path = self.ctx.project_root.join(path);
        std::fs::write(&full_path, content).map_err(GitError::ExecutionError)?;

        self.mark_resolved(path)?;

        tracing::info!(
            path = %path.display(),
            "Resolved conflict with custom content"
        );

        Ok(())
    }

    /// Mark a file as resolved (stage it)
    pub fn mark_resolved(&self, path: &Path) -> Result<(), GitError> {
        self.ensure_git_repo()?;

        let path_str = path.to_string_lossy();
        self.run_git(&["add", &path_str])?;

        Ok(())
    }

    /// Check if all conflicts are resolved
    pub fn all_conflicts_resolved(&self) -> Result<bool, GitError> {
        self.ensure_git_repo()?;

        let conflicts = self.get_conflicting_files()?;
        Ok(conflicts.is_empty())
    }

    /// Complete a merge after resolving conflicts
    pub fn complete_merge(&self) -> Result<CommitInfo, GitError> {
        self.ensure_git_repo()?;

        if !self.is_merging()? {
            return Err(GitError::NotAllowed {
                reason: "Not in a merge state".to_string(),
            });
        }

        if !self.all_conflicts_resolved()? {
            return Err(GitError::NotAllowed {
                reason: "Conflicts not resolved".to_string(),
            });
        }

        // Complete the merge with default message
        self.run_git(&["commit", "--no-edit"])?;

        let hash = self.run_git(&["rev-parse", "HEAD"])?.trim().to_string();
        let short_hash = self
            .run_git(&["rev-parse", "--short", "HEAD"])?
            .trim()
            .to_string();
        let message = self
            .run_git(&["log", "-1", "--format=%s"])?
            .trim()
            .to_string();

        tracing::info!(
            commit = %short_hash,
            "Merge completed"
        );

        Ok(CommitInfo {
            hash,
            short_hash,
            message,
        })
    }

    /// Abort an in-progress merge
    pub fn abort_merge(&self) -> Result<(), GitError> {
        self.ensure_git_repo()?;

        if !self.is_merging()? {
            return Err(GitError::NotAllowed {
                reason: "Not in a merge state".to_string(),
            });
        }

        self.run_git(&["merge", "--abort"])?;

        tracing::info!("Merge aborted");
        Ok(())
    }

    /// Hard reset to a ref (DESTRUCTIVE - requires confirmation)
    pub fn reset_hard(&self, target: &str, confirm: bool) -> Result<(), GitError> {
        self.ensure_git_repo()?;

        if !confirm {
            return Err(GitError::NotAllowed {
                reason: "Hard reset requires explicit confirmation".to_string(),
            });
        }

        tracing::warn!(
            target = %target,
            "Performing hard reset - all uncommitted changes will be lost"
        );

        self.run_git(&["reset", "--hard", target])?;

        tracing::info!(target = %target, "Hard reset completed");
        Ok(())
    }

    /// Discard all changes in working tree (DESTRUCTIVE - requires confirmation)
    pub fn clean_working_tree(&self, confirm: bool) -> Result<(), GitError> {
        self.ensure_git_repo()?;

        if !confirm {
            return Err(GitError::NotAllowed {
                reason: "Clean requires explicit confirmation".to_string(),
            });
        }

        tracing::warn!("Cleaning working tree - all uncommitted changes will be lost");

        // Reset staged changes
        self.run_git(&["reset", "HEAD"])?;

        // Discard working tree changes
        self.run_git(&["checkout", "--", "."])?;

        // Remove untracked files
        self.run_git(&["clean", "-fd"])?;

        tracing::info!("Working tree cleaned");
        Ok(())
    }

    /// Soft reset (keeps changes staged)
    pub fn reset_soft(&self, target: &str) -> Result<(), GitError> {
        self.ensure_git_repo()?;

        self.run_git(&["reset", "--soft", target])?;

        tracing::info!(target = %target, "Soft reset completed");
        Ok(())
    }

    /// Mixed reset (keeps changes unstaged) - default git behavior
    pub fn reset(&self, target: &str) -> Result<(), GitError> {
        self.ensure_git_repo()?;

        self.run_git(&["reset", target])?;

        tracing::info!(target = %target, "Reset completed");
        Ok(())
    }

    /// Check if there are commits to push
    pub fn has_unpushed_commits(&self) -> Result<bool, GitError> {
        self.ensure_git_repo()?;

        let current = self.current_branch()?.ok_or_else(|| GitError::NotAllowed {
            reason: "Cannot check in detached HEAD state".to_string(),
        })?;

        // Try to get the tracking branch
        let upstream =
            match self.run_git(&["rev-parse", "--abbrev-ref", &format!("{}@{{u}}", current)]) {
                Ok(u) => u.trim().to_string(),
                Err(_) => return Ok(false), // No upstream set
            };

        // Count commits ahead
        let output = self.run_git(&["rev-list", "--count", &format!("{}..HEAD", upstream)])?;

        let count: u32 = output.trim().parse().unwrap_or(0);
        Ok(count > 0)
    }

    fn ensure_git_repo(&self) -> Result<(), GitError> {
        let git_dir = self.ctx.project_root.join(".git");
        if !git_dir.exists() {
            return Err(GitError::NotARepo {
                path: self.ctx.project_root.clone(),
            });
        }
        Ok(())
    }

    fn run_git(&self, args: &[&str]) -> Result<String, GitError> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.ctx.project_root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(GitError::CommandFailed {
                command: format!("git {}", args.join(" ")),
                stderr,
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Validate a branch name
    fn validate_branch_name(&self, name: &str) -> Result<(), GitError> {
        // Git branch name rules (simplified)
        if name.is_empty() {
            return Err(GitError::NotAllowed {
                reason: "Branch name cannot be empty".to_string(),
            });
        }

        if name.starts_with('-') {
            return Err(GitError::NotAllowed {
                reason: "Branch name cannot start with -".to_string(),
            });
        }

        // Check for invalid characters
        let invalid_chars = ['~', '^', ':', '\\', ' ', '\t', '\n', '?', '*', '['];
        for c in invalid_chars {
            if name.contains(c) {
                return Err(GitError::NotAllowed {
                    reason: format!("Branch name cannot contain '{}'", c),
                });
            }
        }

        // Check for invalid patterns
        if name.contains("..") || name.contains("@{") || name.ends_with('.') || name.ends_with('/')
        {
            return Err(GitError::NotAllowed {
                reason: "Invalid branch name pattern".to_string(),
            });
        }

        Ok(())
    }

    fn validate_commit_message(&self, message: &str) -> Result<(), GitError> {
        if message.trim().is_empty() {
            return Err(GitError::NotAllowed {
                reason: "Commit message cannot be empty".to_string(),
            });
        }

        // Check first line length (conventional: <=72 chars)
        let first_line = message.lines().next().unwrap_or("");
        if first_line.len() > 100 {
            tracing::warn!(
                length = first_line.len(),
                "Commit message first line is long (>100 chars)"
            );
        }

        Ok(())
    }
}

// Helper types for conflict parsing
#[derive(Debug)]
enum ConflictSection {
    Ours,
    Base,
    Theirs,
}

struct ConflictRegionBuilder {
    start_line: usize,
    ours_lines: Vec<String>,
    theirs_lines: Vec<String>,
    base_lines: Option<Vec<String>>,
    in_section: ConflictSection,
}

mod tests;
