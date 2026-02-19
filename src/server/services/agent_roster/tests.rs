#[cfg(test)]
mod tests {
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
            ..Default::default()
        }
    }

    fn make_step(workflow_id: Uuid) -> WorkflowStepRow {
        WorkflowStepRow {
            id: Uuid::new_v4(),
            workflow_id,
            name: Some("Workforce".to_string()),
            ..Default::default()
        }
    }

    fn make_brief(step_id: Uuid) -> TaskMissionBriefRow {
        TaskMissionBriefRow {
            id: Uuid::new_v4(),
            step_id,
            ..Default::default()
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
                    ..Default::default()
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
