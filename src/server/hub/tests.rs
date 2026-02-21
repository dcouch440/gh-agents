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

    // ========================================================================
    // truncate_str
    // ========================================================================

    #[test]
    fn truncate_str_short_input() {
        assert_eq!(crate::server::hub::truncate_str("hello", 10), "hello");
    }

    #[test]
    fn truncate_str_exact_length() {
        assert_eq!(crate::server::hub::truncate_str("hello", 5), "hello");
    }

    #[test]
    fn truncate_str_long_input() {
        let result = crate::server::hub::truncate_str("hello world", 5);
        assert_eq!(result, "hello");
    }
}
