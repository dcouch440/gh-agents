#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs;

    use tempfile::tempdir;

    use crate::server::services::workflow_agent::validate::{
        cross_reference, detect_cycles, validate_node, validate_topology,
    };

    // ── validate_topology ──────────────────────────────────────────────

    #[test]
    fn valid_topology() {
        let content = r#"{
            "nodes": {
                "research": { "depends_on": [] },
                "report": { "depends_on": ["research"] }
            }
        }"#;
        assert!(validate_topology(content).is_ok());
    }

    #[test]
    fn empty_topology() {
        assert!(validate_topology(r#"{ "nodes": {} }"#).is_ok());
    }

    #[test]
    fn invalid_json() {
        assert!(validate_topology("not json").is_err());
    }

    #[test]
    fn missing_nodes_key() {
        let result = validate_topology(r#"{ "other": {} }"#);
        assert!(result.unwrap_err().contains("nodes"));
    }

    #[test]
    fn missing_depends_on() {
        let content = r#"{ "nodes": { "research": {} } }"#;
        let result = validate_topology(content);
        assert!(result.unwrap_err().contains("depends_on"));
    }

    #[test]
    fn non_string_depends_on() {
        let content = r#"{ "nodes": { "research": { "depends_on": [42] } } }"#;
        let result = validate_topology(content);
        assert!(result.unwrap_err().contains("non-string"));
    }

    // ── validate_node ──────────────────────────────────────────────────

    #[test]
    fn valid_node() {
        assert!(validate_node("# Research\n\nDo research.", "research").is_ok());
    }

    #[test]
    fn empty_node() {
        let result = validate_node("", "research");
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn whitespace_only_node() {
        let result = validate_node("   \n\n  ", "research");
        assert!(result.unwrap_err().contains("empty"));
    }

    // ── detect_cycles ──────────────────────────────────────────────────

    #[test]
    fn no_cycles() {
        let mut topo = HashMap::new();
        topo.insert("a".to_string(), vec!["b".to_string()]);
        topo.insert("b".to_string(), vec![]);
        assert!(detect_cycles(&topo).is_ok());
    }

    #[test]
    fn simple_cycle() {
        let mut topo = HashMap::new();
        topo.insert("a".to_string(), vec!["b".to_string()]);
        topo.insert("b".to_string(), vec!["a".to_string()]);
        let result = detect_cycles(&topo);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cycle detected"));
    }

    #[test]
    fn three_node_cycle() {
        let mut topo = HashMap::new();
        topo.insert("a".to_string(), vec!["b".to_string()]);
        topo.insert("b".to_string(), vec!["c".to_string()]);
        topo.insert("c".to_string(), vec!["a".to_string()]);
        assert!(detect_cycles(&topo).is_err());
    }

    #[test]
    fn self_cycle() {
        let mut topo = HashMap::new();
        topo.insert("a".to_string(), vec!["a".to_string()]);
        assert!(detect_cycles(&topo).is_err());
    }

    #[test]
    fn diamond_no_cycle() {
        let mut topo = HashMap::new();
        topo.insert("a".to_string(), vec![]);
        topo.insert("b".to_string(), vec!["a".to_string()]);
        topo.insert("c".to_string(), vec!["a".to_string()]);
        topo.insert("d".to_string(), vec!["b".to_string(), "c".to_string()]);
        assert!(detect_cycles(&topo).is_ok());
    }

    #[test]
    fn dangling_reference_not_a_cycle() {
        let mut topo = HashMap::new();
        topo.insert("a".to_string(), vec!["nonexistent".to_string()]);
        assert!(detect_cycles(&topo).is_ok());
    }

    // ── cross_reference ────────────────────────────────────────────────

    fn setup_board(dir: &std::path::Path, topo: &str, nodes: &[(&str, &str)]) {
        fs::write(dir.join("topology.json"), topo).unwrap();
        let nodes_dir = dir.join("nodes");
        fs::create_dir_all(&nodes_dir).unwrap();
        for (slug, content) in nodes {
            fs::write(nodes_dir.join(format!("{slug}.md")), content).unwrap();
        }
    }

    #[test]
    fn cross_reference_valid() {
        let dir = tempdir().unwrap();
        setup_board(
            dir.path(),
            r#"{ "nodes": { "research": { "depends_on": [] } } }"#,
            &[("research", "task")],
        );
        let errors = cross_reference(dir.path());
        assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
    }

    #[test]
    fn cross_reference_missing_node_file() {
        let dir = tempdir().unwrap();
        setup_board(
            dir.path(),
            r#"{ "nodes": { "research": { "depends_on": [] } } }"#,
            &[],
        );
        let errors = cross_reference(dir.path());
        assert!(errors.iter().any(|e| e.file.contains("research")));
    }

    #[test]
    fn cross_reference_orphan_node_file() {
        let dir = tempdir().unwrap();
        setup_board(dir.path(), r#"{ "nodes": {} }"#, &[("orphan", "content")]);
        let errors = cross_reference(dir.path());
        assert!(errors.iter().any(|e| e.file.contains("orphan")));
    }

    #[test]
    fn cross_reference_dangling_depends_on() {
        let dir = tempdir().unwrap();
        setup_board(
            dir.path(),
            r#"{ "nodes": { "a": { "depends_on": ["missing"] } } }"#,
            &[("a", "content")],
        );
        let errors = cross_reference(dir.path());
        assert!(errors.iter().any(|e| e.error.contains("missing")));
    }

    #[test]
    fn cross_reference_detects_cycle() {
        let dir = tempdir().unwrap();
        setup_board(
            dir.path(),
            r#"{ "nodes": {
                "a": { "depends_on": ["b"] },
                "b": { "depends_on": ["a"] }
            } }"#,
            &[("a", "content"), ("b", "content")],
        );
        let errors = cross_reference(dir.path());
        assert!(errors.iter().any(|e| e.error.contains("cycle")));
    }
}
