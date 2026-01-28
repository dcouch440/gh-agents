//! Orchestrator prompt templates for different thinking modes.
//!
//! The orchestrator is the "senior architect" that plans, reviews,
//! routes tasks, converses with users, and handles failure recovery.

use crate::prompts::{PromptBuilder, PromptVersion};

/// Orchestrator prompt templates for different thinking modes.
pub struct OrchestratorPrompts;

impl OrchestratorPrompts {
    /// Current version for decomposition prompts
    pub fn decomposition_version() -> PromptVersion {
        PromptVersion::new("orchestrator-decomposition", 1, 0, 0)
    }

    /// Build a prompt for decomposing a ticket into vertical slices.
    ///
    /// # Arguments
    /// * `ticket_title` - The ticket title
    /// * `ticket_description` - Full ticket description
    /// * `codebase_context` - Relevant codebase information
    /// * `conventions` - Project conventions (CLAUDE.md content)
    pub fn decomposition(
        ticket_title: &str,
        ticket_description: &str,
        codebase_context: Option<&str>,
        conventions: Option<&str>,
    ) -> PromptBuilder {
        let mut builder = PromptBuilder::new()
            .version(Self::decomposition_version())
            .role(ORCHESTRATOR_ROLE)
            .task(format!(
                "Decompose this ticket into vertical slices:\n\n\
                 **Title**: {}\n\n\
                 **Description**:\n{}",
                ticket_title, ticket_description
            ))
            .constraint("Each slice must be independently deployable and valuable")
            .constraint("Each slice should be 1-4 hours of work")
            .constraint("Slices must be vertical (touch all necessary layers), not horizontal")
            .constraint("List dependencies between slices explicitly")
            .constraint(
                "If requirements are unclear, list clarifying questions instead of guessing",
            )
            .output_json(DECOMPOSITION_SCHEMA);

        // Add thinking pattern instructions
        builder = builder.constraint(DECOMPOSITION_THINKING);

        if let Some(ctx) = codebase_context {
            builder = builder.reference_file("codebase_overview", ctx);
        }

        if let Some(conv) = conventions {
            builder = builder.conventions(conv);
        }

        builder
    }

    /// Current version for review prompts
    pub fn review_version() -> PromptVersion {
        PromptVersion::new("orchestrator-review", 1, 0, 0)
    }

    /// Build a prompt for reviewing agent work.
    ///
    /// # Arguments
    /// * `task_description` - What the task was supposed to accomplish
    /// * `code_changes` - The code diff or modified files
    /// * `test_results` - Test output if available
    pub fn review(
        task_description: &str,
        code_changes: &str,
        test_results: Option<&str>,
    ) -> PromptBuilder {
        let mut task = format!(
            "Review this work submission:\n\n\
             **Original Task**:\n{}\n\n\
             **Code Changes**:\n```\n{}\n```",
            task_description, code_changes
        );

        if let Some(tests) = test_results {
            task.push_str(&format!("\n\n**Test Results**:\n```\n{}\n```", tests));
        }

        PromptBuilder::new()
            .version(Self::review_version())
            .role(ORCHESTRATOR_ROLE)
            .task(task)
            .constraint(REVIEW_THINKING)
            .constraint("Be specific about issues - include file paths and line numbers")
            .constraint("Distinguish between blocking issues and suggestions")
            .constraint("If approving, explain why the code is good")
            .output_json(REVIEW_SCHEMA)
    }

    /// Current version for routing prompts
    pub fn routing_version() -> PromptVersion {
        PromptVersion::new("orchestrator-routing", 1, 0, 0)
    }

    /// Build a prompt for routing a task to the appropriate tier.
    ///
    /// # Arguments
    /// * `task_title` - The task title
    /// * `task_description` - Full task description
    /// * `available_agents` - Description of available agent tiers and their capabilities
    pub fn routing(
        task_title: &str,
        task_description: &str,
        available_agents: &str,
    ) -> PromptBuilder {
        PromptBuilder::new()
            .version(Self::routing_version())
            .role(ORCHESTRATOR_ROLE)
            .task(format!(
                "Route this task to the appropriate agent tier:\n\n\
                 **Task**: {}\n\n\
                 **Description**:\n{}\n\n\
                 **Available Agents**:\n{}",
                task_title, task_description, available_agents
            ))
            .constraint(ROUTING_THINKING)
            .constraint("Choose the cheapest tier that can successfully complete the task")
            .constraint("When in doubt, route to a higher tier")
            .output_json(ROUTING_SCHEMA)
    }

