//! Task result output schema for worker task completions.

use serde::{Deserialize, Serialize};

/// Output from a worker when completing a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResultOutput {
    /// Current phase of the task
    pub phase: TaskPhase,

    /// The agent's thinking/reasoning
    pub thinking: String,

    /// Implementation plan (filled in planning phase)
    #[serde(default)]
    pub plan: Option<TaskPlan>,

    /// Progress tracking
    pub progress: TaskProgress,

    /// Code changes made
    #[serde(default)]
    pub code_changes: Vec<CodeChange>,

    /// Files/info the agent still needs
    #[serde(default)]
    pub context_requests: Vec<String>,

    /// Self-verification results
    #[serde(default)]
    pub verification: Option<Verification>,

    /// Current status
    pub status: TaskStatus,

    /// If blocked, why
    #[serde(default)]
    pub blocked_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskPhase {
    Planning,
    Implementing,
    Reviewing,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    /// Brief description of approach
    pub approach: String,

    /// Files that will be modified
    #[serde(default)]
    pub files_to_modify: Vec<String>,

    /// New files that will be created
    #[serde(default)]
    pub files_to_create: Vec<String>,

    /// Estimated complexity
    pub estimated_complexity: String, // "low" | "medium" | "high"
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskProgress {
    /// What's being worked on now
    #[serde(default)]
    pub current_step: String,

    /// Completed items
    #[serde(default)]
    pub completed_steps: Vec<String>,

    /// Remaining items
    #[serde(default)]
    pub remaining_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChange {
    /// File path
    pub file: String,

    /// Type of change
    pub action: ChangeAction,

    /// Full file content or diff
    pub content: String,

    /// Why this change was made
    pub explanation: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChangeAction {
    Create,
    Modify,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verification {
    /// Requirements and how they're satisfied
    #[serde(default)]
    pub requirements_met: Vec<RequirementCheck>,

    /// Tests that were added
    #[serde(default)]
    pub tests_added: Vec<String>,

    /// Known issues or limitations
    #[serde(default)]
    pub potential_issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementCheck {
    /// The requirement
    pub requirement: String,

    /// How it's satisfied
    pub evidence: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    NeedsContext,
    InProgress,
    ReadyForReview,
    Blocked,
}

impl TaskResultOutput {
    /// Validate the task result output
    pub fn validate(&self) -> Result<(), TaskResultValidationError> {
        if self.thinking.is_empty() {
            return Err(TaskResultValidationError::MissingThinking);
        }

        if self.status == TaskStatus::Blocked && self.blocked_reason.is_none() {
            return Err(TaskResultValidationError::MissingBlockedReason);
        }

        if self.status == TaskStatus::ReadyForReview {
            // Must have code changes to be ready for review
            if self.code_changes.is_empty() {
                return Err(TaskResultValidationError::NoChangesForReview);
            }

            // Should have verification
            if self.verification.is_none() {
                return Err(TaskResultValidationError::MissingVerification);
            }
        }

        // Validate code changes have content
        for change in &self.code_changes {
            if change.content.is_empty() && change.action != ChangeAction::Delete {
                return Err(TaskResultValidationError::EmptyContent(change.file.clone()));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum TaskResultValidationError {
    MissingThinking,
    MissingBlockedReason,
    NoChangesForReview,
    MissingVerification,
    EmptyContent(String),
}

impl std::fmt::Display for TaskResultValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingThinking => write!(f, "Missing 'thinking' field"),
            Self::MissingBlockedReason => write!(f, "Status is 'blocked' but no reason provided"),
            Self::NoChangesForReview => {
                write!(f, "Status is 'ready_for_review' but no code changes")
            }
            Self::MissingVerification => {
                write!(f, "Status is 'ready_for_review' but no verification")
            }
            Self::EmptyContent(file) => write!(f, "Code change for '{}' has empty content", file),
        }
    }
}

impl std::error::Error for TaskResultValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_valid_output() -> TaskResultOutput {
        TaskResultOutput {
            phase: TaskPhase::Implementing,
            thinking: "Working on the implementation...".to_string(),
            plan: Some(TaskPlan {
                approach: "Modify the existing code".to_string(),
                files_to_modify: vec!["src/main.rs".to_string()],
                files_to_create: vec![],
                estimated_complexity: "medium".to_string(),
            }),
            progress: TaskProgress {
                current_step: "Updating function".to_string(),
                completed_steps: vec!["Read file".to_string()],
                remaining_steps: vec!["Test".to_string()],
            },
            code_changes: vec![],
            context_requests: vec![],
            verification: None,
            status: TaskStatus::InProgress,
            blocked_reason: None,
        }
    }

    fn create_ready_for_review_output() -> TaskResultOutput {
        TaskResultOutput {
            phase: TaskPhase::Complete,
            thinking: "Implementation complete".to_string(),
            plan: None,
            progress: TaskProgress::default(),
            code_changes: vec![CodeChange {
                file: "src/main.rs".to_string(),
                action: ChangeAction::Modify,
                content: "fn main() {}".to_string(),
                explanation: "Added main function".to_string(),
            }],
            context_requests: vec![],
            verification: Some(Verification {
                requirements_met: vec![RequirementCheck {
                    requirement: "Function exists".to_string(),
                    evidence: "Added fn main".to_string(),
                }],
                tests_added: vec![],
                potential_issues: vec![],
            }),
            status: TaskStatus::ReadyForReview,
            blocked_reason: None,
        }
    }

    #[test]
    fn test_valid_in_progress() {
        let output = create_valid_output();
        assert!(output.validate().is_ok());
    }

    #[test]
    fn test_valid_ready_for_review() {
        let output = create_ready_for_review_output();
        assert!(output.validate().is_ok());
    }

    #[test]
    fn test_missing_thinking() {
        let mut output = create_valid_output();
        output.thinking = String::new();
        let result = output.validate();
        assert!(matches!(
            result,
            Err(TaskResultValidationError::MissingThinking)
        ));
    }

    #[test]
    fn test_blocked_without_reason() {
        let mut output = create_valid_output();
        output.status = TaskStatus::Blocked;
        let result = output.validate();
        assert!(matches!(
            result,
            Err(TaskResultValidationError::MissingBlockedReason)
        ));
    }

    #[test]
    fn test_blocked_with_reason_is_ok() {
        let mut output = create_valid_output();
        output.status = TaskStatus::Blocked;
        output.blocked_reason = Some("Missing dependency".to_string());
        assert!(output.validate().is_ok());
    }

    #[test]
    fn test_ready_for_review_without_changes() {
        let mut output = create_valid_output();
        output.status = TaskStatus::ReadyForReview;
        output.verification = Some(Verification {
            requirements_met: vec![],
            tests_added: vec![],
            potential_issues: vec![],
        });
        let result = output.validate();
        assert!(matches!(
            result,
            Err(TaskResultValidationError::NoChangesForReview)
        ));
    }

    #[test]
    fn test_ready_for_review_without_verification() {
        let mut output = create_ready_for_review_output();
        output.verification = None;
        let result = output.validate();
        assert!(matches!(
            result,
            Err(TaskResultValidationError::MissingVerification)
        ));
    }

    #[test]
    fn test_empty_content_on_create() {
        let mut output = create_valid_output();
        output.code_changes = vec![CodeChange {
            file: "src/new.rs".to_string(),
            action: ChangeAction::Create,
            content: String::new(),
            explanation: "New file".to_string(),
        }];
        let result = output.validate();
        assert!(matches!(
            result,
            Err(TaskResultValidationError::EmptyContent(_))
        ));
    }

    #[test]
    fn test_empty_content_on_delete_is_ok() {
        let mut output = create_valid_output();
        output.code_changes = vec![CodeChange {
            file: "src/old.rs".to_string(),
            action: ChangeAction::Delete,
            content: String::new(),
            explanation: "Removing old file".to_string(),
        }];
        assert!(output.validate().is_ok());
    }

    #[test]
    fn test_json_serialization() {
        let output = create_valid_output();
        let json = serde_json::to_string(&output).unwrap();
        let parsed: TaskResultOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.thinking, output.thinking);
    }

    #[test]
    fn test_phase_serialization() {
        let phase = TaskPhase::Planning;
        let json = serde_json::to_string(&phase).unwrap();
        assert_eq!(json, "\"planning\"");
    }

    #[test]
    fn test_status_serialization() {
        let status = TaskStatus::NeedsContext;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"needs_context\"");
    }

    #[test]
    fn test_action_serialization() {
        let action = ChangeAction::Create;
        let json = serde_json::to_string(&action).unwrap();
        assert_eq!(json, "\"create\"");
    }
}
