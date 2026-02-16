#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::WorkflowStepRow;

    use crate::server::tools::shared::classify_content_status;

    fn make_step(mode: &str) -> WorkflowStepRow {
        WorkflowStepRow {
            id: Uuid::new_v4(),
            workflow_id: Uuid::new_v4(),
            agent_id: None,
            execution_mode: mode.to_string(),
            agent_execution_mode: None,
            for_each_ref: None,
            prompt_template_id: None,
            prompt_template: String::new(),
            output_schema_id: None,
            output_variable_name: None,
            interactive_agent_id: None,
            for_each_label_field: None,
            room_id: None,
            routing_mode: None,
            routing_field: None,
            display_order: 0,
            version: 1,
            reasoning_trace: false,
            verification_agent_ids: None,
            position_x: None,
            position_y: None,
            width: None,
            height: None,
            name: Some("Test Step".to_string()),
            system_prompt_suffix: None,
            visible: true,
            description: "Test description".to_string(),
            board_context_cache: String::new(),
            board_context_updated_at: None,
            goal_summary: String::new(),
            goal_summary_updated_at: None,
            sub_workflow_template_id: None,
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
