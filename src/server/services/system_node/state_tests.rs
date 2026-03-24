#[cfg(test)]
mod tests {
    use crate::server::services::system_node::state::build_current_state;

    #[test]
    fn empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let state = build_current_state(dir.path());
        assert!(state.contains("topology status=\"empty\""));
        assert!(state.contains("config status=\"missing\""));
    }

    #[test]
    fn with_topology_and_agents() {
        let dir = tempfile::tempdir().unwrap();
        let agents_dir = dir.path().join("agents");
        std::fs::create_dir(&agents_dir).unwrap();

        std::fs::write(
            dir.path().join("topology.json"),
            r#"{"agents": {"scanner": {"depends_on": []}, "analyzer": {"depends_on": ["scanner"]}}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("config.json"),
            r#"{"name": "Security Audit", "description": "test"}"#,
        )
        .unwrap();
        std::fs::write(agents_dir.join("scanner.json"), "{}").unwrap();
        // analyzer.json missing

        let state = build_current_state(dir.path());
        assert!(state.contains("slug=\"scanner\""));
        assert!(state.contains("status=\"configured\""));
        assert!(state.contains("slug=\"analyzer\""));
        assert!(state.contains("status=\"missing\""));
        assert!(state.contains("name=\"Security Audit\""));
    }
}
