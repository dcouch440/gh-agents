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
