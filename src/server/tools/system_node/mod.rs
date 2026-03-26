//! System node agent tools — tool definitions and handlers for
//! the `complete_system` tool.

use serde_json::json;

use crate::llm::Tool;

mod tests;

/// Build the `complete_system` tool definition for the LLM.
///
/// The agent calls this to signal completion. The backend validates
/// the repository against the verify claims and returns success or
/// structured errors.
pub fn complete_system_tool() -> Tool {
    Tool {
        name: "complete_system".into(),
        description: "Signal that you are done configuring the system. \
            Validates your repository — if something is wrong, you'll get \
            an error and can fix it."
            .into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "summary": {
                    "type": "string",
                    "description": "What you configured and key decisions (1-3 sentences)."
                },
                "verify": {
                    "type": "object",
                    "description": "Verify your work. Each boolean is you signing off that it's correct.",
                    "properties": {
                        "topology_complete": {
                            "type": "boolean",
                            "description": "The topology defines all agents and their dependencies are correct."
                        },
                        "agents_complete": {
                            "type": "boolean",
                            "description": "Every agent has a valid config with system_prompt, assignment, and expected_output."
                        },
                        "config_accurate": {
                            "type": "boolean",
                            "description": "config.json name and description accurately reflect this system."
                        },
                        "no_filenames_prescribed": {
                            "type": "boolean",
                            "description": "No agent assignments or expected_outputs prescribe specific filenames — agents decide what to produce."
                        },
                        "prompts_not_trivial": {
                            "type": "boolean",
                            "description": "Every system_prompt has substantive content, not just a role label."
                        },
                        "assignments_expanded": {
                            "type": "boolean",
                            "description": "Every assignment expands beyond the user's input — not shorter than what the user wrote."
                        }
                    },
                    "required": ["topology_complete", "agents_complete", "config_accurate", "no_filenames_prescribed", "prompts_not_trivial", "assignments_expanded"]
                }
            },
            "required": ["summary", "verify"]
        }),
    }
}
