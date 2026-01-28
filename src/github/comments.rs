//! GitHub issue commenting for progress updates

use crate::github::{CreateIssueComment, GitHubClient, GitHubComment, GitHubError};
use crate::types::{TaskStatus, Ticket, TicketSource, VerticalSlice};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum CommentError {
    #[error("cannot comment on manually created ticket - no GitHub issue associated")]
    ManualTicket,

    #[error("github API error: {0}")]
    GitHubError(#[from] GitHubError),
}

/// Progress for a single slice
#[derive(Debug, Clone)]
pub struct SliceProgress {
    pub title: String,
    pub status: TaskStatus,
    pub task_count: usize,
    pub tasks_completed: usize,
}

/// Summary of progress on a ticket
#[derive(Debug)]
pub struct ProgressSummary {
    pub ticket_title: String,
    pub slices: Vec<SliceProgress>,
    pub current_activity: Option<String>,
    pub errors: Vec<String>,
}

impl ProgressSummary {
    pub fn new(ticket_title: impl Into<String>) -> Self {
        Self {
            ticket_title: ticket_title.into(),
            slices: Vec::new(),
            current_activity: None,
            errors: Vec::new(),
        }
    }

    /// Add a slice's progress
    pub fn add_slice(
        mut self,
        title: impl Into<String>,
        status: TaskStatus,
        task_count: usize,
        tasks_completed: usize,
    ) -> Self {
        self.slices.push(SliceProgress {
            title: title.into(),
            status,
            task_count,
            tasks_completed,
        });
        self
    }

    /// Add multiple slices from slice data
    pub fn with_slices(mut self, slices: Vec<SliceProgress>) -> Self {
        self.slices = slices;
        self
    }

    pub fn with_activity(mut self, activity: impl Into<String>) -> Self {
        self.current_activity = Some(activity.into());
        self
    }

    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.errors.push(error.into());
        self
    }

    /// Build progress info from a slice
    pub fn from_slice(slice: &VerticalSlice) -> SliceProgress {
        SliceProgress {
            title: slice.title.clone(),
            status: slice.status,
            task_count: slice.tasks.len(),
            tasks_completed: 0, // Would need task details to calculate
        }
    }

    /// Generate markdown-formatted progress report
    pub fn generate_markdown(&self) -> String {
        let mut md = String::new();

        // Header
        md.push_str("## nexor Progress Update\n\n");

        // Overall progress
        let completed = self
            .slices
            .iter()
            .filter(|s| s.status == TaskStatus::Completed)
            .count();
        let total = self.slices.len();

        if total > 0 {
            md.push_str(&format!(
                "**Progress:** {} of {} slices complete\n\n",
                completed, total
            ));

            // Progress bar
            let pct = (completed * 100) / total;
            let filled = (completed * 10) / total;
            let bar: String = (0..10)
                .map(|i| if i < filled { '█' } else { '░' })
                .collect();
            md.push_str(&format!("`[{}]` {}%\n\n", bar, pct));
        }

        // Slice status
        if !self.slices.is_empty() {
            md.push_str("### Slices\n\n");
            for slice in &self.slices {
                let emoji = match slice.status {
                    TaskStatus::Completed => "✅",
                    TaskStatus::InProgress => "🔄",
                    TaskStatus::Review => "👀",
                    TaskStatus::Failed => "❌",
                    TaskStatus::Pending => "⏳",
                };

                if slice.task_count > 0 {
                    md.push_str(&format!(
                        "{} **{}** ({}/{})\n",
                        emoji, slice.title, slice.tasks_completed, slice.task_count
                    ));
                } else {
                    md.push_str(&format!("{} **{}**\n", emoji, slice.title));
                }
            }
            md.push('\n');
        }

        // Current activity
        if let Some(ref activity) = self.current_activity {
            md.push_str(&format!("**Currently:** {}\n\n", activity));
        }

        // Errors
        if !self.errors.is_empty() {
            md.push_str("### Issues\n\n");
            for error in &self.errors {
                md.push_str(&format!("- ⚠️ {}\n", error));
            }
            md.push('\n');
        }

        // Footer
        md.push_str("---\n");
        md.push_str("*Updated by [nexor](https://github.com/nexor)*\n");

        md
    }
}

