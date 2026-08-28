#[cfg(test)]
mod tests {
    use serde_json::json;
    use uuid::Uuid;

    use crate::db::fixtures::fixtures::*;
    use crate::db::WorkflowStepRow;

    use crate::server::tools::shared::{classify_content_status, require_str, require_uuid};

    fn make_step(mode: &str) -> WorkflowStepRow {
        WorkflowStepRow {
            execution_mode: mode.to_string(),
            name: Some("Test Step".to_string()),
            description: "Test description".to_string(),
            ..step()
        }
    }

    #[test]
    fn classify_context_with_content_is_populated() {
        let mut step = make_step("context");
        step.prompt_template = "Some user-provided context here".to_string();

        let (status, preview, word_count) = classify_content_status(&step);

        assert_eq!(status, "populated");
        assert!(preview.is_some());
        assert_eq!(word_count, Some(4));
    }

    #[test]
    fn classify_context_without_content_is_empty() {
        let step = make_step("context");

        let (status, preview, word_count) = classify_content_status(&step);

        assert_eq!(status, "empty");
        assert!(preview.is_none());
        assert!(word_count.is_none());
    }

    #[test]
    fn classify_non_context_mode_is_pending() {
        for mode in &["single", "workforce", "container"] {
            let step = make_step(mode);

            let (status, preview, word_count) = classify_content_status(&step);

            assert_eq!(status, "pending", "mode={mode} should be pending");
            assert!(preview.is_none());
            assert!(word_count.is_none());
        }
    }

    #[test]
    fn classify_populated_preview_truncated_to_500_chars() {
        let mut step = make_step("context");
        step.prompt_template = "a".repeat(1000);

        let (status, preview, _) = classify_content_status(&step);

        assert_eq!(status, "populated");
        assert_eq!(preview.unwrap().len(), 500);
    }

    // =====================================================================
    // Parameter extraction helpers
    // =====================================================================

    #[test]
    fn require_str_returns_value_when_present() {
        let input = json!({ "name": "Alice" });
        assert_eq!(require_str(&input, "name").unwrap(), "Alice");
    }

    #[test]
    fn require_str_returns_error_when_missing() {
        let input = json!({});
        let err = require_str(&input, "name").unwrap_err();
        assert_eq!(err["error"], "Missing required parameter: name");
    }

    #[test]
    fn require_str_returns_error_when_wrong_type() {
        let input = json!({ "name": 42 });
        let err = require_str(&input, "name").unwrap_err();
        assert_eq!(err["error"], "Missing required parameter: name");
    }

    #[test]
    fn require_uuid_returns_value_when_valid() {
        let id = Uuid::new_v4();
        let input = json!({ "id": id.to_string() });
        assert_eq!(require_uuid(&input, "id").unwrap(), id);
    }

    #[test]
    fn require_uuid_returns_error_when_missing() {
        let input = json!({});
        let err = require_uuid(&input, "id").unwrap_err();
        assert_eq!(err["error"], "Missing required parameter: id");
    }

    #[test]
    fn require_uuid_returns_error_when_invalid() {
        let input = json!({ "id": "not-a-uuid" });
        let err = require_uuid(&input, "id").unwrap_err();
        assert_eq!(err["error"], "Invalid UUID: not-a-uuid");
    }
}

#[cfg(test)]
mod allowlist_tests {
    use super::super::*;

    #[test]
    fn none_allows_everything() {
        assert!(is_tool_allowed("run_command", None));
        assert!(is_tool_allowed("anything_at_all", None));
    }

    #[test]
    fn listed_tools_are_allowed() {
        let allowed = vec!["read_file".to_string(), "brave_search".to_string()];
        assert!(is_tool_allowed("read_file", Some(&allowed)));
        assert!(is_tool_allowed("brave_search", Some(&allowed)));
    }

    #[test]
    fn unlisted_tools_are_denied() {
        let allowed = vec!["read_file".to_string()];
        assert!(!is_tool_allowed("run_command", Some(&allowed)));
    }

    #[test]
    fn an_empty_allow_list_denies_everything() {
        // Distinct from None: an explicit empty list means "no tools".
        let allowed: Vec<String> = vec![];
        assert!(!is_tool_allowed("read_file", Some(&allowed)));
    }

    #[test]
    fn matching_is_exact_not_prefix() {
        let allowed = vec!["read_file".to_string()];
        assert!(!is_tool_allowed("read_file_secret", Some(&allowed)));
        assert!(!is_tool_allowed("read", Some(&allowed)));
    }

    #[test]
    fn not_allowed_error_is_a_tool_failure() {
        // The engine treats a non-null "error" key as a failure; the refusal
        // must keep that shape or the failure breaker never sees it.
        let v = tool_not_allowed_error("brave_search");
        assert!(v.get("error").is_some());
        assert!(v["error"].as_str().unwrap().contains("brave_search"));
    }
}
