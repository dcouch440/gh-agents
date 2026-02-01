//! Tool definitions and registry for agent tool usage.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Schema for defining a tool that agents can use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Unique name for the tool
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// Parameters the tool accepts
    pub parameters: Vec<ToolParameter>,

    /// What the tool returns
    pub returns: ToolReturn,

    /// Side effects (for safety classification)
    pub side_effects: Vec<SideEffect>,

    /// Whether this tool requires user approval
    pub requires_approval: bool,

    /// Example usage
    pub examples: Vec<ToolExample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    /// Parameter name
    pub name: String,

    /// Parameter type
    pub param_type: ParameterType,

    /// Description of the parameter
    pub description: String,

    /// Whether this parameter is required
    pub required: bool,

    /// Default value if not required
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterType {
    String,
    Number,
    Boolean,
    Array { item_type: Box<ParameterType> },
    Object { properties: HashMap<String, ParameterType> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolReturn {
    /// Return type
    pub return_type: ParameterType,

    /// Description of what's returned
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SideEffect {
    /// Reads from filesystem
    ReadsFilesystem,
    /// Modifies filesystem
    ModifiesFilesystem,
    /// Executes commands
    ExecutesCommands,
    /// Network access
    NetworkAccess,
    /// Modifies git state
    ModifiesGit,
    /// None
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExample {
    /// Description of what this example does
    pub description: String,

    /// The tool invocation
    pub invocation: ToolInvocation,

    /// Expected result
    pub expected_result: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInvocation {
    /// Tool name
    pub tool: String,

    /// Parameters as key-value pairs
    pub params: HashMap<String, serde_json::Value>,
}

impl ToolDefinition {
    /// Generate a schema string for prompt inclusion
    pub fn to_schema_string(&self) -> String {
        let params: Vec<String> = self
            .parameters
            .iter()
            .map(|p| {
                let required = if p.required { " (required)" } else { "" };
                format!("  - {}: {}{} - {}", p.name, p.param_type, required, p.description)
            })
            .collect();

        format!(
            r#"**{}**: {}

Parameters:
{}

Returns: {} - {}

Side effects: {:?}
Requires approval: {}"#,
            self.name,
            self.description,
            params.join("\n"),
            self.returns.return_type,
            self.returns.description,
            self.side_effects,
            self.requires_approval
        )
    }

    /// Check if this tool is safe (no destructive side effects)
    pub fn is_safe(&self) -> bool {
        !self
            .side_effects
            .iter()
            .any(|e| matches!(e, SideEffect::ModifiesFilesystem | SideEffect::ModifiesGit | SideEffect::ExecutesCommands))
    }
}

impl std::fmt::Display for ParameterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String => write!(f, "string"),
            Self::Number => write!(f, "number"),
            Self::Boolean => write!(f, "boolean"),
            Self::Array { item_type } => write!(f, "array<{}>", item_type),
            Self::Object { .. } => write!(f, "object"),
        }
    }
}

/// Registry of all available tools
#[derive(Debug, Default)]
pub struct ToolRegistry {
    tools: HashMap<String, ToolDefinition>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool
    pub fn register(&mut self, tool: ToolDefinition) {
        self.tools.insert(tool.name.clone(), tool);
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.get(name)
    }

    /// Get all tools
    pub fn all(&self) -> impl Iterator<Item = &ToolDefinition> {
        self.tools.values()
    }

    /// Get tools by side effect type
    pub fn by_side_effect(&self, effect: SideEffect) -> Vec<&ToolDefinition> {
        self.tools
            .values()
            .filter(|t: &&ToolDefinition| t.side_effects.iter().any(|e| std::mem::discriminant(e) == std::mem::discriminant(&effect)))
            .collect()
    }

    /// Get safe tools only
    pub fn safe_tools(&self) -> Vec<&ToolDefinition> {
        self.tools.values().filter(|t| t.is_safe()).collect()
    }

    /// Generate a summary for prompt inclusion
    pub fn to_prompt_summary(&self) -> String {
        let summaries: Vec<String> = self.tools.values().map(|t| format!("- **{}**: {}", t.name, t.description)).collect();

        summaries.join("\n")
    }
}

/// Pre-defined file operation tools
pub mod file_tools {
    use super::*;

    pub fn read_file() -> ToolDefinition {
        ToolDefinition {
            name: "read_file".to_string(),
            description: "Read the contents of a file".to_string(),
            parameters: vec![ToolParameter {
                name: "path".to_string(),
                param_type: ParameterType::String,
                description: "Relative path from project root".to_string(),
                required: true,
                default: None,
            }],
            returns: ToolReturn {
                return_type: ParameterType::Object {
                    properties: [("content".to_string(), ParameterType::String), ("exists".to_string(), ParameterType::Boolean)].into_iter().collect(),
                },
                description: "File content and existence status".to_string(),
            },
            side_effects: vec![SideEffect::ReadsFilesystem],
            requires_approval: false,
            examples: vec![ToolExample {
                description: "Read a source file".to_string(),
                invocation: ToolInvocation {
                    tool: "read_file".to_string(),
                    params: [("path".to_string(), serde_json::json!("src/main.rs"))].into_iter().collect(),
                },
                expected_result: r#"{"content": "fn main() {...}", "exists": true}"#.to_string(),
            }],
        }
    }

    pub fn write_file() -> ToolDefinition {
        ToolDefinition {
            name: "write_file".to_string(),
            description: "Write content to a file, creating it if it doesn't exist".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "path".to_string(),
                    param_type: ParameterType::String,
                    description: "Relative path from project root".to_string(),
                    required: true,
                    default: None,
                },
                ToolParameter {
                    name: "content".to_string(),
                    param_type: ParameterType::String,
                    description: "Full file content to write".to_string(),
                    required: true,
                    default: None,
                },
            ],
            returns: ToolReturn {
                return_type: ParameterType::Object {
                    properties: [("success".to_string(), ParameterType::Boolean), ("bytes_written".to_string(), ParameterType::Number)]
                        .into_iter()
                        .collect(),
                },
                description: "Success status and bytes written".to_string(),
            },
            side_effects: vec![SideEffect::ModifiesFilesystem],
            requires_approval: false, // May be true depending on config
            examples: vec![ToolExample {
                description: "Create a new file".to_string(),
                invocation: ToolInvocation {
                    tool: "write_file".to_string(),
                    params: [("path".to_string(), serde_json::json!("src/lib.rs")), ("content".to_string(), serde_json::json!("// New library\n"))]
                        .into_iter()
                        .collect(),
                },
                expected_result: r#"{"success": true, "bytes_written": 15}"#.to_string(),
            }],
        }
    }

    pub fn list_dir() -> ToolDefinition {
        ToolDefinition {
            name: "list_dir".to_string(),
            description: "List files and directories in a path".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "path".to_string(),
                    param_type: ParameterType::String,
                    description: "Relative path from project root".to_string(),
                    required: true,
                    default: None,
                },
                ToolParameter {
                    name: "recursive".to_string(),
                    param_type: ParameterType::Boolean,
                    description: "Whether to list recursively".to_string(),
                    required: false,
                    default: Some("false".to_string()),
                },
            ],
            returns: ToolReturn {
                return_type: ParameterType::Array {
                    item_type: Box::new(ParameterType::Object {
                        properties: [("name".to_string(), ParameterType::String), ("is_dir".to_string(), ParameterType::Boolean)].into_iter().collect(),
                    }),
                },
                description: "List of files and directories".to_string(),
            },
            side_effects: vec![SideEffect::ReadsFilesystem],
            requires_approval: false,
            examples: vec![ToolExample {
                description: "List source directory".to_string(),
                invocation: ToolInvocation {
                    tool: "list_dir".to_string(),
                    params: [("path".to_string(), serde_json::json!("src"))].into_iter().collect(),
                },
                expected_result: r#"[{"name": "main.rs", "is_dir": false}, {"name": "lib.rs", "is_dir": false}]"#.to_string(),
            }],
        }
    }

    /// Get all file operation tools
    pub fn all() -> Vec<ToolDefinition> {
        vec![read_file(), write_file(), list_dir()]
    }
}

