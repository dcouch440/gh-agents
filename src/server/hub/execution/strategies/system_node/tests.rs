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
        assert_eq!(cfg.max_rounds, 10);
        assert_eq!(cfg.context_budget, 480_000);
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
