//! Tool routing: maps tool calls to cluster agents via `request_assistance`.
//!
//! When an agent is in router_mode, it receives a single meta-tool
//! (`request_assistance`) instead of individual execution tools. The router
//! resolves the request to a specific tool, finds the associated cluster,
//! picks an agent, dispatches a sub-task, and returns the result.

use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::sync::Mutex;
use uuid::Uuid;

use super::dispatcher::Dispatcher;
use super::pool::AgentPool;
use super::router_agent::{self, ToolClusterIndex};
use crate::db::ToolRow;
use crate::llm::Tool;

/// Default timeout for cluster-routed tool calls.
const CLUSTER_ROUTING_TIMEOUT: Duration = Duration::from_secs(120);

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

/// Context needed for cluster routing, passed from the executor.
#[derive(Clone)]
pub struct ClusterRoutingContext {
    pub cluster_index: Arc<ToolClusterIndex>,
    pub pool: Arc<Mutex<AgentPool>>,
    pub dispatcher: Arc<Mutex<Dispatcher>>,
    pub execution_context: Option<crate::execution::ExecutionContext>,
}

impl std::fmt::Debug for ClusterRoutingContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClusterRoutingContext").finish_non_exhaustive()
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
    cluster_ctx: Option<&ClusterRoutingContext>,
) -> Value {
    let tool_name = match input["tool_name"].as_str() {
        Some(name) => name,
        None => return json!({ "error": "Missing required parameter: tool_name" }),
    };

    // Find the tool in the user's tool list
    let tool_row = tool_rows.iter().find(|t| t.name == tool_name);

    match tool_row {
        None => json!({ "error": format!("Unknown tool: {}", tool_name) }),
        Some(_row) => {
            // Direct execution tool — use existing dispatcher
            let params = input.get("parameters").unwrap_or(&json!({})).clone();
            match exec_ctx {
                Some(ctx) => super::execution_tools::execute_execution_tool(tool_name, &params, ctx, allowed_tools).await,
                None => json!({ "error": "No execution context available" }),
            }
        }
    }
}

/// Route a tool call to a cluster agent.
async fn route_to_cluster(cluster_id: Uuid, tool_name: &str, input: &Value, cluster_ctx: Option<&ClusterRoutingContext>) -> Value {
    let ctx = match cluster_ctx {
        Some(ctx) => ctx,
        None => {
            return json!({
                "error": format!(
                    "Cluster routing not available. Tool '{}' is mapped to cluster {} but no routing context was provided.",
                    tool_name, cluster_id
                )
            });
        }
    };

    // Look up the cluster in the index
    let cluster_entry = match ctx.cluster_index.find_cluster(tool_name) {
        Some(entry) => entry,
        None => {
            return json!({
                "error": format!(
                    "Tool '{}' has cluster_id {} but no cluster found in the index.",
                    tool_name, cluster_id
                )
            });
        }
    };

    let request = input["request"].as_str().unwrap_or("");
    let empty_params = json!({});
    let parameters = input.get("parameters").unwrap_or(&empty_params);

    router_agent::route_to_cluster_agent(
        cluster_entry,
        tool_name,
        request,
        parameters,
        &ctx.pool,
        &ctx.dispatcher,
        ctx.execution_context.clone(),
        CLUSTER_ROUTING_TIMEOUT,
    )
    .await
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
        let result = execute_request_assistance(&json!({}), &[], None, None, None).await;
        assert!(result["error"].as_str().unwrap().contains("tool_name"));
    }

    #[tokio::test]
    async fn unknown_tool_returns_error() {
        let result = execute_request_assistance(&json!({"tool_name": "nonexistent", "request": "help"}), &[], None, None, None).await;
        assert!(result["error"].as_str().unwrap().contains("Unknown tool"));
    }

    fn make_test_tool(name: &str) -> ToolRow {
        ToolRow {
            id: Uuid::new_v4(),
            user_id: Uuid::nil(),
            name: name.to_string(),
            display_name: name.to_string(),
            description: format!("{} tool", name),
            parameters: json!({}),
            created_at: chrono::Utc::now(),
            version: 1,
        }
    }

    #[tokio::test]
    async fn direct_tool_without_exec_ctx_returns_error() {
        let row = make_test_tool("read_file");
        let result = execute_request_assistance(&json!({"tool_name": "read_file", "request": "read it", "parameters": {"path": "foo.txt"}}), &[row], None, None, None).await;
        assert!(result["error"].as_str().unwrap().contains("execution context"));
    }
}
