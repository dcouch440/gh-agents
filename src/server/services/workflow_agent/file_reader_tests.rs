#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::server::services::workflow_agent::file_reader::{
        read_all_nodes, read_board, read_topology,
    };

    fn write_topology(dir: &std::path::Path, content: &str) {
        fs::write(dir.join("topology.json"), content).unwrap();
    }

    fn write_node(dir: &std::path::Path, slug: &str, content: &str) {
        let nodes_dir = dir.join("nodes");
        fs::create_dir_all(&nodes_dir).unwrap();
        fs::write(nodes_dir.join(format!("{slug}.md")), content).unwrap();
    }

    #[test]
    fn read_topology_basic() {
        let dir = tempdir().unwrap();
        write_topology(
            dir.path(),
            r#"{
                "nodes": {
                    "research": { "depends_on": [] },
                    "report": { "depends_on": ["research"] }
                }
            }"#,
        );

        let topo = read_topology(dir.path()).unwrap();
        assert_eq!(topo.len(), 2);
        assert!(topo["research"].is_empty());
        assert_eq!(topo["report"], vec!["research"]);
    }

    #[test]
    fn read_topology_empty() {
        let dir = tempdir().unwrap();
        write_topology(dir.path(), r#"{ "nodes": {} }"#);

        let topo = read_topology(dir.path()).unwrap();
        assert!(topo.is_empty());
    }

    #[test]
    fn read_topology_missing_file() {
        let dir = tempdir().unwrap();
        let result = read_topology(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot read topology.json"));
    }

    #[test]
    fn read_topology_invalid_json() {
        let dir = tempdir().unwrap();
        write_topology(dir.path(), "not json");
        let result = read_topology(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid JSON"));
    }

    #[test]
    fn read_all_nodes_basic() {
        let dir = tempdir().unwrap();
        write_node(dir.path(), "research", "Research task");
        write_node(dir.path(), "report", "Report task");

        let nodes = read_all_nodes(dir.path()).unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes["research"], "Research task");
        assert_eq!(nodes["report"], "Report task");
    }

    #[test]
    fn read_all_nodes_empty_dir() {
        let dir = tempdir().unwrap();
        let nodes = read_all_nodes(dir.path()).unwrap();
        assert!(nodes.is_empty());
    }

    #[test]
    fn read_all_nodes_ignores_non_md() {
        let dir = tempdir().unwrap();
        write_node(dir.path(), "research", "Research task");
        let nodes_dir = dir.path().join("nodes");
        fs::write(nodes_dir.join("readme.txt"), "ignore me").unwrap();

        let nodes = read_all_nodes(dir.path()).unwrap();
        assert_eq!(nodes.len(), 1);
        assert!(nodes.contains_key("research"));
    }

    #[test]
    fn read_board_combined() {
        let dir = tempdir().unwrap();
        write_topology(
            dir.path(),
            r#"{ "nodes": { "research": { "depends_on": [] } } }"#,
        );
        write_node(dir.path(), "research", "Research task");

        let (topo, nodes) = read_board(dir.path()).unwrap();
        assert_eq!(topo.len(), 1);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes["research"], "Research task");
    }
}
