//! Tool selection prompts for helping agents choose appropriate tools.

use super::*;
use crate::prompts::{PromptBuilder, PromptVersion};

/// Tool selection prompt builder
pub struct ToolSelectionPrompts;

impl ToolSelectionPrompts {
    /// Current version for tool selection
    pub fn selection_version() -> PromptVersion {
        PromptVersion::new("tool-selection", 1, 0, 0)
    }

    /// Build a prompt for selecting tools for a task.
    ///
    /// # Arguments
    /// * `task_description` - What the agent needs to accomplish
    /// * `available_tools` - Registry of tools the agent can use
    pub fn select_tools(task_description: &str, available_tools: &ToolRegistry) -> PromptBuilder {
        let tools_summary = available_tools.to_prompt_summary();

        PromptBuilder::new()
            .version(Self::selection_version())
            .role(TOOL_SELECTION_ROLE)
            .task(format!(
                r#"Determine which tools you need for this task:

**Task**: {}

**Available tools**:
{}

Select the tools you'll need and plan the order of operations."#,
                task_description, tools_summary
            ))
            .constraint(TOOL_SELECTION_GUIDANCE)
            .output_json(TOOL_SELECTION_SCHEMA)
    }

    /// Build a prompt for planning tool usage.
    ///
    /// # Arguments
    /// * `task_description` - What to accomplish
    /// * `selected_tools` - Tools that have been selected
    pub fn plan_tool_usage(task_description: &str, selected_tools: &[&ToolDefinition]) -> PromptBuilder {
        let tools_detail: Vec<String> = selected_tools.iter().map(|t| t.to_schema_string()).collect();

        PromptBuilder::new()
            .version(Self::selection_version())
            .role(TOOL_SELECTION_ROLE)
            .task(format!(
                r#"Plan how to use these tools to complete the task:

**Task**: {}

**Selected tools**:
{}

Create a step-by-step plan showing which tool to use at each step."#,
                task_description,
                tools_detail.join("\n\n---\n\n")
            ))
            .constraint("Plan the minimal set of tool calls needed")
            .constraint("Order operations to avoid unnecessary reads (read once, write once)")
            .output_json(TOOL_PLAN_SCHEMA)
    }
}

const TOOL_SELECTION_ROLE: &str = r#"You are planning which tools to use for a task.

Consider:
- What operations are needed (read, write, test, git)?
- What order minimizes redundant operations?
- Are there any operations that need approval?"#;

const TOOL_SELECTION_GUIDANCE: &str = r#"Tool selection principles:

1. **Minimum necessary**: Only select tools you'll actually use
2. **Read before write**: Always read current state before modifying
3. **Verify after change**: Run tests after making changes
4. **Approval awareness**: Note which operations need approval

Common patterns:
- Modify code: read_file → write_file → run_tests
- Create feature: list_dir → read_file → write_file → git_commit
- Debug: read_file → run_single_test → analyze output"#;

const TOOL_SELECTION_SCHEMA: &str = r#"{
  "selected_tools": ["array of tool names needed"],
  "reasoning": "string - why these tools are needed",
  "operations_requiring_approval": ["array of tool names that need approval"],
  "estimated_calls": "number - approximately how many tool calls"
}"#;

const TOOL_PLAN_SCHEMA: &str = r#"{
  "plan": [
    {
      "step": "number",
      "tool": "string - tool name",
      "purpose": "string - what this step accomplishes",
      "inputs_needed": ["array of what inputs are needed"],
      "outputs_used_by": ["array of later steps that use this output"]
    }
  ],
  "approval_required_at_steps": ["array of step numbers"],
  "rollback_plan": "string - what to do if something fails"
}"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_registry() -> ToolRegistry {
        let mut registry = ToolRegistry::new();
        registry.register(file_tools::read_file());
        registry.register(file_tools::write_file());
        registry.register(git_tools::git_commit());
        registry
    }

    #[test]
    fn test_select_tools_prompt() {
        let registry = create_test_registry();
        let prompt = ToolSelectionPrompts::select_tools("Add a new function", &registry).build();

        assert!(prompt.text.contains("Add a new function"));
        assert!(prompt.text.contains("read_file"));
        assert!(prompt.text.contains("write_file"));
        assert!(prompt.text.contains("selected_tools"));
    }

    #[test]
    fn test_plan_tool_usage_prompt() {
        let read_tool = file_tools::read_file();
        let write_tool = file_tools::write_file();
        let selected = vec![&read_tool, &write_tool];

        let prompt = ToolSelectionPrompts::plan_tool_usage("Modify the config file", &selected).build();

        assert!(prompt.text.contains("Modify the config file"));
        assert!(prompt.text.contains("read_file"));
        assert!(prompt.text.contains("write_file"));
        assert!(prompt.text.contains("step-by-step plan"));
    }

    #[test]
    fn test_selection_version() {
        let version = ToolSelectionPrompts::selection_version();
        assert_eq!(version.family, "tool-selection");
        assert_eq!(version.major, 1);
    }

    #[test]
    fn test_prompt_includes_approval_guidance() {
        let registry = create_test_registry();
        let prompt = ToolSelectionPrompts::select_tools("Make changes", &registry).build();

        assert!(prompt.text.contains("Approval awareness"));
        assert!(prompt.text.contains("operations_requiring_approval"));
    }
}
