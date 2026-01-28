//! Worker prompt templates for focused development tasks.
//!
//! Workers are mid-tier agents that handle most implementation work. They need to be
//! focused and efficient while maintaining quality and knowing when to ask for help.

use crate::prompts::{PromptBuilder, PromptVersion};

/// Worker prompt templates for focused development tasks.
pub struct WorkerPrompts;

impl WorkerPrompts {
    /// Current version for implementation prompts
    pub fn implementation_version() -> PromptVersion {
        PromptVersion::new("worker-implementation", 1, 0, 0)
    }

    /// Build a prompt for implementing a task.
    ///
    /// # Arguments
    /// * `task_title` - The task title
    /// * `task_description` - Full task description with requirements
    /// * `context_files` - Pre-loaded file contents for context
    /// * `conventions` - Project conventions (CLAUDE.md content)
    pub fn implementation(
        task_title: &str,
        task_description: &str,
        context_files: &[(&str, &str)], // (path, content) pairs
        conventions: Option<&str>,
    ) -> PromptBuilder {
        let mut builder = PromptBuilder::new()
            .version(Self::implementation_version())
            .role(WORKER_ROLE)
            .task(format!(
                "Implement this task:\n\n\
                 **Title**: {}\n\n\
                 **Requirements**:\n{}",
                task_title, task_description
            ))
            .constraint(IMPLEMENTATION_THINKING)
            .constraint("Write clean, well-tested code")
            .constraint("Report progress naturally as you work")
            .constraint("Verify your work against requirements before submitting")
            .output_json(IMPLEMENTATION_SCHEMA);

        for (path, content) in context_files {
            builder = builder.file_to_modify(*path, *content);
        }

        if let Some(conv) = conventions {
            builder = builder.conventions(conv);
        }

        builder
    }

    /// Current version for context gathering prompts
    pub fn context_gathering_version() -> PromptVersion {
        PromptVersion::new("worker-context", 1, 0, 0)
    }

    /// Build a prompt for requesting specific context.
    ///
    /// # Arguments
    /// * `task_description` - What the worker is trying to accomplish
    /// * `current_context` - What context the worker already has
    /// * `question` - What the worker is trying to figure out
    pub fn context_gathering(
        task_description: &str,
        current_context: &str,
        question: &str,
    ) -> PromptBuilder {
        PromptBuilder::new()
            .version(Self::context_gathering_version())
            .role(WORKER_ROLE)
            .task(format!(
                "You need more context to complete your task.\n\n\
                 **Task**: {}\n\n\
                 **What you already have**:\n{}\n\n\
                 **What you're trying to figure out**:\n{}",
                task_description, current_context, question
            ))
            .constraint(CONTEXT_GATHERING_THINKING)
            .constraint("Request specific files, not entire directories")
            .constraint("Explain why you need each piece of context")
            .constraint("Don't request more than 5 items at once")
            .output_json(CONTEXT_REQUEST_SCHEMA)
    }

    /// Current version for progress reporting prompts
    pub fn progress_version() -> PromptVersion {
        PromptVersion::new("worker-progress", 1, 0, 0)
    }

    /// Build a prompt for generating a progress report.
    ///
    /// # Arguments
    /// * `task_title` - What task is being worked on
    /// * `work_done` - Description of what's been accomplished
    /// * `work_remaining` - What's left to do
    /// * `verbosity` - Level of detail to include
    pub fn progress_report(
        task_title: &str,
        work_done: &str,
        work_remaining: &str,
        verbosity: VerbosityLevel,
    ) -> PromptBuilder {
        PromptBuilder::new()
            .version(Self::progress_version())
            .role(WORKER_ROLE)
            .task(format!(
                "Generate a progress update for the feed.\n\n\
                 **Task**: {}\n\n\
                 **Completed**:\n{}\n\n\
                 **Remaining**:\n{}\n\n\
                 **Verbosity**: {:?}",
                task_title, work_done, work_remaining, verbosity
            ))
            .constraint(PROGRESS_THINKING)
            .constraint("Be brief but informative")
            .constraint("Use natural language, not bullet points")
            .constraint("Focus on what you're doing, not philosophizing")
            .output_json(PROGRESS_SCHEMA)
    }

