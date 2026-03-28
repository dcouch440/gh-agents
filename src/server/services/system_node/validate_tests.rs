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
        let result = validate_verify(dir.path(), &verify, None);
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
        let result = validate_verify(dir.path(), &verify, None);
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
        let result = validate_verify(dir.path(), &verify, None);
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
        let result = validate_verify(dir.path(), &verify, None);
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
        let result = validate_verify(dir.path(), &verify, None);
        assert!(result.is_ok());
    }

    // ── check_prescribed_filenames ────────────────────────────────────────

    use crate::server::services::system_node::validate::{
        check_assignment_expansion, check_prescribed_filenames, check_prompt_length,
        extract_user_text_words,
    };

    #[test]
    fn filenames_catches_save_as() {
        let content = r#"{
            "name": "Writer",
            "system_prompt": "Write reports.",
            "assignment": "Write a report. Save as report.md in the workspace.",
            "expected_output": "Report location.",
            "capabilities": []
        }"#;
        let issues = check_prescribed_filenames(content);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].0 == "assignment");
        assert!(issues[0].1.contains("report.md"));
    }

    #[test]
    fn filenames_catches_write_to() {
        let content = r#"{
            "name": "Writer",
            "system_prompt": "Write reports.",
            "assignment": "Analyze data.",
            "expected_output": "Write to analysis.json and report path.",
            "capabilities": []
        }"#;
        let issues = check_prescribed_filenames(content);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].0 == "expected_output");
    }

    #[test]
    fn filenames_ignores_read_references() {
        let content = r#"{
            "name": "Reader",
            "system_prompt": "Read files.",
            "assignment": "Read the upstream report.md and analyze it.",
            "expected_output": "Summary of findings.",
            "capabilities": []
        }"#;
        let issues = check_prescribed_filenames(content);
        assert!(
            issues.is_empty(),
            "should not flag 'Read ... report.md': {issues:?}"
        );
    }

    #[test]
    fn filenames_ignores_bare_filenames() {
        let content = r#"{
            "name": "Writer",
            "system_prompt": "Write reports.",
            "assignment": "Analyze the data.",
            "expected_output": "Report filename, section count, word count.",
            "capabilities": []
        }"#;
        let issues = check_prescribed_filenames(content);
        assert!(issues.is_empty());
    }

    #[test]
    fn filenames_ignores_no_extension() {
        let content = r#"{
            "name": "Writer",
            "system_prompt": "Write reports.",
            "assignment": "Save your findings to the workspace.",
            "expected_output": "Where you saved the report.",
            "capabilities": []
        }"#;
        let issues = check_prescribed_filenames(content);
        assert!(issues.is_empty());
    }

    // ── check_prompt_length ───────────────────────────────────────────────

    #[test]
    fn prompt_length_fails_short() {
        let content = r#"{
            "name": "Scanner",
            "system_prompt": "Security scanner. Find vulnerabilities.",
            "assignment": "Scan code.",
            "expected_output": "Results.",
            "capabilities": []
        }"#;
        let result = check_prompt_length(content, 20);
        assert!(result.is_some());
        let msg = result.unwrap();
        assert!(msg.contains("words"), "expected word count in error: {msg}");
    }

    #[test]
    fn prompt_length_passes_at_threshold() {
        // Build a system_prompt with exactly 20 words
        let words: Vec<&str> = vec!["word"; 20];
        let sp = words.join(" ");
        let content = format!(
            r#"{{"name": "Agent", "system_prompt": "{sp}", "assignment": "Do.", "expected_output": "Done.", "capabilities": []}}"#
        );
        let result = check_prompt_length(&content, 20);
        assert!(result.is_none(), "20 words should pass min=20");
    }

    #[test]
    fn prompt_length_lower_threshold_for_single_agent() {
        let content = r#"{
            "name": "Writer",
            "system_prompt": "Tech writer for developer audiences. Structure posts clearly.",
            "assignment": "Write a post.",
            "expected_output": "Post.",
            "capabilities": []
        }"#;
        // 8 words — fails at min=20, passes at min=10
        assert!(check_prompt_length(content, 20).is_some());
        assert!(check_prompt_length(content, 8).is_none());
    }

    // ── check_assignment_expansion ────────────────────────────────────────

    #[test]
    fn assignment_expansion_fails_when_compressed() {
        let content = r#"{
            "name": "Writer",
            "system_prompt": "Writer.",
            "assignment": "Write a blog post about AI research.",
            "expected_output": "Post.",
            "capabilities": []
        }"#;
        // assignment is 7 words, user text was 50
        let result = check_assignment_expansion(content, 50);
        assert!(result.is_some());
        assert!(result.unwrap().contains("7 words"));
    }

    #[test]
    fn assignment_expansion_passes_when_expanded() {
        let content = r#"{
            "name": "Writer",
            "system_prompt": "Writer.",
            "assignment": "Read the ranked research papers from the previous step. Write a blog post covering the top findings, why they matter, and what comes next. Target 1500 words.",
            "expected_output": "Post.",
            "capabilities": []
        }"#;
        // assignment is ~28 words, user text was 10
        let result = check_assignment_expansion(content, 10);
        assert!(result.is_none());
    }

    #[test]
    fn assignment_expansion_passes_when_equal() {
        let content = r#"{
            "name": "Writer",
            "system_prompt": "Writer.",
            "assignment": "one two three four five six seven eight nine ten",
            "expected_output": "Post.",
            "capabilities": []
        }"#;
        let result = check_assignment_expansion(content, 10);
        assert!(result.is_none(), "equal word count should pass");
    }

    // ── extract_user_text_words ───────────────────────────────────────────

    #[test]
    fn extract_user_text_found() {
        let instruction = "Configure this.\n\n<user_text>\nWrite a blog post about AI\n</user_text>\n\nMore stuff.";
        let result = extract_user_text_words(instruction);
        assert_eq!(result, Some(6));
    }

    #[test]
    fn extract_user_text_not_found() {
        let instruction = "The user updated this step.\n\n<change>\nBefore: X\nAfter: Y\n</change>";
        let result = extract_user_text_words(instruction);
        assert!(result.is_none());
    }

    // ── validate_verify with quality flags ────────────────────────────────

    #[test]
    fn verify_no_filenames_catches_violation() {
        let dir = tempfile::tempdir().unwrap();
        let agents_dir = dir.path().join("agents");
        std::fs::create_dir(&agents_dir).unwrap();

        std::fs::write(
            dir.path().join("topology.json"),
            r#"{"agents": {"writer": {"depends_on": []}}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("config.json"),
            r#"{"name": "Test", "description": "Test."}"#,
        )
        .unwrap();
        std::fs::write(
            agents_dir.join("writer.json"),
            r#"{"name": "Writer", "system_prompt": "Write stuff with good methodology and approach.", "assignment": "Write a report. Save as report.md in the workspace.", "expected_output": "Report location.", "capabilities": []}"#,
        )
        .unwrap();

        let verify = json!({ "no_filenames_prescribed": true });
        let result = validate_verify(dir.path(), &verify, None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let errors = err["errors"].as_array().unwrap();
        assert!(errors
            .iter()
            .any(|e| e["verify"] == "no_filenames_prescribed"));
    }

    #[test]
    fn verify_prompts_not_trivial_catches_short_prompt() {
        let dir = tempfile::tempdir().unwrap();
        let agents_dir = dir.path().join("agents");
        std::fs::create_dir(&agents_dir).unwrap();

        std::fs::write(
            dir.path().join("topology.json"),
            r#"{"agents": {"scanner": {"depends_on": []}, "reporter": {"depends_on": ["scanner"]}}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("config.json"),
            r#"{"name": "Test", "description": "Test."}"#,
        )
        .unwrap();
        // Scanner has a trivial 4-word prompt in a multi-agent topology (min=20)
        std::fs::write(
            agents_dir.join("scanner.json"),
            r#"{"name": "Scanner", "system_prompt": "Security scanner. Find bugs.", "assignment": "Scan code.", "expected_output": "Results.", "capabilities": []}"#,
        )
        .unwrap();
        std::fs::write(
            agents_dir.join("reporter.json"),
            r#"{"name": "Reporter", "system_prompt": "Security scanner. Find bugs.", "assignment": "Write.", "expected_output": "Report.", "capabilities": []}"#,
        )
        .unwrap();

        let verify = json!({ "prompts_not_trivial": true });
        let result = validate_verify(dir.path(), &verify, None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let errors = err["errors"].as_array().unwrap();
        assert!(errors.iter().any(|e| e["verify"] == "prompts_not_trivial"));
    }

    #[test]
    fn verify_assignments_expanded_catches_compression() {
        let dir = tempfile::tempdir().unwrap();
        let agents_dir = dir.path().join("agents");
        std::fs::create_dir(&agents_dir).unwrap();

        std::fs::write(
            dir.path().join("topology.json"),
            r#"{"agents": {"writer": {"depends_on": []}}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("config.json"),
            r#"{"name": "Test", "description": "Test."}"#,
        )
        .unwrap();
        // Assignment is only 3 words
        std::fs::write(
            agents_dir.join("writer.json"),
            r#"{"name": "Writer", "system_prompt": "Write.", "assignment": "Write a report.", "expected_output": "Report.", "capabilities": []}"#,
        )
        .unwrap();

        // User text was 30 words — assignment of 3 is compression
        let verify = json!({ "assignments_expanded": true });
        let result = validate_verify(dir.path(), &verify, Some(30));
        assert!(result.is_err());
        let err = result.unwrap_err();
        let errors = err["errors"].as_array().unwrap();
        assert!(errors.iter().any(|e| e["verify"] == "assignments_expanded"));
    }

    #[test]
    fn verify_assignments_expanded_skips_without_user_text() {
        let dir = tempfile::tempdir().unwrap();
        let agents_dir = dir.path().join("agents");
        std::fs::create_dir(&agents_dir).unwrap();

        std::fs::write(
            dir.path().join("topology.json"),
            r#"{"agents": {"writer": {"depends_on": []}}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("config.json"),
            r#"{"name": "Test", "description": "Test."}"#,
        )
        .unwrap();
        std::fs::write(
            agents_dir.join("writer.json"),
            r#"{"name": "Writer", "system_prompt": "Write.", "assignment": "Short.", "expected_output": "Done.", "capabilities": []}"#,
        )
        .unwrap();

        // No user_text (update/propagation) — should pass silently
        let verify = json!({ "assignments_expanded": true });
        let result = validate_verify(dir.path(), &verify, None);
        assert!(result.is_ok(), "should skip when no user_text: {result:?}");
    }
}
