//! Review output schema for orchestrator code reviews.

use serde::{Deserialize, Serialize};

/// Output from the orchestrator when reviewing work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewOutput {
    /// The reviewer's thinking process
    pub thinking: String,

    /// The review verdict
    pub verdict: ReviewVerdict,

    /// Issues found during review
    #[serde(default)]
    pub issues: Vec<ReviewIssue>,

    /// Positive notes about what was done well
    #[serde(default)]
    pub positive_notes: Vec<String>,

    /// Brief overall assessment
    pub summary: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewVerdict {
    /// Work is approved as-is
    Approved,
    /// Changes are needed before approval
    ChangesRequested,
    /// Issue is too complex, escalate to human
    Escalate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewIssue {
    /// How serious is this issue
    pub severity: IssueSeverity,

    /// File where the issue was found
    pub file: String,

    /// Line number (if applicable)
    #[serde(default)]
    pub line: Option<u32>,

    /// Description of the problem
    pub description: String,

    /// Suggested fix
    pub suggestion: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    /// Must be fixed before approval
    Blocking,
    /// Should be considered but not required
    Suggestion,
}

impl ReviewOutput {
    /// Validate the review output
    pub fn validate(&self) -> Result<(), ReviewValidationError> {
        if self.thinking.is_empty() {
            return Err(ReviewValidationError::MissingThinking);
        }

        if self.summary.is_empty() {
            return Err(ReviewValidationError::MissingSummary);
        }

        // If changes requested, must have at least one blocking issue
        if self.verdict == ReviewVerdict::ChangesRequested {
            let has_blocking = self
                .issues
                .iter()
                .any(|i| i.severity == IssueSeverity::Blocking);
            if !has_blocking {
                return Err(ReviewValidationError::ChangesWithoutBlockingIssue);
            }
        }

        // If approved, should not have blocking issues
        if self.verdict == ReviewVerdict::Approved {
            let has_blocking = self
                .issues
                .iter()
                .any(|i| i.severity == IssueSeverity::Blocking);
            if has_blocking {
                return Err(ReviewValidationError::ApprovedWithBlockingIssue);
            }
        }

        // Each issue must have description and suggestion
        for issue in &self.issues {
            if issue.description.is_empty() {
                return Err(ReviewValidationError::EmptyIssueDescription);
            }
            if issue.suggestion.is_empty() {
                return Err(ReviewValidationError::EmptyIssueSuggestion);
            }
        }

        Ok(())
    }

    /// Check if the review has any blocking issues
    pub fn has_blocking_issues(&self) -> bool {
        self.issues
            .iter()
            .any(|i| i.severity == IssueSeverity::Blocking)
    }

    /// Get blocking issues only
    pub fn blocking_issues(&self) -> Vec<&ReviewIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Blocking)
            .collect()
    }
}

#[derive(Debug, Clone)]
pub enum ReviewValidationError {
    MissingThinking,
    MissingSummary,
    ChangesWithoutBlockingIssue,
    ApprovedWithBlockingIssue,
    EmptyIssueDescription,
    EmptyIssueSuggestion,
}

impl std::fmt::Display for ReviewValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingThinking => write!(f, "Missing 'thinking' field"),
            Self::MissingSummary => write!(f, "Missing 'summary' field"),
            Self::ChangesWithoutBlockingIssue => {
                write!(
                    f,
                    "Verdict is 'changes_requested' but no blocking issues listed"
                )
            }
            Self::ApprovedWithBlockingIssue => {
                write!(f, "Verdict is 'approved' but has blocking issues")
            }
            Self::EmptyIssueDescription => write!(f, "Issue has empty description"),
            Self::EmptyIssueSuggestion => write!(f, "Issue has empty suggestion"),
        }
    }
}

