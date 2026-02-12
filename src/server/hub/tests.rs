#[cfg(test)]
mod tests {
    //! Tests for hub module

    use crate::server::hub::ChatConfig;

    #[test]
    fn chat_config_default_has_sane_values() {
        let config = ChatConfig::default();
        assert!(config.system_prompt.is_empty());
        assert!(config.tool_names.is_empty());
        assert!(config.max_rounds > 0);
        assert!(config.context_budget > 0);
    }
}
