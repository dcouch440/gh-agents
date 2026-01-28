//! Recovery prompt templates for handling various failure scenarios.

use crate::prompts::{PromptBuilder, PromptVersion};

/// Recovery prompt templates for handling failures.
pub struct RecoveryPrompts;

impl RecoveryPrompts {
    // =========================================================================
    // Slice 4.9.1: Parse Error Recovery
    // =========================================================================

    /// Current version for parse error recovery
    pub fn parse_error_version() -> PromptVersion {
        PromptVersion::new("recovery-parse", 1, 0, 0)
    }

    /// Build a prompt for recovering from a parse error.
    ///
    /// # Arguments
    /// * `original_output` - The output that couldn't be parsed
    /// * `parse_error` - The specific error message
    /// * `expected_schema` - The schema the output should match
    pub fn parse_error_recovery(
        original_output: &str,
        parse_error: &str,
        expected_schema: &str,
    ) -> PromptBuilder {
        // Truncate original output if too long
        let truncated_output = if original_output.len() > 2000 {
            format!("{}...[truncated]", &original_output[..2000])
        } else {
            original_output.to_string()
        };

        PromptBuilder::new()
            .version(Self::parse_error_version())
            .role(RECOVERY_ROLE)
            .task(format!(
                r#"Your previous output couldn't be processed. Here's what happened:

**Your output**:
```
{}
```

**Error**: {}

**Expected schema**:
```json
{}
```

Please regenerate your response with valid JSON matching the schema."#,
                truncated_output, parse_error, expected_schema
            ))
            .constraint(PARSE_ERROR_GUIDANCE)
            .constraint("Output ONLY the JSON, no explanation before or after")
            .constraint("Make sure all braces and brackets are properly closed")
            .output_json(expected_schema)
    }

    // =========================================================================
    // Slice 4.9.2: Test Failure Analysis
    // =========================================================================

    /// Current version for test failure recovery
    pub fn test_failure_version() -> PromptVersion {
        PromptVersion::new("recovery-test", 1, 0, 0)
    }

    /// Build a prompt for analyzing test failures.
    ///
    /// # Arguments
    /// * `test_name` - Name of the failing test
    /// * `test_output` - The test output/error
    /// * `test_code` - The test code itself
    /// * `implementation_code` - The code being tested
    pub fn test_failure_analysis(
        test_name: &str,
        test_output: &str,
        test_code: &str,
        implementation_code: &str,
    ) -> PromptBuilder {
        PromptBuilder::new()
            .version(Self::test_failure_version())
            .role(RECOVERY_ROLE)
            .task(format!(
                r#"A test is failing. Analyze the failure and determine how to fix it.

**Failing test**: `{}`

**Test output**:
```
{}
```

**Test code**:
```
{}
```

**Implementation being tested**:
```
{}
```

Determine whether the bug is in the test or the implementation, then explain the fix."#,
                test_name, test_output, test_code, implementation_code
            ))
            .constraint(TEST_FAILURE_GUIDANCE)
            .output_json(TEST_FAILURE_SCHEMA)
    }

    // =========================================================================
    // Slice 4.9.3: Review Rejection Recovery
    // =========================================================================

    /// Current version for review rejection recovery
    pub fn review_rejection_version() -> PromptVersion {
        PromptVersion::new("recovery-review", 1, 0, 0)
    }

