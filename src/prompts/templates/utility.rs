//! Utility prompt templates for quick, well-defined tasks.
//!
//! Utilities are the cheapest agent tier, using smaller/faster models. They handle
//! well-defined, repeatable tasks like formatting, linting, and boilerplate generation.
//! They must recognize when a task is beyond their scope.

use crate::prompts::{PromptBuilder, PromptVersion};

/// Utility prompt templates for quick, well-defined tasks.
pub struct UtilityPrompts;

impl UtilityPrompts {
    /// Current version for task recognition prompts
    pub fn recognition_version() -> PromptVersion {
        PromptVersion::new("utility-recognition", 1, 0, 0)
    }

    /// Build a prompt for recognizing and categorizing a task.
    ///
    /// # Arguments
    /// * `task_description` - What the task asks for
    pub fn task_recognition(task_description: &str) -> PromptBuilder {
        PromptBuilder::new()
            .version(Self::recognition_version())
            .role(UTILITY_ROLE)
            .task(format!(
                "Categorize this task:\n\n{}",
                task_description
            ))
            .constraint(RECOGNITION_THINKING)
            .constraint("If unsure about the category, escalate immediately")
            .constraint("Never attempt tasks that require understanding business logic")
            .output_json(RECOGNITION_SCHEMA)
    }
}

const UTILITY_ROLE: &str = r#"You are Helper, handling quick well-defined tasks efficiently.

You handle:
- FORMAT: Apply code formatter to files
- LINT: Run linter, fix auto-fixable issues
- BOILERPLATE: Generate code from templates
- DOCS: Update documentation, add docstrings
- RENAME: Find/replace identifiers

You do NOT handle:
- Tasks requiring business logic understanding
- Architectural decisions
- Complex implementations
- Anything taking more than a few minutes

Be terse. Report only completions and errors."#;

const RECOGNITION_THINKING: &str = r#"Quickly categorize:

1. IDENTIFY: What type of task is this?
2. SCOPE: Is this within my capabilities?
3. DECIDE: Handle it or escalate?

Categories I handle:
- FORMAT: "format this file", "apply rustfmt", "fix indentation"
- LINT: "fix lint errors", "run clippy", "fix warnings"
- BOILERPLATE: "generate struct", "add derive macro", "create test template"
- DOCS: "add docstring", "update readme", "fix typo in docs"
- RENAME: "rename function", "change variable name", "update identifier"

Escalate if:
- Task requires understanding WHY code does something
- Task involves design decisions
- Task is ambiguous or open-ended
- Task would take more than a few minutes"#;

const RECOGNITION_SCHEMA: &str = r#"{
  "category": "format | lint | boilerplate | docs | rename | escalate",
  "confidence": "high | medium | low",
  "reasoning": "string - Brief explanation of categorization",
  "escalation_reason": "string | null - If escalating, why",
  "can_proceed": "boolean"
}"#;

// Slice 4.4.2: Templated Execution Prompts

impl UtilityPrompts {
    /// Current version for execution prompts
    pub fn execution_version() -> PromptVersion {
        PromptVersion::new("utility-execution", 1, 0, 0)
    }

    /// Build a prompt for formatting code.
    pub fn format_execution(
        file_path: &str,
        file_content: &str,
        format_rules: Option<&str>,
    ) -> PromptBuilder {
        let mut builder = PromptBuilder::new()
            .version(Self::execution_version())
            .role(UTILITY_ROLE)
            .task(format!(
                "Format this file according to project standards.\n\n\
                 **File**: {}\n\n\
                 **Content**:\n```\n{}\n```",
                file_path, file_content
            ))
            .constraint("Apply consistent formatting")
            .constraint("Do not change logic or functionality")
            .constraint("Preserve all comments")
            .output_json(FORMAT_SCHEMA);

        if let Some(rules) = format_rules {
            builder = builder.constraint(&format!("Follow these rules: {}", rules));
        }

        builder
    }

