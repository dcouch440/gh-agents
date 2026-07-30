#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::config::protocols::SYSTEM_NODE_AGENT;

    #[test]
    fn config_loads() {
        let cfg = SYSTEM_NODE_AGENT.agent("system");
        assert_eq!(cfg.model_id, crate::constants::MODEL_TIER2);
        assert_eq!(cfg.temperature, 0.3);
        assert_eq!(cfg.max_tokens, 8192);
        assert_eq!(cfg.max_rounds, 30);
        assert_eq!(cfg.context_budget, 480_000);
    }

    #[test]
    fn validate_written_files_catches_truncated_json() {
        let dir = tempfile::tempdir().unwrap();
        let agents_dir = dir.path().join("agents");
        std::fs::create_dir_all(&agents_dir).unwrap();

        // Valid config
        std::fs::write(
            dir.path().join("config.json"),
            r#"{"name": "Test", "description": "test"}"#,
        )
        .unwrap();

        // Truncated topology (simulates heredoc EOF break)
        std::fs::write(
            dir.path().join("topology.json"),
            r#"{"agents": {"scanner": {"depends_on": []}}"#, // missing closing brace
        )
        .unwrap();

        // Valid agent file
        std::fs::write(
            agents_dir.join("scanner.json"),
            r#"{"name": "Scanner", "system_prompt": "scan", "assignment": "scan", "expected_output": "done", "capabilities": []}"#,
        )
        .unwrap();

        let errors = super::super::validate_written_files(dir.path());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("topology.json"), "error: {}", errors[0]);
    }

    #[test]
    fn validate_written_files_all_valid() {
        let dir = tempfile::tempdir().unwrap();
        let agents_dir = dir.path().join("agents");
        std::fs::create_dir_all(&agents_dir).unwrap();

        std::fs::write(
            dir.path().join("config.json"),
            r#"{"name": "Test", "description": "test"}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("topology.json"),
            r#"{"agents": {"scanner": {"depends_on": []}}}"#,
        )
        .unwrap();
        std::fs::write(
            agents_dir.join("scanner.json"),
            r#"{"name": "Scanner", "system_prompt": "scan", "assignment": "scan", "expected_output": "done", "capabilities": []}"#,
        )
        .unwrap();

        let errors = super::super::validate_written_files(dir.path());
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    }

    #[test]
    fn validate_written_files_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let errors = super::super::validate_written_files(dir.path());
        assert!(errors.is_empty(), "empty dir should have no errors");
    }

    #[test]
    fn summary_capture_via_mutex() {
        let input = json!({
            "summary": "Configured 3-agent pipeline.",
            "verify": {
                "topology_complete": true,
                "agents_complete": true,
                "config_accurate": true
            }
        });

        let summary: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
        let s = input["summary"].as_str().unwrap_or("").to_string();
        *summary.lock().unwrap() = Some(s);
        assert_eq!(
            summary.lock().unwrap().as_deref(),
            Some("Configured 3-agent pipeline.")
        );
    }
}