/// Pre-defined git operation tools
pub mod git_tools {
    use super::*;

    pub fn git_status() -> ToolDefinition {
        ToolDefinition {
            name: "git_status".to_string(),
            description: "Get the current git status (modified, staged, untracked files)".to_string(),
            parameters: vec![],
            returns: ToolReturn {
                return_type: ParameterType::Object {
                    properties: [
                        ("branch".to_string(), ParameterType::String),
                        (
                            "modified".to_string(),
                            ParameterType::Array {
                                item_type: Box::new(ParameterType::String),
                            },
                        ),
                        (
                            "staged".to_string(),
                            ParameterType::Array {
                                item_type: Box::new(ParameterType::String),
                            },
                        ),
                        (
                            "untracked".to_string(),
                            ParameterType::Array {
                                item_type: Box::new(ParameterType::String),
                            },
                        ),
                    ]
                    .into_iter()
                    .collect(),
                },
                description: "Current repository status".to_string(),
            },
            side_effects: vec![SideEffect::ReadsFilesystem],
            requires_approval: false,
            examples: vec![ToolExample {
                description: "Check current status".to_string(),
                invocation: ToolInvocation {
                    tool: "git_status".to_string(),
                    params: HashMap::new(),
                },
                expected_result: r#"{"branch": "main", "modified": ["src/lib.rs"], "staged": [], "untracked": []}"#.to_string(),
            }],
        }
    }

