//! PR merge queue management
//!
//! Handles ordered merging of PRs with conflict resolution workflow.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
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
    pool: PgPool,
}

impl MergeQueue {
    pub fn new(pool: PgPool) -> Self {
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
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
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
            WHERE repo_owner = $1 AND repo_name = $2
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
            WHERE repo_owner = $1 AND repo_name = $2 AND pr_number = $3
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
    pub async fn get_queue(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<PrQueueEntry>, QueueError> {
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
            WHERE repo_owner = $1 AND repo_name = $2
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
            SET status = $1, error_message = $2, updated_at = $3
            WHERE repo_owner = $4 AND repo_name = $5 AND pr_number = $6
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
            SET status = $1, conflict_info = $2, updated_at = $3
            WHERE repo_owner = $4 AND repo_name = $5 AND pr_number = $6
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
                        SET queue_position = $1, updated_at = $2
                        WHERE id = $3
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
            SET status = 'pending', updated_at = $1
            WHERE repo_owner = $2 AND repo_name = $3
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
        let cutoff = Utc::now()
            - chrono::Duration::from_std(older_than).unwrap_or(chrono::Duration::days(7));

        let result = sqlx::query(
            r#"
            DELETE FROM pr_merge_queue
            WHERE repo_owner = $1 AND repo_name = $2
            AND status IN ('merged', 'skipped')
            AND updated_at < $3
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
    pub fn new(pool: PgPool, github: GitHubClient, git: GitOps, config: PrMergeConfig) -> Self {
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

    // =====================================================================
    // Database integration tests
    // =====================================================================

    use sqlx::PgPool;
    use tempfile::TempDir;

    async fn setup_test_db() -> PgPool {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for tests");
        crate::db::init_db_with_url(&url).await.unwrap()
    }

    #[tokio::test]
    async fn add_to_queue_returns_entry() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        let entry = mq.add_to_queue("owner", "repo", 1).await.unwrap();
        assert_eq!(entry.repo_owner, "owner");
        assert_eq!(entry.repo_name, "repo");
        assert_eq!(entry.pr_number, 1);
        assert_eq!(entry.queue_position, 1);
        assert_eq!(entry.status, QueueStatus::Pending);
        assert!(entry.conflict_info.is_none());
        assert!(entry.error_message.is_none());
    }

    #[tokio::test]
    async fn get_next_position_empty_then_after_add() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        // First PR gets position 1
        let e1 = mq.add_to_queue("owner", "repo", 1).await.unwrap();
        assert_eq!(e1.queue_position, 1);

        // Second PR gets position 2
        let e2 = mq.add_to_queue("owner", "repo", 2).await.unwrap();
        assert_eq!(e2.queue_position, 2);
    }

    #[tokio::test]
    async fn remove_from_queue_works() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo", 1).await.unwrap();
        let removed = mq.remove_from_queue("owner", "repo", 1).await.unwrap();
        assert!(removed);

        let entry = mq.get_entry("owner", "repo", 1).await.unwrap();
        assert!(entry.is_none());

        // Removing non-existent returns false
        let removed2 = mq.remove_from_queue("owner", "repo", 999).await.unwrap();
        assert!(!removed2);
    }

    #[tokio::test]
    async fn get_queue_returns_ordered() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo", 10).await.unwrap();
        mq.add_to_queue("owner", "repo", 20).await.unwrap();
        mq.add_to_queue("owner", "repo", 30).await.unwrap();

        let queue = mq.get_queue("owner", "repo").await.unwrap();
        assert_eq!(queue.len(), 3);
        assert_eq!(queue[0].pr_number, 10);
        assert_eq!(queue[1].pr_number, 20);
        assert_eq!(queue[2].pr_number, 30);
        assert_eq!(queue[0].queue_position, 1);
        assert_eq!(queue[1].queue_position, 2);
        assert_eq!(queue[2].queue_position, 3);
    }

    #[tokio::test]
    async fn get_queue_empty_repo() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        let queue = mq.get_queue("owner", "repo").await.unwrap();
        assert!(queue.is_empty());
    }

    #[tokio::test]
    async fn get_entry_found_and_not_found() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo", 42).await.unwrap();

