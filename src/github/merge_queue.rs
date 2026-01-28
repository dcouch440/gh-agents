//! PR merge queue management
//!
//! Handles ordered merging of PRs with conflict resolution workflow.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::execution::{ConflictResolution, GitOps, MergeResult};
use crate::github::{GitHubClient, GitHubPullRequest, MergeMethod, MergePrResult};
use crate::types::{MergeStrategy, PrMergeConfig};

// =========================================================================
// Queue Status
// =========================================================================

/// Status of a PR in the merge queue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueStatus {
    /// Waiting to be processed
    Pending,
    /// Currently being processed
    InProgress,
    /// Successfully merged
    Merged,
    /// Failed to merge (non-conflict)
    Failed,
    /// Has conflicts that need resolution
    Conflict,
    /// Skipped (e.g., closed by user)
    Skipped,
}

impl std::fmt::Display for QueueStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueStatus::Pending => write!(f, "pending"),
            QueueStatus::InProgress => write!(f, "in_progress"),
            QueueStatus::Merged => write!(f, "merged"),
            QueueStatus::Failed => write!(f, "failed"),
            QueueStatus::Conflict => write!(f, "conflict"),
            QueueStatus::Skipped => write!(f, "skipped"),
        }
    }
}

impl std::str::FromStr for QueueStatus {
    type Err = QueueError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(QueueStatus::Pending),
            "in_progress" => Ok(QueueStatus::InProgress),
            "merged" => Ok(QueueStatus::Merged),
            "failed" => Ok(QueueStatus::Failed),
            "conflict" => Ok(QueueStatus::Conflict),
            "skipped" => Ok(QueueStatus::Skipped),
            _ => Err(QueueError::InvalidStatus(s.to_string())),
        }
    }
}

// =========================================================================
// Queue Types
// =========================================================================

/// Information about merge conflicts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfoJson {
    /// Files with conflicts
    pub files: Vec<String>,
    /// Timestamp when conflicts were detected
    pub detected_at: DateTime<Utc>,
    /// Whether human review is required
    pub needs_human_review: bool,
}

