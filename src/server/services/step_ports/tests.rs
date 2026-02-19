#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::traits::MockWorkflowRepo;
    use crate::db::{StepInputRow, WorkflowRow, WorkflowStepRow};
    use crate::server::services::step_ports::*;
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
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn create_input_rejects_empty_port_name() {
        let repo = MockWorkflowRepo::new();
        let result = create_step_input(
            &repo,
            CreateStepInputInput {
                user_id: Uuid::new_v4(),
                workflow_id: Uuid::new_v4(),
                step_id: Uuid::new_v4(),
                port_name: "  ".to_string(),
                port_type: "string".to_string(),
                required: false,
                default_value: None,
                description: None,
                json_schema: None,
            },
        )
        .await;
        assert!(matches!(result, Err(ServiceError::Validation(_))));
    }

    #[tokio::test]
    async fn create_output_rejects_empty_json_path() {
        let repo = MockWorkflowRepo::new();
        let result = create_step_output(
            &repo,
            CreateStepOutputInput {
                user_id: Uuid::new_v4(),
                workflow_id: Uuid::new_v4(),
                step_id: Uuid::new_v4(),
                port_name: "out".to_string(),
                port_type: "string".to_string(),
                json_path: "".to_string(),
                description: None,
                json_schema: None,
            },
        )
        .await;
        assert!(matches!(result, Err(ServiceError::Validation(_))));
    }

    #[tokio::test]
    async fn create_input_rejects_wrong_owner() {
        let owner = Uuid::new_v4();
        let attacker = Uuid::new_v4();
        let wf = make_workflow(owner);
        let wf_id = wf.id;

        let mut repo = MockWorkflowRepo::new();
        repo.expect_get_workflow()
            .returning(move |_| Ok(Some(wf.clone())));

        let result = create_step_input(
            &repo,
            CreateStepInputInput {
                user_id: attacker,
                workflow_id: wf_id,
                step_id: Uuid::new_v4(),
                port_name: "input".to_string(),
                port_type: "string".to_string(),
                required: false,
                default_value: None,
                description: None,
                json_schema: None,
            },
        )
        .await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }

    #[tokio::test]
    async fn list_inputs_succeeds_for_owner() {
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
        repo.expect_get_step_inputs().returning(move |_| {
            Ok(vec![StepInputRow {
                id: Uuid::new_v4(),
                workflow_step_id: step_id,
                port_name: "data".to_string(),
                port_type: "string".to_string(),
                required: true,
                ..Default::default()
            }])
        });

        let rows = list_step_inputs(&repo, owner, wf_id, step_id)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].port_name, "data");
    }

    #[tokio::test]
    async fn delete_input_succeeds_for_owner() {
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
        repo.expect_delete_step_input().returning(|_| Ok(()));

        let result = delete_step_input(&repo, owner, wf_id, step_id, Uuid::new_v4()).await;
        assert!(result.is_ok());
    }
}
