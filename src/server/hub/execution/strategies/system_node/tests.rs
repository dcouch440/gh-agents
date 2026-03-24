#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::config::protocols::SYSTEM_NODE_AGENT;
    use crate::server::hub::execution::strategies::system_node::{
        build_current_state, complete_system_tool,
    };

    #[test]
    fn config_loads() {
        let cfg = SYSTEM_NODE_AGENT.agent("system");
        assert_eq!(cfg.model_id, crate::constants::MODEL_TIER2);
        assert_eq!(cfg.temperature, 0.3);
        assert_eq!(cfg.max_tokens, 8192);
        assert_eq!(cfg.max_rounds, 10);
        assert_eq!(cfg.context_budget, 480_000);
    }

    #[test]
    fn complete_system_tool_schema() {
        let tool = complete_system_tool();
        assert_eq!(tool.name, "complete_system");

        let required = tool.input_schema["required"].as_array().unwrap();
        let names: Vec<&str> = required
            .iter()
            .filter_map(|v: &serde_json::Value| v.as_str())
            .collect();
        assert!(names.contains(&"summary"));
        assert!(names.contains(&"verify"));

        let verify_props = &tool.input_schema["properties"]["verify"]["properties"];
        assert!(verify_props["topology_complete"].is_object());
        assert!(verify_props["agents_complete"].is_object());
        assert!(verify_props["config_accurate"].is_object());
    }

    #[test]
    fn build_current_state_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let state = build_current_state(dir.path());
        assert!(state.contains("topology status=\"empty\""));
        assert!(state.contains("config status=\"missing\""));
    }

    #[test]
    fn build_current_state_with_topology_and_agents() {
        let dir = tempfile::tempdir().unwrap();
        let agents_dir = dir.path().join("agents");
        std::fs::create_dir(&agents_dir).unwrap();

        std::fs::write(
            dir.path().join("topology.json"),
            r#"{"agents": {"scanner": {"depends_on": []}, "analyzer": {"depends_on": ["scanner"]}}}"#,
        ).unwrap();
        std::fs::write(
            dir.path().join("config.json"),
            r#"{"name": "Security Audit", "description": "test"}"#,
        ).unwrap();
        std::fs::write(agents_dir.join("scanner.json"), "{}").unwrap();
        // analyzer.json missing

        let state = build_current_state(dir.path());
        assert!(state.contains("slug=\"scanner\""));
        assert!(state.contains("status=\"configured\""));
        assert!(state.contains("slug=\"analyzer\""));
        assert!(state.contains("status=\"missing\""));
        assert!(state.contains("name=\"Security Audit\""));
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
