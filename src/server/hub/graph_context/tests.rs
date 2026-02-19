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
            execution_mode: mode.to_string(),
            name: Some(name.to_string()),
            ..Default::default()
        }
    }

    fn make_edge(workflow_id: Uuid, from: Uuid, to: Uuid) -> WorkflowStepEdgeRow {
        WorkflowStepEdgeRow {
            id: Uuid::new_v4(),
            workflow_id,
            from_step_id: from,
            to_step_id: to,
            ..Default::default()
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