/// Service for posting progress comments to GitHub issues
pub struct CommentService {
    client: GitHubClient,
    /// Track comment IDs for updating instead of creating new
    progress_comments: HashMap<Uuid, u64>, // ticket_id -> comment_id
}

impl CommentService {
    pub fn new(client: GitHubClient) -> Self {
        Self {
            client,
            progress_comments: HashMap::new(),
        }
    }

    /// Get GitHub issue reference from ticket source
    fn get_issue_ref(&self, ticket: &Ticket) -> Result<(String, String, u32), CommentError> {
        match &ticket.source {
            TicketSource::GitHub {
                owner,
                repo,
                issue_number,
            } => Ok((owner.clone(), repo.clone(), *issue_number)),
            TicketSource::Manual => Err(CommentError::ManualTicket),
        }
    }

    /// Post initial comment when work begins
    pub async fn on_work_started(
        &mut self,
        ticket: &Ticket,
        slice_count: usize,
    ) -> Result<GitHubComment, CommentError> {
        let (owner, repo, number) = self.get_issue_ref(ticket)?;

        let body = format!(
            "## 🤖 nexor is working on this\n\n\
            I've started working on this issue. I'll post updates as slices are completed.\n\n\
            **Planned slices:** {}\n\n\
            ---\n*Automated by [nexor](https://github.com/nexor)*",
            slice_count
        );

        let comment = self
            .client
            .create_issue_comment(&owner, &repo, number, &CreateIssueComment { body })
            .await?;

        // Store comment ID for future updates
        self.progress_comments.insert(ticket.id.0, comment.id);

        tracing::info!(
            ticket_id = %ticket.id.0,
            comment_id = comment.id,
            "Posted work started comment"
        );

        Ok(comment)
    }

    /// Update progress when a slice is completed
    pub async fn on_progress_update(
        &mut self,
        ticket: &Ticket,
        summary: &ProgressSummary,
    ) -> Result<GitHubComment, CommentError> {
        let (owner, repo, number) = self.get_issue_ref(ticket)?;

        let body = summary.generate_markdown();

        // Check if we have an existing comment to update
        if let Some(&comment_id) = self.progress_comments.get(&ticket.id.0) {
            let comment = self
                .client
                .update_issue_comment(&owner, &repo, comment_id, &CreateIssueComment { body })
                .await?;

            tracing::info!(
                ticket_id = %ticket.id.0,
                comment_id,
                "Updated progress comment"
            );

            Ok(comment)
        } else {
            // Create new comment
            let comment = self
                .client
                .create_issue_comment(&owner, &repo, number, &CreateIssueComment { body })
                .await?;

            self.progress_comments.insert(ticket.id.0, comment.id);

            tracing::info!(
                ticket_id = %ticket.id.0,
                comment_id = comment.id,
                "Posted new progress comment"
            );

            Ok(comment)
        }
    }

    /// Post final comment when ticket is complete
    pub async fn on_ticket_completed(
        &mut self,
        ticket: &Ticket,
        pr_url: Option<&str>,
    ) -> Result<GitHubComment, CommentError> {
        let (owner, repo, number) = self.get_issue_ref(ticket)?;

        let pr_section = if let Some(url) = pr_url {
            format!("\n\n**Pull Request:** {}", url)
        } else {
            String::new()
        };

        let body = format!(
            "## ✅ Work Complete\n\n\
            All slices have been completed for this issue.{}\n\n\
            ---\n*Automated by [nexor](https://github.com/nexor)*",
            pr_section
        );

        let comment = self
            .client
            .create_issue_comment(&owner, &repo, number, &CreateIssueComment { body })
            .await?;

        // Remove from tracking
        self.progress_comments.remove(&ticket.id.0);

        tracing::info!(ticket_id = %ticket.id.0, "Posted completion comment");

        Ok(comment)
    }

