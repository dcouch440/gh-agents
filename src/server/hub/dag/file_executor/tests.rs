#[cfg(test)]
mod tests {
    use crate::server::hub::dag::pipeline::DesignedAgentPrompt;
    use crate::server::services::system_node::file_reader::read_agent_configs;

    // Re-use compute_execution_levels to verify the full data path:
    // files → read_agent_configs → DesignedAgentPrompt → compute_execution_levels → levels
    fn compute_levels(prompts: &[DesignedAgentPrompt]) -> Vec<Vec<usize>> {
        crate::server::hub::dag::pipeline::compute_execution_levels(prompts)
    }

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

    // ── Bridge contract tests ────────────────────────────────────────────
    // These verify the CONTRACT between file_reader output and the execution
    // scheduler — that the shapes are compatible and produce correct levels.

    #[test]
    fn single_agent_produces_one_level() {
        let dir = tempfile::tempdir().unwrap();
        write_repo(
            dir.path(),
            r#"{"agents": {"reader": {"depends_on": []}}}"#,
            &[("reader", &agent_json("Reader", "Read.", "Read files."))],
            r#"{"name": "Test", "description": "test"}"#,
        );

        let prompts = read_agent_configs(dir.path()).unwrap();
        let levels = compute_levels(&prompts);

        assert_eq!(levels.len(), 1, "single agent should produce 1 level");
        assert_eq!(levels[0].len(), 1, "level 0 should have 1 agent");
        assert_eq!(prompts[levels[0][0]].agent_name, "Reader");
    }

    #[test]
    fn pipeline_produces_sequential_levels() {
        let dir = tempfile::tempdir().unwrap();
        write_repo(
            dir.path(),
            r#"{"agents": {"scanner": {"depends_on": []}, "analyzer": {"depends_on": ["scanner"]}, "reporter": {"depends_on": ["analyzer"]}}}"#,
            &[
                ("scanner", &agent_json("Scanner", "Scan.", "Scan.")),
                ("analyzer", &agent_json("Analyzer", "Analyze.", "Analyze.")),
                ("reporter", &agent_json("Reporter", "Report.", "Report.")),
            ],
            r#"{"name": "Audit", "description": "test"}"#,
        );

        let prompts = read_agent_configs(dir.path()).unwrap();
        let levels = compute_levels(&prompts);

        assert_eq!(
            levels.len(),
            3,
            "pipeline should produce 3 sequential levels"
        );
        for level in &levels {
            assert_eq!(level.len(), 1, "each level should have exactly 1 agent");
        }

        // Verify topological order: scanner → analyzer → reporter
        let names: Vec<&str> = levels
            .iter()
            .map(|l| prompts[l[0]].agent_name.as_str())
            .collect();
        assert_eq!(names, vec!["Scanner", "Analyzer", "Reporter"]);
    }

    #[test]
    fn fan_out_produces_parallel_level() {
        let dir = tempfile::tempdir().unwrap();
        write_repo(
            dir.path(),
            r#"{"agents": {"scraper": {"depends_on": []}, "monitor": {"depends_on": []}, "analyst": {"depends_on": ["scraper", "monitor"]}}}"#,
            &[
                ("scraper", &agent_json("Scraper", "Scrape.", "Scrape.")),
                ("monitor", &agent_json("Monitor", "Monitor.", "Monitor.")),
                ("analyst", &agent_json("Analyst", "Analyze.", "Analyze.")),
            ],
            r#"{"name": "Intel", "description": "test"}"#,
        );

        let prompts = read_agent_configs(dir.path()).unwrap();
        let levels = compute_levels(&prompts);

        assert_eq!(levels.len(), 2, "fan-out should produce 2 levels");
        assert_eq!(
            levels[0].len(),
            2,
            "level 0 should have 2 parallel agents (roots)"
        );
        assert_eq!(
            levels[1].len(),
            1,
            "level 1 should have 1 agent (depends on both)"
        );

        // The fan-in agent should be Analyst
        assert_eq!(prompts[levels[1][0]].agent_name, "Analyst");
    }

    #[test]
    fn diamond_produces_correct_levels() {
        let dir = tempfile::tempdir().unwrap();
        write_repo(
            dir.path(),
            r#"{"agents": {"builder": {"depends_on": []}, "reviewer": {"depends_on": ["builder"]}, "writer": {"depends_on": ["builder"]}, "editor": {"depends_on": ["reviewer", "writer"]}}}"#,
            &[
                ("builder", &agent_json("Builder", "Build.", "Build.")),
                ("reviewer", &agent_json("Reviewer", "Review.", "Review.")),
                ("writer", &agent_json("Writer", "Write.", "Write.")),
                ("editor", &agent_json("Editor", "Edit.", "Edit.")),
            ],
            r#"{"name": "Diamond", "description": "test"}"#,
        );

        let prompts = read_agent_configs(dir.path()).unwrap();
        let levels = compute_levels(&prompts);

        assert_eq!(levels.len(), 3, "diamond should produce 3 levels");
        assert_eq!(levels[0].len(), 1, "level 0: 1 root agent");
        assert_eq!(levels[1].len(), 2, "level 1: 2 parallel agents");
        assert_eq!(levels[2].len(), 1, "level 2: 1 fan-in agent");

        assert_eq!(prompts[levels[0][0]].agent_name, "Builder");
        assert_eq!(prompts[levels[2][0]].agent_name, "Editor");
    }

    #[test]
    fn missing_topology_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let result = read_agent_configs(dir.path());
        assert!(result.is_err());
        assert!(
            result.unwrap_err().contains("topology.json"),
            "error should mention topology.json"
        );
    }

    #[test]
    fn empty_agents_dir_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("agents")).unwrap();
        std::fs::write(
            dir.path().join("topology.json"),
            r#"{"agents": {"ghost": {"depends_on": []}}}"#,
        )
        .unwrap();

        let result = read_agent_configs(dir.path());
        assert!(result.is_err());
        assert!(
            result.unwrap_err().contains("ghost.json"),
            "error should mention missing agent file"
        );
    }
}