    pub fn git_diff() -> ToolDefinition {
        ToolDefinition {
            name: "git_diff".to_string(),
            description: "Get the diff of changes (staged or unstaged)".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "staged".to_string(),
                    param_type: ParameterType::Boolean,
                    description: "Whether to show staged changes (true) or unstaged (false)".to_string(),
                    required: false,
                    default: Some("false".to_string()),
                },
                ToolParameter {
                    name: "file".to_string(),
                    param_type: ParameterType::String,
                    description: "Specific file to diff (optional, defaults to all)".to_string(),
                    required: false,
                    default: None,
                },
            ],
            returns: ToolReturn {
                return_type: ParameterType::String,
                description: "Diff output".to_string(),
            },
            side_effects: vec![SideEffect::ReadsFilesystem],
            requires_approval: false,
            examples: vec![ToolExample {
                description: "View unstaged changes".to_string(),
                invocation: ToolInvocation {
                    tool: "git_diff".to_string(),
                    params: HashMap::new(),
                },
                expected_result: "diff --git a/src/lib.rs b/src/lib.rs\n...".to_string(),
            }],
        }
    }

    pub fn git_commit() -> ToolDefinition {
        ToolDefinition {
            name: "git_commit".to_string(),
            description: "Create a git commit with staged changes".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "message".to_string(),
                    param_type: ParameterType::String,
                    description: "Commit message".to_string(),
                    required: true,
                    default: None,
                },
                ToolParameter {
                    name: "stage_all".to_string(),
                    param_type: ParameterType::Boolean,
                    description: "Whether to stage all modified files before committing".to_string(),
                    required: false,
                    default: Some("false".to_string()),
                },
            ],
            returns: ToolReturn {
                return_type: ParameterType::Object {
                    properties: [("success".to_string(), ParameterType::Boolean), ("commit_hash".to_string(), ParameterType::String)]
                        .into_iter()
                        .collect(),
                },
                description: "Commit result with hash".to_string(),
            },
            side_effects: vec![SideEffect::ModifiesGit],
            requires_approval: true, // Commits should require approval by default
            examples: vec![ToolExample {
                description: "Commit staged changes".to_string(),
                invocation: ToolInvocation {
                    tool: "git_commit".to_string(),
                    params: [("message".to_string(), serde_json::json!("Add user validation"))].into_iter().collect(),
                },
                expected_result: r#"{"success": true, "commit_hash": "abc123"}"#.to_string(),
            }],
        }
    }

    pub fn git_branch() -> ToolDefinition {
        ToolDefinition {
            name: "git_branch".to_string(),
            description: "Create or switch to a git branch".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "name".to_string(),
                    param_type: ParameterType::String,
                    description: "Branch name".to_string(),
                    required: true,
                    default: None,
                },
                ToolParameter {
                    name: "create".to_string(),
                    param_type: ParameterType::Boolean,
                    description: "Whether to create the branch if it doesn't exist".to_string(),
                    required: false,
                    default: Some("false".to_string()),
                },
            ],
            returns: ToolReturn {
                return_type: ParameterType::Object {
                    properties: [("success".to_string(), ParameterType::Boolean), ("current_branch".to_string(), ParameterType::String)]
                        .into_iter()
                        .collect(),
                },
                description: "Branch operation result".to_string(),
            },
            side_effects: vec![SideEffect::ModifiesGit],
            requires_approval: false, // Branches are lightweight
            examples: vec![ToolExample {
                description: "Create and switch to feature branch".to_string(),
                invocation: ToolInvocation {
                    tool: "git_branch".to_string(),
                    params: [("name".to_string(), serde_json::json!("feature/user-auth")), ("create".to_string(), serde_json::json!(true))]
                        .into_iter()
                        .collect(),
                },
                expected_result: r#"{"success": true, "current_branch": "feature/user-auth"}"#.to_string(),
            }],
        }
    }

    /// Get all git operation tools
    pub fn all() -> Vec<ToolDefinition> {
        vec![git_status(), git_diff(), git_commit(), git_branch()]
    }
}

