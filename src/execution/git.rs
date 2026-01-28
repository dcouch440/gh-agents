//! Git operations with audit logging

use crate::execution::ExecutionContext;
use std::path::PathBuf;
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

impl Default for DiffOptions {
    fn default() -> Self {
        Self {
            paths: vec![],
            staged: false,
            base_commit: None,
            context_lines: None,
        }
    }
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

        status.is_dirty =
            !status.staged.is_empty() || !status.unstaged.is_empty() || !status.untracked.is_empty();

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
}