    /// Current version for conversation prompts
    pub fn conversation_version() -> PromptVersion {
        PromptVersion::new("orchestrator-conversation", 1, 0, 0)
    }

    /// Build a prompt for conversing with the user in /main chat.
    ///
    /// # Arguments
    /// * `user_message` - The user's message
    /// * `conversation_history` - Previous messages in the conversation
    /// * `current_work_status` - Summary of what agents are currently doing
    pub fn conversation(
        user_message: &str,
        conversation_history: &[(&str, &str)], // (role, content) pairs
        current_work_status: Option<&str>,
    ) -> PromptBuilder {
        let history_text = if conversation_history.is_empty() {
            "No previous messages.".to_string()
        } else {
            conversation_history
                .iter()
                .map(|(role, content)| format!("**{}**: {}", role, content))
                .collect::<Vec<_>>()
                .join("\n\n")
        };

        let mut task = format!(
            "Respond to the user's message:\n\n\
             **Conversation History**:\n{}\n\n\
             **User's Message**:\n{}",
            history_text, user_message
        );

        if let Some(status) = current_work_status {
            task.push_str(&format!("\n\n**Current Work Status**:\n{}", status));
        }

        PromptBuilder::new()
            .version(Self::conversation_version())
            .role(ORCHESTRATOR_CONVERSATION_ROLE)
            .task(task)
            .constraint(CONVERSATION_THINKING)
            .constraint("Be collaborative and conversational, not robotic")
            .constraint("If the user wants to start a task, decompose it")
            .constraint("If unclear, ask ONE specific clarifying question")
            .output_json(CONVERSATION_SCHEMA)
    }

    /// Current version for recovery prompts
    pub fn recovery_version() -> PromptVersion {
        PromptVersion::new("orchestrator-recovery", 1, 0, 0)
    }

    /// Build a prompt for deciding how to recover from a failure.
    ///
    /// # Arguments
    /// * `failure_type` - Category of failure
    /// * `failure_details` - Specific error information
    /// * `task_context` - What was being attempted
    /// * `attempt_count` - How many times this has been tried
    pub fn recovery(
        failure_type: FailureType,
        failure_details: &str,
        task_context: &str,
        attempt_count: u32,
    ) -> PromptBuilder {
        PromptBuilder::new()
            .version(Self::recovery_version())
            .role(ORCHESTRATOR_ROLE)
            .task(format!(
                "A failure occurred that needs recovery:\n\n\
                 **Failure Type**: {:?}\n\n\
                 **Details**:\n{}\n\n\
                 **Task Context**:\n{}\n\n\
                 **Attempt Count**: {} (max 3 before escalation)",
                failure_type, failure_details, task_context, attempt_count
            ))
            .constraint(RECOVERY_THINKING)
            .constraint("Be honest about whether recovery is likely to succeed")
            .constraint("Escalate after 3 failed attempts")
            .constraint("Never silently ignore failures")
            .output_json(RECOVERY_SCHEMA)
    }
}

/// Types of failures that can occur
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureType {
    /// LLM output couldn't be parsed
    ParseError,
    /// Task execution failed
    TaskFailed,
    /// Agent appears stuck in a loop
    StuckLoop,
    /// External service error (API, git, etc.)
    ExternalError,
    /// Test failures
    TestFailure,
    /// Unknown/unexpected error
    Unknown,
}

const ORCHESTRATOR_ROLE: &str = r#"You are Arch, a senior software architect coordinating a team of AI agents.

Your responsibilities:
- Break down complex problems into vertical slices
- Review work from other agents before approval
- Make architectural decisions and resolve conflicts
- Ask clarifying questions when requirements are ambiguous