    /// Build a prompt for fixing lint issues.
    pub fn lint_execution(
        file_path: &str,
        file_content: &str,
        lint_errors: &str,
    ) -> PromptBuilder {
        PromptBuilder::new()
            .version(Self::execution_version())
            .role(UTILITY_ROLE)
            .task(format!(
                "Fix the lint errors in this file.\n\n\
                 **File**: {}\n\n\
                 **Content**:\n```\n{}\n```\n\n\
                 **Lint Errors**:\n```\n{}\n```",
                file_path, file_content, lint_errors
            ))
            .constraint("Only fix the reported errors")
            .constraint("Do not change unrelated code")
            .constraint("If a fix requires understanding business logic, escalate")
            .output_json(LINT_SCHEMA)
    }

    /// Build a prompt for generating boilerplate.
    pub fn boilerplate_execution(
        template_type: &str,
        parameters: &str,
        target_path: &str,
    ) -> PromptBuilder {
        PromptBuilder::new()
            .version(Self::execution_version())
            .role(UTILITY_ROLE)
            .task(format!(
                "Generate boilerplate code.\n\n\
                 **Template Type**: {}\n\n\
                 **Parameters**:\n{}\n\n\
                 **Target File**: {}",
                template_type, parameters, target_path
            ))
            .constraint("Follow the standard template exactly")
            .constraint("Fill in parameters as specified")
            .constraint("Include appropriate derive macros")
            .output_json(BOILERPLATE_SCHEMA)
    }

    /// Build a prompt for updating documentation.
    pub fn docs_execution(
        file_path: &str,
        file_content: &str,
        doc_request: &str,
    ) -> PromptBuilder {
        PromptBuilder::new()
            .version(Self::execution_version())
            .role(UTILITY_ROLE)
            .task(format!(
                "Update documentation as requested.\n\n\
                 **File**: {}\n\n\
                 **Content**:\n```\n{}\n```\n\n\
                 **Request**: {}",
                file_path, file_content, doc_request
            ))
            .constraint("Only modify documentation, not code")
            .constraint("Use standard docstring format for the language")
            .constraint("Be concise and accurate")
            .output_json(DOCS_SCHEMA)
    }

    /// Build a prompt for renaming identifiers.
    pub fn rename_execution(
        file_path: &str,
        file_content: &str,
        old_name: &str,
        new_name: &str,
    ) -> PromptBuilder {
        PromptBuilder::new()
            .version(Self::execution_version())
            .role(UTILITY_ROLE)
            .task(format!(
                "Rename identifier in this file.\n\n\
                 **File**: {}\n\n\
                 **Content**:\n```\n{}\n```\n\n\
                 **Rename**: `{}` → `{}`",
                file_path, file_content, old_name, new_name
            ))
            .constraint("Rename all occurrences of the identifier")
            .constraint("Update related references (comments, strings if appropriate)")
            .constraint("Do not rename unrelated similar names")
            .output_json(RENAME_SCHEMA)
    }
}

const FORMAT_SCHEMA: &str = r#"{
  "formatted_content": "string - The formatted file content",
  "changes_made": ["array of brief descriptions of formatting changes"],
  "status": "success | no_changes | error",
  "error": "string | null"
}"#;

const LINT_SCHEMA: &str = r#"{
  "fixed_content": "string - The file content with lint fixes",
  "fixes_applied": [
    {
      "line": "number",
      "error": "string - The lint error",
      "fix": "string - What was changed"
    }
  ],
  "unfixable": ["array of errors that couldn't be auto-fixed"],
  "status": "success | partial | escalate",
  "escalation_reason": "string | null"
}"#;

const BOILERPLATE_SCHEMA: &str = r#"{
  "generated_content": "string - The generated boilerplate code",
  "template_used": "string - Which template was applied",
  "parameters_applied": ["array of parameter substitutions made"],
  "status": "success | error",
  "error": "string | null"
}"#;

const DOCS_SCHEMA: &str = r#"{
  "updated_content": "string - The file with updated documentation",
  "docs_added": ["array of documentation additions"],
  "docs_modified": ["array of documentation modifications"],
  "status": "success | error",
  "error": "string | null"
}"#;

const RENAME_SCHEMA: &str = r#"{
  "renamed_content": "string - The file with renamed identifiers",
  "occurrences_renamed": "number - How many occurrences were renamed",
  "locations": ["array of line numbers where renames occurred"],
  "status": "success | error",
  "error": "string | null"
}"#;

