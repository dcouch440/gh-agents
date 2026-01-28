//! Review examples demonstrating constructive, specific feedback.

use super::Example;

/// Examples demonstrating good code review patterns.
pub struct ReviewExamples;

impl ReviewExamples {
    /// Get all review examples
    pub fn all() -> Vec<Example> {
        vec![
            Self::approval_example(),
            Self::changes_requested_example(),
            Self::escalation_example(),
        ]
    }

    fn approval_example() -> Example {
        Example {
            title: "Approving Good Work".to_string(),
            domain: "review".to_string(),
            keywords: vec!["approve", "good", "accept"],
            input: r#"Review: Added email validation function with tests"#.to_string(),
            thinking: r#"CORRECTNESS: Function validates emails using standard regex pattern. Tests cover valid and invalid cases.

INTEGRATION: Uses lazy_static for regex compilation - good for performance, won't break anything.

QUALITY: Code is clean, well-named, follows project conventions.

COMPLETENESS: Has tests for happy path and common invalid cases. Could add more edge cases but current coverage is acceptable.

VERDICT: This is good work. Approve."#
                .to_string(),
            output: r#"{
  "thinking": "Solid implementation with good test coverage. No issues found.",
  "verdict": "approved",
  "issues": [],
  "positive_notes": [
    "Good use of lazy_static for compiled regex",
    "Clear function name and signature",
    "Tests cover both valid and invalid cases"
  ],
  "summary": "Clean implementation of email validation. Ready to merge."
}"#
            .to_string(),
            explanation: "When approving, explain WHY it's good. This reinforces good patterns."
                .to_string(),
        }
    }

    fn changes_requested_example() -> Example {
        Example {
            title: "Requesting Specific Changes".to_string(),
            domain: "review".to_string(),
            keywords: vec!["changes", "fix", "issue"],
            input: r#"Review: Added user deletion endpoint"#.to_string(),
            thinking: r#"CORRECTNESS: Endpoint deletes users, but permanently. Task didn't specify soft delete but that's usually expected.

INTEGRATION: No authorization check - any user can delete any user!

QUALITY: Error handling is missing. Will panic on DB error.

COMPLETENESS: No tests.

VERDICT: Has critical security issue and missing error handling. Request changes."#
                .to_string(),
            output: r#"{
  "thinking": "Found authorization issue and missing error handling",
  "verdict": "changes_requested",
  "issues": [
    {
      "severity": "blocking",
      "file": "src/routes/users.rs",
      "line": 45,
      "description": "No authorization check - anyone can delete any user",
      "suggestion": "Add auth middleware and verify user can only delete themselves (or is admin)"
    },
    {
      "severity": "blocking",
      "file": "src/routes/users.rs",
      "line": 52,
      "description": "Using unwrap() on database result - will panic on error",
      "suggestion": "Use ? operator and return appropriate HTTP error"
    },
    {
      "severity": "suggestion",
      "file": "src/routes/users.rs",
      "line": null,
      "description": "No tests for deletion endpoint",
      "suggestion": "Add tests for: successful delete, unauthorized delete, not found"
    }
  ],
  "positive_notes": [
    "Endpoint structure is correct",
    "Route path follows REST conventions"
  ],
  "summary": "Critical auth issue must be fixed. Add error handling and tests."
}"#
            .to_string(),
            explanation: "Specific issues with file/line, clear severity levels, actionable suggestions. Still noted what was good.".to_string(),
        }
    }

    fn escalation_example() -> Example {
        Example {
            title: "Escalating to Human".to_string(),
            domain: "review".to_string(),
            keywords: vec!["escalate", "human", "unclear"],
            input: r#"Review: Refactored payment processing module"#.to_string(),
            thinking: r#"CORRECTNESS: This changes core payment flow. I can verify it compiles but can't verify business logic.

INTEGRATION: Multiple services affected. Could have subtle billing implications.

QUALITY: Code looks fine structurally.

COMPLETENESS: Tests pass but I'm not confident they cover all billing scenarios.

VERDICT: This is high-risk financial code. Needs human review."#
                .to_string(),
            output: r#"{
  "thinking": "Payment processing changes need human verification of business logic",
  "verdict": "escalate",
  "issues": [
    {
      "severity": "blocking",
      "file": "src/payments/processor.rs",
      "line": null,
      "description": "Changes to payment calculation logic - cannot verify business correctness",
      "suggestion": "Human reviewer should verify calculation matches billing requirements"
    }
  ],
  "positive_notes": [
    "Code structure is clean",
    "Tests pass",
    "Error handling looks appropriate"
  ],
  "summary": "Code quality is fine but payment logic changes need human verification. Escalating."
}"#
            .to_string(),
            explanation:
                "Honest about limitations. High-risk business logic should be human-verified."
                    .to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_examples_have_content() {
        let examples = ReviewExamples::all();
        assert_eq!(examples.len(), 3);

        for ex in &examples {
            assert!(!ex.title.is_empty());
            assert!(!ex.input.is_empty());
            assert!(!ex.output.is_empty());
        }
    }

    #[test]
    fn covers_all_verdicts() {
        let examples = ReviewExamples::all();

        assert!(examples.iter().any(|e| e.output.contains("\"approved\"")));
        assert!(examples
            .iter()
            .any(|e| e.output.contains("\"changes_requested\"")));
        assert!(examples.iter().any(|e| e.output.contains("\"escalate\"")));
    }

    #[test]
    fn positive_notes_included_in_rejections() {
        let examples = ReviewExamples::all();
        let changes_example = examples
            .iter()
            .find(|e| e.title.contains("Changes"))
            .unwrap();

        assert!(changes_example.output.contains("positive_notes"));
    }
}
