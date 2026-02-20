#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::fixtures::fixtures::*;
    use crate::db::traits::MockAgentExecutionRepo;
    use crate::db::AgentExecutionRow;
    use crate::server::services::agent_executions::*;
    use crate::server::services::ServiceError;

    fn make_execution(is_interactive: bool, status: &str) -> AgentExecutionRow {
        AgentExecutionRow {
            workflow_step_id: Some(Uuid::new_v4()),
            is_interactive,
            status: status.to_string(),
            input: "test input".to_string(),
            output: Some("test output".to_string()),
            ..agent_execution()
        }
    }

    #[tokio::test]
    async fn get_returns_not_found() {
        let mut repo = MockAgentExecutionRepo::new();
        repo.expect_get_agent_execution().returning(|_| Ok(None));

        let result = get_agent_execution(&repo, Uuid::new_v4()).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }

    #[tokio::test]
    async fn approve_rejects_non_interactive() {
        let ae = make_execution(false, "completed");
        let ae_id = ae.id;
        let mut repo = MockAgentExecutionRepo::new();
        repo.expect_get_agent_execution()
            .returning(move |_| Ok(Some(ae.clone())));

        let result = approve_execution(&repo, ae_id, None).await;
        assert!(matches!(result, Err(ServiceError::Validation(_))));
    }

    #[tokio::test]
    async fn approve_rejects_wrong_status() {
        let ae = make_execution(true, "completed");
        let ae_id = ae.id;
        let mut repo = MockAgentExecutionRepo::new();
        repo.expect_get_agent_execution()
            .returning(move |_| Ok(Some(ae.clone())));

        let result = approve_execution(&repo, ae_id, None).await;
        assert!(matches!(result, Err(ServiceError::Validation(_))));
    }

    #[tokio::test]
    async fn approve_succeeds_and_signals_resume() {
        let ae = make_execution(true, "awaiting_user");
        let ae_id = ae.id;
        let step_id = ae.workflow_step_id.unwrap();
        let ae_clone = ae.clone();

        let mut completed_ae = ae.clone();
        completed_ae.status = "completed".to_string();
        let completed_clone = completed_ae.clone();

        let mut repo = MockAgentExecutionRepo::new();
        repo.expect_get_agent_execution()
            .returning(move |_| Ok(Some(ae_clone.clone())));
        repo.expect_update_agent_execution_status()
            .returning(move |_, _, _, _| Ok(completed_clone.clone()));
        repo.expect_list_interactive_executions_for_step()
            .returning(move |_| {
                Ok(vec![AgentExecutionRow {
                    status: "completed".to_string(),
                    ..completed_ae.clone()
                }])
            });

        let result = approve_execution(&repo, ae_id, None).await;
        let approval = result.unwrap();
        assert_eq!(approval.execution.status, "completed");
        assert_eq!(approval.resume_step_id, Some(step_id));
    }

    #[tokio::test]
    async fn set_exemplary_returns_not_found() {
        let mut repo = MockAgentExecutionRepo::new();
        repo.expect_get_agent_execution().returning(|_| Ok(None));

        let result = set_exemplary(&repo, Uuid::new_v4(), true).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }
}
