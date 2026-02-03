//! Tool cluster routing: resolves tool calls to cluster agents.
//!
//! The `ToolClusterIndex` maps tool names to clusters. When a router-mode
//! agent calls `request_assistance`, this module finds the right cluster,
//! builds a sub-task scoped to that cluster's tools, dispatches it to an
//! available utility agent, and returns the result.

use std::collections::HashMap;
use uuid::Uuid;
use crate::db::{ClusterRow, ToolRow};

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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tool(name: &str, _cluster_id: Uuid) -> ToolRow {
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