/// Pre-defined test operation tools
pub mod test_tools {
    use super::*;

    pub fn run_tests() -> ToolDefinition {
        ToolDefinition {
            name: "run_tests".to_string(),
            description: "Run the project's test suite".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "filter".to_string(),
                    param_type: ParameterType::String,
                    description: "Optional filter to run only matching tests".to_string(),
                    required: false,
                    default: None,
                },
                ToolParameter {
                    name: "verbose".to_string(),
                    param_type: ParameterType::Boolean,
                    description: "Whether to show verbose output".to_string(),
                    required: false,
                    default: Some("false".to_string()),
                },
            ],
            returns: ToolReturn {
                return_type: ParameterType::Object {
                    properties: [
                        ("success".to_string(), ParameterType::Boolean),
                        ("passed".to_string(), ParameterType::Number),
                        ("failed".to_string(), ParameterType::Number),
                        ("skipped".to_string(), ParameterType::Number),
                        ("output".to_string(), ParameterType::String),
                        (
                            "failures".to_string(),
                            ParameterType::Array {
                                item_type: Box::new(ParameterType::Object {
                                    properties: [("test_name".to_string(), ParameterType::String), ("error".to_string(), ParameterType::String)].into_iter().collect(),
                                }),
                            },
                        ),
                    ]
                    .into_iter()
                    .collect(),
                },
                description: "Test results with pass/fail counts and failure details".to_string(),
            },
            side_effects: vec![SideEffect::ExecutesCommands],
            requires_approval: false, // Tests are generally safe
            examples: vec![
                ToolExample {
                    description: "Run all tests".to_string(),
                    invocation: ToolInvocation {
                        tool: "run_tests".to_string(),
                        params: HashMap::new(),
                    },
                    expected_result: r#"{"success": true, "passed": 42, "failed": 0, "skipped": 2, "output": "...", "failures": []}"#.to_string(),
                },
                ToolExample {
                    description: "Run specific tests".to_string(),
                    invocation: ToolInvocation {
                        tool: "run_tests".to_string(),
                        params: [("filter".to_string(), serde_json::json!("auth"))].into_iter().collect(),
                    },
                    expected_result: r#"{"success": true, "passed": 5, "failed": 0, "skipped": 0, "output": "...", "failures": []}"#.to_string(),
                },
            ],
        }
    }

    pub fn run_single_test() -> ToolDefinition {
        ToolDefinition {
            name: "run_single_test".to_string(),
            description: "Run a single specific test by name".to_string(),
            parameters: vec![ToolParameter {
                name: "test_name".to_string(),
                param_type: ParameterType::String,
                description: "Full test name (e.g., 'tests::auth::test_login')".to_string(),
                required: true,
                default: None,
            }],
            returns: ToolReturn {
                return_type: ParameterType::Object {
                    properties: [
                        ("success".to_string(), ParameterType::Boolean),
                        ("output".to_string(), ParameterType::String),
                        ("error".to_string(), ParameterType::String),
                        ("duration_ms".to_string(), ParameterType::Number),
                    ]
                    .into_iter()
                    .collect(),
                },
                description: "Single test result with output".to_string(),
            },
            side_effects: vec![SideEffect::ExecutesCommands],
            requires_approval: false,
            examples: vec![ToolExample {
                description: "Run a specific failing test".to_string(),
                invocation: ToolInvocation {
                    tool: "run_single_test".to_string(),
                    params: [("test_name".to_string(), serde_json::json!("tests::auth::test_invalid_password"))].into_iter().collect(),
                },
                expected_result: r#"{"success": false, "output": "...", "error": "assertion failed", "duration_ms": 15}"#.to_string(),
            }],
        }
    }

    /// Get all test operation tools
    pub fn all() -> Vec<ToolDefinition> {
        vec![run_tests(), run_single_test()]
    }
}

