#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use crate::db::traits::MockResultRepo;
    use crate::db::ResultRow;
    use crate::server::services::results::*;
    use crate::server::services::ServiceError;

    fn make_result(user_id: Uuid) -> ResultRow {
        ResultRow {
            id: Uuid::new_v4(),
            user_id,
            agent_execution_id: Uuid::new_v4(),
            output_schema_id: None,
            name: "test result".to_string(),
            data: serde_json::json!({"key": "value"}),
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn get_rejects_wrong_owner() {
        let owner = Uuid::new_v4();
        let attacker = Uuid::new_v4();
        let row = make_result(owner);
        let row_id = row.id;

        let mut repo = MockResultRepo::new();
        repo.expect_get_result()
            .returning(move |_| Ok(Some(row.clone())));

        let result = get_result(&repo, attacker, row_id).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }

    #[tokio::test]
    async fn get_succeeds_for_owner() {
        let owner = Uuid::new_v4();
        let row = make_result(owner);
        let row_id = row.id;

        let mut repo = MockResultRepo::new();
        repo.expect_get_result()
            .returning(move |_| Ok(Some(row.clone())));

        let result = get_result(&repo, owner, row_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn delete_rejects_wrong_owner() {
        let owner = Uuid::new_v4();
        let attacker = Uuid::new_v4();
        let row = make_result(owner);
        let row_id = row.id;

        let mut repo = MockResultRepo::new();
        repo.expect_get_result()
            .returning(move |_| Ok(Some(row.clone())));

        let result = delete_result(&repo, attacker, row_id).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }
}