    /// Current version for self-checking prompts
    pub fn self_check_version() -> PromptVersion {
        PromptVersion::new("worker-selfcheck", 1, 0, 0)
    }

    /// Build a prompt for self-checking work before submission.
    ///
    /// # Arguments
    /// * `task_requirements` - The original requirements
    /// * `code_produced` - The code that was written
    /// * `tests_written` - Tests that were written (if any)
    pub fn self_check(
        task_requirements: &str,
        code_produced: &str,
        tests_written: Option<&str>,
    ) -> PromptBuilder {
        let mut task = format!(
            "Verify your work before submitting for review.\n\n\
             **Original Requirements**:\n{}\n\n\
             **Code Produced**:\n```\n{}\n```",
            task_requirements, code_produced
        );

        if let Some(tests) = tests_written {
            task.push_str(&format!("\n\n**Tests Written**:\n```\n{}\n```", tests));
        }

        PromptBuilder::new()
            .version(Self::self_check_version())
            .role(WORKER_ROLE)
            .task(task)
            .constraint(SELF_CHECK_THINKING)
            .constraint("Be honest about issues - catching them now saves time")
            .constraint("If there are problems, fix them before submitting")
            .constraint("Only submit if you're confident in the work")
            .output_json(SELF_CHECK_SCHEMA)
    }

    /// Current version for stuck detection prompts
    pub fn stuck_detection_version() -> PromptVersion {
        PromptVersion::new("worker-stuck", 1, 0, 0)
    }

    /// Build a prompt for evaluating if stuck and should escalate.
    ///
    /// # Arguments
    /// * `task_description` - What was being attempted
    /// * `attempts` - Description of attempts made
    /// * `errors_encountered` - Errors received
    /// * `attempt_count` - Number of attempts
    pub fn stuck_detection(
        task_description: &str,
        attempts: &[&str],
        errors_encountered: &[&str],
        attempt_count: u32,
    ) -> PromptBuilder {
        let attempts_text = attempts
            .iter()
            .enumerate()
            .map(|(i, a)| format!("{}. {}", i + 1, a))
            .collect::<Vec<_>>()
            .join("\n");

        let errors_text = errors_encountered
            .iter()
            .enumerate()
            .map(|(i, e)| format!("{}. {}", i + 1, e))
            .collect::<Vec<_>>()
            .join("\n");

        PromptBuilder::new()
            .version(Self::stuck_detection_version())
            .role(WORKER_ROLE)
            .task(format!(
                "Evaluate whether you're stuck and should escalate.\n\n\
                 **Task**: {}\n\n\
                 **Attempts Made** ({} total):\n{}\n\n\
                 **Errors Encountered**:\n{}",
                task_description, attempt_count, attempts_text, errors_text
            ))
            .constraint(STUCK_DETECTION_THINKING)
            .constraint("After 2-3 failed attempts at the same thing, escalate")
            .constraint("Never submit broken code hoping it works")
            .constraint("Be honest about being stuck - it's better than wasting time")
            .output_json(STUCK_DETECTION_SCHEMA)
    }
}

/// Verbosity levels for progress reports
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerbosityLevel {
    /// Just milestones
    Quiet,
    /// Normal updates
    Normal,
    /// Detailed play-by-play
    Verbose,
}

const WORKER_ROLE: &str = r#"You are Dev, a focused software developer.

Your responsibilities:
- Implement code changes as specified in your task
- Write clean, well-tested code
- Report progress naturally as you work
- Submit work for review when complete

You stay heads-down on your task. You're efficient but thorough. You ask for help when truly stuck."#;

const IMPLEMENTATION_THINKING: &str = r#"Follow this implementation process:

### When starting a task:
1. READ the task description completely
2. IDENTIFY what files you'll need to read/modify
3. REQUEST context if needed (be specific: "I need to see src/auth/mod.rs")
4. PLAN your approach in 2-3 sentences before coding
5. ANNOUNCE: "Starting work on [task]. My approach: [brief plan]"

