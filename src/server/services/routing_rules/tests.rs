#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use crate::db::traits::MockWorkflowRepo;
    use crate::db::{StepRoutingRuleRow, WorkflowRow, WorkflowStepRow};
    use crate::server::services::routing_rules::*;
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

    fn make_rule(step_id: Uuid) -> StepRoutingRuleRow {
        StepRoutingRuleRow {
            id: Uuid::new_v4(),
            workflow_step_id: step_id,
            label_value: "frontend".to_string(),
            description: Some("UI work".to_string()),
            agent_id: Uuid::new_v4(),
            display_order: 0,
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn create_rejects_empty_label() {
        let repo = MockWorkflowRepo::new();
        let result = create_routing_rule(
            &repo,
            CreateRoutingRuleInput {
                user_id: Uuid::new_v4(),
                workflow_id: Uuid::new_v4(),
                step_id: Uuid::new_v4(),
                label_value: "   ".to_string(),
                agent_id: Uuid::new_v4(),
                description: None,
                display_order: 0,
            },
        )
        .await;
        assert!(matches!(result, Err(ServiceError::Validation(_))));
    }

    #[tokio::test]
    async fn create_rejects_wrong_owner() {
        let owner = Uuid::new_v4();
        let attacker = Uuid::new_v4();
        let wf = make_workflow(owner);
        let wf_id = wf.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));

        let result = create_routing_rule(
            &repo,
            CreateRoutingRuleInput {
                user_id: attacker,
                workflow_id: wf_id,
                step_id: Uuid::new_v4(),
                label_value: "frontend".to_string(),
                agent_id: Uuid::new_v4(),
                description: None,
                display_order: 0,
            },
        )
        .await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }

    #[tokio::test]
    async fn create_succeeds_for_owner() {
        let owner = Uuid::new_v4();
        let wf = make_workflow(owner);
        let wf_id = wf.id;
        let step = make_step(wf_id);
        let step_id = step.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));
        repo.expect_get_step()
            .returning(move |_| Ok(Some(step.clone())));
        repo.expect_create_routing_rule()
            .returning(move |sid, label, agent_id, desc, order| {
                Ok(StepRoutingRuleRow {
                    id: Uuid::new_v4(),
                    workflow_step_id: sid,
                    label_value: label.to_string(),
                    description: desc,
                    agent_id,
                    display_order: order,
                    created_at: Utc::now(),
                })
            });

        let result = create_routing_rule(
            &repo,
            CreateRoutingRuleInput {
                user_id: owner,
                workflow_id: wf_id,
                step_id,
                label_value: "frontend".to_string(),
                agent_id: Uuid::new_v4(),
                description: Some("UI work".to_string()),
                display_order: 1,
            },
        )
        .await;
        let row = result.unwrap();
        assert_eq!(row.label_value, "frontend");
        assert_eq!(row.display_order, 1);
    }

    #[tokio::test]
    async fn list_returns_rules_for_owner() {
        let owner = Uuid::new_v4();
        let wf = make_workflow(owner);
        let wf_id = wf.id;
        let step = make_step(wf_id);
        let step_id = step.id;
        let rule = make_rule(step_id);

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));
        repo.expect_get_step()
            .returning(move |_| Ok(Some(step.clone())));
        repo.expect_get_step_routing_rules()
            .returning(move |_| Ok(vec![rule.clone()]));

        let result = list_routing_rules(&repo, owner, wf_id, step_id).await;
        let rows = result.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].label_value, "frontend");
    }

    #[tokio::test]
    async fn delete_succeeds_for_owner() {
        let owner = Uuid::new_v4();
        let wf = make_workflow(owner);
        let wf_id = wf.id;
        let step = make_step(wf_id);
        let step_id = step.id;
        let rule_id = Uuid::new_v4();

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));
        repo.expect_get_step()
            .returning(move |_| Ok(Some(step.clone())));
        repo.expect_delete_routing_rule().returning(|_| Ok(()));

        let result = delete_routing_rule(&repo, owner, wf_id, step_id, rule_id).await;
        assert!(result.is_ok());
    }
}
