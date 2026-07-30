#[cfg(test)]
mod tests {
    use crate::server::services::system_node::file_reader::{
        read_agent_configs, read_config, read_topology,
    };

    fn write_repo(dir: &std::path::Path, topology: &str, agents: &[(&str, &str)], config: &str) {
        let agents_dir = dir.join("agents");
        std::fs::create_dir_all(&agents_dir).unwrap();
        std::fs::write(dir.join("topology.json"), topology).unwrap();
        std::fs::write(dir.join("config.json"), config).unwrap();
        for (slug, content) in agents {
            std::fs::write(agents_dir.join(format!("{slug}.json")), content).unwrap();
        }
    }

    fn agent_json(name: &str, prompt: &str, assignment: &str) -> String {
        format!(
            r#"{{"name": "{name}", "system_prompt": "{prompt}", "assignment": "{assignment}", "expected_output": "Report what you did.", "capabilities": []}}"#
        )
    }

    // ── read_agent_configs ───────────────────────────────────────────────

    #[test]
    fn read_single_agent() {
        let dir = tempfile::tempdir().unwrap();
        write_repo(
            dir.path(),
            r#"{"agents": {"reader": {"depends_on": []}}}"#,
            &[(
                "reader",
                &agent_json("Reader", "OCR specialist.", "Read the image."),
            )],
            r#"{"name": "Test", "description": "test"}"#,
        );

        let prompts = read_agent_configs(dir.path()).unwrap();
        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0].agent_name, "Reader");
        assert_eq!(prompts[0].system_prompt, "OCR specialist.");
        assert_eq!(prompts[0].assignment, "Read the image.");
        assert!(prompts[0].receives_from.is_empty());
        assert!(prompts[0].tools.is_empty());
        assert_eq!(prompts[0].execution_order, 0);
    }

    #[test]
    fn read_pipeline() {
        let dir = tempfile::tempdir().unwrap();
        write_repo(
            dir.path(),
            r#"{"agents": {"scanner": {"depends_on": []}, "analyzer": {"depends_on": ["scanner"]}, "reporter": {"depends_on": ["analyzer"]}}}"#,
            &[
                (
                    "scanner",
                    &agent_json("Scanner", "Security scanner.", "Scan."),
                ),
                (
                    "analyzer",
                    &agent_json("Analyzer", "Security analyst.", "Analyze."),
                ),
                (
                    "reporter",
                    &agent_json("Reporter", "Technical writer.", "Report."),
                ),
            ],
            r#"{"name": "Audit", "description": "test"}"#,
        );

        let prompts = read_agent_configs(dir.path()).unwrap();
        assert_eq!(prompts.len(), 3);

        // Check receives_from is set correctly
        let scanner = prompts.iter().find(|p| p.agent_name == "Scanner").unwrap();
        assert!(scanner.receives_from.is_empty());

        let analyzer = prompts.iter().find(|p| p.agent_name == "Analyzer").unwrap();
        assert_eq!(analyzer.receives_from, vec!["Scanner"]);

        let reporter = prompts.iter().find(|p| p.agent_name == "Reporter").unwrap();
        assert_eq!(reporter.receives_from, vec!["Analyzer"]);
    }

    #[test]
    fn read_fan_out() {
        let dir = tempfile::tempdir().unwrap();
        write_repo(
            dir.path(),
            r#"{"agents": {"researcher": {"depends_on": []}, "writer": {"depends_on": ["researcher"]}, "reviewer": {"depends_on": ["researcher"]}}}"#,
            &[
                (
                    "researcher",
                    &agent_json("Researcher", "Research.", "Research."),
                ),
                ("writer", &agent_json("Writer", "Write.", "Write.")),
                ("reviewer", &agent_json("Reviewer", "Review.", "Review.")),
            ],
            r#"{"name": "Test", "description": "test"}"#,
        );

        let prompts = read_agent_configs(dir.path()).unwrap();
        assert_eq!(prompts.len(), 3);

        let writer = prompts.iter().find(|p| p.agent_name == "Writer").unwrap();
        assert_eq!(writer.receives_from, vec!["Researcher"]);

        let reviewer = prompts.iter().find(|p| p.agent_name == "Reviewer").unwrap();
        assert_eq!(reviewer.receives_from, vec!["Researcher"]);
    }

    /// Regression: topology slugs differ from agent display names.
    /// receives_from must contain display names, not slugs, for downstream
    /// matching in compute_execution_levels and filter_outputs_for_agent.
    #[test]
    fn read_pipeline_slug_differs_from_display_name() {
        let dir = tempfile::tempdir().unwrap();
        write_repo(
            dir.path(),
            r#"{"agents": {"brainstormer": {"depends_on": []}, "curator": {"depends_on": ["brainstormer"]}}}"#,
            &[
                (
                    "brainstormer",
                    &agent_json("Idea Brainstormer", "Brainstorm.", "Generate ideas."),
                ),
                (
                    "curator",
                    &agent_json("Idea Curator", "Curate.", "Pick best ideas."),
                ),
            ],
            r#"{"name": "Fun Ideas", "description": "test"}"#,
        );

        let prompts = read_agent_configs(dir.path()).unwrap();
        let curator = prompts
            .iter()
            .find(|p| p.agent_name == "Idea Curator")
            .unwrap();
        assert_eq!(
            curator.receives_from,
            vec!["Idea Brainstormer"],
            "receives_from should contain display names, not topology slugs"
        );
    }

    #[test]
    fn read_missing_agent_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("agents")).unwrap();
        std::fs::write(
            dir.path().join("topology.json"),
            r#"{"agents": {"missing": {"depends_on": []}}}"#,
        )
        .unwrap();

        let err = read_agent_configs(dir.path()).unwrap_err();
        assert!(
            err.contains("missing.json"),
            "error should mention file: {err}"
        );
    }

    #[test]
    fn read_bad_agent_json() {
        let dir = tempfile::tempdir().unwrap();
        let agents_dir = dir.path().join("agents");
        std::fs::create_dir_all(&agents_dir).unwrap();
        std::fs::write(
            dir.path().join("topology.json"),
            r#"{"agents": {"bad": {"depends_on": []}}}"#,
        )
        .unwrap();
        std::fs::write(agents_dir.join("bad.json"), "not json").unwrap();

        let err = read_agent_configs(dir.path()).unwrap_err();
        assert!(
            err.contains("invalid JSON"),
            "error should mention JSON: {err}"
        );
    }

    #[test]
    fn read_missing_topology() {
        let dir = tempfile::tempdir().unwrap();
        let err = read_agent_configs(dir.path()).unwrap_err();
        assert!(
            err.contains("topology.json"),
            "error should mention topology: {err}"
        );
    }

    // ── read_config ──────────────────────────────────────────────────────

    #[test]
    fn read_config_valid() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("config.json"),
            r#"{"name": "Security Audit", "description": "Scans for vulnerabilities."}"#,
        )
        .unwrap();

        let (name, desc) = read_config(dir.path()).unwrap();
        assert_eq!(name, "Security Audit");
        assert_eq!(desc, "Scans for vulnerabilities.");
    }

    #[test]
    fn read_config_missing() {
        let dir = tempfile::tempdir().unwrap();
        let err = read_config(dir.path()).unwrap_err();
        assert!(
            err.contains("config.json"),
            "error should mention file: {err}"
        );
    }

    // ── read_topology ────────────────────────────────────────────────────

    #[test]
    fn read_topology_valid() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("topology.json"),
            r#"{"agents": {"a": {"depends_on": []}, "b": {"depends_on": ["a"]}}}"#,
        )
        .unwrap();

        let topo = read_topology(dir.path()).unwrap();
        assert_eq!(topo.len(), 2);
        assert!(topo["a"].is_empty());
        assert_eq!(topo["b"], vec!["a"]);
    }
}