### When implementing:
1. WRITE code incrementally - don't try to do everything at once
2. EXPLAIN significant decisions: "Using X approach because Y"
3. TEST mentally: "If I call this with X, it should return Y"
4. REPORT progress every few minutes: "Completed the struct definition, now writing the impl"

### Before submitting:
1. RE-READ the original task requirements
2. CHECK: Does my code satisfy each requirement?
3. LOOK for obvious bugs: off-by-one, null checks, error handling
4. VERIFY: Are there tests? Do they cover happy path and edge cases?
5. If anything is incomplete, note it explicitly"#;

const IMPLEMENTATION_SCHEMA: &str = r#"{
  "phase": "planning | implementing | reviewing | complete",
  "thinking": "string - Your current thinking process",
  "plan": {
    "approach": "string - Brief description of implementation approach",
    "files_to_modify": ["array of file paths"],
    "files_to_create": ["array of new file paths"],
    "estimated_complexity": "low | medium | high"
  },
  "progress": {
    "current_step": "string - What you're working on now",
    "completed_steps": ["array of completed items"],
    "remaining_steps": ["array of remaining items"]
  },
  "code_changes": [
    {
      "file": "string - file path",
      "action": "create | modify | delete",
      "content": "string - full file content or diff",
      "explanation": "string - why this change"
    }
  ],
  "context_requests": ["array of specific files/info needed, if any"],
  "verification": {
    "requirements_met": ["array of requirements and how they're satisfied"],
    "tests_added": ["array of test descriptions"],
    "potential_issues": ["array of known issues or limitations"]
  },
  "status": "needs_context | in_progress | ready_for_review | blocked",
  "blocked_reason": "string | null - if blocked, why"
}"#;

const CONTEXT_GATHERING_THINKING: &str = r#"Think about what you need:

1. GOAL: What are you trying to understand or accomplish?
2. KNOWN: What do you already know from the current context?
3. UNKNOWN: What specific information is missing?
4. SOURCE: Where would that information likely be?
5. REQUEST: Ask for the specific files or information.

Be precise:
- Good: "I need src/auth/middleware.rs to see how auth tokens are validated"
- Bad: "I need to see the auth folder"

Don't request everything - just what you need for the current step."#;

const CONTEXT_REQUEST_SCHEMA: &str = r#"{
  "thinking": "string - Your reasoning about what context you need",
  "goal": "string - What you're trying to accomplish",
  "requests": [
    {
      "type": "file | function | type | example",
      "path": "string - file path or identifier",
      "reason": "string - why you need this specifically",
      "priority": "required | helpful"
    }
  ],
  "questions": ["array of specific questions if context won't answer them"],
  "can_proceed_without": "boolean - can you make progress without this context?"
}"#;

const PROGRESS_THINKING: &str = r#"Generate a progress update:

For QUIET: Only report major milestones ("Completed user model")
For NORMAL: Report current activity ("Working on the auth endpoints, login done, starting logout")
For VERBOSE: Include technical details ("Implementing refresh token rotation using JWT with 7-day expiry")

Style guidelines:
- Sound like a developer, not a robot
- "Looking at the auth module..." not "I am currently examining..."
- "Found an issue with..." not "An anomaly has been detected..."
- Keep it brief - one or two sentences"#;

const PROGRESS_SCHEMA: &str = r#"{
  "message": "string - The natural language progress update for the feed",
  "percentage_complete": "number - estimated 0-100",
  "milestone_reached": "string | null - if a milestone was just completed",
  "blocking_issue": "string | null - if there's something blocking progress"
}"#;

const SELF_CHECK_THINKING: &str = r#"Before submitting, verify your work:

1. REQUIREMENTS: Go through each requirement. Is it met?
2. CORRECTNESS: Would this code work? Any obvious bugs?
3. EDGE CASES: What could go wrong? Are those cases handled?
4. TESTS: Do the tests actually test the right things?
5. COMPLETENESS: Is anything missing or incomplete?

Be your own code reviewer. Would you approve this if someone else wrote it?

If you find issues:
- Fix them if possible
- Note them explicitly if you can't fix them
- Never submit hoping issues won't be noticed"#;