    /// Build a prompt for addressing review feedback.
    ///
    /// # Arguments
    /// * `original_submission` - The code that was submitted
    /// * `review_feedback` - The feedback from the reviewer
    /// * `original_requirements` - The original task requirements
    pub fn review_rejection_recovery(
        original_submission: &str,
        review_feedback: &ReviewFeedback,
        original_requirements: &str,
    ) -> PromptBuilder {
        let issues_text = review_feedback
            .issues
            .iter()
            .enumerate()
            .map(|(i, issue)| {
                format!(
                    "{}. [{}] {} ({})\n   Suggestion: {}",
                    i + 1,
                    issue.severity,
                    issue.description,
                    issue.location.as_deref().unwrap_or("general"),
                    issue.suggestion
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        PromptBuilder::new()
            .version(Self::review_rejection_version())
            .role(RECOVERY_ROLE)
            .task(format!(
                r#"Your submission was returned with feedback. Address the issues and resubmit.

**Original requirements**:
{}

**Your submission**:
```
{}
```

**Review feedback**:
{}

**Issues to address**:
{}

Revise your code to address ALL blocking issues."#,
                original_requirements, original_submission, review_feedback.summary, issues_text
            ))
            .constraint(REVIEW_RECOVERY_GUIDANCE)
            .output_json(REVIEW_RECOVERY_SCHEMA)
    }

    // =========================================================================
    // Slice 4.9.4: Stuck Loop Breakout
    // =========================================================================

    /// Current version for stuck loop recovery
    pub fn stuck_loop_version() -> PromptVersion {
        PromptVersion::new("recovery-stuck", 1, 0, 0)
    }

    /// Build a prompt for breaking out of a stuck loop.
    ///
    /// # Arguments
    /// * `task_description` - What was being attempted
    /// * `attempts` - List of previous attempts and their results
    /// * `pattern_description` - Description of the repetitive pattern
    pub fn stuck_loop_breakout(
        task_description: &str,
        attempts: &[AttemptRecord],
        pattern_description: &str,
    ) -> PromptBuilder {
        let attempts_text = attempts
            .iter()
            .enumerate()
            .map(|(i, a)| {
                format!(
                    "**Attempt {}**:\n  Action: {}\n  Result: {}\n  Error: {}",
                    i + 1,
                    a.action,
                    a.result,
                    a.error.as_deref().unwrap_or("none")
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        PromptBuilder::new()
            .version(Self::stuck_loop_version())
            .role(RECOVERY_ROLE)
            .task(format!(
                r#"You appear to be stuck in a loop. Let's break out of it.

**Task**: {}

**Pattern detected**: {}

**Your attempts**:
{}

Step back and consider a different approach."#,
                task_description, pattern_description, attempts_text
            ))
            .constraint(STUCK_LOOP_GUIDANCE)
            .output_json(STUCK_LOOP_SCHEMA)
    }

    // =========================================================================
    // Slice 4.9.5: Conflicting Requirements
    // =========================================================================

    /// Current version for conflicting requirements
    pub fn conflict_version() -> PromptVersion {
        PromptVersion::new("recovery-conflict", 1, 0, 0)
    }

    /// Build a prompt for clarifying conflicting requirements.
    ///
    /// # Arguments
    /// * `requirements` - The original requirements
    /// * `conflicts_found` - Specific conflicts identified
    pub fn conflicting_requirements(
        requirements: &str,
        conflicts_found: &[RequirementConflict],
    ) -> PromptBuilder {
        let conflicts_text = conflicts_found
            .iter()
            .enumerate()
            .map(|(i, c)| {
                format!(
                    "{}. **{}** vs **{}**\n   Issue: {}",
                    i + 1,
                    c.requirement_a,
                    c.requirement_b,
                    c.conflict_description
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        PromptBuilder::new()
            .version(Self::conflict_version())
            .role(RECOVERY_ROLE)
            .task(format!(
                r#"The requirements contain conflicts or ambiguities that need clarification.

**Original requirements**:
{}

**Conflicts identified**:
{}

Generate specific questions to resolve these conflicts."#,
                requirements, conflicts_text
            ))
            .constraint(CONFLICT_GUIDANCE)
            .output_json(CONFLICT_SCHEMA)
    }
}

// =============================================================================
// Supporting Types
// =============================================================================

/// Review feedback structure for recovery prompts
#[derive(Debug, Clone)]
pub struct ReviewFeedback {
    pub verdict: String,
    pub summary: String,
    pub issues: Vec<ReviewIssue>,
}

impl ReviewFeedback {
    pub fn new(verdict: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            verdict: verdict.into(),
            summary: summary.into(),
            issues: Vec::new(),
        }
    }

    pub fn with_issue(mut self, issue: ReviewIssue) -> Self {
        self.issues.push(issue);
        self
    }
}

#[derive(Debug, Clone)]
pub struct ReviewIssue {
    /// "blocking" | "suggestion"
    pub severity: String,
    pub description: String,
    pub location: Option<String>,
    pub suggestion: String,
}

impl ReviewIssue {
    pub fn blocking(description: impl Into<String>, suggestion: impl Into<String>) -> Self {
        Self {
            severity: "blocking".to_string(),
            description: description.into(),
            location: None,
            suggestion: suggestion.into(),
        }
    }

    pub fn suggestion(description: impl Into<String>, suggestion: impl Into<String>) -> Self {
        Self {
            severity: "suggestion".to_string(),
            description: description.into(),
            location: None,
            suggestion: suggestion.into(),
        }
    }

    pub fn at_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }
}

/// Record of a previous attempt
#[derive(Debug, Clone)]
pub struct AttemptRecord {
    pub action: String,
    pub result: String,
    pub error: Option<String>,
}

impl AttemptRecord {
    pub fn new(action: impl Into<String>, result: impl Into<String>) -> Self {
        Self {
            action: action.into(),
            result: result.into(),
            error: None,
        }
    }

    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }
}

/// A conflict between requirements
#[derive(Debug, Clone)]
pub struct RequirementConflict {
    pub requirement_a: String,
    pub requirement_b: String,
    pub conflict_description: String,
}

impl RequirementConflict {
    pub fn new(
        requirement_a: impl Into<String>,
        requirement_b: impl Into<String>,
        conflict_description: impl Into<String>,
    ) -> Self {
        Self {
            requirement_a: requirement_a.into(),
            requirement_b: requirement_b.into(),
            conflict_description: conflict_description.into(),
        }
    }
}

// =============================================================================
// Constants
// =============================================================================

const RECOVERY_ROLE: &str = r#"You are correcting a previous response that had an issue.

Focus on:
1. Understanding what went wrong
2. Fixing the specific issue
3. Producing correct output this time

Be precise. Don't make the same mistake again."#;

const PARSE_ERROR_GUIDANCE: &str = r#"Common JSON mistakes to avoid:
- Missing closing braces `}` or brackets `]`
- Trailing commas after the last item
- Unescaped special characters in strings
- Using single quotes instead of double quotes
- Missing quotes around property names"#;

const TEST_FAILURE_GUIDANCE: &str = r#"Analyze systematically:

1. READ the test assertion that failed
2. UNDERSTAND what the test expected vs what it got
3. TRACE backward: why did the code produce that result?
4. DETERMINE: is the test wrong, or is the implementation wrong?
5. PROPOSE: specific fix with code changes

Common causes:
- Off-by-one errors
- Null/None handling
- Edge cases (empty input, boundary values)
- Async timing issues
- Wrong assumptions about input format"#;

const TEST_FAILURE_SCHEMA: &str = r#"{
  "analysis": {
    "expected": "string - What the test expected",
    "actual": "string - What the test got",
    "root_cause": "string - Why this mismatch occurred"
  },
  "bug_location": "test | implementation",
  "bug_description": "string - Specific description of the bug",
  "fix": {
    "file": "string - Which file to modify",
    "description": "string - What change to make",
    "code_change": "string - The actual code fix"
  },
  "confidence": "high | medium | low",
  "needs_more_context": "boolean",
  "context_needed": "string | null - What additional context would help"
}"#;

const REVIEW_RECOVERY_GUIDANCE: &str = r#"Address feedback systematically:

1. READ each issue carefully
2. UNDERSTAND what the reviewer is asking for
3. ADDRESS blocking issues first (these must be fixed)
4. CONSIDER suggestions (these are optional but often good)
5. VERIFY your changes actually fix the issues

Don't:
- Ignore feedback hoping it wasn't important
- Make unrelated changes
- Break something else while fixing

Do:
- Address each blocking issue explicitly
- Explain what you changed and why"#;

const REVIEW_RECOVERY_SCHEMA: &str = r#"{
  "understanding": "string - Your understanding of the feedback",
  "changes_made": [
    {
      "issue_number": "number - Which issue this addresses",
      "change_description": "string - What you changed",
      "file": "string - Which file",
      "before": "string - Code before (relevant snippet)",
      "after": "string - Code after"
    }
  ],
  "revised_code": "string - The full revised code",
  "unaddressed_issues": [
    {
      "issue_number": "number",
      "reason": "string - Why this wasn't addressed"
    }
  ],
  "ready_for_review": "boolean"
}"#;

const STUCK_LOOP_GUIDANCE: &str = r#"Break out of the loop:

1. STOP trying the same thing
2. ANALYZE why your attempts aren't working
3. CONSIDER fundamentally different approaches:
   - Different algorithm or strategy
   - Simplify the problem
   - Ask for help/context
   - Break into smaller pieces
4. DECIDE: try new approach or escalate

If you've tried 3+ genuinely different approaches, escalate.
Don't keep trying variations of the same failing idea."#;

const STUCK_LOOP_SCHEMA: &str = r#"{
  "loop_analysis": {
    "what_was_repeated": "string - The pattern you were stuck in",
    "why_it_failed": "string - Root cause of repeated failures"
  },
  "alternative_approaches": [
    {
      "approach": "string - A different way to solve this",
      "likelihood_of_success": "high | medium | low",
      "reason": "string - Why this might work"
    }
  ],
  "recommendation": "try_alternative | simplify | escalate",
  "next_action": "string - Specific next step",
  "if_escalating": {
    "summary_for_human": "string - Clear summary of the problem",
    "what_was_tried": "string - Attempts made",
    "help_needed": "string - Specific help requested"
  }
}"#;

const CONFLICT_GUIDANCE: &str = r#"Clarify conflicts effectively:

1. IDENTIFY the specific tension between requirements
2. EXPLAIN why both can't be true simultaneously
3. GENERATE questions that would resolve the ambiguity
4. SUGGEST possible resolutions if one seems more likely

Good clarifying questions:
- Are specific and answerable
- Address one conflict at a time
- Offer options when possible
- Explain why you're asking

Bad questions:
- "Is this right?" (too vague)
- "What should I do?" (puts burden on user)
- Multiple questions combined"#;

const CONFLICT_SCHEMA: &str = r#"{
  "conflicts_analysis": [
    {
      "conflict_number": "number",
      "interpretation_a": "string - One way to interpret this",
      "interpretation_b": "string - Another way to interpret this",
      "impact": "string - What would be different depending on interpretation"
    }
  ],
  "clarifying_questions": [
    {
      "question": "string - The specific question to ask",
      "addresses_conflict": "number - Which conflict this addresses",
      "options": ["array of possible answers to suggest"],
      "default_suggestion": "string | null - What you'd assume if no answer"
    }
  ],
  "assumptions_if_no_clarification": [
    {
      "conflict_number": "number",
      "assumption": "string - What you'd assume",
      "risk": "string - What could go wrong with this assumption"
    }
  ],
  "can_proceed_with_assumptions": "boolean"
}"#;

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Parse Error Recovery Tests
    // =========================================================================

    #[test]
    fn test_parse_error_version() {
        let version = RecoveryPrompts::parse_error_version();
        assert_eq!(version.family, "recovery-parse");
    }

    #[test]
    fn test_parse_error_recovery_includes_output() {
        let prompt = RecoveryPrompts::parse_error_recovery(
            r#"{"incomplete": true"#,
            "unexpected end of input",
            r#"{"complete": "boolean"}"#,
        );

        let built = prompt.build();
        assert!(built.text.contains("incomplete"));
        assert!(built.text.contains("unexpected end of input"));
    }

    #[test]
    fn test_parse_error_recovery_includes_schema() {
        let prompt = RecoveryPrompts::parse_error_recovery(
            "bad output",
            "error",
            r#"{"expected": "schema"}"#,
        );

        let built = prompt.build();
        assert!(built.text.contains("expected"));
        assert!(built.text.contains("schema"));
    }

    #[test]
    fn test_parse_error_recovery_truncates_long_output() {
        let long_output = "x".repeat(3000);
        let prompt =
            RecoveryPrompts::parse_error_recovery(&long_output, "error", r#"{"test": "schema"}"#);

        let built = prompt.build();
        assert!(built.text.contains("...[truncated]"));
        assert!(built.text.len() < long_output.len() + 1000);
    }

    #[test]
    fn test_parse_error_recovery_includes_guidance() {
        let prompt = RecoveryPrompts::parse_error_recovery("output", "error", "{}");

        let built = prompt.build();
        assert!(built.text.contains("Missing closing braces"));
        assert!(built.text.contains("Trailing commas"));
    }

    // =========================================================================
    // Test Failure Analysis Tests
    // =========================================================================

    #[test]
    fn test_test_failure_version() {
        let version = RecoveryPrompts::test_failure_version();
        assert_eq!(version.family, "recovery-test");
    }

    #[test]
    fn test_test_failure_analysis_includes_test_info() {
        let prompt = RecoveryPrompts::test_failure_analysis(
            "test_user_login",
            "assertion failed: expected 200, got 401",
            "assert_eq!(response.status(), 200);",
            "fn login() { ... }",
        );

        let built = prompt.build();
        assert!(built.text.contains("test_user_login"));
        assert!(built.text.contains("expected 200, got 401"));
    }

    #[test]
    fn test_test_failure_analysis_includes_code() {
        let prompt = RecoveryPrompts::test_failure_analysis(
            "test",
            "error",
            "test code here",
            "impl code here",
        );

        let built = prompt.build();
        assert!(built.text.contains("test code here"));
        assert!(built.text.contains("impl code here"));
    }

    #[test]
    fn test_test_failure_analysis_includes_guidance() {
        let prompt = RecoveryPrompts::test_failure_analysis("test", "error", "code", "impl");

        let built = prompt.build();
        assert!(built.text.contains("Off-by-one errors"));
        assert!(built.text.contains("Edge cases"));
    }

    #[test]
    fn test_test_failure_analysis_has_schema() {
        let prompt = RecoveryPrompts::test_failure_analysis("test", "error", "code", "impl");

        let built = prompt.build();
        assert!(built.text.contains("bug_location"));
        assert!(built.text.contains("confidence"));
    }

    // =========================================================================
    // Review Rejection Recovery Tests
    // =========================================================================

    #[test]
    fn test_review_rejection_version() {
        let version = RecoveryPrompts::review_rejection_version();
        assert_eq!(version.family, "recovery-review");
    }

    #[test]
    fn test_review_feedback_builder() {
        let feedback = ReviewFeedback::new("changes_requested", "Needs work")
            .with_issue(ReviewIssue::blocking(
                "Missing error handling",
                "Add try/catch",
            ))
            .with_issue(
                ReviewIssue::suggestion("Could be more concise", "Simplify logic")
                    .at_location("line 42"),
            );

        assert_eq!(feedback.verdict, "changes_requested");
        assert_eq!(feedback.issues.len(), 2);
        assert!(feedback.issues[1].location.is_some());
    }

    #[test]
    fn test_review_rejection_recovery_includes_feedback() {
        let feedback = ReviewFeedback::new("changes_requested", "Missing tests")
            .with_issue(ReviewIssue::blocking("No unit tests", "Add tests"));

        let prompt = RecoveryPrompts::review_rejection_recovery(
            "fn add(a: i32, b: i32) -> i32 { a + b }",
            &feedback,
            "Create an add function with tests",
        );

        let built = prompt.build();
        assert!(built.text.contains("Missing tests"));
        assert!(built.text.contains("No unit tests"));
        assert!(built.text.contains("Add tests"));
    }

    #[test]
    fn test_review_rejection_recovery_includes_requirements() {
        let feedback = ReviewFeedback::new("rejected", "summary");
        let prompt = RecoveryPrompts::review_rejection_recovery(
            "code",
            &feedback,
            "Original requirements here",
        );

        let built = prompt.build();
        assert!(built.text.contains("Original requirements here"));
    }

    #[test]
    fn test_review_rejection_recovery_has_schema() {
        let feedback = ReviewFeedback::new("rejected", "summary");
        let prompt = RecoveryPrompts::review_rejection_recovery("code", &feedback, "requirements");

        let built = prompt.build();
        assert!(built.text.contains("changes_made"));
        assert!(built.text.contains("unaddressed_issues"));
        assert!(built.text.contains("ready_for_review"));
    }

    // =========================================================================
    // Stuck Loop Breakout Tests
    // =========================================================================

    #[test]
    fn test_stuck_loop_version() {
        let version = RecoveryPrompts::stuck_loop_version();
        assert_eq!(version.family, "recovery-stuck");
    }

    #[test]
    fn test_attempt_record_builder() {
        let attempt = AttemptRecord::new("Tried approach A", "Failed").with_error("Type error");

        assert_eq!(attempt.action, "Tried approach A");
        assert!(attempt.error.is_some());
    }

    #[test]
    fn test_stuck_loop_breakout_includes_attempts() {
        let attempts = vec![
            AttemptRecord::new("Try 1", "Failed").with_error("error 1"),
            AttemptRecord::new("Try 2", "Failed").with_error("error 2"),
            AttemptRecord::new("Try 3", "Failed").with_error("error 3"),
        ];

        let prompt = RecoveryPrompts::stuck_loop_breakout(
            "Fix the compile error",
            &attempts,
            "Same error repeated 3 times",
        );

        let built = prompt.build();
        assert!(built.text.contains("Try 1"));
        assert!(built.text.contains("Try 2"));
        assert!(built.text.contains("Try 3"));
        assert!(built.text.contains("Same error repeated 3 times"));
    }

    #[test]
    fn test_stuck_loop_breakout_includes_task() {
        let prompt = RecoveryPrompts::stuck_loop_breakout("Implement the feature", &[], "pattern");

        let built = prompt.build();
        assert!(built.text.contains("Implement the feature"));
    }

    #[test]
    fn test_stuck_loop_breakout_includes_guidance() {
        let prompt = RecoveryPrompts::stuck_loop_breakout("task", &[], "pattern");

        let built = prompt.build();
        assert!(built.text.contains("STOP trying the same thing"));
        assert!(built.text.contains("fundamentally different approaches"));
    }

    #[test]
    fn test_stuck_loop_breakout_has_schema() {
        let prompt = RecoveryPrompts::stuck_loop_breakout("task", &[], "pattern");

        let built = prompt.build();
        assert!(built.text.contains("loop_analysis"));
        assert!(built.text.contains("alternative_approaches"));
        assert!(built.text.contains("if_escalating"));
    }

    // =========================================================================
    // Conflicting Requirements Tests
    // =========================================================================

    #[test]
    fn test_conflict_version() {
        let version = RecoveryPrompts::conflict_version();
        assert_eq!(version.family, "recovery-conflict");
    }

    #[test]
    fn test_requirement_conflict_builder() {
        let conflict = RequirementConflict::new(
            "Must be fast",
            "Must check all items",
            "Speed vs thoroughness tradeoff",
        );

        assert_eq!(conflict.requirement_a, "Must be fast");
        assert_eq!(conflict.requirement_b, "Must check all items");
    }

    #[test]
    fn test_conflicting_requirements_includes_conflicts() {
        let conflicts = vec![RequirementConflict::new(
            "Real-time updates",
            "Batch processing",
            "Can't be both real-time and batched",
        )];

        let prompt = RecoveryPrompts::conflicting_requirements("Build a data pipeline", &conflicts);

        let built = prompt.build();
        assert!(built.text.contains("Real-time updates"));
        assert!(built.text.contains("Batch processing"));
        assert!(built.text.contains("Can't be both real-time and batched"));
    }

    #[test]
    fn test_conflicting_requirements_includes_original() {
        let prompt = RecoveryPrompts::conflicting_requirements("Original requirements text", &[]);

        let built = prompt.build();
        assert!(built.text.contains("Original requirements text"));
    }

    #[test]
    fn test_conflicting_requirements_includes_guidance() {
        let prompt = RecoveryPrompts::conflicting_requirements("requirements", &[]);

        let built = prompt.build();
        assert!(built.text.contains("specific and answerable"));
        assert!(built.text.contains("Address one conflict at a time"));
    }

    #[test]
    fn test_conflicting_requirements_has_schema() {
        let prompt = RecoveryPrompts::conflicting_requirements("requirements", &[]);

        let built = prompt.build();
        assert!(built.text.contains("conflicts_analysis"));
        assert!(built.text.contains("clarifying_questions"));
        assert!(built.text.contains("assumptions_if_no_clarification"));
        assert!(built.text.contains("can_proceed_with_assumptions"));
    }

    // =========================================================================
    // Integration Tests
    // =========================================================================

    #[test]
    fn test_all_prompts_have_recovery_role() {
        let prompts = vec![
            RecoveryPrompts::parse_error_recovery("out", "err", "{}").build(),
            RecoveryPrompts::test_failure_analysis("test", "out", "code", "impl").build(),
            RecoveryPrompts::review_rejection_recovery(
                "code",
                &ReviewFeedback::new("rejected", "summary"),
                "requirements",
            )
            .build(),
            RecoveryPrompts::stuck_loop_breakout("task", &[], "pattern").build(),
            RecoveryPrompts::conflicting_requirements("requirements", &[]).build(),
        ];

        for prompt in prompts {
            assert!(
                prompt.text.contains("correcting a previous response"),
                "Prompt should include recovery role"
            );
        }
    }

    #[test]
    fn test_all_prompts_have_versions() {
        assert!(RecoveryPrompts::parse_error_version()
            .family
            .starts_with("recovery-"));
        assert!(RecoveryPrompts::test_failure_version()
            .family
            .starts_with("recovery-"));
        assert!(RecoveryPrompts::review_rejection_version()
            .family
            .starts_with("recovery-"));
        assert!(RecoveryPrompts::stuck_loop_version()
            .family
            .starts_with("recovery-"));
        assert!(RecoveryPrompts::conflict_version()
            .family
            .starts_with("recovery-"));
    }
}
