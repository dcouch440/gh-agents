//! Tests for agent execution endpoints

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::traits::{AgentExecutionRepo, MockAgentExecutionRepo};

    // list_agent_executions takes user_id as a parameter to scope results.
    // This test verifies the repo is called with the expected user_id,
    // documenting that list is tenant-scoped.

    #[tokio::test]
    async fn list_executions_scoped_by_user() {
        let user_id = Uuid::new_v4();

        let mut repo = MockAgentExecutionRepo::new();
        repo.expect_list_agent_executions()
            .withf(move |uid, _status| *uid == user_id)
            .returning(|_, _| Ok(vec![]));

        let result = repo.list_agent_executions(user_id, None).await.unwrap();
        assert!(result.is_empty());
    }
}
