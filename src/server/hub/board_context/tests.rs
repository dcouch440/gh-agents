#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::traits::MockWorkflowRepo;
    use crate::db::{WorkflowStepEdgeRow, WorkflowStepRow};

    use super::super::renderer::render_board;

    fn make_step(id: Uuid, workflow_id: Uuid, mode: &str, name: &str) -> WorkflowStepRow {
        WorkflowStepRow {
            id,
            workflow_id,
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
            name: Some(name.to_string()),
            system_prompt_suffix: None,
            visible: true,
            description: String::new(),
            board_context_cache: String::new(),
            board_context_updated_at: None,
            goal_summary: String::new(),
            goal_summary_updated_at: None,
        }
    }

    fn make_edge(workflow_id: Uuid, from: Uuid, to: Uuid) -> WorkflowStepEdgeRow {
        WorkflowStepEdgeRow {
            id: Uuid::new_v4(),
            from_step_id: from,
            to_step_id: to,
            from_output_port: None,
            to_input_port: None,
            transform_jsonpath: None,
            condition_type: None,
            condition_value: None,
            edge_label: None,
            workflow_id,
        }
    }

    // =========================================================================
    // render_board
    // =========================================================================

    #[tokio::test]
    async fn render_board_includes_all_nodes() {
        let wid = Uuid::new_v4();
        let s1 = Uuid::new_v4();
        let s2 = Uuid::new_v4();
        let s3 = Uuid::new_v4();

        let mut step1 = make_step(s1, wid, "context", "Requirements");
        step1.description = "Q2 product requirements".to_string();

        let mut step2 = make_step(s2, wid, "documenter", "Doc Gen");
        step2.goal_summary = "Generate API specifications from product requirements".to_string();

        let step3 = make_step(s3, wid, "single", "Reviewer");

        let steps = vec![step1, step2, step3];
        let edges = vec![make_edge(wid, s1, s2), make_edge(wid, s2, s3)];

        let steps_clone = steps.clone();
        let edges_clone = edges.clone();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_list_steps()
            .returning(move |_| Ok(steps_clone.clone()));
        repo.expect_list_edges()
            .returning(move |_| Ok(edges_clone.clone()));

        let result = render_board(&repo, wid).await.unwrap();

        // Header
        assert!(result.contains("3 nodes"));
        assert!(result.contains("2 connections"));

        // All nodes present
        assert!(result.contains("[Node: Requirements] (context)"));
        assert!(result.contains("[Node: Doc Gen] (documenter)"));
        assert!(result.contains("[Node: Reviewer] (single)"));

        // Description
        assert!(result.contains("Q2 product requirements"));

        // Goal summary
        assert!(result.contains("Generate API specifications from product requirements"));

        // Default goal
        assert!(result.contains("(not yet established)"));

        // Connections
        assert!(result.contains("\u{2190} Requirements")); // Doc Gen incoming
        assert!(result.contains("\u{2192} Reviewer")); // Doc Gen outgoing
    }

    #[tokio::test]
    async fn render_board_empty_workflow() {
        let wid = Uuid::new_v4();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_list_steps().returning(|_| Ok(vec![]));
        repo.expect_list_edges().returning(|_| Ok(vec![]));

        let result = render_board(&repo, wid).await.unwrap();

        assert!(result.contains("0 nodes"));
        assert!(result.contains("0 connections"));
    }

    #[tokio::test]
    async fn render_board_truncates_long_descriptions() {
        let wid = Uuid::new_v4();
        let s1 = Uuid::new_v4();

        let mut step = make_step(s1, wid, "single", "Verbose");
        step.description = "a".repeat(300);

        let steps = vec![step];
        let steps_clone = steps.clone();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_list_steps()
            .returning(move |_| Ok(steps_clone.clone()));
        repo.expect_list_edges().returning(|_| Ok(vec![]));

        let result = render_board(&repo, wid).await.unwrap();

        assert!(result.contains("..."));
        assert!(!result.contains(&"a".repeat(300)));
    }

    #[tokio::test]
    async fn render_board_no_edges_omits_connections() {
        let wid = Uuid::new_v4();
        let s1 = Uuid::new_v4();

        let step = make_step(s1, wid, "single", "Solo");

        let steps = vec![step];
        let steps_clone = steps.clone();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_list_steps()
            .returning(move |_| Ok(steps_clone.clone()));
        repo.expect_list_edges().returning(|_| Ok(vec![]));

        let result = render_board(&repo, wid).await.unwrap();

        assert!(result.contains("[Node: Solo] (single)"));
        assert!(!result.contains("Connections:"));
    }

    #[tokio::test]
    async fn render_board_shows_goal_summaries() {
        let wid = Uuid::new_v4();
        let s1 = Uuid::new_v4();
        let s2 = Uuid::new_v4();

        let mut step1 = make_step(s1, wid, "task_force", "Research Team");
        step1.goal_summary =
            "Investigating agent behavior with emphasis on real data".to_string();

        let step2 = make_step(s2, wid, "room", "Review Board");

        let steps = vec![step1, step2];
        let steps_clone = steps.clone();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_list_steps()
            .returning(move |_| Ok(steps_clone.clone()));
        repo.expect_list_edges().returning(|_| Ok(vec![]));

        let result = render_board(&repo, wid).await.unwrap();

        assert!(result.contains("Investigating agent behavior with emphasis on real data"));
        assert!(result.contains("(not yet established)"));
    }
}