    /// Post an error notification
    pub async fn on_error(
        &self,
        ticket: &Ticket,
        error_msg: &str,
    ) -> Result<GitHubComment, CommentError> {
        let (owner, repo, number) = self.get_issue_ref(ticket)?;

        let body = format!(
            "## ⚠️ Issue Encountered\n\n\
            An error occurred while working on this issue:\n\n\
            ```\n{}\n```\n\n\
            Human review may be needed.\n\n\
            ---\n*Automated by [nexor](https://github.com/nexor)*",
            error_msg
        );

        let comment = self
            .client
            .create_issue_comment(&owner, &repo, number, &CreateIssueComment { body })
            .await?;

        tracing::warn!(ticket_id = %ticket.id.0, "Posted error comment");

        Ok(comment)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TicketId;
    use chrono::Utc;

    #[test]
    fn progress_summary_basic() {
        let summary = ProgressSummary::new("Test ticket")
            .add_slice("Slice 1", TaskStatus::Completed, 3, 3)
            .add_slice("Slice 2", TaskStatus::InProgress, 4, 2);

        let md = summary.generate_markdown();

        assert!(md.contains("## nexor Progress Update"));
        assert!(md.contains("1 of 2 slices complete"));
        assert!(md.contains("✅ **Slice 1**"));
        assert!(md.contains("🔄 **Slice 2**"));
    }

    #[test]
    fn progress_summary_with_activity() {
        let summary = ProgressSummary::new("Test")
            .add_slice("Slice 1", TaskStatus::InProgress, 2, 1)
            .with_activity("Implementing authentication");

        let md = summary.generate_markdown();

        assert!(md.contains("**Currently:** Implementing authentication"));
    }

    #[test]
    fn progress_summary_with_errors() {
        let summary =
            ProgressSummary::new("Test").with_error("Test failed: assertion error at line 42");

        let md = summary.generate_markdown();

        assert!(md.contains("### Issues"));
        assert!(md.contains("⚠️ Test failed"));
    }

    #[test]
    fn progress_bar_calculation() {
        // 0/4 complete = 0%
        let summary = ProgressSummary::new("Test")
            .add_slice("S1", TaskStatus::Pending, 1, 0)
            .add_slice("S2", TaskStatus::Pending, 1, 0)
            .add_slice("S3", TaskStatus::Pending, 1, 0)
            .add_slice("S4", TaskStatus::Pending, 1, 0);

        let md = summary.generate_markdown();
        assert!(md.contains("0 of 4"));
        assert!(md.contains("0%"));

        // 2/4 complete = 50%
        let summary = ProgressSummary::new("Test")
            .add_slice("S1", TaskStatus::Completed, 1, 1)
            .add_slice("S2", TaskStatus::Completed, 1, 1)
            .add_slice("S3", TaskStatus::Pending, 1, 0)
            .add_slice("S4", TaskStatus::Pending, 1, 0);

        let md = summary.generate_markdown();
        assert!(md.contains("2 of 4"));
        assert!(md.contains("50%"));
    }

    #[test]
    fn all_status_emojis() {
        let summary = ProgressSummary::new("Test")
            .add_slice("Completed", TaskStatus::Completed, 0, 0)
            .add_slice("In Progress", TaskStatus::InProgress, 0, 0)
            .add_slice("Review", TaskStatus::Review, 0, 0)
            .add_slice("Failed", TaskStatus::Failed, 0, 0)
            .add_slice("Pending", TaskStatus::Pending, 0, 0);

        let md = summary.generate_markdown();

        assert!(md.contains("✅ **Completed**"));
        assert!(md.contains("🔄 **In Progress**"));
        assert!(md.contains("👀 **Review**"));
        assert!(md.contains("❌ **Failed**"));
        assert!(md.contains("⏳ **Pending**"));
    }

    #[test]
    fn slice_progress_from_vertical_slice() {
        let slice = VerticalSlice {
            id: crate::types::SliceId::new(),
            ticket_id: uuid::Uuid::new_v4(),
            title: "Test slice".to_string(),
            description: "Description".to_string(),
            tasks: vec![crate::types::TaskId::new(), crate::types::TaskId::new()],
            status: TaskStatus::InProgress,
            created_at: Utc::now(),
        };

        let progress = ProgressSummary::from_slice(&slice);

        assert_eq!(progress.title, "Test slice");
        assert_eq!(progress.status, TaskStatus::InProgress);
        assert_eq!(progress.task_count, 2);
    }
}
