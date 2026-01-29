//! Prompt template and builder implementation.

use crate::prompts::version::PromptVersion;

/// The canonical prompt template structure.
/// All prompts follow this skeleton for consistency.
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    /// The role/persona section (who the agent is)
    pub role: String,
    /// Context about the task (codebase info, history, conventions)
    pub context: PromptContext,
    /// The specific task to perform
    pub task: String,
    /// Constraints and rules to follow
    pub constraints: Vec<String>,
    /// Expected output format specification
    pub output_format: OutputFormat,
    /// Few-shot examples (optional)
    pub examples: Vec<Example>,
}

#[derive(Debug, Clone, Default)]
pub struct PromptContext {
    /// Files being modified (always include full if small)
    pub files_to_modify: Vec<FileContent>,
    /// Reference files (include on request)
    pub reference_files: Vec<FileContent>,
    /// Task-specific context
    pub task_context: Option<String>,
    /// Conversation history
    pub history: Vec<HistoryEntry>,
    /// Project conventions (CLAUDE.md content)
    pub conventions: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FileContent {
    pub path: String,
    pub content: String,
    /// Used when file is too large
    pub summary: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    /// "user" or "assistant"
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub enum OutputFormat {
    /// Expect JSON matching a schema
    Json {
        schema: String,
        example: Option<String>,
    },
    /// Expect natural language
    Text { guidelines: Option<String> },
    /// Expect code
    Code {
        language: String,
        guidelines: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub struct Example {
    pub input: String,
    pub output: String,
    pub explanation: Option<String>,
}

impl Default for PromptTemplate {
    fn default() -> Self {
        Self {
            role: String::new(),
            context: PromptContext::default(),
            task: String::new(),
            constraints: Vec::new(),
            output_format: OutputFormat::Text { guidelines: None },
            examples: Vec::new(),
        }
    }
}

/// Builder for assembling prompts from components.
#[derive(Debug, Default)]
pub struct PromptBuilder {
    template: PromptTemplate,
    version: Option<PromptVersion>,
}

impl PromptBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the role/persona section
    pub fn role(mut self, role: impl Into<String>) -> Self {
        self.template.role = role.into();
        self
    }

    /// Add a file to the context (will be modified)
    pub fn file_to_modify(mut self, path: impl Into<String>, content: impl Into<String>) -> Self {
        self.template.context.files_to_modify.push(FileContent {
            path: path.into(),
            content: content.into(),
            summary: None,
        });
        self
    }

    /// Add a reference file to the context
    pub fn reference_file(mut self, path: impl Into<String>, content: impl Into<String>) -> Self {
        self.template.context.reference_files.push(FileContent {
            path: path.into(),
            content: content.into(),
            summary: None,
        });
        self
    }

    /// Set the task description
    pub fn task(mut self, task: impl Into<String>) -> Self {
        self.template.task = task.into();
        self
    }

    /// Add a constraint
    pub fn constraint(mut self, constraint: impl Into<String>) -> Self {
        self.template.constraints.push(constraint.into());
        self
    }

    /// Set expected output format as JSON with schema
    pub fn output_json(mut self, schema: impl Into<String>) -> Self {
        self.template.output_format = OutputFormat::Json {
            schema: schema.into(),
            example: None,
        };
        self
    }

    /// Set expected output format as JSON with schema and example
    pub fn output_json_with_example(
        mut self,
        schema: impl Into<String>,
        example: impl Into<String>,
    ) -> Self {
        self.template.output_format = OutputFormat::Json {
            schema: schema.into(),
            example: Some(example.into()),
        };
        self
    }

    /// Set expected output format as text
    pub fn output_text(mut self, guidelines: Option<String>) -> Self {
        self.template.output_format = OutputFormat::Text { guidelines };
        self
    }

    /// Set expected output format as code
    pub fn output_code(mut self, language: impl Into<String>, guidelines: Option<String>) -> Self {
        self.template.output_format = OutputFormat::Code {
            language: language.into(),
            guidelines,
        };
        self
    }

    /// Add a few-shot example
    pub fn example(mut self, input: impl Into<String>, output: impl Into<String>) -> Self {
        self.template.examples.push(Example {
            input: input.into(),
            output: output.into(),
            explanation: None,
        });
        self
    }

    /// Add a few-shot example with explanation
    pub fn example_with_explanation(
        mut self,
        input: impl Into<String>,
        output: impl Into<String>,
        explanation: impl Into<String>,
    ) -> Self {
        self.template.examples.push(Example {
            input: input.into(),
            output: output.into(),
            explanation: Some(explanation.into()),
        });
        self
    }

    /// Set conventions (CLAUDE.md content)
    pub fn conventions(mut self, conventions: impl Into<String>) -> Self {
        self.template.context.conventions = Some(conventions.into());
        self
    }

    /// Set task context
    pub fn task_context(mut self, context: impl Into<String>) -> Self {
        self.template.context.task_context = Some(context.into());
        self
    }

    /// Add a history entry
    pub fn history(mut self, role: impl Into<String>, content: impl Into<String>) -> Self {
        self.template.context.history.push(HistoryEntry {
            role: role.into(),
            content: content.into(),
        });
        self
    }

    /// Set the prompt version for tracking
    pub fn version(mut self, version: PromptVersion) -> Self {
        self.version = Some(version);
        self
    }

    /// Build the final prompt string
    pub fn build(self) -> BuiltPrompt {
        let prompt_text = self.render();
        BuiltPrompt {
            text: prompt_text,
            version: self.version,
            template: self.template,
        }
    }

    fn render(&self) -> String {
        let mut sections = Vec::new();

        // Role section
        if !self.template.role.is_empty() {
            sections.push(format!("## Your Role\n\n{}", self.template.role));
        }

        // Context section
        let context = self.render_context();
        if !context.is_empty() {
            sections.push(format!("## Context\n\n{}", context));
        }

        // Task section
        if !self.template.task.is_empty() {
            sections.push(format!("## Task\n\n{}", self.template.task));
        }

        // Constraints section
        if !self.template.constraints.is_empty() {
            let constraints = self
                .template
                .constraints
                .iter()
                .map(|c| format!("- {}", c))
                .collect::<Vec<_>>()
                .join("\n");
            sections.push(format!("## Constraints\n\n{}", constraints));
        }

        // Output format section
        let output_format = self.render_output_format();
        if !output_format.is_empty() {
            sections.push(format!("## Output Format\n\n{}", output_format));
        }

        // Examples section
        if !self.template.examples.is_empty() {
            let examples = self.render_examples();
            sections.push(format!("## Examples\n\n{}", examples));
        }

        sections.join("\n\n---\n\n")
    }

    fn render_context(&self) -> String {
        let mut parts = Vec::new();

        // Conventions first (project style)
        if let Some(ref conv) = self.template.context.conventions {
            parts.push(format!("### Project Conventions\n\n{}", conv));
        }

        // Task context
        if let Some(ref ctx) = self.template.context.task_context {
            parts.push(format!("### Task Context\n\n{}", ctx));
        }

        // Files to modify
        if !self.template.context.files_to_modify.is_empty() {
            let files = self
                .template
                .context
                .files_to_modify
                .iter()
                .map(|f| format!("**{}**:\n```\n{}\n```", f.path, f.content))
                .collect::<Vec<_>>()
                .join("\n\n");
            parts.push(format!("### Files to Modify\n\n{}", files));
        }

        // Reference files
        if !self.template.context.reference_files.is_empty() {
            let files = self
                .template
                .context
                .reference_files
                .iter()
                .map(|f| format!("**{}**:\n```\n{}\n```", f.path, f.content))
                .collect::<Vec<_>>()
                .join("\n\n");
            parts.push(format!("### Reference Files\n\n{}", files));
        }

        // Conversation history
        if !self.template.context.history.is_empty() {
            let history = self
                .template
                .context
                .history
                .iter()
                .map(|h| format!("**{}**: {}", h.role, h.content))
                .collect::<Vec<_>>()
                .join("\n\n");
            parts.push(format!("### Conversation History\n\n{}", history));
        }

        parts.join("\n\n")
    }

    fn render_output_format(&self) -> String {
        match &self.template.output_format {
            OutputFormat::Json { schema, example } => {
                let mut s = format!(
                    "Respond with valid JSON matching this schema:\n\n```json\n{}\n```",
                    schema
                );
                if let Some(ex) = example {
                    s.push_str(&format!("\n\nExample:\n```json\n{}\n```", ex));
                }
                s
            }
            OutputFormat::Text { guidelines } => guidelines
                .clone()
                .unwrap_or_else(|| "Respond in natural language.".to_string()),
            OutputFormat::Code {
                language,
                guidelines,
            } => {
                let mut s = format!("Respond with {} code.", language);
                if let Some(g) = guidelines {
                    s.push_str(&format!("\n\n{}", g));
                }
                s
            }
        }
    }

    fn render_examples(&self) -> String {
        self.template
            .examples
            .iter()
            .enumerate()
            .map(|(i, ex)| {
                let mut s = format!(
                    "### Example {}\n\n**Input:**\n{}\n\n**Output:**\n{}",
                    i + 1,
                    ex.input,
                    ex.output
                );
                if let Some(ref explanation) = ex.explanation {
                    s.push_str(&format!("\n\n**Why:** {}", explanation));
                }
                s
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

/// The result of building a prompt
#[derive(Debug, Clone)]
pub struct BuiltPrompt {
    /// The final prompt text to send to the LLM
    pub text: String,
    /// Version info for tracking
    pub version: Option<PromptVersion>,
    /// The original template (for debugging)
    pub template: PromptTemplate,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_template_default() {
        let template = PromptTemplate::default();
        assert!(template.role.is_empty());
        assert!(template.task.is_empty());
        assert!(template.constraints.is_empty());
        assert!(template.examples.is_empty());
    }

    #[test]
    fn test_prompt_builder_basic() {
        let prompt = PromptBuilder::new()
            .role("You are a code reviewer")
            .task("Review this code for bugs")
            .constraint("Focus on security issues")
            .constraint("Keep feedback concise")
            .build();

        assert!(prompt.text.contains("## Your Role"));
        assert!(prompt.text.contains("You are a code reviewer"));
        assert!(prompt.text.contains("## Task"));
        assert!(prompt.text.contains("Review this code for bugs"));
        assert!(prompt.text.contains("## Constraints"));
        assert!(prompt.text.contains("Focus on security issues"));
    }

    #[test]
    fn test_prompt_builder_with_files() {
        let prompt = PromptBuilder::new()
            .role("You are a developer")
            .file_to_modify("src/main.rs", "fn main() {}")
            .reference_file("src/lib.rs", "pub mod utils;")
            .task("Add logging")
            .build();

        assert!(prompt.text.contains("### Files to Modify"));
        assert!(prompt.text.contains("src/main.rs"));
        assert!(prompt.text.contains("### Reference Files"));
        assert!(prompt.text.contains("src/lib.rs"));
    }

    #[test]
    fn test_prompt_builder_with_json_output() {
        let prompt = PromptBuilder::new()
            .task("Analyze code")
            .output_json(r#"{"analysis": "string", "score": "number"}"#)
            .build();

        assert!(prompt.text.contains("## Output Format"));
        assert!(prompt.text.contains("Respond with valid JSON"));
    }

    #[test]
    fn test_prompt_builder_with_examples() {
        let prompt = PromptBuilder::new()
            .task("Summarize")
            .example("Long text here", "Short summary")
            .example_with_explanation("Another long text", "Another summary", "This shows brevity")
            .build();

        assert!(prompt.text.contains("## Examples"));
        assert!(prompt.text.contains("### Example 1"));
        assert!(prompt.text.contains("### Example 2"));
        assert!(prompt.text.contains("**Why:**"));
    }

    #[test]
    fn test_prompt_builder_with_conventions() {
        let prompt = PromptBuilder::new()
            .task("Do work")
            .conventions("Use snake_case everywhere")
            .build();

        assert!(prompt.text.contains("### Project Conventions"));
        assert!(prompt.text.contains("Use snake_case everywhere"));
    }

    #[test]
    fn test_prompt_builder_with_task_context() {
        let prompt = PromptBuilder::new()
            .task("Implement feature")
            .task_context("This is part of milestone M3")
            .build();

        assert!(prompt.text.contains("### Task Context"));
        assert!(prompt.text.contains("milestone M3"));
    }

    #[test]
    fn test_prompt_builder_with_history() {
        let prompt = PromptBuilder::new()
            .task("Continue work")
            .history("user", "Please fix the bug")
            .history("assistant", "I found the issue in line 42")
            .build();

        assert!(prompt.text.contains("### Conversation History"));
        assert!(prompt.text.contains("**user**: Please fix the bug"));
        assert!(prompt.text.contains("**assistant**: I found the issue"));
    }

    #[test]
    fn test_prompt_builder_output_json_with_example() {
        let prompt = PromptBuilder::new()
            .task("Analyze")
            .output_json_with_example(
                r#"{"score": "number"}"#,
                r#"{"score": 42}"#,
            )
            .build();

        assert!(prompt.text.contains("Respond with valid JSON"));
        assert!(prompt.text.contains(r#"{"score": "number"}"#));
        assert!(prompt.text.contains("Example:"));
        assert!(prompt.text.contains(r#"{"score": 42}"#));
    }

    #[test]
    fn test_prompt_builder_output_text_with_guidelines() {
        let prompt = PromptBuilder::new()
            .task("Summarize")
            .output_text(Some("Keep it under 100 words".to_string()))
            .build();

        assert!(prompt.text.contains("## Output Format"));
        assert!(prompt.text.contains("Keep it under 100 words"));
    }

    #[test]
    fn test_prompt_builder_output_text_no_guidelines() {
        let prompt = PromptBuilder::new()
            .task("Summarize")
            .output_text(None)
            .build();

        assert!(prompt.text.contains("Respond in natural language."));
    }

    #[test]
    fn test_prompt_builder_output_code() {
        let prompt = PromptBuilder::new()
            .task("Write code")
            .output_code("rust", Some("Follow clippy lints".to_string()))
            .build();

        assert!(prompt.text.contains("Respond with rust code."));
        assert!(prompt.text.contains("Follow clippy lints"));
    }

    #[test]
    fn test_prompt_builder_output_code_no_guidelines() {
        let prompt = PromptBuilder::new()
            .task("Write code")
            .output_code("python", None)
            .build();

        assert!(prompt.text.contains("Respond with python code."));
        assert!(!prompt.text.contains("\n\n\n")); // no extra blank from missing guidelines
    }

    #[test]
    fn test_prompt_builder_with_version() {
        let version = PromptVersion::new("test", 1, 0, 0);
        let prompt = PromptBuilder::new()
            .task("Do something")
            .version(version.clone())
            .build();

        assert!(prompt.version.is_some());
        assert_eq!(prompt.version.unwrap().semver(), "1.0.0");
    }

    #[test]
    fn test_prompt_builder_no_version() {
        let prompt = PromptBuilder::new().task("Do something").build();
        assert!(prompt.version.is_none());
    }

    #[test]
    fn test_prompt_builder_full_kitchen_sink() {
        let prompt = PromptBuilder::new()
            .role("You are an expert Rust developer")
            .conventions("Use thiserror for errors")
            .task_context("Working on auth module")
            .file_to_modify("src/auth.rs", "fn login() {}")
            .reference_file("src/types.rs", "pub struct User {}")
            .history("user", "Add OAuth support")
            .task("Implement OAuth2 flow")
            .constraint("Must support Google and GitHub")
            .constraint("Use async/await")
            .output_code("rust", Some("Include tests".to_string()))
            .example("Simple OAuth", "fn oauth() -> Result<Token>")
            .example_with_explanation(
                "Error handling",
                "fn handle() -> Result<(), AuthError>",
                "Shows proper error types",
            )
            .version(PromptVersion::new("implementation", 2, 0, 0))
            .build();

        // All sections present
        assert!(prompt.text.contains("## Your Role"));
        assert!(prompt.text.contains("## Context"));
        assert!(prompt.text.contains("## Task"));
        assert!(prompt.text.contains("## Constraints"));
        assert!(prompt.text.contains("## Output Format"));
        assert!(prompt.text.contains("## Examples"));

        // Sections separated by ---
        assert!(prompt.text.contains("---"));

        // Content checks
        assert!(prompt.text.contains("expert Rust developer"));
        assert!(prompt.text.contains("thiserror"));
        assert!(prompt.text.contains("auth module"));
        assert!(prompt.text.contains("src/auth.rs"));
        assert!(prompt.text.contains("src/types.rs"));
        assert!(prompt.text.contains("OAuth2 flow"));
        assert!(prompt.text.contains("Google and GitHub"));
        assert!(prompt.text.contains("async/await"));
        assert!(prompt.text.contains("**Why:** Shows proper error types"));
    }

    #[test]
    fn test_prompt_builder_empty_build() {
        let prompt = PromptBuilder::new().build();
        // Output format defaults to Text with no guidelines => "Respond in natural language."
        assert!(prompt.text.contains("Respond in natural language."));
        // No role, task, constraints, examples sections
        assert!(!prompt.text.contains("## Your Role"));
        assert!(!prompt.text.contains("## Task"));
        assert!(!prompt.text.contains("## Constraints"));
        assert!(!prompt.text.contains("## Examples"));
    }

    #[test]
    fn test_built_prompt_template_preserved() {
        let prompt = PromptBuilder::new()
            .role("tester")
            .task("test things")
            .constraint("be thorough")
            .build();

        assert_eq!(prompt.template.role, "tester");
        assert_eq!(prompt.template.task, "test things");
        assert_eq!(prompt.template.constraints.len(), 1);
    }

    #[test]
    fn test_prompt_context_default() {
        let ctx = PromptContext::default();
        assert!(ctx.files_to_modify.is_empty());
        assert!(ctx.reference_files.is_empty());
        assert!(ctx.task_context.is_none());
        assert!(ctx.history.is_empty());
        assert!(ctx.conventions.is_none());
    }

    #[test]
    fn test_prompt_builder_omits_empty_sections() {
        let prompt = PromptBuilder::new().task("Just a task").build();

        assert!(!prompt.text.contains("## Your Role"));
        assert!(prompt.text.contains("## Task"));
        assert!(!prompt.text.contains("## Constraints"));
        assert!(!prompt.text.contains("## Examples"));
    }
}
