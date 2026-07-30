#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use crate::server::hub::execution::strategies::workflow_agent::validate_written_files;

    // ── validate_written_files ─────────────────────────────────────────

    fn setup_valid_board(dir: &Path) {
        fs::write(
            dir.join("topology.json"),
            r#"{ "nodes": { "research": { "depends_on": [] } } }"#,
        )
        .unwrap();
        let nodes_dir = dir.join("nodes");
        fs::create_dir_all(&nodes_dir).unwrap();
        fs::write(nodes_dir.join("research.md"), "Do research").unwrap();
    }

    #[test]
    fn validate_valid_board() {
        let dir = tempdir().unwrap();
        setup_valid_board(dir.path());
        let errors = validate_written_files(dir.path());
        assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
    }

    #[test]
    fn validate_invalid_topology() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("topology.json"), "not json").unwrap();
        let errors = validate_written_files(dir.path());
        assert!(errors.iter().any(|e| e.contains("topology.json")));
    }

    #[test]
    fn validate_empty_node() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("topology.json"),
            r#"{ "nodes": { "empty": { "depends_on": [] } } }"#,
        )
        .unwrap();
        let nodes_dir = dir.path().join("nodes");
        fs::create_dir_all(&nodes_dir).unwrap();
        fs::write(nodes_dir.join("empty.md"), "   ").unwrap();
        let errors = validate_written_files(dir.path());
        assert!(errors.iter().any(|e| e.contains("empty.md")));
    }

    #[test]
    fn validate_missing_files_no_error() {
        let dir = tempdir().unwrap();
        // No files at all — should not error (agent hasn't written anything yet)
        let errors = validate_written_files(dir.path());
        assert!(errors.is_empty());
    }
}
