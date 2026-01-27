//! Refactor agent prompt template.
//!
//! The refactor agent is a specialized persona for the orchestrator
//! that handles mid-stream plan modifications through conversation.

use crate::prompts::{PromptBuilder, PromptVersion};

/// Refactor agent prompt templates
pub struct RefactorPrompts;

impl RefactorPrompts {
    /// Current version for refactor prompts
    pub fn version() -> PromptVersion {
        PromptVersion::new("refactor-agent", 1, 0, 0)
    }

    /// Build a prompt for the refactor agent conversation.
    ///
    /// # Arguments
    /// * `user_message` - The user's message
    /// * `conversation_history` - Previous messages in the conversation
    /// * `current_work_status` - Summary of in-progress work
    /// * `plan_context` - Content from relevant planning files
    pub fn conversation(
        user_message: &str,
        conversation_history: &[(&str, &str)],
        current_work_status: Option<&str>,
        plan_context: Option<&str>,
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
            "User message:\n{}\n\nConversation history:\n{}",
            user_message, history_text
        );

        if let Some(status) = current_work_status {
            task.push_str(&format!("\n\nCurrent work in progress:\n{}", status));
        }

        if let Some(context) = plan_context {
            task.push_str(&format!("\n\nRelevant planning files:\n{}", context));
        }

        PromptBuilder::new()
            .version(Self::version())
            .role(REFACTOR_ROLE)
            .task(task)
            .constraint(INTENT_DETECTION)
            .constraint(HALT_GUIDELINES)
            .constraint(CHANGE_GUIDELINES)
            .output_json(REFACTOR_SCHEMA)
    }

    /// Build a prompt for generating proposed changes.
    ///
    /// # Arguments
    /// * `change_request` - What the user wants to change
    /// * `affected_files` - Contents of files that need modification
    /// * `in_progress_work` - Summary of work that might be affected
    pub fn propose_changes(
        change_request: &str,
        affected_files: &[(&str, &str)], // (path, content) pairs
        in_progress_work: Option<&str>,
    ) -> PromptBuilder {
        let files_text = affected_files
            .iter()
            .map(|(path, content)| format!("### {}\n```\n{}\n```", path, content))
            .collect::<Vec<_>>()
            .join("\n\n");

        let mut task = format!(
            "Generate proposed changes for this refactor request:\n\n\
             **Request**: {}\n\n\
             **Files to modify**:\n{}",
            change_request, files_text
        );

        if let Some(work) = in_progress_work {
            task.push_str(&format!(
                "\n\n**In-progress work that may be affected**:\n{}",
                work
            ));
        }

        PromptBuilder::new()
            .version(Self::version())
            .role(REFACTOR_ROLE)
            .task(task)
            .constraint("Generate minimal, targeted changes")
            .constraint("Preserve existing structure and conventions")
            .constraint("If changes affect in-progress work, note the impact")
            .output_json(CHANGE_PROPOSAL_SCHEMA)
    }

    /// Build a prompt for analyzing whether a change affects in-progress work.
    ///
    /// # Arguments
    /// * `proposed_change` - Description of the proposed change
    /// * `in_progress_tasks` - List of in-progress task summaries
    pub fn analyze_impact(
        proposed_change: &str,
        in_progress_tasks: &[&str],
    ) -> PromptBuilder {
        let tasks_text = in_progress_tasks
            .iter()
            .enumerate()
            .map(|(i, t)| format!("{}. {}", i + 1, t))
            .collect::<Vec<_>>()
            .join("\n");

        PromptBuilder::new()
            .version(Self::version())
            .role(REFACTOR_ROLE)
            .task(format!(
                "Analyze whether this proposed change affects in-progress work:\n\n\
                 **Proposed Change**:\n{}\n\n\
                 **In-Progress Tasks**:\n{}",
                proposed_change, tasks_text
            ))
            .constraint("Be conservative - if uncertain, assume there's an impact")
            .constraint("Identify specific conflicts")
            .output_json(IMPACT_SCHEMA)
    }
}

const REFACTOR_ROLE: &str = r#"You are the Refactor Orchestrator for nexor.