// Slice 4.4.3: Minimal Reporting Prompt

impl UtilityPrompts {
    /// Current version for reporting prompts
    pub fn reporting_version() -> PromptVersion {
        PromptVersion::new("utility-reporting", 1, 0, 0)
    }

    /// Build a prompt for generating a completion report.
    ///
    /// # Arguments
    /// * `task_category` - What type of task was performed
    /// * `files_affected` - List of files that were modified
    /// * `success` - Whether the task succeeded
    /// * `error_message` - Error message if the task failed
    pub fn completion_report(
        task_category: &str,
        files_affected: &[&str],
        success: bool,
        error_message: Option<&str>,
    ) -> PromptBuilder {
        let files_list = files_affected.join(", ");

        PromptBuilder::new()
            .version(Self::reporting_version())
            .role(UTILITY_ROLE)
            .task(format!(
                "Generate a completion report.\n\n\
                 **Task Type**: {}\n\
                 **Files**: {}\n\
                 **Success**: {}\n\
                 **Error**: {}",
                task_category,
                files_list,
                success,
                error_message.unwrap_or("None")
            ))
            .constraint(REPORTING_RULES)
            .output_json(REPORT_SCHEMA)
    }
}

const REPORTING_RULES: &str = r#"Generate a terse report:

Success format:
- "Done. Formatted src/*.rs (4 files)"
- "Done. Fixed 3 lint errors in src/main.rs"
- "Done. Renamed `foo` to `bar` (12 occurrences)"

Error format:
- "Failed: src/lib.rs: syntax error on line 42"
- "Escalated: Lint fix requires business logic understanding"

Rules:
- Never explain reasoning unless asked
- One line only
- Include counts where applicable
- Be specific about errors"#;

const REPORT_SCHEMA: &str = r#"{
  "report": "string - The one-line report for the feed",
  "status": "done | failed | escalated",
  "details": {
    "files_count": "number",
    "changes_count": "number | null",
    "error_file": "string | null",
    "error_line": "number | null"
  }
}"#;

// Slice 4.4.4: Escalation Trigger Prompt

impl UtilityPrompts {
    /// Current version for escalation prompts
    pub fn escalation_version() -> PromptVersion {
        PromptVersion::new("utility-escalation", 1, 0, 0)
    }

    /// Build a prompt for deciding whether to escalate a task.
    ///
    /// # Arguments
    /// * `task_description` - The task that might need escalation
    /// * `complexity_indicators` - Signs that this might be too complex
    pub fn escalation_check(
        task_description: &str,
        complexity_indicators: &[&str],
    ) -> PromptBuilder {
        let indicators = complexity_indicators
            .iter()
            .map(|i| format!("- {}", i))
            .collect::<Vec<_>>()
            .join("\n");

        PromptBuilder::new()
            .version(Self::escalation_version())
            .role(UTILITY_ROLE)
            .task(format!(
                "Decide if this task should be escalated.\n\n\
                 **Task**: {}\n\n\
                 **Complexity Indicators**:\n{}",
                task_description, indicators
            ))
            .constraint(ESCALATION_THINKING)
            .constraint("When in doubt, escalate")
            .constraint("Better to escalate unnecessarily than fail badly")
            .output_json(ESCALATION_SCHEMA)
    }
}

const ESCALATION_THINKING: &str = r#"Quickly decide: handle or escalate?

Escalate immediately if:
- Task requires understanding business logic
- Task involves architectural decisions
- You're unsure what the right answer is
- Task would take more than a few minutes
- Multiple files need coordinated changes
- Task has unclear requirements

Handle if:
- Clear, mechanical transformation
- Well-defined template application
- Simple find/replace
- Standard formatting

Rule: 5-second decision. If you're thinking hard, escalate."#;

const ESCALATION_SCHEMA: &str = r#"{
  "decision": "handle | escalate",
  "confidence": "high | medium | low",
  "reason": "string - Very brief reason (under 20 words)",
  "recommended_tier": "worker | orchestrator | null",
  "simplified_version": "string | null - If task could be simplified to be handleable"
}"#;