        let found = mq.get_entry("owner", "repo", 42).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().pr_number, 42);

        let not_found = mq.get_entry("owner", "repo", 999).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn update_status_works() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo", 1).await.unwrap();

        let updated = mq
            .update_status("owner", "repo", 1, QueueStatus::InProgress, None)
            .await
            .unwrap();
        assert!(updated);

        let entry = mq.get_entry("owner", "repo", 1).await.unwrap().unwrap();
        assert_eq!(entry.status, QueueStatus::InProgress);
    }

    #[tokio::test]
    async fn update_status_with_error_message() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo", 1).await.unwrap();
        mq.update_status(
            "owner",
            "repo",
            1,
            QueueStatus::Failed,
            Some("something broke"),
        )
        .await
        .unwrap();

        let entry = mq.get_entry("owner", "repo", 1).await.unwrap().unwrap();
        assert_eq!(entry.status, QueueStatus::Failed);
        assert_eq!(entry.error_message.as_deref(), Some("something broke"));
    }

    #[tokio::test]
    async fn update_status_nonexistent_returns_false() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        let updated = mq
            .update_status("owner", "repo", 999, QueueStatus::Merged, None)
            .await
            .unwrap();
        assert!(!updated);
    }

    #[tokio::test]
    async fn set_conflict_info_works() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo", 1).await.unwrap();

        let info = ConflictInfoJson {
            files: vec!["src/main.rs".to_string(), "Cargo.toml".to_string()],
            detected_at: Utc::now(),
            needs_human_review: true,
        };

        let updated = mq
            .set_conflict_info("owner", "repo", 1, info)
            .await
            .unwrap();
        assert!(updated);

        let entry = mq.get_entry("owner", "repo", 1).await.unwrap().unwrap();
        assert_eq!(entry.status, QueueStatus::Conflict);
        let ci = entry.conflict_info.unwrap();
        assert_eq!(ci.files.len(), 2);
        assert!(ci.needs_human_review);
    }

    #[tokio::test]
    async fn set_conflict_info_nonexistent_returns_false() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        let info = ConflictInfoJson {
            files: vec![],
            detected_at: Utc::now(),
            needs_human_review: false,
        };
        let updated = mq
            .set_conflict_info("owner", "repo", 999, info)
            .await
            .unwrap();
        assert!(!updated);
    }

    #[tokio::test]
    async fn get_next_to_merge_returns_first_pending() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo", 1).await.unwrap();
        mq.add_to_queue("owner", "repo", 2).await.unwrap();
        mq.add_to_queue("owner", "repo", 3).await.unwrap();

        // Mark first as merged
        mq.update_status("owner", "repo", 1, QueueStatus::Merged, None)
            .await
            .unwrap();

        let next = mq.get_next_to_merge("owner", "repo").await.unwrap();
        assert_eq!(next.unwrap().pr_number, 2);
    }

    #[tokio::test]
    async fn get_next_to_merge_empty_queue() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        let next = mq.get_next_to_merge("owner", "repo").await.unwrap();
        assert!(next.is_none());
    }

    #[tokio::test]
    async fn get_next_to_merge_all_non_pending() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo", 1).await.unwrap();
        mq.update_status("owner", "repo", 1, QueueStatus::Merged, None)
            .await
            .unwrap();

        let next = mq.get_next_to_merge("owner", "repo").await.unwrap();
        assert!(next.is_none());
    }

    #[tokio::test]
    async fn can_merge_first_pending() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo", 1).await.unwrap();
        mq.add_to_queue("owner", "repo", 2).await.unwrap();

        assert!(mq.can_merge("owner", "repo", 1).await.unwrap());
        assert!(!mq.can_merge("owner", "repo", 2).await.unwrap());
    }

    #[tokio::test]
    async fn can_merge_no_pending() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        assert!(!mq.can_merge("owner", "repo", 1).await.unwrap());
    }

    #[tokio::test]
    async fn get_position_pending_entries() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo", 1).await.unwrap();
        mq.add_to_queue("owner", "repo", 2).await.unwrap();
        mq.add_to_queue("owner", "repo", 3).await.unwrap();

        assert_eq!(mq.get_position("owner", "repo", 1).await.unwrap(), Some(1));
        assert_eq!(mq.get_position("owner", "repo", 2).await.unwrap(), Some(2));
        assert_eq!(mq.get_position("owner", "repo", 3).await.unwrap(), Some(3));
    }

    #[tokio::test]
    async fn get_position_non_pending_returns_none() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo", 1).await.unwrap();
        mq.update_status("owner", "repo", 1, QueueStatus::Merged, None)
            .await
            .unwrap();

        assert_eq!(mq.get_position("owner", "repo", 1).await.unwrap(), None);
    }

    #[tokio::test]
    async fn get_position_not_in_queue() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        assert_eq!(mq.get_position("owner", "repo", 999).await.unwrap(), None);
    }

    #[tokio::test]
    async fn prs_ahead_counts_correctly() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo", 1).await.unwrap();
        mq.add_to_queue("owner", "repo", 2).await.unwrap();
        mq.add_to_queue("owner", "repo", 3).await.unwrap();

        assert_eq!(mq.prs_ahead("owner", "repo", 1).await.unwrap(), 0);
        assert_eq!(mq.prs_ahead("owner", "repo", 2).await.unwrap(), 1);
        assert_eq!(mq.prs_ahead("owner", "repo", 3).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn prs_ahead_not_in_queue() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        assert_eq!(mq.prs_ahead("owner", "repo", 999).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn compact_queue_renumbers_after_removal() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo", 1).await.unwrap();
        mq.add_to_queue("owner", "repo", 2).await.unwrap();
        mq.add_to_queue("owner", "repo", 3).await.unwrap();

        // Remove the middle one
        mq.remove_from_queue("owner", "repo", 2).await.unwrap();

        // Before compact, positions are 1 and 3
        let queue = mq.get_queue("owner", "repo").await.unwrap();
        assert_eq!(queue[0].queue_position, 1);
        assert_eq!(queue[1].queue_position, 3);

        // Compact
        mq.compact_queue("owner", "repo").await.unwrap();

        // After compact, positions are 1 and 2
        let queue = mq.get_queue("owner", "repo").await.unwrap();
        assert_eq!(queue.len(), 2);
        assert_eq!(queue[0].pr_number, 1);
        assert_eq!(queue[0].queue_position, 1);
        assert_eq!(queue[1].pr_number, 3);
        assert_eq!(queue[1].queue_position, 2);
    }

    #[tokio::test]
    async fn compact_queue_skips_non_pending() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo", 1).await.unwrap();
        mq.add_to_queue("owner", "repo", 2).await.unwrap();

        // Mark first as merged - compact should only renumber pending
        mq.update_status("owner", "repo", 1, QueueStatus::Merged, None)
            .await
            .unwrap();

        mq.compact_queue("owner", "repo").await.unwrap();

        // PR 2 should now be position 1 (renumbered among pending)
        let entry = mq.get_entry("owner", "repo", 2).await.unwrap().unwrap();
        assert_eq!(entry.queue_position, 1);
    }

    #[tokio::test]
    async fn get_queue_stats_counts_all_statuses() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo", 1).await.unwrap(); // pending
        mq.add_to_queue("owner", "repo", 2).await.unwrap();
        mq.update_status("owner", "repo", 2, QueueStatus::InProgress, None)
            .await
            .unwrap();
        mq.add_to_queue("owner", "repo", 3).await.unwrap();
        mq.update_status("owner", "repo", 3, QueueStatus::Merged, None)
            .await
            .unwrap();
        mq.add_to_queue("owner", "repo", 4).await.unwrap();
        mq.update_status("owner", "repo", 4, QueueStatus::Failed, Some("err"))
            .await
            .unwrap();
        mq.add_to_queue("owner", "repo", 5).await.unwrap();
        mq.set_conflict_info(
            "owner",
            "repo",
            5,
            ConflictInfoJson {
                files: vec!["f.rs".into()],
                detected_at: Utc::now(),
                needs_human_review: false,
            },
        )
        .await
        .unwrap();
        mq.add_to_queue("owner", "repo", 6).await.unwrap();
        mq.update_status("owner", "repo", 6, QueueStatus::Skipped, None)
            .await
            .unwrap();

        let stats = mq.get_queue_stats("owner", "repo").await.unwrap();
        assert_eq!(stats.total, 6);
        assert_eq!(stats.pending, 1);
        assert_eq!(stats.in_progress, 1);
        assert_eq!(stats.merged, 1);
        assert_eq!(stats.failed, 1);
        assert_eq!(stats.with_conflicts, 1);
        assert_eq!(stats.skipped, 1);
    }

    #[tokio::test]
    async fn get_queue_stats_empty() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        let stats = mq.get_queue_stats("owner", "repo").await.unwrap();
        assert_eq!(stats.total, 0);
    }

    #[tokio::test]
    async fn resume_processing_resets_in_progress() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo", 1).await.unwrap();
        mq.add_to_queue("owner", "repo", 2).await.unwrap();

        // Simulate interrupted state
        mq.update_status("owner", "repo", 1, QueueStatus::InProgress, None)
            .await
            .unwrap();

        let pending = mq.resume_processing("owner", "repo").await.unwrap();

        // Both should now be pending
        assert_eq!(pending.len(), 2);
        assert!(pending.iter().all(|e| e.status == QueueStatus::Pending));
    }

    #[tokio::test]
    async fn resume_processing_empty_queue() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        let pending = mq.resume_processing("owner", "repo").await.unwrap();
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn resume_processing_no_interrupted() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo", 1).await.unwrap();

        let pending = mq.resume_processing("owner", "repo").await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].pr_number, 1);
    }

    #[tokio::test]
    async fn get_needs_attention_conflict_and_failed() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo", 1).await.unwrap(); // pending - not attention
        mq.add_to_queue("owner", "repo", 2).await.unwrap();
        mq.set_conflict_info(
            "owner",
            "repo",
            2,
            ConflictInfoJson {
                files: vec!["a.rs".into()],
                detected_at: Utc::now(),
                needs_human_review: true,
            },
        )
        .await
        .unwrap();
        mq.add_to_queue("owner", "repo", 3).await.unwrap();
        mq.update_status("owner", "repo", 3, QueueStatus::Failed, Some("oops"))
            .await
            .unwrap();

        let attention = mq.get_needs_attention("owner", "repo").await.unwrap();
        assert_eq!(attention.len(), 2);
        let prs: Vec<u32> = attention.iter().map(|e| e.pr_number).collect();
        assert!(prs.contains(&2));
        assert!(prs.contains(&3));
    }

    #[tokio::test]
    async fn get_needs_attention_none() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo", 1).await.unwrap();

        let attention = mq.get_needs_attention("owner", "repo").await.unwrap();
        assert!(attention.is_empty());
    }

    #[tokio::test]
    async fn has_pending_work_empty() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        assert!(!mq.has_pending_work("owner", "repo").await.unwrap());
    }

    #[tokio::test]
    async fn has_pending_work_with_pending() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo", 1).await.unwrap();
        assert!(mq.has_pending_work("owner", "repo").await.unwrap());
    }

    #[tokio::test]
    async fn has_pending_work_with_in_progress() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo", 1).await.unwrap();
        mq.update_status("owner", "repo", 1, QueueStatus::InProgress, None)
            .await
            .unwrap();

        assert!(mq.has_pending_work("owner", "repo").await.unwrap());
    }

    #[tokio::test]
    async fn has_pending_work_only_completed() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo", 1).await.unwrap();
        mq.update_status("owner", "repo", 1, QueueStatus::Merged, None)
            .await
            .unwrap();

        assert!(!mq.has_pending_work("owner", "repo").await.unwrap());
    }

    #[tokio::test]
    async fn cleanup_old_entries_removes_old_merged() {
        let pool = setup_test_db().await;
        let pool2 = pool.clone();
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo", 1).await.unwrap();
        mq.update_status("owner", "repo", 1, QueueStatus::Merged, None)
            .await
            .unwrap();

        // Manually backdate the updated_at
        sqlx::query(
            "UPDATE pr_merge_queue SET updated_at = '2020-01-01T00:00:00+00:00' WHERE pr_number = 1",
        )
        .execute(&pool2)
        .await
        .unwrap();

        // Also add a pending entry that should NOT be removed
        mq.add_to_queue("owner", "repo", 2).await.unwrap();

        let deleted = mq
            .cleanup_old_entries("owner", "repo", std::time::Duration::from_secs(60))
            .await
            .unwrap();
        assert_eq!(deleted, 1);

        // Pending entry still there
        let queue = mq.get_queue("owner", "repo").await.unwrap();
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].pr_number, 2);
    }

    #[tokio::test]
    async fn cleanup_old_entries_keeps_recent() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo", 1).await.unwrap();
        mq.update_status("owner", "repo", 1, QueueStatus::Merged, None)
            .await
            .unwrap();

        // Don't backdate - entry is fresh
        let deleted = mq
            .cleanup_old_entries("owner", "repo", std::time::Duration::from_secs(3600))
            .await
            .unwrap();
        assert_eq!(deleted, 0);
    }

    #[tokio::test]
    async fn cleanup_old_entries_removes_skipped_too() {
        let pool = setup_test_db().await;
        let pool2 = pool.clone();
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo", 1).await.unwrap();
        mq.update_status("owner", "repo", 1, QueueStatus::Skipped, None)
            .await
            .unwrap();

        sqlx::query(
            "UPDATE pr_merge_queue SET updated_at = '2020-01-01T00:00:00+00:00' WHERE pr_number = 1",
        )
        .execute(&pool2)
        .await
        .unwrap();

        let deleted = mq
            .cleanup_old_entries("owner", "repo", std::time::Duration::from_secs(60))
            .await
            .unwrap();
        assert_eq!(deleted, 1);
    }

    #[tokio::test]
    async fn cleanup_does_not_remove_failed() {
        let pool = setup_test_db().await;
        let pool2 = pool.clone();
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo", 1).await.unwrap();
        mq.update_status("owner", "repo", 1, QueueStatus::Failed, Some("err"))
            .await
            .unwrap();

        sqlx::query(
            "UPDATE pr_merge_queue SET updated_at = '2020-01-01T00:00:00+00:00' WHERE pr_number = 1",
        )
        .execute(&pool2)
        .await
        .unwrap();

        let deleted = mq
            .cleanup_old_entries("owner", "repo", std::time::Duration::from_secs(60))
            .await
            .unwrap();
        assert_eq!(deleted, 0);
    }

    #[tokio::test]
    async fn add_duplicate_pr_updates_timestamp() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        let _e1 = mq.add_to_queue("owner", "repo", 1).await.unwrap();
        // Adding same PR again should hit ON CONFLICT and update
        let _e2 = mq.add_to_queue("owner", "repo", 1).await.unwrap();

        // Should still be only one entry
        let queue = mq.get_queue("owner", "repo").await.unwrap();
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].pr_number, 1);
    }

    #[tokio::test]
    async fn queues_are_isolated_by_repo() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo1", 1).await.unwrap();
        mq.add_to_queue("owner", "repo2", 1).await.unwrap();

        let q1 = mq.get_queue("owner", "repo1").await.unwrap();
        let q2 = mq.get_queue("owner", "repo2").await.unwrap();
        assert_eq!(q1.len(), 1);
        assert_eq!(q2.len(), 1);

        // Positions are independent
        assert_eq!(q1[0].queue_position, 1);
        assert_eq!(q2[0].queue_position, 1);
    }

    #[tokio::test]
    async fn compact_queue_empty_is_noop() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        // Should not error
        mq.compact_queue("owner", "repo").await.unwrap();
    }

    // =====================================================================
    // Additional coverage tests
    // =====================================================================

    #[test]
    fn queue_status_serde_json_roundtrip() {
        // Tests the serde rename_all = "snake_case" attribute
        for (status, expected_json) in [
            (QueueStatus::Pending, "\"pending\""),
            (QueueStatus::InProgress, "\"in_progress\""),
            (QueueStatus::Merged, "\"merged\""),
            (QueueStatus::Failed, "\"failed\""),
            (QueueStatus::Conflict, "\"conflict\""),
            (QueueStatus::Skipped, "\"skipped\""),
        ] {
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, expected_json);
            let parsed: QueueStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, status);
        }
    }

    #[test]
    fn queue_status_clone_copy_eq() {
        let s = QueueStatus::Pending;
        let s2 = s; // Copy
        let s3 = s.clone(); // Clone
        assert_eq!(s, s2);
        assert_eq!(s, s3);
    }

    #[test]
    fn queue_error_display_messages() {
        let err = QueueError::InvalidStatus("bogus".to_string());
        assert_eq!(err.to_string(), "invalid queue status: bogus");

        let err = QueueError::NotInQueue {
            owner: "alice".to_string(),
            repo: "myrepo".to_string(),
            pr_number: 42,
        };
        assert_eq!(err.to_string(), "PR #42 not in queue for alice/myrepo");

        let err = QueueError::OutOfOrder {
            pr_number: 5,
            next_in_queue: 3,
        };
        assert_eq!(
            err.to_string(),
            "cannot merge PR #5 out of order, PR #3 is next"
        );

        let err = QueueError::Other("something went wrong".to_string());
        assert_eq!(err.to_string(), "something went wrong");
    }

    #[test]
    fn queue_error_from_github_error() {
        let gh_err = crate::github::GitHubError::ConfigError("bad config".to_string());
        let q_err: QueueError = gh_err.into();
        assert!(q_err.to_string().contains("bad config"));
    }

    #[test]
    fn queue_error_debug_format() {
        let err = QueueError::InvalidStatus("xyz".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("InvalidStatus"));
        assert!(debug.contains("xyz"));
    }

    #[test]
    fn process_result_debug_variants() {
        let r = ProcessResult::Merged {
            sha: "abc123".to_string(),
        };
        let d = format!("{:?}", r);
        assert!(d.contains("Merged"));
        assert!(d.contains("abc123"));

        let r = ProcessResult::MergedAfterResolution {
            sha: "def456".to_string(),
            conflicts_resolved: 3,
        };
        let d = format!("{:?}", r);
        assert!(d.contains("MergedAfterResolution"));
        assert!(d.contains("def456"));
        assert!(d.contains("3"));

        let r = ProcessResult::NeedsHumanReview {
            files: vec!["a.rs".into(), "b.rs".into()],
        };
        let d = format!("{:?}", r);
        assert!(d.contains("NeedsHumanReview"));
        assert!(d.contains("a.rs"));

        let r = ProcessResult::Skipped {
            reason: "closed".to_string(),
        };
        let d = format!("{:?}", r);
        assert!(d.contains("Skipped"));
        assert!(d.contains("closed"));

        let r = ProcessResult::Failed {
            error: "timeout".to_string(),
        };
        let d = format!("{:?}", r);
        assert!(d.contains("Failed"));
        assert!(d.contains("timeout"));
    }

    #[test]
    fn notification_options_clone() {
        let opts = NotificationOptions {
            on_queued: false,
            on_merge_start: true,
            on_merged: false,
            on_conflicts: false,
            on_failed: false,
            on_position_change: true,
        };
        let cloned = opts.clone();
        assert_eq!(cloned.on_queued, false);
        assert_eq!(cloned.on_merge_start, true);
        assert_eq!(cloned.on_merged, false);
        assert_eq!(cloned.on_conflicts, false);
        assert_eq!(cloned.on_failed, false);
        assert_eq!(cloned.on_position_change, true);
    }

    #[test]
    fn notification_options_debug() {
        let opts = NotificationOptions::default();
        let d = format!("{:?}", opts);
        assert!(d.contains("NotificationOptions"));
        assert!(d.contains("on_queued"));
    }

    #[test]
    fn conflict_info_json_empty_files() {
        let info = ConflictInfoJson {
            files: vec![],
            detected_at: Utc::now(),
            needs_human_review: false,
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: ConflictInfoJson = serde_json::from_str(&json).unwrap();
        assert!(parsed.files.is_empty());
        assert!(!parsed.needs_human_review);
    }

    #[test]
    fn conflict_info_json_clone_and_debug() {
        let info = ConflictInfoJson {
            files: vec!["file.rs".into()],
            detected_at: Utc::now(),
            needs_human_review: true,
        };
        let cloned = info.clone();
        assert_eq!(cloned.files, info.files);
        assert_eq!(cloned.needs_human_review, info.needs_human_review);

        let d = format!("{:?}", info);
        assert!(d.contains("ConflictInfoJson"));
        assert!(d.contains("file.rs"));
    }

    #[test]
    fn pr_queue_entry_clone_and_debug() {
        let entry = PrQueueEntry {
            id: Uuid::new_v4(),
            repo_owner: "owner".to_string(),
            repo_name: "repo".to_string(),
            pr_number: 7,
            queue_position: 2,
            status: QueueStatus::Conflict,
            conflict_info: Some(ConflictInfoJson {
                files: vec!["x.rs".into()],
                detected_at: Utc::now(),
                needs_human_review: true,
            }),
            error_message: Some("conflict detected".into()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let cloned = entry.clone();
        assert_eq!(cloned.pr_number, 7);
        assert_eq!(cloned.status, QueueStatus::Conflict);
        assert!(cloned.conflict_info.is_some());
        assert_eq!(cloned.error_message.as_deref(), Some("conflict detected"));

        let d = format!("{:?}", entry);
        assert!(d.contains("PrQueueEntry"));
        assert!(d.contains("owner"));
    }

    #[test]
    fn queue_stats_debug_and_clone() {
        let stats = QueueStats {
            total: 10,
            pending: 3,
            in_progress: 2,
            merged: 4,
            failed: 1,
            with_conflicts: 0,
            skipped: 0,
        };
        let cloned = stats.clone();
        assert_eq!(cloned.total, 10);
        assert_eq!(cloned.pending, 3);
        assert_eq!(cloned.in_progress, 2);
        assert_eq!(cloned.merged, 4);
        assert_eq!(cloned.failed, 1);
        assert_eq!(cloned.with_conflicts, 0);
        assert_eq!(cloned.skipped, 0);

        let d = format!("{:?}", stats);
        assert!(d.contains("QueueStats"));
    }

    #[test]
    fn invalid_status_parse_various() {
        for bad in ["", "PENDING", "Pending", "in-progress", "123", "conflict "] {
            let result: Result<QueueStatus, _> = bad.parse();
            assert!(result.is_err(), "Expected error for {:?}", bad);
            let err = result.unwrap_err();
            assert!(err.to_string().contains(bad));
        }
    }

    #[test]
    fn merge_strategy_all_conversions() {
        // Exhaustive coverage of the From impl
        let pairs = [
            (MergeStrategy::Merge, MergeMethod::Merge),
            (MergeStrategy::Squash, MergeMethod::Squash),
            (MergeStrategy::Rebase, MergeMethod::Rebase),
        ];
        for (strategy, expected) in pairs {
            let method: MergeMethod = strategy.into();
            assert_eq!(method, expected);
        }
    }

    // =====================================================================
    // Additional DB integration tests for coverage
    // =====================================================================

    #[tokio::test]
    async fn add_three_then_remove_first_and_compact() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("o", "r", 1).await.unwrap();
        mq.add_to_queue("o", "r", 2).await.unwrap();
        mq.add_to_queue("o", "r", 3).await.unwrap();

        mq.remove_from_queue("o", "r", 1).await.unwrap();
        mq.compact_queue("o", "r").await.unwrap();

        let queue = mq.get_queue("o", "r").await.unwrap();
        assert_eq!(queue.len(), 2);
        assert_eq!(queue[0].pr_number, 2);
        assert_eq!(queue[0].queue_position, 1);
        assert_eq!(queue[1].pr_number, 3);
        assert_eq!(queue[1].queue_position, 2);
    }

    #[tokio::test]
    async fn compact_queue_already_correct_positions() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("o", "r", 1).await.unwrap();
        mq.add_to_queue("o", "r", 2).await.unwrap();

        // Positions are already 1,2 - compact should be a no-op
        mq.compact_queue("o", "r").await.unwrap();

        let queue = mq.get_queue("o", "r").await.unwrap();
        assert_eq!(queue[0].queue_position, 1);
        assert_eq!(queue[1].queue_position, 2);
    }

    #[tokio::test]
    async fn get_position_skips_non_pending_in_counting() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("o", "r", 1).await.unwrap();
        mq.add_to_queue("o", "r", 2).await.unwrap();
        mq.add_to_queue("o", "r", 3).await.unwrap();

        // Mark PR 1 as merged - it shouldn't count in position
        mq.update_status("o", "r", 1, QueueStatus::Merged, None)
            .await
            .unwrap();

        // PR 2 should be position 1 among pending
        assert_eq!(mq.get_position("o", "r", 2).await.unwrap(), Some(1));
        assert_eq!(mq.get_position("o", "r", 3).await.unwrap(), Some(2));
        // PR 1 is merged, not pending
        assert_eq!(mq.get_position("o", "r", 1).await.unwrap(), None);
    }

    #[tokio::test]
    async fn prs_ahead_after_front_merged() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("o", "r", 1).await.unwrap();
        mq.add_to_queue("o", "r", 2).await.unwrap();
        mq.add_to_queue("o", "r", 3).await.unwrap();

        mq.update_status("o", "r", 1, QueueStatus::Merged, None)
            .await
            .unwrap();

        // PR 2 is now first pending, 0 ahead
        assert_eq!(mq.prs_ahead("o", "r", 2).await.unwrap(), 0);
        // PR 3 has 1 ahead
        assert_eq!(mq.prs_ahead("o", "r", 3).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn can_merge_after_front_completed() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("o", "r", 1).await.unwrap();
        mq.add_to_queue("o", "r", 2).await.unwrap();
        mq.add_to_queue("o", "r", 3).await.unwrap();

        // Complete PR 1
        mq.update_status("o", "r", 1, QueueStatus::Merged, None)
            .await
            .unwrap();

        // Now PR 2 can merge
        assert!(mq.can_merge("o", "r", 2).await.unwrap());
        // PR 3 still can't
        assert!(!mq.can_merge("o", "r", 3).await.unwrap());
    }

    #[tokio::test]
    async fn get_needs_attention_empty_queue() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        let attention = mq.get_needs_attention("o", "r").await.unwrap();
        assert!(attention.is_empty());
    }

    #[tokio::test]
    async fn get_needs_attention_only_pending_and_merged() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("o", "r", 1).await.unwrap(); // pending
        mq.add_to_queue("o", "r", 2).await.unwrap();
        mq.update_status("o", "r", 2, QueueStatus::Merged, None)
            .await
            .unwrap();
        mq.add_to_queue("o", "r", 3).await.unwrap();
        mq.update_status("o", "r", 3, QueueStatus::InProgress, None)
            .await
            .unwrap();
        mq.add_to_queue("o", "r", 4).await.unwrap();
        mq.update_status("o", "r", 4, QueueStatus::Skipped, None)
            .await
            .unwrap();

        // None of those need attention
        let attention = mq.get_needs_attention("o", "r").await.unwrap();
        assert!(attention.is_empty());
    }

    #[tokio::test]
    async fn has_pending_work_with_conflict_and_failed() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("o", "r", 1).await.unwrap();
        mq.set_conflict_info(
            "o",
            "r",
            1,
            ConflictInfoJson {
                files: vec![],
                detected_at: Utc::now(),
                needs_human_review: false,
            },
        )
        .await
        .unwrap();

        mq.add_to_queue("o", "r", 2).await.unwrap();
        mq.update_status("o", "r", 2, QueueStatus::Failed, Some("err"))
            .await
            .unwrap();

        // Neither conflict nor failed count as pending work
        assert!(!mq.has_pending_work("o", "r").await.unwrap());
    }

    #[tokio::test]
    async fn has_pending_work_with_skipped() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("o", "r", 1).await.unwrap();
        mq.update_status("o", "r", 1, QueueStatus::Skipped, None)
            .await
            .unwrap();

        assert!(!mq.has_pending_work("o", "r").await.unwrap());
    }

    #[tokio::test]
    async fn cleanup_old_entries_empty_queue() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        let deleted = mq
            .cleanup_old_entries("o", "r", std::time::Duration::from_secs(60))
            .await
            .unwrap();
        assert_eq!(deleted, 0);
    }

    #[tokio::test]
    async fn cleanup_does_not_remove_conflict_or_in_progress() {
        let pool = setup_test_db().await;
        let pool2 = pool.clone();
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("o", "r", 1).await.unwrap();
        mq.set_conflict_info(
            "o",
            "r",
            1,
            ConflictInfoJson {
                files: vec!["a.rs".into()],
                detected_at: Utc::now(),
                needs_human_review: true,
            },
        )
        .await
        .unwrap();

        mq.add_to_queue("o", "r", 2).await.unwrap();
        mq.update_status("o", "r", 2, QueueStatus::InProgress, None)
            .await
            .unwrap();

        // Backdate both
        sqlx::query("UPDATE pr_merge_queue SET updated_at = '2020-01-01T00:00:00+00:00'")
            .execute(&pool2)
            .await
            .unwrap();

        let deleted = mq
            .cleanup_old_entries("o", "r", std::time::Duration::from_secs(60))
            .await
            .unwrap();
        assert_eq!(deleted, 0);
    }

    #[tokio::test]
    async fn cleanup_multiple_old_entries() {
        let pool = setup_test_db().await;
        let pool2 = pool.clone();
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("o", "r", 1).await.unwrap();
        mq.update_status("o", "r", 1, QueueStatus::Merged, None)
            .await
            .unwrap();
        mq.add_to_queue("o", "r", 2).await.unwrap();
        mq.update_status("o", "r", 2, QueueStatus::Skipped, None)
            .await
            .unwrap();
        mq.add_to_queue("o", "r", 3).await.unwrap();
        mq.update_status("o", "r", 3, QueueStatus::Merged, None)
            .await
            .unwrap();

        // Backdate all
        sqlx::query("UPDATE pr_merge_queue SET updated_at = '2020-01-01T00:00:00+00:00'")
            .execute(&pool2)
            .await
            .unwrap();

        let deleted = mq
            .cleanup_old_entries("o", "r", std::time::Duration::from_secs(60))
            .await
            .unwrap();
        assert_eq!(deleted, 3);
    }

    #[tokio::test]
    async fn queues_isolated_by_owner() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("alice", "repo", 1).await.unwrap();
        mq.add_to_queue("bob", "repo", 1).await.unwrap();

        let q1 = mq.get_queue("alice", "repo").await.unwrap();
        let q2 = mq.get_queue("bob", "repo").await.unwrap();
        assert_eq!(q1.len(), 1);
        assert_eq!(q2.len(), 1);

        // Operations on one don't affect the other
        mq.update_status("alice", "repo", 1, QueueStatus::Merged, None)
            .await
            .unwrap();
        assert!(mq.can_merge("bob", "repo", 1).await.unwrap());
    }

    #[tokio::test]
    async fn update_status_all_statuses() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        // Test cycling through all statuses
        mq.add_to_queue("o", "r", 1).await.unwrap();

        for status in [
            QueueStatus::InProgress,
            QueueStatus::Conflict,
            QueueStatus::Failed,
            QueueStatus::Skipped,
            QueueStatus::Merged,
            QueueStatus::Pending, // back to pending
        ] {
            let updated = mq.update_status("o", "r", 1, status, None).await.unwrap();
            assert!(updated);
            let entry = mq.get_entry("o", "r", 1).await.unwrap().unwrap();
            assert_eq!(entry.status, status);
        }
    }

    #[tokio::test]
    async fn set_conflict_info_overwrites_previous() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("o", "r", 1).await.unwrap();

        let info1 = ConflictInfoJson {
            files: vec!["a.rs".into()],
            detected_at: Utc::now(),
            needs_human_review: false,
        };
        mq.set_conflict_info("o", "r", 1, info1).await.unwrap();

        let info2 = ConflictInfoJson {
            files: vec!["b.rs".into(), "c.rs".into()],
            detected_at: Utc::now(),
            needs_human_review: true,
        };
        mq.set_conflict_info("o", "r", 1, info2).await.unwrap();

        let entry = mq.get_entry("o", "r", 1).await.unwrap().unwrap();
        let ci = entry.conflict_info.unwrap();
        assert_eq!(ci.files.len(), 2);
        assert!(ci.needs_human_review);
    }

    #[tokio::test]
    async fn compact_queue_with_mixed_statuses() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("o", "r", 1).await.unwrap(); // pos 1
        mq.add_to_queue("o", "r", 2).await.unwrap(); // pos 2
        mq.add_to_queue("o", "r", 3).await.unwrap(); // pos 3
        mq.add_to_queue("o", "r", 4).await.unwrap(); // pos 4
        mq.add_to_queue("o", "r", 5).await.unwrap(); // pos 5

        // Set various non-pending statuses
        mq.update_status("o", "r", 1, QueueStatus::Merged, None)
            .await
            .unwrap();
        mq.update_status("o", "r", 3, QueueStatus::Failed, Some("err"))
            .await
            .unwrap();
        mq.update_status("o", "r", 5, QueueStatus::Skipped, None)
            .await
            .unwrap();

        // Remove merged entry to create gap
        mq.remove_from_queue("o", "r", 1).await.unwrap();

        // Compact should only renumber pending entries (2 and 4)
        mq.compact_queue("o", "r").await.unwrap();

        let e2 = mq.get_entry("o", "r", 2).await.unwrap().unwrap();
        let e4 = mq.get_entry("o", "r", 4).await.unwrap().unwrap();
        assert_eq!(e2.queue_position, 1);
        assert_eq!(e4.queue_position, 2);
    }

    #[tokio::test]
    async fn resume_processing_only_returns_pending_not_other_statuses() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("o", "r", 1).await.unwrap();
        mq.add_to_queue("o", "r", 2).await.unwrap();
        mq.add_to_queue("o", "r", 3).await.unwrap();
        mq.add_to_queue("o", "r", 4).await.unwrap();

        mq.update_status("o", "r", 1, QueueStatus::Merged, None)
            .await
            .unwrap();
        mq.update_status("o", "r", 2, QueueStatus::Failed, Some("x"))
            .await
            .unwrap();
        mq.set_conflict_info(
            "o",
            "r",
            3,
            ConflictInfoJson {
                files: vec![],
                detected_at: Utc::now(),
                needs_human_review: false,
            },
        )
        .await
        .unwrap();
        // PR 4 remains pending

        let pending = mq.resume_processing("o", "r").await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].pr_number, 4);
    }

    #[tokio::test]
    async fn get_queue_different_repos_independent() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("o", "repo-a", 1).await.unwrap();
        mq.add_to_queue("o", "repo-a", 2).await.unwrap();
        mq.add_to_queue("o", "repo-b", 10).await.unwrap();

        let qa = mq.get_queue("o", "repo-a").await.unwrap();
        let qb = mq.get_queue("o", "repo-b").await.unwrap();
        assert_eq!(qa.len(), 2);
        assert_eq!(qb.len(), 1);
        assert_eq!(qb[0].pr_number, 10);
        assert_eq!(qb[0].queue_position, 1);
    }

    #[tokio::test]
    async fn remove_from_queue_different_repo_no_effect() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("o", "repo-a", 1).await.unwrap();

        // Remove from different repo - should not affect repo-a
        let removed = mq.remove_from_queue("o", "repo-b", 1).await.unwrap();
        assert!(!removed);

        let queue = mq.get_queue("o", "repo-a").await.unwrap();
        assert_eq!(queue.len(), 1);
    }

    #[tokio::test]
    async fn get_next_to_merge_skips_conflict_and_failed() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("o", "r", 1).await.unwrap();
        mq.add_to_queue("o", "r", 2).await.unwrap();
        mq.add_to_queue("o", "r", 3).await.unwrap();

        mq.set_conflict_info(
            "o",
            "r",
            1,
            ConflictInfoJson {
                files: vec!["x.rs".into()],
                detected_at: Utc::now(),
                needs_human_review: true,
            },
        )
        .await
        .unwrap();
        mq.update_status("o", "r", 2, QueueStatus::Failed, Some("e"))
            .await
            .unwrap();

        // PR 3 is the first pending
        let next = mq.get_next_to_merge("o", "r").await.unwrap();
        assert_eq!(next.unwrap().pr_number, 3);
    }

    #[tokio::test]
    async fn can_merge_wrong_pr_returns_false() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("o", "r", 1).await.unwrap();
        mq.add_to_queue("o", "r", 2).await.unwrap();

        // PR 2 can't merge because PR 1 is first
        assert!(!mq.can_merge("o", "r", 2).await.unwrap());
        // Non-existent PR
        assert!(!mq.can_merge("o", "r", 999).await.unwrap());
    }

    #[tokio::test]
    async fn cleanup_with_zero_duration() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("o", "r", 1).await.unwrap();
        mq.update_status("o", "r", 1, QueueStatus::Merged, None)
            .await
            .unwrap();

        // Zero duration - should remove anything older than "now"
        // The entry was just created so it might or might not be cleaned
        // depending on timing, but it should not error
        let _deleted = mq
            .cleanup_old_entries("o", "r", std::time::Duration::from_secs(0))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn get_queue_stats_all_pending() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("o", "r", 1).await.unwrap();
        mq.add_to_queue("o", "r", 2).await.unwrap();
        mq.add_to_queue("o", "r", 3).await.unwrap();

        let stats = mq.get_queue_stats("o", "r").await.unwrap();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.pending, 3);
        assert_eq!(stats.in_progress, 0);
        assert_eq!(stats.merged, 0);
        assert_eq!(stats.failed, 0);
        assert_eq!(stats.with_conflicts, 0);
        assert_eq!(stats.skipped, 0);
    }

    #[tokio::test]
    async fn entry_has_correct_timestamps() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        let before = Utc::now();
        mq.add_to_queue("o", "r", 1).await.unwrap();
        let after = Utc::now();

        let entry = mq.get_entry("o", "r", 1).await.unwrap().unwrap();
        assert!(entry.created_at >= before && entry.created_at <= after);
        assert!(entry.updated_at >= before && entry.updated_at <= after);
    }

    #[tokio::test]
    async fn update_status_changes_updated_at() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("o", "r", 1).await.unwrap();
        let entry1 = mq.get_entry("o", "r", 1).await.unwrap().unwrap();

        // Small delay to ensure different timestamp
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        mq.update_status("o", "r", 1, QueueStatus::InProgress, None)
            .await
            .unwrap();
        let entry2 = mq.get_entry("o", "r", 1).await.unwrap().unwrap();

        assert!(entry2.updated_at >= entry1.updated_at);
    }

    #[tokio::test]
    async fn reset_interrupted_via_resume_multiple() {
        let pool = setup_test_db().await;
        let mq = MergeQueue::new(pool);

        mq.add_to_queue("owner", "repo", 1).await.unwrap();
        mq.add_to_queue("owner", "repo", 2).await.unwrap();
        mq.add_to_queue("owner", "repo", 3).await.unwrap();

        // Mark two as in_progress
        mq.update_status("owner", "repo", 1, QueueStatus::InProgress, None)
            .await
            .unwrap();
        mq.update_status("owner", "repo", 2, QueueStatus::InProgress, None)
            .await
            .unwrap();
        // Mark one as failed (should NOT be reset)
        mq.update_status("owner", "repo", 3, QueueStatus::Failed, Some("bad"))
            .await
            .unwrap();

        let pending = mq.resume_processing("owner", "repo").await.unwrap();
        // Only the two previously in_progress + none failed
        assert_eq!(pending.len(), 2);

        // Verify the failed one is still failed
        let entry = mq.get_entry("owner", "repo", 3).await.unwrap().unwrap();
        assert_eq!(entry.status, QueueStatus::Failed);
    }
}
