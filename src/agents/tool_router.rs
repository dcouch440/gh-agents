//! Tool routing: maps tool calls to cluster agents via `request_assistance`.
//!
//! When an agent is in router_mode, it receives a single meta-tool
//! (`request_assistance`) instead of individual execution tools. The router
//! resolves the request to a specific tool, finds the associated cluster,
//! picks an agent, dispatches a sub-task, and returns the result.

use serde_json::{json, Value};
use uuid::Uuid;

use crate::db::ToolRow;
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

/// Execute a `request_assistance` tool call by routing to the appropriate tool/cluster.
///
/// For direct execution tools (cluster_id = None), delegates to `execute_execution_tool`.
/// For cluster-routed tools, dispatches to a cluster agent and awaits the result.
pub async fn execute_request_assistance(
    input: &Value,
    tool_rows: &[ToolRow],
    exec_ctx: Option<&crate::execution::ExecutionContext>,
    allowed_tools: Option<&[String]>,
) -> Value {
    let tool_name = match input["tool_name"].as_str() {
        Some(name) => name,
        None => return json!({ "error": "Missing required parameter: tool_name" }),
    };

    // Find the tool in the user's tool list
    let tool_row = tool_rows.iter().find(|t| t.name == tool_name);

    match tool_row {
        None => json!({ "error": format!("Unknown tool: {}", tool_name) }),
        Some(row) if !row.enabled => {
            json!({ "error": format!("Tool '{}' is disabled", tool_name) })
        }
        Some(row) => {
            match row.cluster_id {
                None => {
                    // Direct execution tool — use existing dispatcher
                    let params = input.get("parameters").unwrap_or(&json!({})).clone();
                    match exec_ctx {
                        Some(ctx) => {
                            super::execution_tools::execute_execution_tool(
                                tool_name,
                                &params,
                                ctx,
                                allowed_tools,
                            )
                            .await
                        }
                        None => json!({ "error": "No execution context available" }),
                    }
                }
                Some(cluster_id) => {
                    // Cluster-routed tool — dispatch to cluster agent
                    route_to_cluster(cluster_id, tool_name, input).await
                }
            }
        }
    }
}

/// Route a tool call to a cluster agent. Currently a placeholder that will be
/// wired to the dispatcher + oneshot channel pattern in a future iteration.
async fn route_to_cluster(cluster_id: Uuid, tool_name: &str, _input: &Value) -> Value {
    // TODO: Look up cluster members, pick an agent, create a sub-task,
    // dispatch via the existing dispatcher, and wait for the result.
    json!({
        "error": format!(
            "Cluster routing not yet fully implemented. Tool '{}' is mapped to cluster {}",
            tool_name, cluster_id
        )
    })
}

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

    #[tokio::test]
    async fn missing_tool_name_returns_error() {
        let result = execute_request_assistance(&json!({}), &[], None, None).await;
        assert!(result["error"].as_str().unwrap().contains("tool_name"));
    }

    #[tokio::test]
    async fn unknown_tool_returns_error() {
        let result = execute_request_assistance(
            &json!({"tool_name": "nonexistent", "request": "help"}),
            &[],
            None,
            None,
        )
        .await;
        assert!(result["error"].as_str().unwrap().contains("Unknown tool"));
    }

    #[tokio::test]
    async fn disabled_tool_returns_error() {
        let row = ToolRow {
            id: Uuid::new_v4(),
            name: "my_tool".to_string(),
            description: "test".to_string(),
            category: "general".to_string(),
            parameter_schema: json!({}),
            output_schema: json!({}),
            enabled: false,
            cluster_id: None,
            is_builtin: false,
        };
        let result = execute_request_assistance(
            &json!({"tool_name": "my_tool", "request": "help"}),
            &[row],
            None,
            None,
        )
        .await;
        assert!(result["error"].as_str().unwrap().contains("disabled"));
    }

    #[tokio::test]
    async fn cluster_routed_tool_returns_placeholder() {
        let cid = Uuid::new_v4();
        let row = ToolRow {
            id: Uuid::new_v4(),
            name: "cluster_tool".to_string(),
            description: "test".to_string(),
            category: "general".to_string(),
            parameter_schema: json!({}),
            output_schema: json!({}),
            enabled: true,
            cluster_id: Some(cid),
            is_builtin: false,
        };
        let result = execute_request_assistance(
            &json!({"tool_name": "cluster_tool", "request": "help"}),
            &[row],
            None,
            None,
        )
        .await;
        let err = result["error"].as_str().unwrap();
        assert!(err.contains("Cluster routing"));
        assert!(err.contains(&cid.to_string()));
    }

    #[tokio::test]
    async fn direct_tool_without_exec_ctx_returns_error() {
        let row = ToolRow {
            id: Uuid::new_v4(),
            name: "read_file".to_string(),
            description: "Read a file".to_string(),
            category: "execution".to_string(),
            parameter_schema: json!({}),
            output_schema: json!({}),
            enabled: true,
            cluster_id: None,
            is_builtin: true,
        };
        let result = execute_request_assistance(
            &json!({"tool_name": "read_file", "request": "read it", "parameters": {"path": "foo.txt"}}),
            &[row],
            None,
            None,
        )
        .await;
        assert!(result["error"]
            .as_str()
            .unwrap()
            .contains("execution context"));
    }
}
