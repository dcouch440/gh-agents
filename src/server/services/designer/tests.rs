#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::traits::MockWorkflowRepo;
    use crate::db::{AgentDesignerOutputRow, AgentDesignerRunRow, WorkflowRow, WorkflowStepRow};
    use crate::server::services::designer::get_latest_design;

    fn workforce_step(workflow_id: Uuid, step_id: Uuid) -> WorkflowStepRow {
        WorkflowStepRow {
            id: step_id,
            workflow_id,
            execution_mode: "workforce".to_string(),
            ..Default::default()
        }
    }

    fn workflow_row(user_id: Uuid, workflow_id: Uuid) -> WorkflowRow {
        WorkflowRow {
            id: workflow_id,
            user_id,
            ..Default::default()
        }
    }

    // ── get_latest_design tests ─────────────────────────────────────────

    #[tokio::test]
    async fn get_latest_design_returns_none_when_no_runs() {
        let user_id = Uuid::new_v4();
        let workflow_id = Uuid::new_v4();
        let step_id = Uuid::new_v4();

        let mut mock = MockWorkflowRepo::new();
        mock.expect_get_workflow()
            .returning(move |_| Ok(Some(workflow_row(user_id, workflow_id))));
        mock.expect_get_step()
            .returning(move |_| Ok(Some(workforce_step(workflow_id, step_id))));
        mock.expect_get_latest_designer_run_for_step()
            .returning(|_| Ok(None));

        let result = get_latest_design(&mock, workflow_id, step_id, user_id)
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn get_latest_design_returns_run_and_outputs() {
        let user_id = Uuid::new_v4();
        let workflow_id = Uuid::new_v4();
        let step_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();

        let mut mock = MockWorkflowRepo::new();
        mock.expect_get_workflow()
            .returning(move |_| Ok(Some(workflow_row(user_id, workflow_id))));
        mock.expect_get_step()
            .returning(move |_| Ok(Some(workforce_step(workflow_id, step_id))));

        let run = AgentDesignerRunRow {
            id: run_id,
            workflow_execution_id: Uuid::new_v4(),
            stage_execution_id: Uuid::new_v4(),
            step_id,
            mission_brief_id: None,
            archetype: "workforce".to_string(),
            phase: "standalone".to_string(),
            model_id: "tier:1".to_string(),
            input_tokens: 1000,
            output_tokens: 500,
            cost_usd: 0.02,
            created_at: chrono::Utc::now(),
        };
        let run_clone = run.clone();
        mock.expect_get_latest_designer_run_for_step()
            .returning(move |_| Ok(Some(run_clone.clone())));

        let output = AgentDesignerOutputRow {
            id: Uuid::new_v4(),
            designer_run_id: run_id,
            agent_roster_entry_id: None,
            agent_name: "Scanner".to_string(),
            assigned_tools: vec!["file_read".to_string()],
            generated_system_prompt: "You are Scanner...".to_string(),
            generated_task_prompt: "Scan the codebase.".to_string(),
            design_reasoning: "Identity framing for scanning".to_string(),
            execution_order: 0,
            source_entity_id: Uuid::new_v4().to_string(),
            source_archetype: "workforce".to_string(),
            protocol_execution_id: None,
            created_at: chrono::Utc::now(),
        };
        let output_clone = output.clone();
        mock.expect_list_designer_outputs()
            .returning(move |_| Ok(vec![output_clone.clone()]));

        let result = get_latest_design(&mock, workflow_id, step_id, user_id)
            .await
            .unwrap()
            .expect("should have a design");

        assert_eq!(result.run.id, run_id);
        assert_eq!(result.outputs.len(), 1);
        assert_eq!(result.outputs[0].agent_name, "Scanner");
    }

    #[tokio::test]
    async fn get_latest_design_wrong_user_returns_not_found() {
        let owner_id = Uuid::new_v4();
        let caller_id = Uuid::new_v4();
        let workflow_id = Uuid::new_v4();
        let step_id = Uuid::new_v4();

        let mut mock = MockWorkflowRepo::new();
        mock.expect_get_workflow()
            .returning(move |_| Ok(Some(workflow_row(owner_id, workflow_id))));

        let result = get_latest_design(&mock, workflow_id, step_id, caller_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn get_latest_design_step_wrong_workflow_returns_not_found() {
        let user_id = Uuid::new_v4();
        let workflow_id = Uuid::new_v4();
        let other_workflow_id = Uuid::new_v4();
        let step_id = Uuid::new_v4();

        let mut mock = MockWorkflowRepo::new();
        mock.expect_get_workflow()
            .returning(move |_| Ok(Some(workflow_row(user_id, workflow_id))));
        // Step belongs to a different workflow
        mock.expect_get_step()
            .returning(move |_| Ok(Some(workforce_step(other_workflow_id, step_id))));

        let result = get_latest_design(&mock, workflow_id, step_id, user_id).await;
        assert!(result.is_err());
    }
}