const SELF_CHECK_SCHEMA: &str = r#"{
  "thinking": "string - Your self-review reasoning",
  "requirements_check": [
    {
      "requirement": "string - the requirement",
      "status": "met | partial | not_met",
      "evidence": "string - how/where it's satisfied",
      "issue": "string | null - if partial or not_met, what's wrong"
    }
  ],
  "bugs_found": [
    {
      "description": "string - what the bug is",
      "location": "string - where in the code",
      "fix_applied": "boolean - did you fix it?",
      "fix_description": "string | null - how you fixed it"
    }
  ],
  "test_coverage": {
    "happy_path_covered": "boolean",
    "edge_cases_covered": ["array of edge cases that are tested"],
    "missing_tests": ["array of tests that should be added"]
  },
  "confidence": "low | medium | high",
  "ready_to_submit": "boolean",
  "blocking_issues": ["array of issues that prevent submission"]
}"#;

const STUCK_DETECTION_THINKING: &str = r#"Evaluate your situation honestly:

1. PATTERN: Are you trying the same thing repeatedly?
2. PROGRESS: Has anything actually changed between attempts?
3. UNDERSTANDING: Do you understand why it's failing?
4. OPTIONS: Are there other approaches you haven't tried?
5. DECISION: Should you try something new, or escalate?

Signs you're stuck:
- Same error 2-3 times
- Trying small variations of the same approach
- Not understanding why attempts fail
- Spending more time debugging than implementing

When to escalate:
- You've tried 2-3 different approaches
- The error is unclear or beyond your capabilities
- You need information you can't access
- The task might be incorrectly scoped"#;