/// Parser and validator for tool invocation requests from agents.
pub struct ToolInvocationParser;

impl ToolInvocationParser {
    /// Parse a tool invocation from agent output.
    pub fn parse(text: &str) -> Result<Vec<ToolInvocation>, InvocationParseError> {
        // Try to find JSON tool invocations
        let mut invocations = Vec::new();

        // Look for tool invocation blocks
        for block in Self::extract_tool_blocks(text) {
            let invocation: ToolInvocation = serde_json::from_str(&block).map_err(|e| InvocationParseError::InvalidJson(e.to_string()))?;
            invocations.push(invocation);
        }

        if invocations.is_empty() {
            return Err(InvocationParseError::NoInvocationsFound);
        }

        Ok(invocations)
    }

    /// Validate an invocation against a tool definition.
    pub fn validate(invocation: &ToolInvocation, registry: &ToolRegistry) -> Result<(), InvocationValidationError> {
        let tool = registry.get(&invocation.tool).ok_or(InvocationValidationError::UnknownTool(invocation.tool.clone()))?;

        // Check required parameters
        for param in &tool.parameters {
            if param.required && !invocation.params.contains_key(&param.name) {
                return Err(InvocationValidationError::MissingRequired(param.name.clone()));
            }
        }

        // Check for unknown parameters
        for param_name in invocation.params.keys() {
            if !tool.parameters.iter().any(|p| &p.name == param_name) {
                return Err(InvocationValidationError::UnknownParameter(param_name.clone()));
            }
        }

        Ok(())
    }

    fn extract_tool_blocks(text: &str) -> Vec<String> {
        let mut blocks = Vec::new();

        // Look for ```tool blocks
        let mut in_block = false;
        let mut current_block = String::new();

        for line in text.lines() {
            if line.trim().starts_with("```tool") {
                in_block = true;
                continue;
            }
            if in_block && line.trim() == "```" {
                blocks.push(current_block.clone());
                current_block.clear();
                in_block = false;
                continue;
            }
            if in_block {
                current_block.push_str(line);
                current_block.push('\n');
            }
        }

        // Also try to find inline JSON with "tool" field
        if blocks.is_empty() {
            if let Some(start) = text.find(r#"{"tool":"#) {
                if let Some(end) = text[start..].find('}') {
                    let potential = &text[start..=start + end];
                    if serde_json::from_str::<ToolInvocation>(potential).is_ok() {
                        blocks.push(potential.to_string());
                    }
                }
            }
        }

        blocks
    }
}

#[derive(Debug)]
pub enum InvocationParseError {
    NoInvocationsFound,
    InvalidJson(String),
}

impl std::fmt::Display for InvocationParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoInvocationsFound => write!(f, "No tool invocations found in output"),
            Self::InvalidJson(e) => write!(f, "Invalid JSON in tool invocation: {}", e),
        }
    }
}

impl std::error::Error for InvocationParseError {}

#[derive(Debug)]
pub enum InvocationValidationError {
    UnknownTool(String),
    MissingRequired(String),
    UnknownParameter(String),
    InvalidParameterType { param: String, expected: String },
}

impl std::fmt::Display for InvocationValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownTool(name) => write!(f, "Unknown tool: {}", name),
            Self::MissingRequired(param) => write!(f, "Missing required parameter: {}", param),
            Self::UnknownParameter(param) => write!(f, "Unknown parameter: {}", param),
            Self::InvalidParameterType { param, expected } => {
                write!(f, "Invalid type for {}: expected {}", param, expected)
            }
        }
    }
}

impl std::error::Error for InvocationValidationError {}

/// Format for agents to request tool usage
pub const TOOL_INVOCATION_FORMAT: &str = r#"To use a tool, output a tool block:

```tool
{
  "tool": "tool_name",
  "params": {
    "param1": "value1",
    "param2": "value2"
  }
}
```

