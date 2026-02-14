#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::server::tools::exploration::execute_think;

    #[test]
    fn think_records_thought_length() {
        let input = json!({ "thought": "I need to plan my approach carefully." });
        let result = execute_think(&input);

        assert_eq!(result["thought_recorded"], true);
        assert_eq!(result["length"], 37);
    }

    #[test]
    fn think_handles_empty_thought() {
        let input = json!({});
        let result = execute_think(&input);

        assert_eq!(result["thought_recorded"], true);
        assert_eq!(result["length"], 0);
    }
}