/// An entry in the PR merge queue
#[derive(Debug, Clone)]
pub struct PrQueueEntry {
    pub id: Uuid,
    pub repo_owner: String,
    pub repo_name: String,
    pub pr_number: u32,
    pub queue_position: u32,
    pub status: QueueStatus,
    pub conflict_info: Option<ConflictInfoJson>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Queue statistics
#[derive(Debug, Clone, Default)]
pub struct QueueStats {
    pub total: u32,
    pub pending: u32,
    pub in_progress: u32,
    pub merged: u32,
    pub failed: u32,
    pub with_conflicts: u32,
    pub skipped: u32,
}

// =========================================================================
// Queue Errors
// =========================================================================

#[derive(Error, Debug)]
pub enum QueueError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("invalid queue status: {0}")]
    InvalidStatus(String),

    #[error("PR #{pr_number} not in queue for {owner}/{repo}")]
    NotInQueue {
        owner: String,
        repo: String,
        pr_number: u32,
    },

    #[error("cannot merge PR #{pr_number} out of order, PR #{next_in_queue} is next")]
    OutOfOrder { pr_number: u32, next_in_queue: u32 },

    #[error("github error: {0}")]
    GitHub(#[from] crate::github::GitHubError),

    #[error("git error: {0}")]
    Git(#[from] crate::execution::GitError),

    #[error("{0}")]
    Other(String),
}

// =========================================================================
// Merge Queue
// =========================================================================

/// Manages the PR merge queue
pub struct MergeQueue {
    pool: SqlitePool,
}

impl MergeQueue {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Add a PR to the merge queue
    ///
    /// Returns the queue entry with position (1-indexed)
    pub async fn add_to_queue(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u32,
    ) -> Result<PrQueueEntry, QueueError> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        // Get next position for this repo
        let next_position = self.get_next_position(owner, repo).await?;

        sqlx::query(
            r#"
            INSERT INTO pr_merge_queue (
                id, repo_owner, repo_name, pr_number,
                queue_position, status, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT (repo_owner, repo_name, pr_number)
            DO UPDATE SET updated_at = excluded.updated_at
            "#,
        )
        .bind(id.to_string())
        .bind(owner)
        .bind(repo)
        .bind(pr_number as i64)
        .bind(next_position as i64)
        .bind("pending")
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        tracing::info!(
            owner = %owner,
            repo = %repo,
            pr = pr_number,
            position = next_position,
            "Added PR to merge queue"
        );

        Ok(PrQueueEntry {
            id,
            repo_owner: owner.to_string(),
            repo_name: repo.to_string(),
            pr_number,
            queue_position: next_position,
            status: QueueStatus::Pending,
            conflict_info: None,
            error_message: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// Get the next queue position for a repo
    async fn get_next_position(&self, owner: &str, repo: &str) -> Result<u32, QueueError> {
        let row: Option<(i64,)> = sqlx::query_as(
            r#"
            SELECT COALESCE(MAX(queue_position), 0) + 1
            FROM pr_merge_queue
            WHERE repo_owner = ? AND repo_name = ?
            "#,
        )
        .bind(owner)
        .bind(repo)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(n,)| n as u32).unwrap_or(1))
    }

    /// Remove a PR from the queue
    pub async fn remove_from_queue(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u32,
    ) -> Result<bool, QueueError> {
        let result = sqlx::query(
            r#"
            DELETE FROM pr_merge_queue
            WHERE repo_owner = ? AND repo_name = ? AND pr_number = ?
            "#,
        )
        .bind(owner)
        .bind(repo)
        .bind(pr_number as i64)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get all entries in the queue for a repo, ordered by position
    pub async fn get_queue(&self, owner: &str, repo: &str) -> Result<Vec<PrQueueEntry>, QueueError> {
        let rows: Vec<(
            String,
            String,
            String,
            i64,
            i64,
            String,
            Option<String>,
            Option<String>,
            String,
            String,
        )> = sqlx::query_as(
            r#"
            SELECT
                id, repo_owner, repo_name, pr_number,
                queue_position, status, conflict_info,
                error_message, created_at, updated_at
            FROM pr_merge_queue
            WHERE repo_owner = ? AND repo_name = ?
            ORDER BY queue_position ASC
            "#,
        )
        .bind(owner)
        .bind(repo)
        .fetch_all(&self.pool)
        .await?;

        let entries = rows
            .into_iter()
            .filter_map(|row| {
                Some(PrQueueEntry {
                    id: Uuid::parse_str(&row.0).ok()?,
                    repo_owner: row.1,
                    repo_name: row.2,
                    pr_number: row.3 as u32,
                    queue_position: row.4 as u32,
                    status: row.5.parse().ok()?,
                    conflict_info: row.6.and_then(|s| serde_json::from_str(&s).ok()),
                    error_message: row.7,
                    created_at: DateTime::parse_from_rfc3339(&row.8)
                        .ok()?
                        .with_timezone(&Utc),
                    updated_at: DateTime::parse_from_rfc3339(&row.9)
                        .ok()?
                        .with_timezone(&Utc),
                })
            })
            .collect();

        Ok(entries)
    }

    /// Get a specific entry by PR number
    pub async fn get_entry(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u32,
    ) -> Result<Option<PrQueueEntry>, QueueError> {
        let queue = self.get_queue(owner, repo).await?;
        Ok(queue.into_iter().find(|e| e.pr_number == pr_number))
    }

    /// Update the status of a queue entry
    pub async fn update_status(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u32,
        status: QueueStatus,
        error_message: Option<&str>,
    ) -> Result<bool, QueueError> {
        let now = Utc::now();

        let result = sqlx::query(
            r#"
            UPDATE pr_merge_queue
            SET status = ?, error_message = ?, updated_at = ?
            WHERE repo_owner = ? AND repo_name = ? AND pr_number = ?
            "#,
        )
        .bind(status.to_string())
        .bind(error_message)
        .bind(now.to_rfc3339())
        .bind(owner)
        .bind(repo)
        .bind(pr_number as i64)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Set conflict info for a queue entry
    pub async fn set_conflict_info(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u32,
        info: ConflictInfoJson,
    ) -> Result<bool, QueueError> {
        let now = Utc::now();
        let info_json = serde_json::to_string(&info).unwrap_or_default();

        let result = sqlx::query(
            r#"
            UPDATE pr_merge_queue
            SET status = ?, conflict_info = ?, updated_at = ?
            WHERE repo_owner = ? AND repo_name = ? AND pr_number = ?
            "#,
        )
        .bind("conflict")
        .bind(info_json)
        .bind(now.to_rfc3339())
        .bind(owner)
        .bind(repo)
        .bind(pr_number as i64)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // =======================================================================
    // Ordering Enforcement (Slice 8.7.2)
    // =======================================================================

    /// Get the next PR to merge (first pending in queue)
    ///
    /// This is the ONLY entry that should be merged. Never merge
    /// a PR that isn't at the front of the queue.
    pub async fn get_next_to_merge(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Option<PrQueueEntry>, QueueError> {
        let queue = self.get_queue(owner, repo).await?;

        // Find first pending entry
        Ok(queue.into_iter().find(|e| e.status == QueueStatus::Pending))
    }

    /// Check if a PR can be merged (is it at the front of the queue?)
    ///
    /// Returns true only if:
    /// 1. The PR is in the queue
    /// 2. It's the first pending entry
    pub async fn can_merge(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u32,
    ) -> Result<bool, QueueError> {
        let next = self.get_next_to_merge(owner, repo).await?;

        match next {
            Some(entry) if entry.pr_number == pr_number => Ok(true),
            Some(entry) => {
                tracing::warn!(
                    requested = pr_number,
                    next_in_queue = entry.pr_number,
                    "Cannot merge PR out of order"
                );
                Ok(false)
            }
            None => {
                tracing::debug!(pr = pr_number, "No pending PRs in queue");
                Ok(false)
            }
        }
    }

    /// Get the position of a PR in the pending queue
    /// Returns None if PR is not pending
    pub async fn get_position(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u32,
    ) -> Result<Option<u32>, QueueError> {
        let queue = self.get_queue(owner, repo).await?;

        let pending: Vec<_> = queue
            .iter()
            .filter(|e| e.status == QueueStatus::Pending)
            .collect();

        for (idx, entry) in pending.iter().enumerate() {
            if entry.pr_number == pr_number {
                return Ok(Some((idx + 1) as u32));
            }
        }

        Ok(None)
    }

    /// Get count of PRs ahead in queue
    pub async fn prs_ahead(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u32,
    ) -> Result<u32, QueueError> {
        match self.get_position(owner, repo, pr_number).await? {
            Some(pos) => Ok(pos - 1),
            None => Ok(0),
        }
    }

    /// Reorder queue after a merge (compact positions)
    pub async fn compact_queue(&self, owner: &str, repo: &str) -> Result<(), QueueError> {
        let queue = self.get_queue(owner, repo).await?;

        // Re-number pending entries starting at 1
        let mut new_position = 1u32;
        for entry in queue {
            if entry.status == QueueStatus::Pending {
                if entry.queue_position != new_position {
                    sqlx::query(
                        r#"
                        UPDATE pr_merge_queue
                        SET queue_position = ?, updated_at = ?
                        WHERE id = ?
                        "#,
                    )
                    .bind(new_position as i64)
                    .bind(Utc::now().to_rfc3339())
                    .bind(entry.id.to_string())
                    .execute(&self.pool)
                    .await?;
                }
                new_position += 1;
            }
        }

        Ok(())
    }

    /// Get queue statistics
    pub async fn get_queue_stats(&self, owner: &str, repo: &str) -> Result<QueueStats, QueueError> {
        let queue = self.get_queue(owner, repo).await?;

        let mut stats = QueueStats::default();

        for entry in queue {
            stats.total += 1;
            match entry.status {
                QueueStatus::Pending => stats.pending += 1,
                QueueStatus::InProgress => stats.in_progress += 1,
                QueueStatus::Merged => stats.merged += 1,
                QueueStatus::Failed => stats.failed += 1,
                QueueStatus::Conflict => stats.with_conflicts += 1,
                QueueStatus::Skipped => stats.skipped += 1,
            }
        }

        Ok(stats)
    }

    // =======================================================================
    // Resume Capability (Slice 8.7.4)
    // =======================================================================

    /// Resume queue processing after restart
    ///
    /// Call this when auto-merge is re-enabled or on startup.
    pub async fn resume_processing(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<PrQueueEntry>, QueueError> {
        // First, reset any in_progress back to pending (they were interrupted)
        let reset_count = self.reset_interrupted(owner, repo).await?;

        if reset_count > 0 {
            tracing::info!(
                owner = %owner,
                repo = %repo,
                count = reset_count,
                "Reset interrupted queue entries"
            );
        }

        // Get all pending entries
        let queue = self.get_queue(owner, repo).await?;
        let pending: Vec<_> = queue
            .into_iter()
            .filter(|e| e.status == QueueStatus::Pending)
            .collect();

        if !pending.is_empty() {
            tracing::info!(
                owner = %owner,
                repo = %repo,
                pending = pending.len(),
                "Resuming queue processing"
            );
        }

        Ok(pending)
    }

    /// Reset in_progress entries back to pending
    async fn reset_interrupted(&self, owner: &str, repo: &str) -> Result<u32, QueueError> {
        let now = Utc::now();

        let result = sqlx::query(
            r#"
            UPDATE pr_merge_queue
            SET status = 'pending', updated_at = ?
            WHERE repo_owner = ? AND repo_name = ?
            AND status = 'in_progress'
            "#,
        )
        .bind(now.to_rfc3339())
        .bind(owner)
        .bind(repo)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as u32)
    }

    /// Get entries that need attention (conflicts, failures)
    pub async fn get_needs_attention(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<PrQueueEntry>, QueueError> {
        let queue = self.get_queue(owner, repo).await?;

        Ok(queue
            .into_iter()
            .filter(|e| e.status == QueueStatus::Conflict || e.status == QueueStatus::Failed)
            .collect())
    }

    /// Check if there's work to resume
    pub async fn has_pending_work(&self, owner: &str, repo: &str) -> Result<bool, QueueError> {
        let queue = self.get_queue(owner, repo).await?;

        Ok(queue
            .iter()
            .any(|e| e.status == QueueStatus::Pending || e.status == QueueStatus::InProgress))
    }

    /// Clear completed entries older than the given duration
    pub async fn cleanup_old_entries(
        &self,
        owner: &str,
        repo: &str,
        older_than: std::time::Duration,
    ) -> Result<u32, QueueError> {
        let cutoff =
            Utc::now() - chrono::Duration::from_std(older_than).unwrap_or(chrono::Duration::days(7));

        let result = sqlx::query(
            r#"
            DELETE FROM pr_merge_queue
            WHERE repo_owner = ? AND repo_name = ?
            AND status IN ('merged', 'skipped')
            AND updated_at < ?
            "#,
        )
        .bind(owner)
        .bind(repo)
        .bind(cutoff.to_rfc3339())
        .execute(&self.pool)
        .await?;

        let deleted = result.rows_affected() as u32;

        if deleted > 0 {
            tracing::info!(
                owner = %owner,
                repo = %repo,
                deleted = deleted,
                "Cleaned up old queue entries"
            );
        }

        Ok(deleted)
    }
}

// =========================================================================
// Queue Processor (Slice 8.7.5)
// =========================================================================

/// Result of processing a single PR
#[derive(Debug)]
pub enum ProcessResult {
    /// PR was merged successfully
    Merged { sha: String },
    /// Conflicts were resolved and PR merged
    MergedAfterResolution {
        sha: String,
        conflicts_resolved: u32,
    },
    /// Conflicts need human review
    NeedsHumanReview { files: Vec<String> },
    /// PR was skipped (closed, already merged, etc.)
    Skipped { reason: String },
    /// Processing failed
    Failed { error: String },
}

/// Options for controlling progress notifications
#[derive(Debug, Clone)]
pub struct NotificationOptions {
    /// Post when PR is added to queue
    pub on_queued: bool,
    /// Post when merge starts
    pub on_merge_start: bool,
    /// Post when merge completes
    pub on_merged: bool,
    /// Post when conflicts detected
    pub on_conflicts: bool,
    /// Post when merge fails
    pub on_failed: bool,
    /// Post when position changes
    pub on_position_change: bool,
}

impl Default for NotificationOptions {
    fn default() -> Self {
        Self {
            on_queued: true,
            on_merge_start: false, // Too noisy by default
            on_merged: true,
            on_conflicts: true,
            on_failed: true,
            on_position_change: false, // Too noisy by default
        }
    }
}

/// Processes the merge queue
pub struct MergeQueueProcessor {
    queue: MergeQueue,
    github: GitHubClient,
    git: GitOps,
    config: PrMergeConfig,
    notifications: NotificationOptions,
}

impl MergeQueueProcessor {
    pub fn new(
        pool: SqlitePool,
        github: GitHubClient,
        git: GitOps,
        config: PrMergeConfig,
    ) -> Self {
        Self {
            queue: MergeQueue::new(pool),
            github,
            git,
            config,
            notifications: NotificationOptions::default(),
        }
    }

    pub fn with_notifications(mut self, opts: NotificationOptions) -> Self {
        self.notifications = opts;
        self
    }

    /// Get reference to the queue
    pub fn queue(&self) -> &MergeQueue {
        &self.queue
    }

    /// Add a PR to the queue and notify
    pub async fn enqueue(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u32,
    ) -> Result<PrQueueEntry, QueueError> {
        let entry = self.queue.add_to_queue(owner, repo, pr_number).await?;

        if self.notifications.on_queued {
            if let Err(e) = self
                .notify_queued(owner, repo, pr_number, entry.queue_position)
                .await
            {
                tracing::warn!(error = %e, "Failed to post queue notification");
            }
        }

        Ok(entry)
    }

    /// Process the next PR in the queue
    ///
    /// Returns the PR number and result, or None if queue is empty.
    pub async fn process_next(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Option<(u32, ProcessResult)>, QueueError> {
        // Get next PR to process
        let entry = match self.queue.get_next_to_merge(owner, repo).await? {
            Some(e) => e,
            None => return Ok(None),
        };

        let pr_number = entry.pr_number;

        // Mark as in progress
        self.queue
            .update_status(owner, repo, pr_number, QueueStatus::InProgress, None)
            .await?;

        tracing::info!(
            pr = pr_number,
            position = entry.queue_position,
            "Processing PR from queue"
        );

        if self.notifications.on_merge_start {
            if let Err(e) = self.notify_merge_started(owner, repo, pr_number).await {
                tracing::warn!(error = %e, "Failed to post merge start notification");
            }
        }

        // Process the PR
        let result = self.process_pr(owner, repo, pr_number).await;

        // Update status based on result
        match &result {
            Ok(ProcessResult::Merged { sha, .. })
            | Ok(ProcessResult::MergedAfterResolution { sha, .. }) => {
                self.queue
                    .update_status(owner, repo, pr_number, QueueStatus::Merged, None)
                    .await?;

                if self.notifications.on_merged {
                    if let Err(e) = self.notify_merged(owner, repo, pr_number, sha).await {
                        tracing::warn!(error = %e, "Failed to post merged notification");
                    }
                }
            }
            Ok(ProcessResult::NeedsHumanReview { files }) => {
                self.queue
                    .set_conflict_info(
                        owner,
                        repo,
                        pr_number,
                        ConflictInfoJson {
                            files: files.clone(),
                            detected_at: Utc::now(),
                            needs_human_review: true,
                        },
                    )
                    .await?;

                if self.notifications.on_conflicts {
                    if let Err(e) = self.notify_conflicts(owner, repo, pr_number, files).await {
                        tracing::warn!(error = %e, "Failed to post conflict notification");
                    }
                }
            }
            Ok(ProcessResult::Skipped { reason }) => {
                self.queue
                    .update_status(owner, repo, pr_number, QueueStatus::Skipped, Some(reason))
                    .await?;
            }
            Ok(ProcessResult::Failed { error }) | Err(QueueError::Other(error)) => {
                self.queue
                    .update_status(
                        owner,
                        repo,
                        pr_number,
                        QueueStatus::Failed,
                        Some(error.as_str()),
                    )
                    .await?;

                if self.notifications.on_failed {
                    if let Err(e) = self.notify_failed(owner, repo, pr_number, error).await {
                        tracing::warn!(error = %e, "Failed to post failure notification");
                    }
                }
            }
            Err(e) => {
                self.queue
                    .update_status(
                        owner,
                        repo,
                        pr_number,
                        QueueStatus::Failed,
                        Some(&e.to_string()),
                    )
                    .await?;
            }
        }

        Ok(Some((pr_number, result?)))
    }

    /// Process a single PR
    async fn process_pr(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u32,
    ) -> Result<ProcessResult, QueueError> {
        // Get PR details
        let pr = self.github.get_pull_request(owner, repo, pr_number).await?;

        // Check if already merged or closed
        if pr.merged.unwrap_or(false) {
            return Ok(ProcessResult::Skipped {
                reason: "PR already merged".to_string(),
            });
        }

        if pr.state == "closed" {
            return Ok(ProcessResult::Skipped {
                reason: "PR is closed".to_string(),
            });
        }

        // Try to merge directly first
        let merge_method: MergeMethod = self.config.merge_strategy.into();
        let merge_result = self
            .github
            .merge_pr_simple(owner, repo, pr_number, merge_method)
            .await?;

        match merge_result {
            MergePrResult::Merged { sha, .. } => Ok(ProcessResult::Merged { sha }),
            MergePrResult::HasConflicts => {
                // Need to resolve conflicts
                self.handle_conflicts(owner, repo, pr_number, &pr).await
            }
            MergePrResult::NotMergeable { reason } => Ok(ProcessResult::Failed {
                error: format!("Not mergeable: {}", reason),
            }),
            MergePrResult::AlreadyMerged => Ok(ProcessResult::Skipped {
                reason: "PR already merged".to_string(),
            }),
            MergePrResult::HeadMismatch { .. } => Ok(ProcessResult::Failed {
                error: "PR was updated during merge".to_string(),
            }),
            MergePrResult::Failed { message, .. } => Ok(ProcessResult::Failed { error: message }),
        }
    }

    /// Handle merge conflicts for a PR
    async fn handle_conflicts(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u32,
        pr: &GitHubPullRequest,
    ) -> Result<ProcessResult, QueueError> {
        tracing::info!(pr = pr_number, "PR has conflicts, attempting resolution");

        // Fetch the PR branch locally
        let local_branch = self.git.fetch_pr("origin", pr_number)?;

        // Checkout the base branch
        self.git.checkout_branch(&pr.base.ref_name)?;

        // Attempt local merge
        let merge_result = self.git.merge(&local_branch)?;

        match merge_result {
            MergeResult::Success { .. } => {
                // No conflicts after all? Push and merge
                self.git.push()?;
                let merge_method: MergeMethod = self.config.merge_strategy.into();
                let result = self
                    .github
                    .merge_pr_simple(owner, repo, pr_number, merge_method)
                    .await?;

                match result {
                    MergePrResult::Merged { sha, .. } => Ok(ProcessResult::Merged { sha }),
                    _ => Ok(ProcessResult::Failed {
                        error: "Merge failed after local resolution".to_string(),
                    }),
                }
            }
            MergeResult::Conflict { conflicting_files } => {
                // Check if we can auto-resolve
                if self.config.require_approval_for_conflicts {
                    // Need human review
                    self.git.abort_merge()?;
                    return Ok(ProcessResult::NeedsHumanReview {
                        files: conflicting_files
                            .iter()
                            .map(|p| p.to_string_lossy().to_string())
                            .collect(),
                    });
                }

                // Try auto-resolution (accept theirs - the PR changes)
                let resolved = self.git.resolve_all_conflicts(ConflictResolution::Theirs)?;

                // Complete the merge
                self.git.complete_merge()?;

                // Push the resolution
                self.git.push()?;

                // Now try the API merge again
                let merge_method: MergeMethod = self.config.merge_strategy.into();
                let result = self
                    .github
                    .merge_pr_simple(owner, repo, pr_number, merge_method)
                    .await?;

                match result {
                    MergePrResult::Merged { sha, .. } => Ok(ProcessResult::MergedAfterResolution {
                        sha,
                        conflicts_resolved: resolved,
                    }),
                    _ => Ok(ProcessResult::Failed {
                        error: "Merge failed after conflict resolution".to_string(),
                    }),
                }
            }
            MergeResult::Failed { reason } => {
                let _ = self.git.abort_merge(); // Best effort cleanup
                Ok(ProcessResult::Failed { error: reason })
            }
        }
    }

    /// Run the queue processor continuously
    pub async fn run(
        &self,
        owner: &str,
        repo: &str,
        mut shutdown: tokio::sync::watch::Receiver<bool>,
    ) -> Result<(), QueueError> {
        tracing::info!(
            owner = %owner,
            repo = %repo,
            "Starting merge queue processor"
        );

        // Resume any interrupted work
        self.queue.resume_processing(owner, repo).await?;

        loop {
            // Check for shutdown
            if *shutdown.borrow() {
                tracing::info!("Merge queue processor shutting down");
                break;
            }

            // Check if auto-merge is enabled
            if !self.config.auto_merge_enabled {
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                continue;
            }

            // Process next PR
            match self.process_next(owner, repo).await {
                Ok(Some((pr, result))) => {
                    tracing::info!(pr = pr, result = ?result, "Processed PR");
                }
                Ok(None) => {
                    // No work, wait a bit
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
                Err(e) => {
                    tracing::error!(error = %e, "Error processing merge queue");
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                }
            }

            // Short sleep between iterations to avoid spinning
            tokio::select! {
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        tracing::info!("Merge queue processor shutting down");
                        break;
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {}
            }
        }

        Ok(())
    }

    // =======================================================================
    // Progress Updates (Slice 8.7.6)
    // =======================================================================

    /// Post a status update to the PR
    async fn post_status(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u32,
        status: &str,
        details: &str,
    ) -> Result<(), QueueError> {
        let body = format!(
            "**Merge Queue Update**: {}\n\n{}\n\n---\n*Automated by nexor*",
            status, details
        );

        self.github
            .create_issue_comment(
                owner,
                repo,
                pr_number,
                &crate::github::CreateIssueComment { body },
            )
            .await?;

        Ok(())
    }

    /// Notify PR was added to queue
    async fn notify_queued(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u32,
        position: u32,
    ) -> Result<(), QueueError> {
        let details = if position == 1 {
            "This PR is next in line and will be processed shortly.".to_string()
        } else {
            format!(
                "This PR is #{} in the queue. {} PR(s) ahead.",
                position,
                position - 1
            )
        };

        self.post_status(owner, repo, pr_number, "Added to Merge Queue", &details)
            .await
    }

    /// Notify merge started
    async fn notify_merge_started(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u32,
    ) -> Result<(), QueueError> {
        self.post_status(
            owner,
            repo,
            pr_number,
            "Merge In Progress",
            "Attempting to merge this PR...",
        )
        .await
    }

    /// Notify merge completed
    async fn notify_merged(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u32,
        sha: &str,
    ) -> Result<(), QueueError> {
        self.post_status(
            owner,
            repo,
            pr_number,
            "Merged",
            &format!("Successfully merged in commit `{}`", sha),
        )
        .await
    }

    /// Notify conflicts need resolution
    async fn notify_conflicts(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u32,
        files: &[String],
    ) -> Result<(), QueueError> {
        let file_list = files
            .iter()
            .take(10)
            .map(|f| format!("- `{}`", f))
            .collect::<Vec<_>>()
            .join("\n");

        let more = if files.len() > 10 {
            format!("\n...and {} more files", files.len() - 10)
        } else {
            String::new()
        };

        let details = format!(
            "This PR has merge conflicts that need resolution:\n\n{}{}\n\n\
             Please resolve conflicts and update the PR.",
            file_list, more
        );

        self.post_status(owner, repo, pr_number, "Conflicts Detected", &details)
            .await
    }

    /// Notify merge failed
    async fn notify_failed(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u32,
        error: &str,
    ) -> Result<(), QueueError> {
        self.post_status(
            owner,
            repo,
            pr_number,
            "Merge Failed",
            &format!("Failed to merge: {}", error),
        )
        .await
    }

    /// Notify queue position changed
    pub async fn notify_position_changed(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u32,
        new_position: u32,
    ) -> Result<(), QueueError> {
        if new_position == 1 {
            self.post_status(
                owner,
                repo,
                pr_number,
                "Queue Position Update",
                "This PR is now next in line!",
            )
            .await
        } else {
            self.post_status(
                owner,
                repo,
                pr_number,
                "Queue Position Update",
                &format!("This PR is now #{} in the queue.", new_position),
            )
            .await
        }
    }
}

// Helper conversion
impl From<MergeStrategy> for MergeMethod {
    fn from(s: MergeStrategy) -> Self {
        match s {
            MergeStrategy::Merge => MergeMethod::Merge,
            MergeStrategy::Squash => MergeMethod::Squash,
            MergeStrategy::Rebase => MergeMethod::Rebase,
        }
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_status_roundtrip() {
        for status in [
            QueueStatus::Pending,
            QueueStatus::InProgress,
            QueueStatus::Merged,
            QueueStatus::Failed,
            QueueStatus::Conflict,
            QueueStatus::Skipped,
        ] {
            let s = status.to_string();
            let parsed: QueueStatus = s.parse().unwrap();
            assert_eq!(status, parsed);
        }
    }

    #[test]
    fn queue_status_display() {
        assert_eq!(QueueStatus::Pending.to_string(), "pending");
        assert_eq!(QueueStatus::InProgress.to_string(), "in_progress");
        assert_eq!(QueueStatus::Merged.to_string(), "merged");
        assert_eq!(QueueStatus::Failed.to_string(), "failed");
        assert_eq!(QueueStatus::Conflict.to_string(), "conflict");
        assert_eq!(QueueStatus::Skipped.to_string(), "skipped");
    }

    #[test]
    fn invalid_status_parse() {
        let result: Result<QueueStatus, _> = "invalid".parse();
        assert!(result.is_err());
    }

    #[test]
    fn conflict_info_json_serialization() {
        let info = ConflictInfoJson {
            files: vec!["src/main.rs".to_string(), "Cargo.toml".to_string()],
            detected_at: Utc::now(),
            needs_human_review: true,
        };

        let json = serde_json::to_string(&info).unwrap();
        let parsed: ConflictInfoJson = serde_json::from_str(&json).unwrap();

        assert_eq!(info.files, parsed.files);
        assert_eq!(info.needs_human_review, parsed.needs_human_review);
    }

    #[test]
    fn queue_stats_default() {
        let stats = QueueStats::default();
        assert_eq!(stats.total, 0);
        assert_eq!(stats.pending, 0);
        assert_eq!(stats.merged, 0);
    }

    #[test]
    fn merge_strategy_to_merge_method() {
        assert_eq!(MergeMethod::from(MergeStrategy::Merge), MergeMethod::Merge);
        assert_eq!(
            MergeMethod::from(MergeStrategy::Squash),
            MergeMethod::Squash
        );
        assert_eq!(
            MergeMethod::from(MergeStrategy::Rebase),
            MergeMethod::Rebase
        );
    }

    #[test]
    fn notification_options_default() {
        let opts = NotificationOptions::default();
        assert!(opts.on_queued);
        assert!(!opts.on_merge_start);
        assert!(opts.on_merged);
        assert!(opts.on_conflicts);
        assert!(opts.on_failed);
        assert!(!opts.on_position_change);
    }

    #[test]
    fn pr_queue_entry_struct() {
        let entry = PrQueueEntry {
            id: Uuid::new_v4(),
            repo_owner: "owner".to_string(),
            repo_name: "repo".to_string(),
            pr_number: 123,
            queue_position: 1,
            status: QueueStatus::Pending,
            conflict_info: None,
            error_message: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(entry.pr_number, 123);
        assert_eq!(entry.status, QueueStatus::Pending);
    }
}