impl std::error::Error for ReviewValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_approved_output() -> ReviewOutput {
        ReviewOutput {
            thinking: "Reviewing the code...".to_string(),
            verdict: ReviewVerdict::Approved,
            issues: vec![],
            positive_notes: vec!["Clean code".to_string()],
            summary: "Code looks good".to_string(),
        }
    }

    fn create_changes_requested_output() -> ReviewOutput {
        ReviewOutput {
            thinking: "Found some issues...".to_string(),
            verdict: ReviewVerdict::ChangesRequested,
            issues: vec![ReviewIssue {
                severity: IssueSeverity::Blocking,
                file: "src/main.rs".to_string(),
                line: Some(10),
                description: "Missing error handling".to_string(),
                suggestion: "Add Result return type".to_string(),
            }],
            positive_notes: vec![],
            summary: "Needs error handling".to_string(),
        }
    }

    #[test]
    fn test_valid_approved() {
        let output = create_approved_output();
        assert!(output.validate().is_ok());
    }

    #[test]
    fn test_valid_changes_requested() {
        let output = create_changes_requested_output();
        assert!(output.validate().is_ok());
    }

    #[test]
    fn test_missing_thinking() {
        let mut output = create_approved_output();
        output.thinking = String::new();
        let result = output.validate();
        assert!(matches!(
            result,
            Err(ReviewValidationError::MissingThinking)
        ));
    }

    #[test]
    fn test_missing_summary() {
        let mut output = create_approved_output();
        output.summary = String::new();
        let result = output.validate();
        assert!(matches!(result, Err(ReviewValidationError::MissingSummary)));
    }

    #[test]
    fn test_changes_requested_without_blocking_issue() {
        let mut output = create_approved_output();
        output.verdict = ReviewVerdict::ChangesRequested;
        output.issues = vec![ReviewIssue {
            severity: IssueSeverity::Suggestion,
            file: "src/main.rs".to_string(),
            line: None,
            description: "Could use better names".to_string(),
            suggestion: "Rename variable".to_string(),
        }];
        let result = output.validate();
        assert!(matches!(
            result,
            Err(ReviewValidationError::ChangesWithoutBlockingIssue)
        ));
    }

    #[test]
    fn test_approved_with_blocking_issue() {
        let mut output = create_approved_output();
        output.issues = vec![ReviewIssue {
            severity: IssueSeverity::Blocking,
            file: "src/main.rs".to_string(),
            line: Some(5),
            description: "Critical bug".to_string(),
            suggestion: "Fix the bug".to_string(),
        }];
        let result = output.validate();
        assert!(matches!(
            result,
            Err(ReviewValidationError::ApprovedWithBlockingIssue)
        ));
    }

    #[test]
    fn test_approved_with_suggestion_is_ok() {
        let mut output = create_approved_output();
        output.issues = vec![ReviewIssue {
            severity: IssueSeverity::Suggestion,
            file: "src/main.rs".to_string(),
            line: None,
            description: "Minor improvement".to_string(),
            suggestion: "Consider refactoring".to_string(),
        }];
        assert!(output.validate().is_ok());
    }

    #[test]
    fn test_empty_issue_description() {
        let mut output = create_changes_requested_output();
        output.issues[0].description = String::new();
        let result = output.validate();
        assert!(matches!(
            result,
            Err(ReviewValidationError::EmptyIssueDescription)
        ));
    }

    #[test]
    fn test_empty_issue_suggestion() {
        let mut output = create_changes_requested_output();
        output.issues[0].suggestion = String::new();
        let result = output.validate();
        assert!(matches!(
            result,
            Err(ReviewValidationError::EmptyIssueSuggestion)
        ));
    }

    #[test]
    fn test_has_blocking_issues() {
        let output = create_changes_requested_output();
        assert!(output.has_blocking_issues());

        let output = create_approved_output();
        assert!(!output.has_blocking_issues());
    }

    #[test]
    fn test_blocking_issues_filter() {
        let mut output = create_changes_requested_output();
        output.issues.push(ReviewIssue {
            severity: IssueSeverity::Suggestion,
            file: "src/lib.rs".to_string(),
            line: None,
            description: "Minor".to_string(),
            suggestion: "Fix".to_string(),
        });

        let blocking = output.blocking_issues();
        assert_eq!(blocking.len(), 1);
        assert_eq!(blocking[0].file, "src/main.rs");
    }

    #[test]
    fn test_json_serialization() {
        let output = create_approved_output();
        let json = serde_json::to_string(&output).unwrap();
        let parsed: ReviewOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.thinking, output.thinking);
    }

    #[test]
    fn test_verdict_serialization() {
        let verdict = ReviewVerdict::ChangesRequested;
        let json = serde_json::to_string(&verdict).unwrap();
        assert_eq!(json, "\"changes_requested\"");

        let verdict = ReviewVerdict::Escalate;
        let json = serde_json::to_string(&verdict).unwrap();
        assert_eq!(json, "\"escalate\"");
    }

    #[test]
    fn test_severity_serialization() {
        let severity = IssueSeverity::Blocking;
        let json = serde_json::to_string(&severity).unwrap();
        assert_eq!(json, "\"blocking\"");
    }

    #[test]
    fn test_escalate_verdict() {
        let output = ReviewOutput {
            thinking: "This is too complex...".to_string(),
            verdict: ReviewVerdict::Escalate,
            issues: vec![],
            positive_notes: vec![],
            summary: "Needs human review".to_string(),
        };
        assert!(output.validate().is_ok());
    }
}
