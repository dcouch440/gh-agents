#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::WorkflowStepRow;

    use crate::server::tools::shared::classify_content_status;

    fn make_step(mode: &str) -> WorkflowStepRow {
        WorkflowStepRow {
            id: Uuid::new_v4(),
            workflow_id: Uuid::new_v4(),
            execution_mode: mode.to_string(),
            name: Some("Test Step".to_string()),
            description: "Test description".to_string(),
            ..Default::default()
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
        for mode in &["single", "for_each", "documenter", "room"] {
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
}
