#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::WorkflowStepRow;
    use crate::server::services::workflow_agent::project::{is_valid_slug, resolve_slug};

    // ── is_valid_slug ──────────────────────────────────────────────────

    #[test]
    fn valid_slugs() {
        assert!(is_valid_slug("research"));
        assert!(is_valid_slug("fact_checker"));
        assert!(is_valid_slug("a123"));
        assert!(is_valid_slug("unnamed_01"));
    }

    #[test]
    fn invalid_slugs() {
        assert!(!is_valid_slug(""));
        assert!(!is_valid_slug("123abc")); // starts with digit
        assert!(!is_valid_slug("Research")); // uppercase
        assert!(!is_valid_slug("workforce-1")); // contains hyphen
        assert!(!is_valid_slug("has space"));
    }

    // ── resolve_slug ───────────────────────────────────────────────────

    #[test]
    fn resolve_slug_uses_valid_ref_id() {
        let step = WorkflowStepRow {
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
            name: Some("Research".to_string()),
            system_prompt_suffix: None,
            visible: true,
            description: "desc".to_string(),
            board_context_cache: String::new(),
            board_context_updated_at: None,
            goal_summary: String::new(),
            goal_summary_updated_at: None,
            child_workflow_id: None,
            ref_id: Some("research".to_string()),
            pinned: false,
            run_results_summary: String::new(),
            designer_handoff: String::new(),
        };

        let (slug, changed) = resolve_slug(&step, &[]);
        assert_eq!(slug, "research");
        assert!(!changed);
    }

    #[test]
    fn resolve_slug_generates_from_name_for_old_ref_id() {
        let step = WorkflowStepRow {
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
            name: Some("Market Research".to_string()),
            system_prompt_suffix: None,
            visible: true,
            description: "desc".to_string(),
            board_context_cache: String::new(),
            board_context_updated_at: None,
            goal_summary: String::new(),
            goal_summary_updated_at: None,
            child_workflow_id: None,
            ref_id: Some("workforce-1".to_string()), // old format
            pinned: false,
            run_results_summary: String::new(),
            designer_handoff: String::new(),
        };

        let (slug, changed) = resolve_slug(&step, &[]);
        assert_eq!(slug, "market_research");
        assert!(changed);
    }

    #[test]
    fn resolve_slug_falls_back_to_unnamed() {
        let step = WorkflowStepRow {
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
            name: None, // no name
            system_prompt_suffix: None,
            visible: true,
            description: String::new(),
            board_context_cache: String::new(),
            board_context_updated_at: None,
            goal_summary: String::new(),
            goal_summary_updated_at: None,
            child_workflow_id: None,
            ref_id: None, // no ref_id
            pinned: false,
            run_results_summary: String::new(),
            designer_handoff: String::new(),
        };

        let (slug, changed) = resolve_slug(&step, &[]);
        assert_eq!(slug, "unnamed_01");
        assert!(changed);
    }
}
