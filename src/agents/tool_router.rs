//! Tool routing: maps tool calls to cluster agents via `request_assistance`.
//!
//! When an agent is in router_mode, it receives a single meta-tool
//! (`request_assistance`) instead of individual execution tools. The router
//! resolves the request to a specific tool, finds the associated cluster,
//! picks an agent, dispatches a sub-task, and returns the result.

use serde_json::json;
use crate::llm::Tool;

/// Return the `request_assistance` meta-tool definition for router-mode agents.
pub fn request_assistance_tool() -> Tool {
    Tool {
        name: "request_assistance".into(),
        description: "Request help from a specialized agent cluster. Provide the tool name and describe what you need. The request will be routed to the appropriate specialist.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "tool_name": {
                    "type": "string",
                    "description": "Name of the tool to invoke (e.g. 'read_file', 'run_tests', or a custom tool)"
                },
                "request": {
                    "type": "string",
                    "description": "Description of what you need done"
                },
                "parameters": {
                    "type": "object",
                    "description": "Parameters to pass to the tool"
                }
            },
            "required": ["tool_name", "request"]
        }),
    }
}

// LEGACY CODE REMOVED:
// - ClusterRoutingContext struct (required pool/dispatcher)
// - execute_request_assistance() function (required ClusterRoutingContext)
// - route_to_cluster() function (required pool/dispatcher)
// Tool routing now handled by RouterStrategy in hub/strategies/router.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_assistance_tool_has_valid_schema() {
        let tool = request_assistance_tool();
        assert_eq!(tool.name, "request_assistance");
        assert!(tool.input_schema.is_object());
        let required = tool.input_schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("tool_name")));
        assert!(required.contains(&json!("request")));
    }
}