You think carefully before acting. You explain your reasoning. You never guess at requirements."#;

const DECOMPOSITION_THINKING: &str = r#"Follow this thinking process:

1. UNDERSTAND: What is the user actually trying to accomplish? What's the business value?
2. INVENTORY: What components/layers does this touch? (DB, API, UI, tests, etc.)
3. DEPENDENCIES: What must exist before other parts can work?
4. SLICE VERTICALLY: Each slice must be deployable alone. Ask: "If we stopped after this slice, would something work?"
5. SIZE CHECK: Each slice should be 1-4 hours of work. Too big? Split it. Too small? Combine."#;

const DECOMPOSITION_SCHEMA: &str = r#"{
  "thinking": "string - Your reasoning through the 5 steps above",
  "slices": [
    {
      "title": "string - Short descriptive title",
      "description": "string - What this slice accomplishes",
      "tasks": [
        {
          "title": "string - Specific task",
          "tier": "worker | utility",
          "estimated_complexity": "low | medium | high",
          "context_files": ["array of file paths the agent will need"]
        }
      ],
      "dependencies": ["array of slice titles this depends on"],
      "acceptance_criteria": ["array of verification steps"]
    }
  ],
  "questions": ["array of clarifying questions if requirements unclear"],
  "risks": ["array of potential issues or unknowns"]
}"#;

const REVIEW_THINKING: &str = r#"Follow this review process:

1. CORRECTNESS: Does the code do what the task asked? Check each requirement.
2. INTEGRATION: Will this break anything else? Consider side effects.
3. QUALITY: Is this code maintainable? Any obvious code smells?
4. COMPLETENESS: Are edge cases handled? Tests included?
5. VERDICT: Approve, request changes (be specific), or escalate to human."#;

const REVIEW_SCHEMA: &str = r#"{
  "thinking": "string - Your reasoning through the review steps",
  "verdict": "approved | changes_requested | escalate",
  "issues": [
    {
      "severity": "blocking | suggestion",
      "file": "string - file path",
      "line": "number | null",
      "description": "string - what's wrong",
      "suggestion": "string - how to fix it"
    }
  ],
  "positive_notes": ["array - what was done well"],
  "summary": "string - brief overall assessment"
}"#;

const ROUTING_THINKING: &str = r#"Follow this routing process:

1. ANALYZE: What kind of task is this? (implementation, formatting, review, etc.)
2. COMPLEXITY: How complex is the task? What skills does it require?
3. MATCH: Which tier has the capabilities needed?
4. COST: Could a cheaper tier handle this?
5. DECIDE: Route to the most appropriate tier with justification.

Tier capabilities:
- UTILITY (cheap): Formatting, linting, boilerplate, simple docs, renames
- WORKER (mid): Implementation, bug fixes, tests, complex changes
- ORCHESTRATOR (expensive): Planning, architecture, complex reviews, user interaction

Route to the cheapest tier that can succeed."#;

const ROUTING_SCHEMA: &str = r#"{
  "thinking": "string - Your reasoning about task characteristics and tier match",
  "task_type": "string - categorization (implementation, formatting, review, etc.)",
  "complexity": "low | medium | high",
  "selected_tier": "utility | worker | orchestrator",
  "justification": "string - why this tier is appropriate",
  "alternative_tier": "string | null - could another tier work?",
  "special_requirements": ["array - any special needs (specific files, tools, etc.)"]
}"#;

const ORCHESTRATOR_CONVERSATION_ROLE: &str = r#"You are Arch, a senior software architect having a conversation with a developer.

You are:
- Collaborative and thoughtful
- Direct but friendly
- Willing to brainstorm and discuss
- Quick to ask clarifying questions when needed

You can help with:
- Planning and decomposing work
- Answering architecture questions
- Checking on current work status
- General discussion about the codebase"#;

const CONVERSATION_THINKING: &str = r#"Before responding, consider:

1. INTENT: What does the user actually want? A task? Information? Discussion?
2. CLARITY: Do I have enough information to help? What's ambiguous?
3. ACTION: Should I start a task, answer a question, or ask for clarification?
4. RESPONSE: Plan a helpful response that moves things forward.