You help users modify the project plan mid-stream. You understand:
- The decomp file structure (decomp/M{N}/*.md ticket files)
- PROGRESS.md for tracking work status
- ROADMAP.md for the overall plan

Your responsibilities:
- Detect user intent from conversation
- Propose changes to planning files
- Determine when to halt production
- Guide the user through the refactor process

You are collaborative and thoughtful. You ask clarifying questions when needed.
You never make changes without understanding the full impact."#;

const INTENT_DETECTION: &str = r#"Detect user intent from their message:

**HALT_NOW**: User explicitly wants to stop production
- Examples: "STOP", "halt production", "pause everything", "stop all work"

**REFACTOR_NEEDED**: User describes changes that affect existing tickets
- Examples: "I want to change how X works", "Let's restructure Y", "The approach to Z isn't working"

**CLARIFYING**: User is exploring options, no action needed yet
- Examples: "What if we...", "Could we maybe...", "I'm thinking about..."

**JUST_CHATTING**: Casual conversation, no refactor intent
- Examples: "Hey", "What's up", "How's the project going?"

**EXIT_REFACTOR**: User wants to leave refactor mode
- Examples: "done", "exit", "let's continue", "resume production"

If uncertain between REFACTOR_NEEDED and CLARIFYING, choose CLARIFYING and ask a question."#;

const HALT_GUIDELINES: &str = r#"When to halt production:
- User explicitly requests it (HALT_NOW intent)
- Proposed changes affect in-progress tickets
- Changes affect dependencies of in-progress work

When NOT to halt:
- Casual questions about the project
- Changes to future (not yet started) tickets only
- User is just exploring ideas (CLARIFYING intent)

Always prefer the least disruptive action. If changes can wait until a natural checkpoint, prefer that."#;

const CHANGE_GUIDELINES: &str = r#"When proposing changes:
- Read the full file before modifying
- Make minimal, targeted changes
- Preserve the existing file structure and conventions
- Update PROGRESS.md to reflect new ticket states
- Note any dependencies that need updating
- If a ticket is in-progress, explain the impact

Files you can modify:
- decomp/M{N}/*.md - Ticket specification files
- PROGRESS.md - Work tracking
- ROADMAP.md - High-level plan (rarely)"#;

const REFACTOR_SCHEMA: &str = r#"{
  "intent": "halt_now | refactor_needed | clarifying | just_chatting | exit_refactor",
  "confidence": "low | medium | high",
  "reasoning": "string - Why you classified the intent this way",
  "should_halt_production": "boolean - Should production be halted?",
  "halt_reason": "string | null - If halting, why?",
  "affected_files": ["array of file paths that would be affected by changes"],
  "clarifying_question": "string | null - Question to ask if intent is unclear",
  "response": "string - Your conversational response to the user"
}"#;

const CHANGE_PROPOSAL_SCHEMA: &str = r#"{
  "summary": "string - Brief summary of the proposed changes",
  "changes": [
    {
      "file_path": "string - Path to the file",
      "change_type": "create | modify | delete",
      "reason": "string - Why this change is needed",
      "before_summary": "string | null - Summary of current content (for modify/delete)",
      "after_summary": "string - Summary of new content (for create/modify)",
      "full_content": "string - Complete new file content"
    }
  ],
  "impacts_in_progress": "boolean - Do these changes affect in-progress work?",
  "impact_details": "string | null - If impactful, what's affected?",
  "requires_halt": "boolean - Should production be halted for these changes?",
  "next_steps": ["array - What to do after applying changes"]
}"#;

const IMPACT_SCHEMA: &str = r#"{
  "has_impact": "boolean - Does this affect in-progress work?",
  "affected_tasks": ["array of task identifiers that are affected"],
  "impact_severity": "none | low | medium | high",
  "explanation": "string - Why these tasks are (or aren't) affected",
  "recommendation": "continue | wait_for_checkpoint | halt_immediately",
  "reasoning": "string - Why you recommend this action"
}"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_prompt_basic() {
        let builder = RefactorPrompts::conversation(
            "I want to change how we handle errors",
            &[],
            None,
            None,
        );

        let prompt = builder.build();

        assert!(prompt.text.contains("Refactor Orchestrator"));
        assert!(prompt.text.contains("I want to change how we handle errors"));
        assert!(prompt.text.contains("HALT_NOW"));
        assert!(prompt.text.contains("REFACTOR_NEEDED"));
    }

    #[test]
    fn test_conversation_prompt_with_history() {
        let history = vec![
            ("User", "Can we restructure the tickets?"),
            ("Refactor", "Sure, what did you have in mind?"),
        ];
        let builder = RefactorPrompts::conversation(
            "I want to split ticket 2.3",
            &history,
            Some("Worker is on slice 2.1.3"),
            None,
        );

        let prompt = builder.build();

        assert!(prompt.text.contains("Can we restructure the tickets?"));
        assert!(prompt.text.contains("Worker is on slice 2.1.3"));
    }

    #[test]
    fn test_conversation_prompt_with_plan_context() {
        let builder = RefactorPrompts::conversation(
            "What tickets are affected?",
            &[],
            None,
            Some("# Ticket 2.3\n\nImplement cost tracking"),
        );

        let prompt = builder.build();

        assert!(prompt.text.contains("Ticket 2.3"));
        assert!(prompt.text.contains("cost tracking"));
    }

    #[test]
    fn test_conversation_prompt_has_output_schema() {
        let builder = RefactorPrompts::conversation("test", &[], None, None);
        let prompt = builder.build();

        assert!(prompt.text.contains("intent"));
        assert!(prompt.text.contains("should_halt_production"));
        assert!(prompt.text.contains("response"));
    }

    #[test]
    fn test_propose_changes_prompt_basic() {
        let files = vec![
            ("decomp/M2/2.3.md", "# Ticket 2.3\n\nOld content"),
        ];
        let builder = RefactorPrompts::propose_changes(
            "Split ticket 2.3 into smaller slices",
            &files,
            None,
        );

        let prompt = builder.build();

        assert!(prompt.text.contains("Split ticket 2.3"));
        assert!(prompt.text.contains("decomp/M2/2.3.md"));
        assert!(prompt.text.contains("Old content"));
    }

    #[test]
    fn test_propose_changes_with_in_progress_work() {
        let files = vec![("PROGRESS.md", "# Progress\n\n...")];
        let builder = RefactorPrompts::propose_changes(
            "Update the roadmap",
            &files,
            Some("Agent is working on ticket 2.1"),
        );

        let prompt = builder.build();

        assert!(prompt.text.contains("working on ticket 2.1"));
        assert!(prompt.text.contains("may be affected"));
    }

    #[test]
    fn test_propose_changes_has_output_schema() {
        let builder = RefactorPrompts::propose_changes("test", &[], None);
        let prompt = builder.build();

        assert!(prompt.text.contains("change_type"));
        assert!(prompt.text.contains("create | modify | delete"));
        assert!(prompt.text.contains("requires_halt"));
    }

    #[test]
    fn test_analyze_impact_prompt_basic() {
        let tasks = vec![
            "Implement database schema (ticket 1.4)",
            "Add API endpoints (ticket 2.1)",
        ];
        let builder = RefactorPrompts::analyze_impact(
            "Change the database schema for tickets",
            &tasks,
        );

        let prompt = builder.build();

        assert!(prompt.text.contains("Change the database schema"));
        assert!(prompt.text.contains("1. Implement database schema"));
        assert!(prompt.text.contains("2. Add API endpoints"));
    }

    #[test]
    fn test_analyze_impact_has_output_schema() {
        let builder = RefactorPrompts::analyze_impact("test", &[]);
        let prompt = builder.build();

        assert!(prompt.text.contains("has_impact"));
        assert!(prompt.text.contains("impact_severity"));
        assert!(prompt.text.contains("halt_immediately"));
    }

    #[test]
    fn test_version() {
        let version = RefactorPrompts::version();
        assert_eq!(version.family, "refactor-agent");
        assert_eq!(version.major, 1);
    }
}
