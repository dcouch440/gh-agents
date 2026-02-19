#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use crate::db::traits::MockWorkflowRepo;
    use crate::db::{WorkflowRow, WorkflowStepRow};
    use crate::server::services::steps::*;
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

    fn make_step(workflow_id: Uuid) -> WorkflowStepRow {
        WorkflowStepRow {
            id: Uuid::new_v4(),
            workflow_id,
            agent_id: None,
            execution_mode: "single".to_string(),
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
            name: None,
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

    // ── verify_step_access ────────────────────────────────────────────

    #[tokio::test]
    async fn verify_access_rejects_wrong_owner() {
        let owner = Uuid::new_v4();
        let attacker = Uuid::new_v4();
        let wf = make_workflow(owner);
        let wf_id = wf.id;
        let step = make_step(wf_id);
        let step_id = step.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));

        let result = verify_step_access(&repo, attacker, wf_id, step_id).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }

    #[tokio::test]
    async fn verify_access_rejects_step_not_in_workflow() {
        let owner = Uuid::new_v4();
        let wf = make_workflow(owner);
        let wf_id = wf.id;
        let other_workflow_id = Uuid::new_v4();
        let step = make_step(other_workflow_id); // step belongs to a different workflow
        let step_id = step.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));
        repo.expect_get_step()
            .returning(move |_| Ok(Some(step.clone())));

        let result = verify_step_access(&repo, owner, wf_id, step_id).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }

    // ── create_step ───────────────────────────────────────────────────

    #[tokio::test]
    async fn create_applies_defaults() {
        let owner = Uuid::new_v4();
        let wf = make_workflow(owner);
        let wf_id = wf.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));
        repo.expect_list_steps().returning(|_| Ok(vec![]));
        repo.expect_create_step().returning(|step| Ok(step));

        let result = create_step(
            &repo,
            CreateStepInput {
                workflow_id: wf_id,
                user_id: owner,
                agent_id: None,
                execution_mode: None,
                for_each_ref: None,
                prompt_template_id: None,
                prompt_template: None,
                output_schema_id: None,
                output_variable_name: None,
                interactive_agent_id: None,
                for_each_label_field: None,
                display_order: None,
                reasoning_trace: None,
                verification_agent_ids: None,
                position_x: None,
                position_y: None,
                width: None,
                height: None,
                name: None,
                system_prompt_suffix: None,
                description: None,
                sub_workflow_template_id: None,
            },
        )
        .await;
        let step = result.unwrap();
        assert_eq!(step.execution_mode, "single");
        assert_eq!(step.agent_id, Some(crate::constants::DEFAULT_AGENT_ID));
        assert!(!step.reasoning_trace);
        assert_eq!(step.display_order, 0);
    }

    #[tokio::test]
    async fn create_context_step_clears_agent_id() {
        let owner = Uuid::new_v4();
        let wf = make_workflow(owner);
        let wf_id = wf.id;
        let explicit_agent = Uuid::new_v4();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));
        repo.expect_list_steps().returning(|_| Ok(vec![]));
        repo.expect_create_step().returning(|step| Ok(step));

        let result = create_step(
            &repo,
            CreateStepInput {
                workflow_id: wf_id,
                user_id: owner,
                agent_id: Some(explicit_agent),
                execution_mode: Some("context".to_string()),
                for_each_ref: None,
                prompt_template_id: None,
                prompt_template: None,
                output_schema_id: None,
                output_variable_name: None,
                interactive_agent_id: None,
                for_each_label_field: None,
                display_order: None,
                reasoning_trace: None,
                verification_agent_ids: None,
                position_x: None,
                position_y: None,
                width: None,
                height: None,
                name: None,
                system_prompt_suffix: None,
                description: None,
                sub_workflow_template_id: None,
            },
        )
        .await;
        let step = result.unwrap();
        assert_eq!(step.execution_mode, "context");
        assert_eq!(step.agent_id, None);
    }

    // ── get_step ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn get_rejects_wrong_owner() {
        let owner = Uuid::new_v4();
        let attacker = Uuid::new_v4();
        let wf = make_workflow(owner);
        let wf_id = wf.id;
        let step = make_step(wf_id);
        let step_id = step.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));

        let result = get_step(&repo, attacker, wf_id, step_id).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }
}