Guidelines:
- If the user describes work to be done, offer to decompose it into slices
- If asking about status, summarize current work clearly
- If unclear, ask ONE specific question (not a list)
- Be conversational, not robotic"#;

const CONVERSATION_SCHEMA: &str = r#"{
  "intent_analysis": "string - What you understand the user wants",
  "needs_clarification": "boolean - Do you need more information?",
  "clarifying_question": "string | null - If needs_clarification, what to ask",
  "action_type": "decompose | status_update | answer | discuss | clarify",
  "response": "string - Your conversational response to the user",
  "suggested_next_steps": ["array - optional suggestions for what to do next"]
}"#;

const RECOVERY_THINKING: &str = r#"Follow this recovery process:

1. ANALYZE: What exactly went wrong? Look at the specific error.
2. CAUSE: Why did this happen? (bad prompt, bad code, external issue, etc.)
3. RECOVERABLE: Can this be fixed with a retry or small adjustment?
4. STRATEGY: What's the best recovery approach?
5. ESCALATE: Should this go to a higher tier or human?

Recovery strategies:
- RETRY: Same approach, might work on second attempt
- ADJUST: Modify the approach based on the error
- SIMPLIFY: Break into smaller pieces
- ESCALATE: Pass to higher tier
- HUMAN: Requires human intervention

Escalate to human after 3 failed attempts or if the error is unclear."#;

