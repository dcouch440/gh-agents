#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use crate::db::traits::MockWorkflowRepo;
    use crate::db::{TaskAgentRosterRow, TaskMissionBriefRow, WorkflowRow, WorkflowStepRow};
    use crate::server::services::agent_roster::*;
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
            name: Some("Workforce".to_string()),
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

    fn make_brief(step_id: Uuid) -> TaskMissionBriefRow {
        TaskMissionBriefRow {
            id: Uuid::new_v4(),
            step_id,
            task_description: String::new(),
            available_capabilities: vec![],
            failure_mode: "fail_fast".to_string(),
            downstream_context: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
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

        let result = create_roster_agent(
            &repo,
            CreateRosterAgentInput {
                user_id: attacker,
                workflow_id: wf_id,
                step_id: Uuid::new_v4(),
                name: "Researcher".to_string(),
                role_description: "Research agent".to_string(),
                capabilities: vec![],
                execution_order: 0,
            },
        )
        .await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }

    #[tokio::test]
    async fn create_auto_creates_mission_brief() {
        let owner = Uuid::new_v4();
        let wf = make_workflow(owner);
        let wf_id = wf.id;
        let step = make_step(wf_id);
        let step_id = step.id;
        let brief = make_brief(step_id);
        let brief_id = brief.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));
        repo.expect_get_step()
            .returning(move |_| Ok(Some(step.clone())));
        repo.expect_upsert_mission_brief()
            .returning(move |_, _, _, _, _| Ok(brief.clone()));
        repo.expect_add_roster_agent()
            .returning(move |bid, name, role, caps, order| {
                Ok(TaskAgentRosterRow {
                    id: Uuid::new_v4(),
                    mission_brief_id: bid,
                    name: name.to_string(),
                    role_description: role.to_string(),
                    capabilities: caps.to_vec(),
                    execution_order: order,
                    created_at: Utc::now(),
                    child_step_id: None,
                })
            });

        let result = create_roster_agent(
            &repo,
            CreateRosterAgentInput {
                user_id: owner,
                workflow_id: wf_id,
                step_id,
                name: "Researcher".to_string(),
                role_description: "Research agent".to_string(),
                capabilities: vec!["search".to_string()],
                execution_order: 1,
            },
        )
        .await;
        let row = result.unwrap();
        assert_eq!(row.name, "Researcher");
        assert_eq!(row.mission_brief_id, brief_id);
    }

    #[tokio::test]
    async fn list_returns_empty_when_no_brief() {
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
        repo.expect_get_mission_brief().returning(|_| Ok(None));

        let result = list_roster_agents(&repo, owner, wf_id, step_id).await;
        let agents = result.unwrap();
        assert!(agents.is_empty());
    }
}
