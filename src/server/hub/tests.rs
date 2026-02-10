#[cfg(test)]
mod tests {
    //! Tests for hub module

    use crate::db::AgentModeRow;
    use crate::server::hub::{apply_mode_overlay, ChatConfig};
    use chrono::Utc;
    use uuid::Uuid;

    fn make_mode(name: &str, hint: &str) -> AgentModeRow {
        AgentModeRow {
            id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            name: name.to_string(),
            system_prompt_suffix: None,
            temperature_override: None,
            model_override: None,
            tool_overrides: None,
            classifier_hint: hint.to_string(),
            created_at: Utc::now(),
            version: 1,
        }
    }

    #[test]
    fn apply_mode_overlay_prompt_suffix() {
        let mut config = ChatConfig {
            system_prompt: "Base prompt.".into(),
            ..Default::default()
        };
        let mut mode = make_mode("technical", "For technical questions");
        mode.system_prompt_suffix = Some("Be precise and technical.".into());

        apply_mode_overlay(&mut config, &mode);
        assert!(config.system_prompt.contains("Base prompt."));
        assert!(config.system_prompt.contains("Be precise and technical."));
    }

    #[test]
    fn apply_mode_overlay_temperature() {
        let mut config = ChatConfig {
            temperature: 0.7,
            ..Default::default()
        };
        let mut mode = make_mode("creative", "For creative writing");
        mode.temperature_override = Some(0.95);

        apply_mode_overlay(&mut config, &mode);
        assert!((config.temperature - 0.95).abs() < f32::EPSILON);
    }

    #[test]
    fn apply_mode_overlay_model() {
        let mut config = ChatConfig {
            model_id: "claude-3-haiku".into(),
            ..Default::default()
        };
        let mut mode = make_mode("deep", "For deep analysis");
        mode.model_override = Some("claude-sonnet-4-20250514".into());

        apply_mode_overlay(&mut config, &mode);
        assert_eq!(config.model_id, "claude-sonnet-4-20250514");
    }

    #[test]
    fn apply_mode_overlay_tools() {
        let mut config = ChatConfig {
            tool_names: vec!["think".into(), "search".into()],
            ..Default::default()
        };
        let mut mode = make_mode("code", "For coding tasks");
        mode.tool_overrides = Some(vec!["think".into(), "write_file".into(), "run_test".into()]);

        apply_mode_overlay(&mut config, &mode);
        assert_eq!(config.tool_names, vec!["think", "write_file", "run_test"]);
    }

    #[test]
    fn apply_mode_overlay_no_overrides() {
        let mut config = ChatConfig {
            system_prompt: "Original.".into(),
            model_id: "haiku".into(),
            temperature: 0.5,
            tool_names: vec!["think".into()],
            ..Default::default()
        };
        let mode = make_mode("plain", "No overrides");

        apply_mode_overlay(&mut config, &mode);
        assert_eq!(config.system_prompt, "Original.");
        assert_eq!(config.model_id, "haiku");
        assert!((config.temperature - 0.5).abs() < f32::EPSILON);
        assert_eq!(config.tool_names, vec!["think"]);
    }
}
