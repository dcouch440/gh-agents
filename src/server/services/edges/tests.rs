#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use crate::db::traits::MockWorkflowRepo;
    use crate::db::{WorkflowRow, WorkflowStepEdgeRow, WorkflowStepRow};
    use crate::server::services::edges::*;
    use crate::server::services::ServiceError;

    fn make_workflow(user_id: Uuid) -> WorkflowRow {
        WorkflowRow {
            id: Uuid::new_v4(),
            user_id,
            name: "wf".to_string(),
            description: String::new(),
            execution_mode: "dag".to_string(),
            version: 1,
            container_enabled: false,
            target_repo_url: None,
            target_branch: None,
            vpn_enabled: false,
            created_at: Utc::now(),
            board_overview_summary: String::new(),
        }
    }

    fn make_step(workflow_id: Uuid, mode: &str) -> WorkflowStepRow {
        WorkflowStepRow {
            id: Uuid::new_v4(),
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
            name: Some("Step".to_string()),
            system_prompt_suffix: None,
            visible: true,
            description: String::new(),
            board_context_cache: String::new(),
            board_context_updated_at: None,
            goal_summary: String::new(),
            goal_summary_updated_at: None,
            sub_workflow_template_id: None,
            child_workflow_id: None,
            is_designer_step: false,
            pinned: false,
            run_results_summary: String::new(),
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
    async fn add_edge_rejects_context_node_target() {
        let owner = Uuid::new_v4();
        let wf = make_workflow(owner);
        let wf_id = wf.id;
        let from_step = make_step(wf_id, "single");
        let to_step = make_step(wf_id, "context");
        let from_id = from_step.id;
        let to_id = to_step.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));
        repo.expect_get_step().returning(move |id| {
            if id == to_id {
                Ok(Some(to_step.clone()))
            } else {
                Ok(Some(from_step.clone()))
            }
        });

        let result = add_edge(&repo, owner, wf_id, from_id, to_id).await;
        assert!(matches!(result, Err(ServiceError::Validation(_))));
    }

    #[tokio::test]
    async fn add_edge_succeeds_for_non_context_target() {
        let owner = Uuid::new_v4();
        let wf = make_workflow(owner);
        let wf_id = wf.id;
        let to_step = make_step(wf_id, "single");
        let from_id = Uuid::new_v4();
        let to_id = to_step.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));
        repo.expect_get_step()
            .returning(move |_| Ok(Some(to_step.clone())));
        repo.expect_add_edge()
            .returning(move |wid, fid, tid| Ok(make_edge(wid, fid, tid)));

        let result = add_edge(&repo, owner, wf_id, from_id, to_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_edges_rejects_wrong_owner() {
        let owner = Uuid::new_v4();
        let attacker = Uuid::new_v4();
        let wf = make_workflow(owner);
        let wf_id = wf.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));

        let result = list_edges(&repo, attacker, wf_id).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }

    #[tokio::test]
    async fn list_edges_succeeds_for_owner() {
        let owner = Uuid::new_v4();
        let wf = make_workflow(owner);
        let wf_id = wf.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));
        repo.expect_list_edges().returning(|_| Ok(vec![]));

        let result = list_edges(&repo, owner, wf_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn delete_edge_by_id_rejects_wrong_owner() {
        let owner = Uuid::new_v4();
        let attacker = Uuid::new_v4();
        let wf = make_workflow(owner);
        let wf_id = wf.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));

        let result = delete_edge_by_id(&repo, attacker, wf_id, Uuid::new_v4()).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }
}
