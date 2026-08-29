#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::WorkflowStepRow;
    use crate::server::services::workflow_agent::file_reader::BOARD_SPEC_FILE;
    use crate::server::services::workflow_agent::project::{
        is_valid_slug, resolve_slug, write_board_spec,
    };

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

    // ── write_board_spec ───────────────────────────────────────────────
    //
    // DB → repo is a full overwrite, so this file is rewritten from the DB
    // before every turn. What it must never do is leave contracts on disk
    // that the board no longer holds.

    /// The spec reaches the agent through `cat`, so it has to land byte for
    /// byte — a schema that arrives reformatted has lost the alignment that
    /// made its types readable.
    #[test]
    fn a_spec_is_written_verbatim() {
        let dir = tempfile::tempdir().unwrap();
        let spec = "# Output\n\n  id     string\n  score  number  0.0 to 1.0\n";

        write_board_spec(dir.path(), spec).unwrap();

        let written = std::fs::read_to_string(dir.path().join(BOARD_SPEC_FILE)).unwrap();
        assert_eq!(written, spec);
    }

    /// A board whose spec was cleared must not keep serving the old one. The
    /// agent cannot tell a stale board.md from a current one.
    #[test]
    fn clearing_the_spec_removes_the_file() {
        let dir = tempfile::tempdir().unwrap();
        write_board_spec(dir.path(), "One rule.").unwrap();
        assert!(dir.path().join(BOARD_SPEC_FILE).exists());

        write_board_spec(dir.path(), "").unwrap();

        assert!(!dir.path().join(BOARD_SPEC_FILE).exists());
    }

    /// Whitespace is not a spec. A heredoc that wrote nothing but newlines
    /// would otherwise leave a file that reads as "this board has contracts".
    #[test]
    fn a_whitespace_spec_is_no_spec() {
        let dir = tempfile::tempdir().unwrap();
        write_board_spec(dir.path(), "One rule.").unwrap();

        write_board_spec(dir.path(), "\n\n   \n").unwrap();

        assert!(!dir.path().join(BOARD_SPEC_FILE).exists());
    }

    /// Most boards never have a spec, and projecting one must not fail
    /// because there was no file to remove.
    #[test]
    fn removing_an_absent_spec_is_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(write_board_spec(dir.path(), "").is_ok());
    }
}
