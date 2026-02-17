#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::traits::MockServerRepo;
    use crate::server::services::agent_context::*;
    use crate::server::services::ServiceError;

    #[tokio::test]
    async fn set_rejects_invalid_uuid() {
        let repo = MockServerRepo::new();
        let result = set_agent_context(
            &repo,
            SetAgentContextInput {
                agent_id: Uuid::new_v4(),
                document_ids: vec!["not-a-uuid".to_string()],
            },
        )
        .await;
        assert!(matches!(result, Err(ServiceError::Validation(_))));
    }

    #[tokio::test]
    async fn get_returns_empty_list() {
        let agent_id = Uuid::new_v4();
        let mut repo = MockServerRepo::new();
        repo.expect_get_agent_context().returning(|_| Ok(vec![]));

        let result = get_agent_context(&repo, agent_id).await;
        assert!(result.unwrap().is_empty());
    }
}
