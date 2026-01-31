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

#[derive(Debug, Clone)]
#[derive(Default)]
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
        let content =
            std::fs::read_to_string(&full_path).map_err(GitError::ExecutionError)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn init_git_repo(dir: &TempDir) {
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir.path())
            .output()
            .unwrap();
    }

    #[test]
    fn status_empty_repo() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        let status = git.status().unwrap();
        assert!(!status.is_dirty);
    }

    #[test]
    fn status_with_untracked_file() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);
        std::fs::write(tmp.path().join("new_file.txt"), "content").unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        let status = git.status().unwrap();
        assert!(status.is_dirty);
        assert_eq!(status.untracked.len(), 1);
    }

    #[test]
    fn create_and_checkout_branch() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);

        // Need at least one commit
        std::fs::write(tmp.path().join("file.txt"), "content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        let branch_info = git.create_and_checkout_branch("feature/test").unwrap();
        assert_eq!(
            git.current_branch().unwrap(),
            Some("feature/test".to_string())
        );
        // Verify parent branch is tracked
        assert!(branch_info.parent_branch.is_some());
        assert_eq!(branch_info.name, "feature/test");
        assert!(!branch_info.base_commit.is_empty());
    }

    #[test]
    fn invalid_branch_name_rejected() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        assert!(git.validate_branch_name("valid-name").is_ok());
        assert!(git.validate_branch_name("feature/foo").is_ok());
        assert!(git.validate_branch_name("-invalid").is_err());
        assert!(git.validate_branch_name("has space").is_err());
        assert!(git.validate_branch_name("has..dots").is_err());
    }

    #[test]
    fn commit_with_staged_changes() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);

        // Create and stage a file
        std::fs::write(tmp.path().join("file.txt"), "content").unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        git.add_files(&["file.txt"]).unwrap();
        let commit = git.commit("test: add file").unwrap();

        assert!(!commit.hash.is_empty());
        assert!(!commit.short_hash.is_empty());
    }

    #[test]
    fn commit_without_staged_changes_fails() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        let result = git.commit("empty commit");
        assert!(matches!(result, Err(GitError::NotAllowed { .. })));
    }

    #[test]
    fn diff_shows_unstaged_changes() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);

        // Create, commit, then modify
        std::fs::write(tmp.path().join("file.txt"), "original").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        std::fs::write(tmp.path().join("file.txt"), "modified").unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        let diff = git.diff().unwrap();
        assert!(diff.contains("-original"));
        assert!(diff.contains("+modified"));
    }

    #[test]
    fn diff_staged_shows_cached_changes() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);

        // Create and commit
        std::fs::write(tmp.path().join("file.txt"), "original").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        // Modify and stage
        std::fs::write(tmp.path().join("file.txt"), "modified").unwrap();
        Command::new("git")
            .args(["add", "file.txt"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        let diff = git.diff_staged().unwrap();
        assert!(diff.contains("-original"));
        assert!(diff.contains("+modified"));

        // Unstaged diff should be empty
        let unstaged = git.diff().unwrap();
        assert!(unstaged.trim().is_empty());
    }

    #[test]
    fn force_push_rejected() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        let result = git.push_with_options(PushOptions {
            force: true,
            ..Default::default()
        });

        assert!(matches!(result, Err(GitError::NotAllowed { .. })));
    }

    // ========================================================================
    // Merge Operation Tests (Ticket 7.6)
    // ========================================================================

    fn get_default_branch(dir: &TempDir) -> String {
        let output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn create_conflicting_branches(dir: &TempDir) {
        // Create a file and commit on main
        std::fs::write(dir.path().join("file.txt"), "main content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "main commit"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // Get the default branch name (may be master or main)
        let default_branch = get_default_branch(dir);

        // Create branch with different content
        Command::new("git")
            .args(["checkout", "-b", "feature"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        std::fs::write(dir.path().join("file.txt"), "feature content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "feature commit"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // Go back to main/master and make conflicting change
        Command::new("git")
            .args(["checkout", &default_branch])
            .current_dir(dir.path())
            .output()
            .unwrap();
        std::fs::write(dir.path().join("file.txt"), "different main content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "main conflict"])
            .current_dir(dir.path())
            .output()
            .unwrap();
    }

    #[test]
    fn merge_detects_conflicts() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);
        create_conflicting_branches(&tmp);

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        let result = git.merge("feature").unwrap();
        assert!(result.has_conflicts());

        if let MergeResult::Conflict { conflicting_files } = result {
            assert!(conflicting_files.contains(&PathBuf::from("file.txt")));
        }
    }

    #[test]
    fn merge_fast_forward() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);

        // Create initial commit
        std::fs::write(tmp.path().join("file.txt"), "content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        // Get default branch name
        let default_branch = get_default_branch(&tmp);

        // Create branch with new commit
        Command::new("git")
            .args(["checkout", "-b", "feature"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        std::fs::write(tmp.path().join("new.txt"), "new").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "feature"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        // Go back to main
        Command::new("git")
            .args(["checkout", &default_branch])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        let result = git.merge("feature").unwrap();
        assert!(result.is_success());

        if let MergeResult::Success { fast_forward, .. } = result {
            assert!(fast_forward);
        }
    }

    #[test]
    fn abort_merge_cancels() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);
        create_conflicting_branches(&tmp);

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        // Create conflict
        let result = git.merge("feature").unwrap();
        assert!(result.has_conflicts());
        assert!(git.is_merging().unwrap());

        // Abort
        git.abort_merge().unwrap();
        assert!(!git.is_merging().unwrap());
    }

    #[test]
    fn reset_hard_requires_confirmation() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);

        // Create commit
        std::fs::write(tmp.path().join("file.txt"), "content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        // Without confirmation, should fail
        let result = git.reset_hard("HEAD~1", false);
        assert!(result.is_err());

        // With confirmation, should work
        let result = git.reset_hard("HEAD", true);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_simple_conflict() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);

        let conflict_content = r#"some code
<<<<<<< HEAD
our changes
=======
their changes
>>>>>>> feature
more code"#;

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        let regions = git.parse_conflict_markers(conflict_content).unwrap();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].ours, "our changes");
        assert_eq!(regions[0].theirs, "their changes");
        assert!(regions[0].base.is_none());
    }

    #[test]
    fn parse_3way_conflict() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);

        let conflict_content = r#"<<<<<<< HEAD
