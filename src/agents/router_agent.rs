//! Tool cluster routing: resolves tool calls to cluster agents.
//!
//! The `ToolClusterIndex` maps tool names to clusters. When a router-mode
//! agent calls `request_assistance`, this module finds the right cluster,
//! builds a sub-task scoped to that cluster's tools, dispatches it to an
//! available utility agent, and returns the result.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::sync::Mutex;
use tracing::{info, warn};
use uuid::Uuid;

use super::channels::{RoleContext, TaskAssignment, TaskConstraints, TaskContext};
use super::dispatcher::Dispatcher;
use super::pool::AgentPool;
use super::roles::{CommunicationStyle, OutputFormat, RoleId};
use crate::db::{ClusterRow, ToolRow};
use crate::types::AgentTier;

/// Index mapping tool names to their owning clusters for fast lookup.
#[derive(Debug, Clone)]
pub struct ToolClusterIndex {
    /// All clusters with their tools
    clusters: Vec<ClusterEntry>,
    /// Reverse map: tool_name → index into `clusters`
    tool_to_cluster: HashMap<String, usize>,
}

/// A cluster entry with its metadata and tools.
#[derive(Debug, Clone)]
pub struct ClusterEntry {
    pub cluster_id: Uuid,
    pub name: String,
    pub description: String,
    pub tools: Vec<ToolRow>,
}

impl ToolClusterIndex {
    /// Build the index from DB rows of (cluster, tools) pairs.
    pub fn new(cluster_tool_pairs: Vec<(ClusterRow, Vec<ToolRow>)>) -> Self {
        let mut clusters = Vec::new();
        let mut tool_to_cluster = HashMap::new();

        for (i, (cluster, tools)) in cluster_tool_pairs.into_iter().enumerate() {
            for tool in &tools {
                tool_to_cluster.insert(tool.name.clone(), i);
            }
            clusters.push(ClusterEntry {
                cluster_id: cluster.id,
                name: cluster.name,
                description: cluster.description,
                tools,
            });
        }

        Self { clusters, tool_to_cluster }
    }

    /// Find the cluster that owns a given tool.
    pub fn find_cluster(&self, tool_name: &str) -> Option<&ClusterEntry> {
        self.tool_to_cluster.get(tool_name).and_then(|&idx| self.clusters.get(idx))
    }

    /// Build a summary of all clusters and their tools for the router agent's
    /// system prompt, so it knows what's available.
    pub fn cluster_summary(&self) -> String {
        let mut summary = String::from("Available tool clusters:\n\n");
        for entry in &self.clusters {
            summary.push_str(&format!("## {} ({})\n", entry.name, entry.cluster_id));
            summary.push_str(&format!("{}\n", entry.description));
            summary.push_str("Tools:\n");
            for tool in &entry.tools {
                summary.push_str(&format!("  - {}: {}\n", tool.name, tool.description));
            }
            summary.push('\n');
        }
        summary
    }

    /// Returns true if the index has any clusters.
    pub fn is_empty(&self) -> bool {
        self.clusters.is_empty()
    }

    /// Number of clusters in the index.
    pub fn len(&self) -> usize {
        self.clusters.len()
    }
}

