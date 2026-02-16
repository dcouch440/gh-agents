#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::traits::MockWorkflowRepo;
    use crate::db::{WorkflowStepEdgeRow, WorkflowStepRow};

    use super::super::build_graph_context;

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
            width: None,
            height: None,
            name: Some(name.to_string()),
            system_prompt_suffix: None,
            visible: true,
            description: String::new(),
            board_context_cache: String::new(),
            board_context_updated_at: None,
            goal_summary: String::new(),
            goal_summary_updated_at: None,
            sub_workflow_template_id: None,
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

    #[tokio::test]
    async fn formats_graph_with_selected_node() {
        let wid = Uuid::new_v4();
        let s1 = Uuid::new_v4();
        let s2 = Uuid::new_v4();
        let s3 = Uuid::new_v4();

        let steps = vec![
            make_step(s1, wid, "context", "Requirements"),
            make_step(s2, wid, "documenter", "Doc Gen"),
            make_step(s3, wid, "single", "Reviewer"),
        ];
        let edges = vec![make_edge(wid, s1, s2), make_edge(wid, s2, s3)];

        let steps_clone = steps.clone();
        let edges_clone = edges.clone();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_list_steps()
            .returning(move |_| Ok(steps_clone.clone()));
        repo.expect_list_edges()
            .returning(move |_| Ok(edges_clone.clone()));

        let result = build_graph_context(&repo, wid, s2).await.unwrap();

        assert!(result.contains("Requirements (context)"));
        assert!(result.contains("Doc Gen (documenter) [SELECTED]"));
        assert!(result.contains("Reviewer (single)"));
        assert!(result.contains("Requirements -> Doc Gen"));
        assert!(result.contains("Doc Gen -> Reviewer"));
    }

    #[tokio::test]
    async fn handles_no_edges() {
        let wid = Uuid::new_v4();
        let s1 = Uuid::new_v4();

        let steps = vec![make_step(s1, wid, "single", "Lone Node")];

        let steps_clone = steps.clone();
        let mut repo = MockWorkflowRepo::new();
        repo.expect_list_steps()
            .returning(move |_| Ok(steps_clone.clone()));
        repo.expect_list_edges().returning(|_| Ok(vec![]));

        let result = build_graph_context(&repo, wid, s1).await.unwrap();

        assert!(result.contains("Lone Node (single) [SELECTED]"));
        assert!(!result.contains("Connections:"));
    }

    #[tokio::test]
    async fn truncates_long_descriptions() {
        let wid = Uuid::new_v4();
        let s1 = Uuid::new_v4();

        let mut step = make_step(s1, wid, "single", "Verbose");
        step.description = "a".repeat(200);

        let steps = vec![step];
        let steps_clone = steps.clone();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_list_steps()
            .returning(move |_| Ok(steps_clone.clone()));
        repo.expect_list_edges().returning(|_| Ok(vec![]));

        let result = build_graph_context(&repo, wid, s1).await.unwrap();

        assert!(result.contains("..."));
        assert!(!result.contains(&"a".repeat(200)));
    }
}