const RECOVERY_SCHEMA: &str = r#"{
  "thinking": "string - Your analysis of the failure",
  "root_cause": "string - What you believe caused this",
  "recoverable": "boolean - Can this be automatically recovered?",
  "strategy": "retry | adjust | simplify | escalate | human",
  "adjustment_details": "string | null - If adjusting, what to change",
  "escalation_reason": "string | null - If escalating, why",
  "recommended_action": "string - Specific next step to take",
  "confidence": "low | medium | high - How confident in this recovery"
}"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decomposition_prompt_basic() {
        let builder = OrchestratorPrompts::decomposition(
            "Add user authentication",
            "Implement login and registration",
            None,
            None,
        );

        let prompt = builder.build();

        assert!(prompt.text.contains("Arch"));
        assert!(prompt.text.contains("Add user authentication"));
        assert!(prompt.text.contains("UNDERSTAND"));
        assert!(prompt.text.contains("INVENTORY"));
        assert!(prompt.text.contains("DEPENDENCIES"));
        assert!(prompt.text.contains("SLICE VERTICALLY"));
        assert!(prompt.text.contains("SIZE CHECK"));
    }

    #[test]
    fn test_decomposition_prompt_with_context() {
        let builder = OrchestratorPrompts::decomposition(
            "Test ticket",
            "Test description",
            Some("Project uses Rust with tokio"),
            Some("Use snake_case for functions"),
        );

        let prompt = builder.build();

        assert!(prompt.text.contains("Project uses Rust with tokio"));
        assert!(prompt.text.contains("Use snake_case for functions"));
    }

    #[test]
    fn test_decomposition_prompt_has_json_output() {
        let builder = OrchestratorPrompts::decomposition("Test", "Test", None, None);

        let prompt = builder.build();

        assert!(prompt.text.contains("slices"));
        assert!(prompt.text.contains("tasks"));
        assert!(prompt.text.contains("acceptance_criteria"));
    }

    #[test]
    fn test_decomposition_version() {
        let version = OrchestratorPrompts::decomposition_version();
        assert_eq!(version.family, "orchestrator-decomposition");
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 0);
        assert_eq!(version.patch, 0);
    }

    #[test]
    fn test_review_prompt_basic() {
        let builder = OrchestratorPrompts::review(
            "Add logging to main function",
            "fn main() {\n    info!(\"Starting\");\n}",
            None,
        );

        let prompt = builder.build();

        assert!(prompt.text.contains("Arch"));
        assert!(prompt.text.contains("Add logging to main function"));
        assert!(prompt.text.contains("CORRECTNESS"));
        assert!(prompt.text.contains("INTEGRATION"));
        assert!(prompt.text.contains("QUALITY"));
        assert!(prompt.text.contains("COMPLETENESS"));
        assert!(prompt.text.contains("VERDICT"));
    }

    #[test]
    fn test_review_prompt_with_test_results() {
        let builder = OrchestratorPrompts::review("Fix bug", "let x = 1;", Some("test result: ok"));

        let prompt = builder.build();

        assert!(prompt.text.contains("Test Results"));
        assert!(prompt.text.contains("test result: ok"));
    }

    #[test]
    fn test_review_prompt_has_severity_levels() {
        let builder = OrchestratorPrompts::review("Task", "Code", None);

        let prompt = builder.build();

        assert!(prompt.text.contains("blocking"));
        assert!(prompt.text.contains("suggestion"));
    }

    #[test]
    fn test_review_version() {
        let version = OrchestratorPrompts::review_version();
        assert_eq!(version.family, "orchestrator-review");
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 0);
        assert_eq!(version.patch, 0);
    }

    #[test]
    fn test_routing_prompt_basic() {
        let builder = OrchestratorPrompts::routing(
            "Format code",
            "Run rustfmt on main.rs",
            "UTILITY: formatting\nWORKER: implementation\nORCHESTRATOR: planning",
        );

        let prompt = builder.build();

        assert!(prompt.text.contains("Arch"));
        assert!(prompt.text.contains("Format code"));
        assert!(prompt.text.contains("ANALYZE"));
        assert!(prompt.text.contains("COMPLEXITY"));
        assert!(prompt.text.contains("MATCH"));
        assert!(prompt.text.contains("COST"));
        assert!(prompt.text.contains("DECIDE"));
    }

    #[test]
    fn test_routing_prompt_has_tier_capabilities() {
        let builder = OrchestratorPrompts::routing("Task", "Description", "Agents");

        let prompt = builder.build();

        assert!(prompt.text.contains("UTILITY (cheap)"));
        assert!(prompt.text.contains("WORKER (mid)"));
        assert!(prompt.text.contains("ORCHESTRATOR (expensive)"));
    }

    #[test]
    fn test_routing_prompt_supports_alternative_tier() {
        let builder = OrchestratorPrompts::routing("Task", "Description", "Agents");

        let prompt = builder.build();

        assert!(prompt.text.contains("alternative_tier"));
        assert!(prompt.text.contains("justification"));
    }

    #[test]
    fn test_routing_version() {
        let version = OrchestratorPrompts::routing_version();
        assert_eq!(version.family, "orchestrator-routing");
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 0);
        assert_eq!(version.patch, 0);
    }

    #[test]
    fn test_conversation_prompt_basic() {
        let builder = OrchestratorPrompts::conversation("I want to add a new feature", &[], None);

        let prompt = builder.build();

        assert!(prompt.text.contains("Arch"));
        assert!(prompt.text.contains("I want to add a new feature"));
        assert!(prompt.text.contains("INTENT"));
        assert!(prompt.text.contains("CLARITY"));
        assert!(prompt.text.contains("ACTION"));
        assert!(prompt.text.contains("RESPONSE"));
    }

    #[test]
    fn test_conversation_prompt_handles_empty_history() {
        let builder = OrchestratorPrompts::conversation("Hello", &[], None);

        let prompt = builder.build();

        assert!(prompt.text.contains("No previous messages"));
    }

    #[test]
    fn test_conversation_prompt_with_history() {
        let history = vec![
            ("User", "Can you help me?"),
            ("Arch", "Of course, what do you need?"),
        ];
        let builder = OrchestratorPrompts::conversation("I need to fix a bug", &history, None);

        let prompt = builder.build();

        assert!(prompt.text.contains("**User**: Can you help me?"));
        assert!(prompt
            .text
            .contains("**Arch**: Of course, what do you need?"));
    }

    #[test]
    fn test_conversation_prompt_with_work_status() {
        let builder = OrchestratorPrompts::conversation(
            "What's happening?",
            &[],
            Some("Worker is implementing task 2.1"),
        );

        let prompt = builder.build();

        assert!(prompt.text.contains("Current Work Status"));
        assert!(prompt.text.contains("Worker is implementing task 2.1"));
    }

    #[test]
    fn test_conversation_prompt_includes_intent_analysis() {
        let builder = OrchestratorPrompts::conversation("Test", &[], None);

        let prompt = builder.build();

        assert!(prompt.text.contains("intent_analysis"));
        assert!(prompt.text.contains("action_type"));
    }

    #[test]
    fn test_conversation_version() {
        let version = OrchestratorPrompts::conversation_version();
        assert_eq!(version.family, "orchestrator-conversation");
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 0);
        assert_eq!(version.patch, 0);
    }

    #[test]
    fn test_recovery_prompt_basic() {
        let builder = OrchestratorPrompts::recovery(
            FailureType::ParseError,
            "Expected JSON, got plain text",
            "Implementing user authentication",
            1,
        );

        let prompt = builder.build();

        assert!(prompt.text.contains("Arch"));
        assert!(prompt.text.contains("ParseError"));
        assert!(prompt.text.contains("Expected JSON, got plain text"));
        assert!(prompt.text.contains("ANALYZE"));
        assert!(prompt.text.contains("CAUSE"));
        assert!(prompt.text.contains("RECOVERABLE"));
        assert!(prompt.text.contains("STRATEGY"));
        assert!(prompt.text.contains("ESCALATE"));
    }

    #[test]
    fn test_recovery_prompt_all_failure_types() {
        let failure_types = vec![
            FailureType::ParseError,
            FailureType::TaskFailed,
            FailureType::StuckLoop,
            FailureType::ExternalError,
            FailureType::TestFailure,
            FailureType::Unknown,
        ];

        for failure_type in failure_types {
            let builder =
                OrchestratorPrompts::recovery(failure_type, "Error details", "Context", 1);

            let prompt = builder.build();
            assert!(prompt.text.contains(&format!("{:?}", failure_type)));
        }
    }

    #[test]
    fn test_recovery_prompt_includes_attempt_count() {
        let builder = OrchestratorPrompts::recovery(
            FailureType::TaskFailed,
            "Build failed",
            "Building project",
            2,
        );

        let prompt = builder.build();

        assert!(prompt.text.contains("Attempt Count"));
        assert!(prompt.text.contains("2"));
        assert!(prompt.text.contains("max 3 before escalation"));
    }

    #[test]
    fn test_recovery_prompt_includes_confidence() {
        let builder = OrchestratorPrompts::recovery(FailureType::Unknown, "Error", "Context", 1);

        let prompt = builder.build();

        assert!(prompt.text.contains("confidence"));
        assert!(prompt.text.contains("low | medium | high"));
    }

    #[test]
    fn test_recovery_prompt_includes_strategies() {
        let builder = OrchestratorPrompts::recovery(FailureType::TaskFailed, "Error", "Context", 1);

        let prompt = builder.build();

        assert!(prompt.text.contains("RETRY"));
        assert!(prompt.text.contains("ADJUST"));
        assert!(prompt.text.contains("SIMPLIFY"));
        assert!(prompt.text.contains("ESCALATE"));
        assert!(prompt.text.contains("HUMAN"));
    }

    #[test]
    fn test_recovery_version() {
        let version = OrchestratorPrompts::recovery_version();
        assert_eq!(version.family, "orchestrator-recovery");
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 0);
        assert_eq!(version.patch, 0);
    }

    #[test]
    fn test_failure_type_debug() {
        assert_eq!(format!("{:?}", FailureType::ParseError), "ParseError");
        assert_eq!(format!("{:?}", FailureType::TaskFailed), "TaskFailed");
        assert_eq!(format!("{:?}", FailureType::StuckLoop), "StuckLoop");
        assert_eq!(format!("{:?}", FailureType::ExternalError), "ExternalError");
        assert_eq!(format!("{:?}", FailureType::TestFailure), "TestFailure");
        assert_eq!(format!("{:?}", FailureType::Unknown), "Unknown");
    }
}