const STUCK_DETECTION_SCHEMA: &str = r#"{
  "thinking": "string - Honest assessment of your situation",
  "is_stuck": "boolean - Are you genuinely stuck?",
  "pattern_detected": "string | null - What pattern of failure you see",
  "understanding_level": "none | partial | good - How well you understand the issue",
  "untried_approaches": ["array of things you haven't tried yet"],
  "recommendation": "continue | try_different_approach | escalate",
  "escalation_summary": {
    "what_was_tried": "string - Summary of attempts",
    "what_failed": "string - Summary of failures",
    "what_help_needed": "string - Specific help requested",
    "suggested_expert": "orchestrator | human"
  },
  "next_action": "string - What should happen next"
}"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_implementation_version() {
        let version = WorkerPrompts::implementation_version();
        assert_eq!(version.family, "worker-implementation");
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 0);
        assert_eq!(version.patch, 0);
    }

    #[test]
    fn test_implementation_prompt_basic() {
        let prompt = WorkerPrompts::implementation(
            "Add logging",
            "Add tracing to the main function",
            &[],
            None,
        )
        .build();

        assert!(prompt.text.contains("Dev"));
        assert!(prompt.text.contains("Add logging"));
        assert!(prompt.text.contains("Add tracing to the main function"));
        assert!(prompt.text.contains("Follow this implementation process"));
    }

    #[test]
    fn test_implementation_prompt_with_context_files() {
        let prompt = WorkerPrompts::implementation(
            "Fix bug",
            "Fix the off-by-one error",
            &[
                ("src/main.rs", "fn main() {}"),
                ("src/lib.rs", "pub mod utils;"),
            ],
            None,
        )
        .build();

        assert!(prompt.text.contains("src/main.rs"));
        assert!(prompt.text.contains("src/lib.rs"));
    }

    #[test]
    fn test_implementation_prompt_with_conventions() {
        let prompt = WorkerPrompts::implementation(
            "New feature",
            "Add user authentication",
            &[],
            Some("Use snake_case for function names"),
        )
        .build();

        assert!(prompt.text.contains("Use snake_case for function names"));
    }

    #[test]
    fn test_implementation_prompt_includes_output_schema() {
        let prompt =
            WorkerPrompts::implementation("Test task", "Test description", &[], None).build();

        assert!(prompt.text.contains("phase"));
        assert!(prompt
            .text
            .contains("planning | implementing | reviewing | complete"));
        assert!(prompt.text.contains("code_changes"));
        assert!(prompt.text.contains("verification"));
    }

    #[test]
    fn test_context_gathering_version() {
        let version = WorkerPrompts::context_gathering_version();
        assert_eq!(version.family, "worker-context");
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 0);
        assert_eq!(version.patch, 0);
    }

    #[test]
    fn test_context_gathering_prompt_basic() {
        let prompt = WorkerPrompts::context_gathering(
            "Add user authentication",
            "I have the main.rs file",
            "How does the existing auth middleware work?",
        )
        .build();

        assert!(prompt.text.contains("Add user authentication"));
        assert!(prompt.text.contains("I have the main.rs file"));
        assert!(prompt
            .text
            .contains("How does the existing auth middleware work?"));
        assert!(prompt.text.contains("Think about what you need"));
    }

    #[test]
    fn test_context_gathering_prompt_limits_requests() {
        let prompt =
            WorkerPrompts::context_gathering("Test task", "Current context", "Question").build();

        assert!(prompt.text.contains("Don't request more than 5 items"));
    }

    #[test]
    fn test_context_gathering_prompt_includes_priority() {
        let prompt =
            WorkerPrompts::context_gathering("Test task", "Current context", "Question").build();

        assert!(prompt.text.contains("priority"));
        assert!(prompt.text.contains("required | helpful"));
    }

    #[test]
    fn test_context_gathering_prompt_can_proceed_without() {
        let prompt =
            WorkerPrompts::context_gathering("Test task", "Current context", "Question").build();

        assert!(prompt.text.contains("can_proceed_without"));
    }

    #[test]
    fn test_progress_version() {
        let version = WorkerPrompts::progress_version();
        assert_eq!(version.family, "worker-progress");
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 0);
        assert_eq!(version.patch, 0);
    }

    #[test]
    fn test_progress_report_basic() {
        let prompt = WorkerPrompts::progress_report(
            "Add user authentication",
            "Created the user model and migration",
            "Still need auth endpoints and tests",
            VerbosityLevel::Normal,
        )
        .build();

        assert!(prompt.text.contains("Add user authentication"));
        assert!(prompt.text.contains("Created the user model"));
        assert!(prompt.text.contains("Still need auth endpoints"));
        assert!(prompt.text.contains("Generate a progress update"));
    }

    #[test]
    fn test_progress_report_verbosity_levels() {
        // Test that verbosity level is included in the prompt
        let quiet =
            WorkerPrompts::progress_report("Task", "Done", "Remaining", VerbosityLevel::Quiet)
                .build();
        assert!(quiet.text.contains("Quiet"));

        let normal =
            WorkerPrompts::progress_report("Task", "Done", "Remaining", VerbosityLevel::Normal)
                .build();
        assert!(normal.text.contains("Normal"));

        let verbose =
            WorkerPrompts::progress_report("Task", "Done", "Remaining", VerbosityLevel::Verbose)
                .build();
        assert!(verbose.text.contains("Verbose"));
    }

    #[test]
    fn test_progress_report_includes_natural_language_guidelines() {
        let prompt =
            WorkerPrompts::progress_report("Task", "Done", "Remaining", VerbosityLevel::Normal)
                .build();

        assert!(prompt.text.contains("Sound like a developer"));
        assert!(prompt.text.contains("Use natural language"));
    }

    #[test]
    fn test_progress_report_includes_percentage() {
        let prompt =
            WorkerPrompts::progress_report("Task", "Done", "Remaining", VerbosityLevel::Normal)
                .build();

        assert!(prompt.text.contains("percentage_complete"));
    }

    #[test]
    fn test_self_check_version() {
        let version = WorkerPrompts::self_check_version();
        assert_eq!(version.family, "worker-selfcheck");
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 0);
        assert_eq!(version.patch, 0);
    }

    #[test]
    fn test_self_check_basic() {
        let prompt = WorkerPrompts::self_check(
            "Implement user registration endpoint",
            "fn register_user() { ... }",
            None,
        )
        .build();

        assert!(prompt.text.contains("Implement user registration endpoint"));
        assert!(prompt.text.contains("fn register_user()"));
        assert!(prompt.text.contains("Before submitting, verify your work"));
    }

    #[test]
    fn test_self_check_with_tests() {
        let prompt = WorkerPrompts::self_check(
            "Requirements",
            "code",
            Some("#[test] fn test_register() { ... }"),
        )
        .build();

        assert!(prompt.text.contains("Tests Written"));
        assert!(prompt.text.contains("test_register"));
    }

    #[test]
    fn test_self_check_without_tests() {
        let prompt = WorkerPrompts::self_check("Requirements", "code", None).build();

        // Should not contain "Tests Written" section when no tests provided
        assert!(!prompt.text.contains("Tests Written"));
    }

    #[test]
    fn test_self_check_includes_confidence() {
        let prompt = WorkerPrompts::self_check("Requirements", "code", None).build();

        assert!(prompt.text.contains("confidence"));
        assert!(prompt.text.contains("low | medium | high"));
    }

    #[test]
    fn test_self_check_validates_requirements() {
        let prompt = WorkerPrompts::self_check("Requirements", "code", None).build();

        assert!(prompt.text.contains("requirements_check"));
        assert!(prompt.text.contains("met | partial | not_met"));
    }

    #[test]
    fn test_self_check_includes_bug_tracking() {
        let prompt = WorkerPrompts::self_check("Requirements", "code", None).build();

        assert!(prompt.text.contains("bugs_found"));
        assert!(prompt.text.contains("fix_applied"));
    }

    #[test]
    fn test_stuck_detection_version() {
        let version = WorkerPrompts::stuck_detection_version();
        assert_eq!(version.family, "worker-stuck");
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 0);
        assert_eq!(version.patch, 0);
    }

    #[test]
    fn test_stuck_detection_basic() {
        let prompt = WorkerPrompts::stuck_detection(
            "Implement user auth",
            &["Tried using bcrypt", "Tried using argon2"],
            &["bcrypt crate version conflict", "argon2 build failed"],
            2,
        )
        .build();

        assert!(prompt.text.contains("Implement user auth"));
        assert!(prompt.text.contains("Tried using bcrypt"));
        assert!(prompt.text.contains("Tried using argon2"));
        assert!(prompt.text.contains("bcrypt crate version conflict"));
        assert!(prompt.text.contains("2 total"));
    }

    #[test]
    fn test_stuck_detection_formats_attempts() {
        let prompt = WorkerPrompts::stuck_detection(
            "Task",
            &["First attempt", "Second attempt", "Third attempt"],
            &["Error 1"],
            3,
        )
        .build();

        // Attempts should be numbered
        assert!(prompt.text.contains("1. First attempt"));
        assert!(prompt.text.contains("2. Second attempt"));
        assert!(prompt.text.contains("3. Third attempt"));
    }

    #[test]
    fn test_stuck_detection_escalation_threshold() {
        let prompt = WorkerPrompts::stuck_detection("Task", &[], &[], 0).build();

        assert!(prompt.text.contains("After 2-3 failed attempts"));
        assert!(prompt.text.contains("escalate"));
    }

    #[test]
    fn test_stuck_detection_includes_untried_approaches() {
        let prompt = WorkerPrompts::stuck_detection("Task", &[], &[], 0).build();

        assert!(prompt.text.contains("untried_approaches"));
    }

    #[test]
    fn test_stuck_detection_includes_escalation_summary() {
        let prompt = WorkerPrompts::stuck_detection("Task", &[], &[], 0).build();

        assert!(prompt.text.contains("escalation_summary"));
        assert!(prompt.text.contains("what_was_tried"));
        assert!(prompt.text.contains("what_failed"));
        assert!(prompt.text.contains("what_help_needed"));
        assert!(prompt.text.contains("suggested_expert"));
    }

    #[test]
    fn test_stuck_detection_includes_recommendation() {
        let prompt = WorkerPrompts::stuck_detection("Task", &[], &[], 0).build();

        assert!(prompt.text.contains("recommendation"));
        assert!(prompt
            .text
            .contains("continue | try_different_approach | escalate"));
    }
}
