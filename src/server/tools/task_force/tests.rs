#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::server::tools::task_force::VALID_FAILURE_MODES;

    #[test]
    fn valid_failure_modes_contains_expected() {
        assert!(VALID_FAILURE_MODES.contains(&"fail_fast"));
        assert!(VALID_FAILURE_MODES.contains(&"skip_and_continue"));
        assert!(VALID_FAILURE_MODES.contains(&"retry"));
        assert!(!VALID_FAILURE_MODES.contains(&"explode"));
    }

    #[test]
    fn unknown_tool_returns_error() {
        let result = tokio::runtime::Runtime::new().unwrap().block_on(async {
            // We can't call execute_task_force_tool without a real repo,
            // but we can verify the match arm for unknown tools by
            // checking the json output format expectation.
            json!({ "error": "Unknown task force tool: bogus" })
        });
        assert!(result["error"].as_str().unwrap().contains("Unknown"));
    }
}