You can request multiple tools in sequence. Each tool block will be executed in order."#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_definition_creation() {
        let tool = ToolDefinition {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            parameters: vec![ToolParameter {
                name: "input".to_string(),
                param_type: ParameterType::String,
                description: "Test input".to_string(),
                required: true,
                default: None,
            }],
            returns: ToolReturn {
                return_type: ParameterType::String,
                description: "Test output".to_string(),
            },
            side_effects: vec![SideEffect::None],
            requires_approval: false,
            examples: vec![],
        };

        assert_eq!(tool.name, "test_tool");
        assert!(tool.is_safe());
    }

    #[test]
    fn test_tool_is_safe() {
        let safe_tool = ToolDefinition {
            name: "safe".to_string(),
            description: "Safe tool".to_string(),
            parameters: vec![],
            returns: ToolReturn {
                return_type: ParameterType::String,
                description: "Output".to_string(),
            },
            side_effects: vec![SideEffect::ReadsFilesystem],
            requires_approval: false,
            examples: vec![],
        };

        let unsafe_tool = ToolDefinition {
            name: "unsafe".to_string(),
            description: "Unsafe tool".to_string(),
            parameters: vec![],
            returns: ToolReturn {
                return_type: ParameterType::String,
                description: "Output".to_string(),
            },
            side_effects: vec![SideEffect::ModifiesFilesystem],
            requires_approval: true,
            examples: vec![],
        };

        assert!(safe_tool.is_safe());
        assert!(!unsafe_tool.is_safe());
    }

    #[test]
    fn test_tool_registry() {
        let mut registry = ToolRegistry::new();

        registry.register(ToolDefinition {
            name: "tool1".to_string(),
            description: "Tool 1".to_string(),
            parameters: vec![],
            returns: ToolReturn {
                return_type: ParameterType::String,
                description: "Output".to_string(),
            },
            side_effects: vec![SideEffect::ReadsFilesystem],
            requires_approval: false,
            examples: vec![],
        });

        registry.register(ToolDefinition {
            name: "tool2".to_string(),
            description: "Tool 2".to_string(),
            parameters: vec![],
            returns: ToolReturn {
                return_type: ParameterType::String,
                description: "Output".to_string(),
            },
            side_effects: vec![SideEffect::ModifiesFilesystem],
            requires_approval: true,
            examples: vec![],
        });

        assert!(registry.get("tool1").is_some());
        assert!(registry.get("tool2").is_some());
        assert!(registry.get("nonexistent").is_none());

        let safe = registry.safe_tools();
        assert_eq!(safe.len(), 1);
        assert_eq!(safe[0].name, "tool1");
    }

    #[test]
    fn test_tool_registry_by_side_effect() {
        let mut registry = ToolRegistry::new();

        registry.register(ToolDefinition {
            name: "reader".to_string(),
            description: "Reads files".to_string(),
            parameters: vec![],
            returns: ToolReturn {
                return_type: ParameterType::String,
                description: "Content".to_string(),
            },
            side_effects: vec![SideEffect::ReadsFilesystem],
            requires_approval: false,
            examples: vec![],
        });

        registry.register(ToolDefinition {
            name: "writer".to_string(),
            description: "Writes files".to_string(),
            parameters: vec![],
            returns: ToolReturn {
                return_type: ParameterType::Boolean,
                description: "Success".to_string(),
            },
            side_effects: vec![SideEffect::ModifiesFilesystem],
            requires_approval: true,
            examples: vec![],
        });

        let readers = registry.by_side_effect(SideEffect::ReadsFilesystem);
        assert_eq!(readers.len(), 1);
        assert_eq!(readers[0].name, "reader");

        let writers = registry.by_side_effect(SideEffect::ModifiesFilesystem);
        assert_eq!(writers.len(), 1);
        assert_eq!(writers[0].name, "writer");
    }

    #[test]
    fn test_to_schema_string() {
        let tool = ToolDefinition {
            name: "read_file".to_string(),
            description: "Read a file".to_string(),
            parameters: vec![ToolParameter {
                name: "path".to_string(),
                param_type: ParameterType::String,
                description: "File path".to_string(),
                required: true,
                default: None,
            }],
            returns: ToolReturn {
                return_type: ParameterType::String,
                description: "File content".to_string(),
            },
            side_effects: vec![SideEffect::ReadsFilesystem],
            requires_approval: false,
            examples: vec![],
        };

        let schema = tool.to_schema_string();
        assert!(schema.contains("**read_file**"));
        assert!(schema.contains("Read a file"));
        assert!(schema.contains("path"));
        assert!(schema.contains("(required)"));
    }

    #[test]
    fn test_parameter_type_to_string() {
        assert_eq!(ParameterType::String.to_string(), "string");
        assert_eq!(ParameterType::Number.to_string(), "number");
        assert_eq!(ParameterType::Boolean.to_string(), "boolean");
        assert_eq!(
            ParameterType::Array {
                item_type: Box::new(ParameterType::String)
            }
            .to_string(),
            "array<string>"
        );
        assert_eq!(ParameterType::Object { properties: HashMap::new() }.to_string(), "object");
    }

    #[test]
    fn test_to_prompt_summary() {
        let mut registry = ToolRegistry::new();

        registry.register(ToolDefinition {
            name: "tool_a".to_string(),
            description: "Does A".to_string(),
            parameters: vec![],
            returns: ToolReturn {
                return_type: ParameterType::String,
                description: "Output".to_string(),
            },
            side_effects: vec![SideEffect::None],
            requires_approval: false,
            examples: vec![],
        });

        let summary = registry.to_prompt_summary();
        assert!(summary.contains("**tool_a**"));
        assert!(summary.contains("Does A"));
    }

    #[test]
    fn test_file_tools_read_file() {
        let tool = file_tools::read_file();
        assert_eq!(tool.name, "read_file");
        assert!(tool.is_safe()); // read_file is safe (read-only)
        assert!(!tool.requires_approval);
        assert_eq!(tool.parameters.len(), 1);
        assert_eq!(tool.parameters[0].name, "path");
        assert!(tool.parameters[0].required);
    }

    #[test]
    fn test_file_tools_write_file() {
        let tool = file_tools::write_file();
        assert_eq!(tool.name, "write_file");
        assert!(!tool.is_safe()); // write_file modifies filesystem
        assert_eq!(tool.parameters.len(), 2);
        assert!(tool.side_effects.contains(&SideEffect::ModifiesFilesystem));
    }

    #[test]
    fn test_file_tools_list_dir() {
        let tool = file_tools::list_dir();
        assert_eq!(tool.name, "list_dir");
        assert!(tool.is_safe()); // list_dir is read-only
        assert_eq!(tool.parameters.len(), 2);
        assert!(!tool.parameters[1].required); // recursive is optional
    }

    #[test]
    fn test_file_tools_all() {
        let tools = file_tools::all();
        assert_eq!(tools.len(), 3);

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"read_file"));
        assert!(names.contains(&"write_file"));
        assert!(names.contains(&"list_dir"));
    }

    #[test]
    fn test_git_tools_status() {
        let tool = git_tools::git_status();
        assert_eq!(tool.name, "git_status");
        assert!(tool.is_safe()); // git_status is read-only
        assert!(!tool.requires_approval);
        assert!(tool.parameters.is_empty());
    }

    #[test]
    fn test_git_tools_diff() {
        let tool = git_tools::git_diff();
        assert_eq!(tool.name, "git_diff");
        assert!(tool.is_safe()); // git_diff is read-only
        assert!(!tool.requires_approval);
        assert_eq!(tool.parameters.len(), 2);
        assert!(!tool.parameters[0].required); // staged is optional
    }

    #[test]
    fn test_git_tools_commit() {
        let tool = git_tools::git_commit();
        assert_eq!(tool.name, "git_commit");
        assert!(!tool.is_safe()); // git_commit modifies git
        assert!(tool.requires_approval); // Commits require approval
        assert!(tool.side_effects.contains(&SideEffect::ModifiesGit));
        assert!(tool.parameters[0].required); // message is required
    }

    #[test]
    fn test_git_tools_branch() {
        let tool = git_tools::git_branch();
        assert_eq!(tool.name, "git_branch");
        assert!(!tool.is_safe()); // git_branch modifies git
        assert!(!tool.requires_approval); // Branches are lightweight
        assert!(tool.parameters[0].required); // name is required
    }

    #[test]
    fn test_git_tools_all() {
        let tools = git_tools::all();
        assert_eq!(tools.len(), 4);

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"git_status"));
        assert!(names.contains(&"git_diff"));
        assert!(names.contains(&"git_commit"));
        assert!(names.contains(&"git_branch"));
    }

    #[test]
    fn test_test_tools_run_tests() {
        let tool = test_tools::run_tests();
        assert_eq!(tool.name, "run_tests");
        assert!(!tool.is_safe()); // run_tests executes commands
        assert!(!tool.requires_approval); // Tests are generally safe
        assert_eq!(tool.parameters.len(), 2);
        assert!(!tool.parameters[0].required); // filter is optional
        assert!(tool.side_effects.contains(&SideEffect::ExecutesCommands));
    }

    #[test]
    fn test_test_tools_run_single_test() {
        let tool = test_tools::run_single_test();
        assert_eq!(tool.name, "run_single_test");
        assert!(!tool.is_safe()); // run_single_test executes commands
        assert!(!tool.requires_approval);
        assert_eq!(tool.parameters.len(), 1);
        assert!(tool.parameters[0].required); // test_name is required
    }

    #[test]
    fn test_test_tools_all() {
        let tools = test_tools::all();
        assert_eq!(tools.len(), 2);

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"run_tests"));
        assert!(names.contains(&"run_single_test"));
    }

    #[test]
    fn test_parse_tool_block() {
        let text = r#"I'll read the file first.

```tool
{"tool": "read_file", "params": {"path": "src/main.rs"}}
```

Then I'll modify it."#;

        let result = ToolInvocationParser::parse(text).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].tool, "read_file");
        assert_eq!(result[0].params.get("path"), Some(&serde_json::json!("src/main.rs")));
    }

    #[test]
    fn test_parse_multiple_tool_blocks() {
        let text = r#"First, read the file:

```tool
{"tool": "read_file", "params": {"path": "src/main.rs"}}
```

Then write the changes:

```tool
{"tool": "write_file", "params": {"path": "src/main.rs", "content": "new content"}}
```
"#;

        let result = ToolInvocationParser::parse(text).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].tool, "read_file");
        assert_eq!(result[1].tool, "write_file");
    }

    #[test]
    fn test_parse_no_invocations() {
        let text = "Just some text without any tool blocks";
        let result = ToolInvocationParser::parse(text);
        assert!(matches!(result, Err(InvocationParseError::NoInvocationsFound)));
    }

    #[test]
    fn test_validate_required_param() {
        let mut registry = ToolRegistry::new();
        registry.register(file_tools::read_file());

        let invocation = ToolInvocation {
            tool: "read_file".to_string(),
            params: HashMap::new(), // Missing required "path"
        };

        let result = ToolInvocationParser::validate(&invocation, &registry);
        assert!(matches!(result, Err(InvocationValidationError::MissingRequired(_))));
    }

    #[test]
    fn test_validate_unknown_tool() {
        let registry = ToolRegistry::new(); // Empty registry

        let invocation = ToolInvocation {
            tool: "nonexistent_tool".to_string(),
            params: HashMap::new(),
        };

        let result = ToolInvocationParser::validate(&invocation, &registry);
        assert!(matches!(result, Err(InvocationValidationError::UnknownTool(_))));
    }

    #[test]
    fn test_validate_unknown_parameter() {
        let mut registry = ToolRegistry::new();
        registry.register(file_tools::read_file());

        let invocation = ToolInvocation {
            tool: "read_file".to_string(),
            params: [("path".to_string(), serde_json::json!("src/main.rs")), ("invalid_param".to_string(), serde_json::json!("value"))]
                .into_iter()
                .collect(),
        };

        let result = ToolInvocationParser::validate(&invocation, &registry);
        assert!(matches!(result, Err(InvocationValidationError::UnknownParameter(_))));
    }

    #[test]
    fn test_validate_valid_invocation() {
        let mut registry = ToolRegistry::new();
        registry.register(file_tools::read_file());

        let invocation = ToolInvocation {
            tool: "read_file".to_string(),
            params: [("path".to_string(), serde_json::json!("src/main.rs"))].into_iter().collect(),
        };

        let result = ToolInvocationParser::validate(&invocation, &registry);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tool_invocation_format_constant() {
        assert!(TOOL_INVOCATION_FORMAT.contains("```tool"));
        assert!(TOOL_INVOCATION_FORMAT.contains("tool_name"));
        assert!(TOOL_INVOCATION_FORMAT.contains("params"));
    }
}