our version
||||||| merged common ancestors
original version
=======
their version
>>>>>>> feature"#;

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        let regions = git.parse_conflict_markers(conflict_content).unwrap();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].ours, "our version");
        assert_eq!(regions[0].theirs, "their version");
        assert_eq!(regions[0].base.as_ref().unwrap(), "original version");
    }

    #[test]
    fn resolve_conflict_ours() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);
        create_conflicting_branches(&tmp);

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        // Create conflict
        let result = git.merge("feature").unwrap();
        assert!(result.has_conflicts());

        // Resolve with ours
        git.resolve_conflict(Path::new("file.txt"), ConflictResolution::Ours)
            .unwrap();

        // Should be resolved
        assert!(git.all_conflicts_resolved().unwrap());

        // Complete merge
        let commit = git.complete_merge().unwrap();
        assert!(!commit.hash.is_empty());
    }

    fn setup_test_repo() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_path_buf();
        Command::new("git")
            .args(["init"])
            .current_dir(&repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(&repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(&repo_path)
            .output()
            .unwrap();
        (temp_dir, repo_path)
    }

    fn make_initial_commit(repo_path: &Path) {
        std::fs::write(repo_path.join("init.txt"), "init").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(repo_path)
            .output()
            .unwrap();
    }

    #[test]
    fn current_branch_returns_default() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let branch = git.current_branch().unwrap();
        assert!(branch.is_some());
        // Default branch is either "main" or "master"
        let name = branch.unwrap();
        assert!(name == "main" || name == "master");
    }

    #[test]
    fn list_branches_includes_created() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        git.create_branch("branch-a").unwrap();
        git.create_branch("branch-b").unwrap();

        let branches = git.list_branches().unwrap();
        assert!(branches.contains(&"branch-a".to_string()));
        assert!(branches.contains(&"branch-b".to_string()));
        assert!(branches.len() >= 3); // default + a + b
    }

    #[test]
    fn delete_branch_removes_it() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        git.create_branch("to-delete").unwrap();

        let branches_before = git.list_branches().unwrap();
        assert!(branches_before.contains(&"to-delete".to_string()));

        git.delete_branch("to-delete").unwrap();

        let branches_after = git.list_branches().unwrap();
        assert!(!branches_after.contains(&"to-delete".to_string()));
    }

    #[test]
    fn delete_current_branch_fails() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let current = git.current_branch().unwrap().unwrap();
        let result = git.delete_branch(&current);
        assert!(matches!(result, Err(GitError::NotAllowed { .. })));
    }

    #[test]
    fn add_files_stages_specific_files() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        std::fs::write(repo_path.join("a.txt"), "a").unwrap();
        std::fs::write(repo_path.join("b.txt"), "b").unwrap();

        let git = GitOps::new(ExecutionContext::new(repo_path));
        git.add_files(&["a.txt"]).unwrap();

        let status = git.status().unwrap();
        assert_eq!(status.staged.len(), 1);
        assert_eq!(status.staged[0].path, PathBuf::from("a.txt"));
        // b.txt should still be untracked
        assert!(status.untracked.contains(&PathBuf::from("b.txt")));
    }

    #[test]
    fn add_files_empty_fails() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.add_files(&[]);
        assert!(matches!(result, Err(GitError::NotAllowed { .. })));
    }

    #[test]
    fn add_all_stages_everything() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        std::fs::write(repo_path.join("a.txt"), "a").unwrap();
        std::fs::write(repo_path.join("b.txt"), "b").unwrap();

        let git = GitOps::new(ExecutionContext::new(repo_path));
        git.add_all().unwrap();

        let status = git.status().unwrap();
        assert_eq!(status.staged.len(), 2);
        assert!(status.untracked.is_empty());
    }

    #[test]
    fn diff_stat_shows_changes() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        // Modify tracked file
        std::fs::write(repo_path.join("init.txt"), "modified").unwrap();

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let stat = git.diff_stat().unwrap();
        assert!(stat.contains("init.txt"));
    }

    #[test]
    fn diff_files_between_commits() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path.clone()));

        // Get first commit hash
        let first = git
            .run_git(&["rev-parse", "HEAD"])
            .unwrap()
            .trim()
            .to_string();

        // Make second commit
        std::fs::write(repo_path.join("new.txt"), "new").unwrap();
        git.add_all().unwrap();
        git.commit("add new file").unwrap();

        let second = git
            .run_git(&["rev-parse", "HEAD"])
            .unwrap()
            .trim()
            .to_string();

        let files = git.diff_files(&first, &second).unwrap();
        assert!(files.contains(&"new.txt".to_string()));
    }

    #[test]
    fn amend_commit_changes_message() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let amended = git.amend_commit(Some("amended message")).unwrap();
        assert_eq!(amended.message, "amended message");
    }

    #[test]
    fn amend_commit_no_edit() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let amended = git.amend_commit(None).unwrap();
        assert_eq!(amended.message, "initial");
    }

    #[test]
    fn reset_soft_keeps_changes_staged() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path.clone()));

        // Make second commit
        std::fs::write(repo_path.join("second.txt"), "second").unwrap();
        git.add_all().unwrap();
        git.commit("second commit").unwrap();

        // Soft reset back one commit
        git.reset_soft("HEAD~1").unwrap();

        let status = git.status().unwrap();
        // File should be staged after soft reset
        assert!(!status.staged.is_empty());
        assert!(status
            .staged
            .iter()
            .any(|f| f.path == PathBuf::from("second.txt")));
    }

    #[test]
    fn reset_mixed_unstages_changes() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path.clone()));

        // Make second commit
        std::fs::write(repo_path.join("second.txt"), "second").unwrap();
        git.add_all().unwrap();
        git.commit("second commit").unwrap();

        // Mixed reset back one commit
        git.reset("HEAD~1").unwrap();

        let status = git.status().unwrap();
        // File should be untracked (not staged) after mixed reset
        assert!(status.staged.is_empty());
        assert!(status.untracked.contains(&PathBuf::from("second.txt")));
    }

    #[test]
    fn has_unpushed_commits_no_upstream() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        // No upstream configured, should return false
        let result = git.has_unpushed_commits().unwrap();
        assert!(!result);
    }

    #[test]
    fn parse_branch_line_with_tracking() {
        let (_tmp, repo_path) = setup_test_repo();
        let git = GitOps::new(ExecutionContext::new(repo_path));

        let result = git.parse_branch_line("## main...origin/main");
        assert_eq!(result, Some("main".to_string()));
    }

    #[test]
    fn parse_branch_line_no_tracking() {
        let (_tmp, repo_path) = setup_test_repo();
        let git = GitOps::new(ExecutionContext::new(repo_path));

        let result = git.parse_branch_line("## feature-branch");
        assert_eq!(result, Some("feature-branch".to_string()));
    }

    #[test]
    fn parse_branch_line_detached_head() {
        let (_tmp, repo_path) = setup_test_repo();
        let git = GitOps::new(ExecutionContext::new(repo_path));

        let result = git.parse_branch_line("## HEAD (no branch)");
        assert_eq!(result, None);
    }

    #[test]
    fn parse_change_type_all_variants() {
        assert_eq!(GitOps::parse_change_type('A'), ChangeType::Added);
        assert_eq!(GitOps::parse_change_type('M'), ChangeType::Modified);
        assert_eq!(GitOps::parse_change_type('D'), ChangeType::Deleted);
        assert_eq!(GitOps::parse_change_type('R'), ChangeType::Renamed);
        assert_eq!(GitOps::parse_change_type('C'), ChangeType::Copied);
        assert_eq!(GitOps::parse_change_type('X'), ChangeType::Unknown);
    }

    #[test]
    fn not_a_repo_error() {
        let tmp = TempDir::new().unwrap();
        // Don't init git
        let git = GitOps::new(ExecutionContext::new(tmp.path().to_path_buf()));
        let result = git.status();
        assert!(matches!(result, Err(GitError::NotARepo { .. })));
    }

    #[test]
    fn validate_commit_message_empty_fails() {
        let (_tmp, repo_path) = setup_test_repo();
        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.validate_commit_message("   ");
        assert!(matches!(result, Err(GitError::NotAllowed { .. })));
    }

    #[test]
    fn validate_branch_name_edge_cases() {
        let (_tmp, repo_path) = setup_test_repo();
        let git = GitOps::new(ExecutionContext::new(repo_path));

        assert!(git.validate_branch_name("").is_err());
        assert!(git.validate_branch_name("has~tilde").is_err());
        assert!(git.validate_branch_name("has^caret").is_err());
        assert!(git.validate_branch_name("has:colon").is_err());
        assert!(git.validate_branch_name("ends.").is_err());
        assert!(git.validate_branch_name("ends/").is_err());
        assert!(git.validate_branch_name("has@{ref").is_err());
        assert!(git.validate_branch_name("has[bracket").is_err());
        assert!(git.validate_branch_name("ok-name").is_ok());
    }

    #[test]
    fn status_with_staged_and_unstaged_changes() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        // Modify tracked file and stage it
        std::fs::write(repo_path.join("init.txt"), "modified").unwrap();
        Command::new("git")
            .args(["add", "init.txt"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        // Modify it again so there are both staged and unstaged changes
        std::fs::write(repo_path.join("init.txt"), "modified again").unwrap();

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let status = git.status().unwrap();

        assert!(status.is_dirty);
        assert!(!status.staged.is_empty());
        assert!(!status.unstaged.is_empty());
        assert_eq!(status.staged[0].change_type, ChangeType::Modified);
        assert_eq!(status.unstaged[0].change_type, ChangeType::Modified);
    }

    #[test]
    fn status_with_deleted_file() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        // Delete the tracked file and stage the deletion
        std::fs::remove_file(repo_path.join("init.txt")).unwrap();
        Command::new("git")
            .args(["add", "init.txt"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let status = git.status().unwrap();

        assert!(status.is_dirty);
        assert_eq!(status.staged.len(), 1);
        assert_eq!(status.staged[0].change_type, ChangeType::Deleted);
    }

    #[test]
    fn status_branch_name_populated() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let status = git.status().unwrap();
        assert!(status.branch.is_some());
    }

    #[test]
    fn checkout_branch_switches() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        git.create_branch("other").unwrap();
        git.checkout_branch("other").unwrap();

        assert_eq!(git.current_branch().unwrap(), Some("other".to_string()));
    }

    #[test]
    fn checkout_branch_invalid_name() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.checkout_branch("-bad");
        assert!(matches!(result, Err(GitError::NotAllowed { .. })));
    }

    #[test]
    fn create_branch_invalid_name() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.create_branch("has space");
        assert!(matches!(result, Err(GitError::NotAllowed { .. })));
    }

    #[test]
    fn create_and_checkout_branch_invalid_name() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.create_and_checkout_branch("has space");
        assert!(matches!(result, Err(GitError::NotAllowed { .. })));
    }

    #[test]
    fn diff_with_options_base_commit() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path.clone()));
        let first = git
            .run_git(&["rev-parse", "HEAD"])
            .unwrap()
            .trim()
            .to_string();

        std::fs::write(repo_path.join("new.txt"), "new").unwrap();
        git.add_all().unwrap();
        git.commit("second").unwrap();

        let diff = git
            .diff_with_options(DiffOptions {
                base_commit: Some(first),
                ..Default::default()
            })
            .unwrap();
        assert!(diff.contains("new.txt"));
    }

    #[test]
    fn diff_with_options_paths_filter() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        std::fs::write(repo_path.join("init.txt"), "changed").unwrap();
        std::fs::write(repo_path.join("other.txt"), "other").unwrap();

        let git = GitOps::new(ExecutionContext::new(repo_path));
        git.add_all().unwrap();

        let diff = git
            .diff_with_options(DiffOptions {
                staged: true,
                paths: vec!["init.txt".to_string()],
                ..Default::default()
            })
            .unwrap();
        assert!(diff.contains("init.txt"));
    }

    #[test]
    fn diff_commit_shows_changes() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path.clone()));

        std::fs::write(repo_path.join("another.txt"), "data").unwrap();
        git.add_all().unwrap();
        git.commit("add another").unwrap();

        let hash = git
            .run_git(&["rev-parse", "HEAD"])
            .unwrap()
            .trim()
            .to_string();
        let diff = git.diff_commit(&hash).unwrap();
        assert!(diff.contains("another.txt"));
    }

    #[test]
    fn commit_message_validation_accepts_valid() {
        let (_tmp, repo_path) = setup_test_repo();
        let git = GitOps::new(ExecutionContext::new(repo_path));

        assert!(git.validate_commit_message("feat: add feature").is_ok());
    }

    #[test]
    fn commit_message_long_first_line_still_valid() {
        let (_tmp, repo_path) = setup_test_repo();
        let git = GitOps::new(ExecutionContext::new(repo_path));

        let long_msg = "a".repeat(120);
        // Should still succeed (just a warning, not an error)
        assert!(git.validate_commit_message(&long_msg).is_ok());
    }

    #[test]
    fn not_a_repo_various_operations() {
        let tmp = TempDir::new().unwrap();
        let git = GitOps::new(ExecutionContext::new(tmp.path().to_path_buf()));

        assert!(matches!(
            git.current_branch(),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.list_branches(),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.create_branch("x"),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.create_and_checkout_branch("x"),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.checkout_branch("x"),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.delete_branch("x"),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.add_files(&["x"]),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(git.add_all(), Err(GitError::NotARepo { .. })));
        assert!(matches!(git.commit("msg"), Err(GitError::NotARepo { .. })));
        assert!(matches!(
            git.amend_commit(None),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(git.diff(), Err(GitError::NotARepo { .. })));
        assert!(matches!(git.diff_staged(), Err(GitError::NotARepo { .. })));
        assert!(matches!(
            git.diff_commit("HEAD"),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.diff_files("a", "b"),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(git.diff_stat(), Err(GitError::NotARepo { .. })));
        assert!(matches!(git.push(), Err(GitError::NotARepo { .. })));
        assert!(matches!(git.pull(), Err(GitError::NotARepo { .. })));
        assert!(matches!(
            git.fetch("origin"),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.fetch_remote("origin"),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.fetch_refspec("origin", "main"),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.fetch_pr("origin", 1),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(git.merge("main"), Err(GitError::NotARepo { .. })));
        assert!(matches!(
            git.get_conflicting_files(),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(git.is_merging(), Err(GitError::NotARepo { .. })));
        assert!(matches!(
            git.get_conflict_info(Path::new("x")),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.resolve_conflict(Path::new("x"), ConflictResolution::Ours),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.resolve_all_conflicts(ConflictResolution::Ours),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.resolve_conflict_manual(Path::new("x"), "c"),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.mark_resolved(Path::new("x")),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.all_conflicts_resolved(),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.complete_merge(),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(git.abort_merge(), Err(GitError::NotARepo { .. })));
        assert!(matches!(
            git.reset_hard("HEAD", true),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.clean_working_tree(true),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(
            git.reset_soft("HEAD"),
            Err(GitError::NotARepo { .. })
        ));
        assert!(matches!(git.reset("HEAD"), Err(GitError::NotARepo { .. })));
        assert!(matches!(
            git.has_unpushed_commits(),
            Err(GitError::NotARepo { .. })
        ));
    }

    #[test]
    fn merge_result_methods() {
        let success = MergeResult::Success {
            merge_commit: None,
            fast_forward: true,
        };
        assert!(success.is_success());
        assert!(!success.has_conflicts());

        let conflict = MergeResult::Conflict {
            conflicting_files: vec![],
        };
        assert!(!conflict.is_success());
        assert!(conflict.has_conflicts());

        let failed = MergeResult::Failed {
            reason: "bad".to_string(),
        };
        assert!(!failed.is_success());
        assert!(!failed.has_conflicts());
    }

    #[test]
    fn is_merging_false_normally() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        assert!(!git.is_merging().unwrap());
    }

    #[test]
    fn complete_merge_fails_when_not_merging() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.complete_merge();
        assert!(matches!(result, Err(GitError::NotAllowed { .. })));
    }

    #[test]
    fn abort_merge_fails_when_not_merging() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.abort_merge();
        assert!(matches!(result, Err(GitError::NotAllowed { .. })));
    }

    #[test]
    fn resolve_conflict_theirs() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);
        create_conflicting_branches(&tmp);

        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let git = GitOps::new(ctx);

        let result = git.merge("feature").unwrap();
        assert!(result.has_conflicts());

        git.resolve_conflict(Path::new("file.txt"), ConflictResolution::Theirs)
            .unwrap();
        assert!(git.all_conflicts_resolved().unwrap());

        let commit = git.complete_merge().unwrap();
        assert!(!commit.hash.is_empty());

        // Verify theirs content won
        let content = std::fs::read_to_string(tmp.path().join("file.txt")).unwrap();
        assert_eq!(content, "feature content");
    }

    #[test]
    fn resolve_all_conflicts_returns_count() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);

        // Create initial commit with two files
        std::fs::write(tmp.path().join("a.txt"), "main a").unwrap();
        std::fs::write(tmp.path().join("b.txt"), "main b").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        let default_branch = get_default_branch(&tmp);

        // Feature branch modifies both
        Command::new("git")
            .args(["checkout", "-b", "feat"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        std::fs::write(tmp.path().join("a.txt"), "feat a").unwrap();
        std::fs::write(tmp.path().join("b.txt"), "feat b").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "feat"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        // Main branch modifies both differently
        Command::new("git")
            .args(["checkout", &default_branch])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        std::fs::write(tmp.path().join("a.txt"), "main a v2").unwrap();
        std::fs::write(tmp.path().join("b.txt"), "main b v2").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "main v2"])
            .current_dir(tmp.path())
            .output()
            .unwrap();

        let git = GitOps::new(ExecutionContext::new(tmp.path().to_path_buf()));
        let result = git.merge("feat").unwrap();
        assert!(result.has_conflicts());

        let count = git.resolve_all_conflicts(ConflictResolution::Ours).unwrap();
        assert_eq!(count, 2);
        assert!(git.all_conflicts_resolved().unwrap());
    }

    #[test]
    fn resolve_conflict_manual_with_custom_content() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);
        create_conflicting_branches(&tmp);

        let git = GitOps::new(ExecutionContext::new(tmp.path().to_path_buf()));
        let result = git.merge("feature").unwrap();
        assert!(result.has_conflicts());

        git.resolve_conflict_manual(Path::new("file.txt"), "custom resolution")
            .unwrap();
        assert!(git.all_conflicts_resolved().unwrap());

        let content = std::fs::read_to_string(tmp.path().join("file.txt")).unwrap();
        assert_eq!(content, "custom resolution");
    }

    #[test]
    fn get_conflict_info_parses_markers() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);
        create_conflicting_branches(&tmp);

        let git = GitOps::new(ExecutionContext::new(tmp.path().to_path_buf()));
        let result = git.merge("feature").unwrap();
        assert!(result.has_conflicts());

        let info = git.get_conflict_info(Path::new("file.txt")).unwrap();
        assert_eq!(info.path, PathBuf::from("file.txt"));
        assert!(!info.regions.is_empty());
        // The conflict should have ours and theirs content
        assert!(!info.regions[0].ours.is_empty());
        assert!(!info.regions[0].theirs.is_empty());
    }

    #[test]
    fn parse_multiple_conflict_regions() {
        let (_tmp, repo_path) = setup_test_repo();
        let git = GitOps::new(ExecutionContext::new(repo_path));

        let content = "before\n<<<<<<< HEAD\nours1\n=======\ntheirs1\n>>>>>>> b\nmiddle\n<<<<<<< HEAD\nours2\n=======\ntheirs2\n>>>>>>> b\nafter";
        let regions = git.parse_conflict_markers(content).unwrap();
        assert_eq!(regions.len(), 2);
        assert_eq!(regions[0].ours, "ours1");
        assert_eq!(regions[0].theirs, "theirs1");
        assert_eq!(regions[1].ours, "ours2");
        assert_eq!(regions[1].theirs, "theirs2");
    }

    #[test]
    fn parse_conflict_no_markers() {
        let (_tmp, repo_path) = setup_test_repo();
        let git = GitOps::new(ExecutionContext::new(repo_path));

        let regions = git
            .parse_conflict_markers("just normal content\nno conflicts")
            .unwrap();
        assert!(regions.is_empty());
    }

    #[test]
    fn clean_working_tree_requires_confirmation() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.clean_working_tree(false);
        assert!(matches!(result, Err(GitError::NotAllowed { .. })));
    }

    #[test]
    fn clean_working_tree_removes_changes() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        // Add untracked file and modify tracked file
        std::fs::write(repo_path.join("untracked.txt"), "junk").unwrap();
        std::fs::write(repo_path.join("init.txt"), "modified").unwrap();

        let git = GitOps::new(ExecutionContext::new(repo_path.clone()));
        git.clean_working_tree(true).unwrap();

        let status = git.status().unwrap();
        assert!(!status.is_dirty);
        assert!(!repo_path.join("untracked.txt").exists());
    }

    #[test]
    fn fetch_on_local_repo() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        // fetch with no remote configured should fail with CommandFailed
        let result = git.fetch("origin");
        assert!(matches!(result, Err(GitError::CommandFailed { .. })));
    }

    #[test]
    fn fetch_remote_no_remote() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.fetch_remote("origin");
        assert!(matches!(result, Err(GitError::CommandFailed { .. })));
    }

    #[test]
    fn fetch_refspec_no_remote() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.fetch_refspec("origin", "main");
        assert!(matches!(result, Err(GitError::CommandFailed { .. })));
    }

    #[test]
    fn fetch_pr_no_remote() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.fetch_pr("origin", 42);
        assert!(matches!(result, Err(GitError::CommandFailed { .. })));
    }

    #[test]
    fn push_with_options_force_rejected() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.push_with_options(PushOptions {
            force: true,
            ..Default::default()
        });
        assert!(matches!(result, Err(GitError::NotAllowed { .. })));
    }

    #[test]
    fn push_no_remote_fails() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.push();
        // Should fail because no remote is configured
        assert!(result.is_err());
    }

    #[test]
    fn pull_no_remote_fails() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.pull();
        assert!(result.is_err());
    }

    #[test]
    fn diff_options_default() {
        let opts = DiffOptions::default();
        assert!(opts.paths.is_empty());
        assert!(!opts.staged);
        assert!(opts.base_commit.is_none());
        assert!(opts.context_lines.is_none());
    }

    #[test]
    fn push_options_default() {
        let opts = PushOptions::default();
        assert_eq!(opts.remote, "origin");
        assert!(opts.branch.is_none());
        assert!(!opts.set_upstream);
        assert!(!opts.force);
    }

    #[test]
    fn get_conflicting_files_empty_when_no_conflicts() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let files = git.get_conflicting_files().unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn complete_merge_fails_with_unresolved_conflicts() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);
        create_conflicting_branches(&tmp);

        let git = GitOps::new(ExecutionContext::new(tmp.path().to_path_buf()));
        git.merge("feature").unwrap();
        assert!(git.is_merging().unwrap());

        // Try to complete without resolving
        let result = git.complete_merge();
        assert!(matches!(result, Err(GitError::NotAllowed { .. })));
    }

    #[test]
    fn validate_branch_name_backslash() {
        let (_tmp, repo_path) = setup_test_repo();
        let git = GitOps::new(ExecutionContext::new(repo_path));
        assert!(git.validate_branch_name("has\\backslash").is_err());
    }

    #[test]
    fn validate_branch_name_tab() {
        let (_tmp, repo_path) = setup_test_repo();
        let git = GitOps::new(ExecutionContext::new(repo_path));
        assert!(git.validate_branch_name("has\ttab").is_err());
    }

    #[test]
    fn validate_branch_name_newline() {
        let (_tmp, repo_path) = setup_test_repo();
        let git = GitOps::new(ExecutionContext::new(repo_path));
        assert!(git.validate_branch_name("has\nnewline").is_err());
    }

    #[test]
    fn validate_branch_name_question_mark() {
        let (_tmp, repo_path) = setup_test_repo();
        let git = GitOps::new(ExecutionContext::new(repo_path));
        assert!(git.validate_branch_name("has?question").is_err());
    }

    #[test]
    fn validate_branch_name_asterisk() {
        let (_tmp, repo_path) = setup_test_repo();
        let git = GitOps::new(ExecutionContext::new(repo_path));
        assert!(git.validate_branch_name("has*star").is_err());
    }

    #[test]
    fn mark_resolved_stages_file() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(&tmp);
        create_conflicting_branches(&tmp);

        let git = GitOps::new(ExecutionContext::new(tmp.path().to_path_buf()));
        git.merge("feature").unwrap();

        // Write resolved content manually
        std::fs::write(tmp.path().join("file.txt"), "resolved").unwrap();
        git.mark_resolved(Path::new("file.txt")).unwrap();

        assert!(git.all_conflicts_resolved().unwrap());
    }

    #[test]
    fn push_with_set_upstream_option() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        // Will fail due to no remote, but exercises the set_upstream code path
        let result = git.push_with_options(PushOptions {
            set_upstream: true,
            ..Default::default()
        });
        assert!(result.is_err());
    }

    #[test]
    fn push_with_custom_branch() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.push_with_options(PushOptions {
            branch: Some("custom-branch".to_string()),
            ..Default::default()
        });
        assert!(result.is_err());
    }

    #[test]
    fn conflict_region_line_numbers() {
        let (_tmp, repo_path) = setup_test_repo();
        let git = GitOps::new(ExecutionContext::new(repo_path));

        let content = "line1\nline2\n<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> b\nline8";
        let regions = git.parse_conflict_markers(content).unwrap();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].start_line, 3);
        assert_eq!(regions[0].end_line, 7);
    }

    #[test]
    fn pull_from_no_remote_fails() {
        let (_tmp, repo_path) = setup_test_repo();
        make_initial_commit(&repo_path);

        let git = GitOps::new(ExecutionContext::new(repo_path));
        let result = git.pull_from("origin", "main");
        assert!(result.is_err());
    }
}
