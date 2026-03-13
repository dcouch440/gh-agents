#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::WorkflowStepRow;
    use crate::server::tools::manager::resolve::check_name_unique;

    fn make_step(name: Option<&str>, ref_id: Option<&str>) -> WorkflowStepRow {
        WorkflowStepRow {
            id: Uuid::new_v4(),
            workflow_id: Uuid::new_v4(),
            agent_id: None,
            execution_mode: "workforce".to_string(),
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
            name: name.map(|s| s.to_string()),
            system_prompt_suffix: None,
            visible: true,
            description: String::new(),
            board_context_cache: String::new(),
            board_context_updated_at: None,
            goal_summary: String::new(),
            goal_summary_updated_at: None,

            child_workflow_id: None,
            ref_id: ref_id.map(|s| s.to_string()),
            pinned: false,
            run_results_summary: String::new(),
        }
    }

    #[test]
    fn check_name_unique_passes_for_new_name() {
        let steps = vec![
            make_step(Some("Collector"), Some("workforce-1")),
            make_step(Some("Analyzer"), Some("workforce-2")),
        ];
        assert!(check_name_unique(&steps, "Reporter").is_ok());
    }

    #[test]
    fn check_name_unique_rejects_duplicate_case_insensitive() {
        let steps = vec![
            make_step(Some("Collector"), Some("workforce-1")),
            make_step(Some("Analyzer"), Some("workforce-2")),
        ];
        let err = check_name_unique(&steps, "collector").unwrap_err();
        assert!(err.contains("already exists"));
        assert!(err.contains("workforce-1"));
    }

    #[test]
    fn check_name_unique_passes_when_no_names_set() {
        let steps = vec![make_step(None, Some("workforce-1"))];
        assert!(check_name_unique(&steps, "Collector").is_ok());
    }

    #[test]
    fn check_name_unique_rejects_exact_match() {
        let steps = vec![make_step(Some("Reporter"), Some("workforce-3"))];
        let err = check_name_unique(&steps, "Reporter").unwrap_err();
        assert!(err.contains("Reporter"));
    }
}