/// Route a tool call to a cluster agent and await the result.
///
/// 1. Finds the cluster owning `tool_name`
/// 2. Builds a TaskAssignment scoped to that cluster's tools
/// 3. Picks an available utility agent from the pool
/// 4. Dispatches the task and waits for the result
/// 5. Returns the agent's output as a JSON value
pub async fn route_to_cluster_agent(
    cluster: &ClusterEntry,
    tool_name: &str,
    request: &str,
    parameters: &Value,
    pool: &Arc<Mutex<AgentPool>>,
    dispatcher: &Arc<Mutex<Dispatcher>>,
    execution_context: Option<crate::execution::ExecutionContext>,
    routing_timeout: Duration,
) -> Value {
    let task_id = Uuid::new_v4();

    info!(
        task_id = %task_id,
        tool_name = %tool_name,
        cluster = %cluster.name,
        "Routing tool call to cluster agent"
    );

    // Build a task that tells the cluster agent to execute the specific tool
    let task_description = format!(
        "Execute the tool `{}` with the following request:\n\n{}\n\nParameters:\n```json\n{}\n```",
        tool_name,
        request,
        serde_json::to_string_pretty(parameters).unwrap_or_else(|_| parameters.to_string()),
    );

    // Scope tools to only this cluster's tools
    let cluster_tools: Vec<ToolRow> = cluster.tools.clone();
    let allowed_tool_names: Vec<String> = cluster_tools.iter().map(|t| t.name.clone()).collect();

    let assignment = TaskAssignment {
        task_id,
        title: format!("Cluster tool: {} ({})", tool_name, cluster.name),
        description: task_description,
        context: TaskContext {
            required_reading: vec![],
            files: vec![],
            history: vec![],
            conventions: String::new(),
            role_context: RoleContext {
                system_prompt: format!(
                    "You are a specialist agent in the '{}' cluster. \
                     Execute the requested tool and return the result. \
                     Be precise and concise.",
                    cluster.name
                ),
                style: CommunicationStyle::Technical,
                output_format: OutputFormat::Result,
            },
            chat_messages: vec![],
            execution_context,
            tool_rows: cluster_tools,
            router_mode: false,
            cluster_routing: None,
            context_docs: vec![],
        },
        constraints: TaskConstraints {
            allowed_tools: Some(allowed_tool_names),
            ..TaskConstraints::default()
        },
        timeout: routing_timeout,
        role_id: RoleId::new("utility"),
    };

    // Find an available utility agent
    let agent_id = {
        let p = pool.lock().await;
        match p.get_available_agent_id(AgentTier::Utility) {
            Some(id) => id,
            None => {
                warn!("No available utility agent for cluster routing");
                return json!({
                    "error": "No available agent to handle this request. All utility agents are busy."
                });
            }
        }
    };

    // Send the task to the agent
    {
        let d = dispatcher.lock().await;
        if let Err(e) = d.send_to_agent(&agent_id, super::channels::AgentCommand::AssignTask(Box::new(assignment))).await {
            warn!(error = %e, "Failed to dispatch routed task");
            return json!({ "error": format!("Failed to dispatch: {}", e) });
        }
    }

    // Wait for the result by subscribing to the dispatcher's response channel.
    // We use the response_sender to create a dedicated listener.
    let _response_rx = {
        let d = dispatcher.lock().await;
        d.response_sender()
    };

    // Poll for the result with timeout.
    // In practice the dispatcher's main loop processes responses. Here we need
    // a mechanism to get notified. Since the dispatcher's response_rx may be
    // taken by the main consumer loop, we'll use a simpler approach: poll the
    // task_results map if available, or use a dedicated oneshot.
    //
    // For now, return a "routing dispatched" acknowledgment. The result will
    // arrive via the dispatcher's normal response flow and be broadcast on
    // the feed/task channels.
    //
    // TODO: Wire up a oneshot or result-waiting mechanism for synchronous
    // tool routing. For now the executor will see this as an async dispatch.
    json!({
        "status": "dispatched",
        "task_id": task_id.to_string(),
        "cluster": cluster.name,
        "tool": tool_name,
        "message": format!(
            "Tool '{}' has been dispatched to a specialist agent in the '{}' cluster. Task ID: {}",
            tool_name, cluster.name, task_id
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tool(name: &str, cluster_id: Uuid) -> ToolRow {
        ToolRow {
            id: Uuid::new_v4(),
            name: name.to_string(),
            description: format!("{} tool", name),
            category: "test".to_string(),
            parameter_schema: json!({}),
            output_schema: json!({}),
            enabled: true,
            cluster_id: Some(cluster_id),
            is_builtin: false,
        }
    }

    fn make_cluster(name: &str) -> ClusterRow {
        ClusterRow {
            id: Uuid::new_v4(),
            name: name.to_string(),
            description: format!("{} cluster", name),
            conventions: String::new(),
            shared_files: json!([]),
        }
    }

    #[test]
    fn empty_index() {
        let index = ToolClusterIndex::new(vec![]);
        assert!(index.is_empty());
        assert_eq!(index.len(), 0);
        assert!(index.find_cluster("anything").is_none());
    }

    #[test]
    fn single_cluster_with_tools() {
        let cluster = make_cluster("codebase");
        let t1 = make_tool("read_file", cluster.id);
        let t2 = make_tool("list_files", cluster.id);

        let index = ToolClusterIndex::new(vec![(cluster.clone(), vec![t1, t2])]);

        assert_eq!(index.len(), 1);
        assert!(!index.is_empty());

        let found = index.find_cluster("read_file").unwrap();
        assert_eq!(found.name, "codebase");
        assert_eq!(found.tools.len(), 2);

        let found2 = index.find_cluster("list_files").unwrap();
        assert_eq!(found2.cluster_id, cluster.id);
    }

    #[test]
    fn multiple_clusters() {
        let c1 = make_cluster("codebase");
        let c2 = make_cluster("execution");

        let t1 = make_tool("read_file", c1.id);
        let t2 = make_tool("run_tests", c2.id);
        let t3 = make_tool("run_command", c2.id);

        let index = ToolClusterIndex::new(vec![(c1.clone(), vec![t1]), (c2.clone(), vec![t2, t3])]);

        assert_eq!(index.len(), 2);

        let found_code = index.find_cluster("read_file").unwrap();
        assert_eq!(found_code.name, "codebase");

        let found_exec = index.find_cluster("run_tests").unwrap();
        assert_eq!(found_exec.name, "execution");

        let found_cmd = index.find_cluster("run_command").unwrap();
        assert_eq!(found_cmd.cluster_id, c2.id);
    }

    #[test]
    fn unknown_tool_returns_none() {
        let c = make_cluster("codebase");
        let t = make_tool("read_file", c.id);
        let index = ToolClusterIndex::new(vec![(c, vec![t])]);

        assert!(index.find_cluster("nonexistent").is_none());
    }

    #[test]
    fn cluster_summary_contains_info() {
        let c = make_cluster("codebase");
        let t1 = make_tool("read_file", c.id);
        let t2 = make_tool("write_file", c.id);

        let index = ToolClusterIndex::new(vec![(c, vec![t1, t2])]);
        let summary = index.cluster_summary();

        assert!(summary.contains("codebase"));
        assert!(summary.contains("read_file"));
        assert!(summary.contains("write_file"));
        assert!(summary.contains("codebase cluster"));
    }

    #[test]
    fn duplicate_tool_name_last_cluster_wins() {
        let c1 = make_cluster("alpha");
        let c2 = make_cluster("beta");
        let t1 = make_tool("shared_tool", c1.id);
        let t2 = make_tool("shared_tool", c2.id);

        let index = ToolClusterIndex::new(vec![(c1, vec![t1]), (c2, vec![t2])]);

        // Last insertion wins in HashMap
        let found = index.find_cluster("shared_tool").unwrap();
        assert_eq!(found.name, "beta");
    }
}
