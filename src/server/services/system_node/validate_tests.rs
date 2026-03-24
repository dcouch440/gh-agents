#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::server::services::system_node::validate::{
        cross_reference, validate_agent, validate_config, validate_topology, validate_verify,
    };

    // ── config.json ──────────────────────────────────────────────────────

    #[test]
    fn validate_config_valid() {
        let content = r#"{"name": "Security Audit", "description": "Scans for vulnerabilities."}"#;
        assert!(validate_config(content).is_ok());
    }

    #[test]
    fn validate_config_missing_name() {
        let content = r#"{"description": "Scans for vulnerabilities."}"#;
        let err = validate_config(content).unwrap_err();
        assert!(err.contains("name"), "expected 'name' in error: {err}");
    }

    #[test]
    fn validate_config_missing_description() {
        let content = r#"{"name": "Security Audit"}"#;
        let err = validate_config(content).unwrap_err();
        assert!(
            err.contains("description"),
            "expected 'description' in error: {err}"
        );
    }

    #[test]
    fn validate_config_empty_name() {
        let content = r#"{"name": "  ", "description": "valid"}"#;
        let err = validate_config(content).unwrap_err();
        assert!(err.contains("empty"), "expected 'empty' in error: {err}");
    }

    #[test]
    fn validate_config_bad_json() {
        let err = validate_config("not json").unwrap_err();
        assert!(
            err.contains("invalid JSON"),
            "expected 'invalid JSON' in error: {err}"
        );
    }

    // ── topology.json ────────────────────────────────────────────────────

    #[test]
    fn validate_topology_valid() {
        let content = r#"{"agents": {"scanner": {"depends_on": []}, "analyzer": {"depends_on": ["scanner"]}}}"#;
        assert!(validate_topology(content).is_ok());
    }

    #[test]
    fn validate_topology_missing_agents() {
        let content = r#"{}"#;
        let err = validate_topology(content).unwrap_err();
        assert!(err.contains("agents"), "expected 'agents' in error: {err}");
    }

    #[test]
    fn validate_topology_missing_depends_on() {
        let content = r#"{"agents": {"scanner": {}}}"#;
        let err = validate_topology(content).unwrap_err();
        assert!(
            err.contains("depends_on"),
            "expected 'depends_on' in error: {err}"
        );
    }

    #[test]
    fn validate_topology_non_string_dep() {
        let content = r#"{"agents": {"scanner": {"depends_on": [123]}}}"#;
        let err = validate_topology(content).unwrap_err();
        assert!(
            err.contains("non-string"),
            "expected 'non-string' in error: {err}"
        );
    }

    // ── agents/*.json ────────────────────────────────────────────────────

    #[test]
    fn validate_agent_valid() {
        let content = r#"{
            "name": "Scanner",
            "system_prompt": "Security scanner.",
            "assignment": "Find vulnerabilities.",
            "expected_output": "What you found.",
            "capabilities": []
        }"#;
        assert!(validate_agent(content).is_ok());
    }

    #[test]
    fn validate_agent_missing_system_prompt() {
        let content = r#"{
            "name": "Scanner",
            "assignment": "Find vulnerabilities.",
            "expected_output": "What you found.",
            "capabilities": []
        }"#;
        let err = validate_agent(content).unwrap_err();
        assert!(
            err.contains("system_prompt"),
            "expected 'system_prompt' in error: {err}"
        );
    }

    #[test]
    fn validate_agent_missing_assignment() {
        let content = r#"{
            "name": "Scanner",
            "system_prompt": "Security scanner.",
            "expected_output": "What you found.",
            "capabilities": []
        }"#;
        let err = validate_agent(content).unwrap_err();
        assert!(
            err.contains("assignment"),
            "expected 'assignment' in error: {err}"
        );
    }

    #[test]
    fn validate_agent_missing_capabilities() {
        let content = r#"{
            "name": "Scanner",
            "system_prompt": "Security scanner.",
            "assignment": "Find vulnerabilities.",
            "expected_output": "What you found."
        }"#;
        let err = validate_agent(content).unwrap_err();
        assert!(
            err.contains("capabilities"),
            "expected 'capabilities' in error: {err}"
        );
    }

    #[test]
    fn validate_agent_non_string_capability() {
        let content = r#"{
            "name": "Scanner",
            "system_prompt": "Security scanner.",
            "assignment": "Find vulnerabilities.",
            "expected_output": "What you found.",
            "capabilities": [123]
        }"#;
        let err = validate_agent(content).unwrap_err();
        assert!(
            err.contains("strings"),
            "expected 'strings' in error: {err}"
        );
    }

    // ── cross-reference ──────────────────────────────────────────────────

    #[test]
    fn cross_reference_all_match() {
        let dir = tempfile::tempdir().unwrap();
        let agents_dir = dir.path().join("agents");
        std::fs::create_dir(&agents_dir).unwrap();

        std::fs::write(
            dir.path().join("topology.json"),
            r#"{"agents": {"scanner": {"depends_on": []}}}"#,
        )
        .unwrap();
        std::fs::write(agents_dir.join("scanner.json"), "{}").unwrap();
        std::fs::write(
            dir.path().join("config.json"),
            r#"{"name": "test", "description": "test"}"#,
        )
        .unwrap();

        let errors = cross_reference(dir.path());
        assert!(errors.is_empty(), "expected no errors: {errors:?}");
    }

    #[test]
    fn cross_reference_missing_agent_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("agents")).unwrap();

        std::fs::write(
            dir.path().join("topology.json"),
            r#"{"agents": {"scanner": {"depends_on": []}}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("config.json"),
            r#"{"name": "test", "description": "test"}"#,
        )
        .unwrap();

        let errors = cross_reference(dir.path());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].file.contains("scanner"));
    }

    #[test]
    fn cross_reference_orphaned_file() {
        let dir = tempfile::tempdir().unwrap();
        let agents_dir = dir.path().join("agents");
        std::fs::create_dir(&agents_dir).unwrap();

        std::fs::write(dir.path().join("topology.json"), r#"{"agents": {}}"#).unwrap();
        std::fs::write(agents_dir.join("orphan.json"), "{}").unwrap();
        std::fs::write(
            dir.path().join("config.json"),
            r#"{"name": "test", "description": "test"}"#,
        )
        .unwrap();

        let errors = cross_reference(dir.path());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].error.contains("not listed"));
    }

    #[test]
    fn cross_reference_missing_config() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("agents")).unwrap();

        std::fs::write(dir.path().join("topology.json"), r#"{"agents": {}}"#).unwrap();

        let errors = cross_reference(dir.path());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].file.contains("config.json"));
    }

    #[test]
    fn cross_reference_missing_topology() {
        let dir = tempfile::tempdir().unwrap();
        let errors = cross_reference(dir.path());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].file.contains("topology.json"));
    }

    // ── validate_verify ──────────────────────────────────────────────────

    #[test]
    fn validate_verify_all_pass() {
        let dir = tempfile::tempdir().unwrap();
        let agents_dir = dir.path().join("agents");
        std::fs::create_dir(&agents_dir).unwrap();

        std::fs::write(
            dir.path().join("config.json"),
            r#"{"name": "Test", "description": "A test system."}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("topology.json"),
            r#"{"agents": {"reader": {"depends_on": []}}}"#,
        )
        .unwrap();
        std::fs::write(
            agents_dir.join("reader.json"),
            r#"{"name": "Reader", "system_prompt": "OCR.", "assignment": "Read.", "expected_output": "What.", "capabilities": []}"#,
        )
        .unwrap();

        let verify = json!({
            "topology_complete": true,
            "agents_complete": true,
            "config_accurate": true
        });
        let result = validate_verify(dir.path(), &verify);
        assert!(result.is_ok(), "expected Ok: {result:?}");
    }

    #[test]
    fn validate_verify_topology_fails() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("agents")).unwrap();

        std::fs::write(
            dir.path().join("topology.json"),
            r#"{"agents": {"missing": {"depends_on": []}}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("config.json"),
            r#"{"name": "Test", "description": "A test."}"#,
        )
        .unwrap();

        let verify = json!({
            "topology_complete": true,
            "agents_complete": true,
            "config_accurate": true
        });
        let result = validate_verify(dir.path(), &verify);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err["status"], "verification_failed");
        assert!(!err["errors"].as_array().unwrap().is_empty());
    }

    #[test]
    fn validate_verify_agents_fails() {
        let dir = tempfile::tempdir().unwrap();
        let agents_dir = dir.path().join("agents");
        std::fs::create_dir(&agents_dir).unwrap();

        std::fs::write(
            dir.path().join("config.json"),
            r#"{"name": "Test", "description": "A test."}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("topology.json"),
            r#"{"agents": {"bad": {"depends_on": []}}}"#,
        )
        .unwrap();
        // Agent file missing required fields
        std::fs::write(agents_dir.join("bad.json"), r#"{"name": "Bad"}"#).unwrap();

        let verify = json!({
            "topology_complete": true,
            "agents_complete": true,
            "config_accurate": true
        });
        let result = validate_verify(dir.path(), &verify);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let errors = err["errors"].as_array().unwrap();
        let agent_errors: Vec<&Value> = errors
            .iter()
            .filter(|e| e["verify"] == "agents_complete")
            .collect();
        assert!(!agent_errors.is_empty());
    }

    #[test]
    fn validate_verify_config_fails() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("agents")).unwrap();

        std::fs::write(dir.path().join("topology.json"), r#"{"agents": {}}"#).unwrap();
        // config missing description
        std::fs::write(dir.path().join("config.json"), r#"{"name": "Test"}"#).unwrap();

        let verify = json!({
            "topology_complete": true,
            "agents_complete": true,
            "config_accurate": true
        });
        let result = validate_verify(dir.path(), &verify);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let errors = err["errors"].as_array().unwrap();
        let config_errors: Vec<&Value> = errors
            .iter()
            .filter(|e| e["verify"] == "config_accurate")
            .collect();
        assert!(!config_errors.is_empty());
    }

    #[test]
    fn validate_verify_skips_unchecked() {
        let dir = tempfile::tempdir().unwrap();
        // Empty dir — nothing exists. But all verify = false, so no checks run.
        let verify = json!({
            "topology_complete": false,
            "agents_complete": false,
            "config_accurate": false
        });
        let result = validate_verify(dir.path(), &verify);
        assert!(result.is_ok());
    }
}
