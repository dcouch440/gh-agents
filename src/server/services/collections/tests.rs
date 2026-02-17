#[cfg(test)]
mod tests {
    use chrono::Utc;
    use mockall::predicate::*;
    use uuid::Uuid;

    use crate::db::traits::MockWorkflowCollectionRepo;
    use crate::db::WorkflowCollectionRow;
    use crate::server::services::collections::*;
    use crate::server::services::ServiceError;

    fn make_collection_row(user_id: Uuid) -> WorkflowCollectionRow {
        WorkflowCollectionRow {
            id: Uuid::new_v4(),
            user_id,
            name: "Test Collection".to_string(),
            description: Some("A test collection".to_string()),
            execution_mode: "sequential".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn create_collection_rejects_invalid_execution_mode() {
        let repo = MockWorkflowCollectionRepo::new();
        let result = create_collection(
            &repo,
            Uuid::new_v4(),
            "My Collection".to_string(),
            None,
            "invalid_mode".to_string(),
        )
        .await;
        assert!(matches!(result, Err(ServiceError::Validation(_))));
    }

    #[tokio::test]
    async fn create_collection_accepts_sequential_mode() {
        let user_id = Uuid::new_v4();
        let expected = make_collection_row(user_id);
        let expected_clone = expected.clone();

        let mut repo = MockWorkflowCollectionRepo::new();
        repo.expect_create_collection()
            .withf(move |uid, name, _desc, mode| {
                *uid == user_id && name == "My Collection" && mode == "sequential"
            })
            .returning(move |_, _, _, _| Ok(expected_clone.clone()));

        let result = create_collection(
            &repo,
            user_id,
            "My Collection".to_string(),
            None,
            "sequential".to_string(),
        )
        .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, expected.id);
    }

    #[tokio::test]
    async fn create_collection_accepts_parallel_mode() {
        let user_id = Uuid::new_v4();
        let expected = make_collection_row(user_id);
        let expected_clone = expected.clone();

        let mut repo = MockWorkflowCollectionRepo::new();
        repo.expect_create_collection()
            .returning(move |_, _, _, _| Ok(expected_clone.clone()));

        let result = create_collection(
            &repo,
            user_id,
            "My Collection".to_string(),
            None,
            "parallel".to_string(),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn get_collection_returns_not_found_for_wrong_owner() {
        let owner_id = Uuid::new_v4();
        let other_user_id = Uuid::new_v4();
        let collection_id = Uuid::new_v4();

        let row = WorkflowCollectionRow {
            id: collection_id,
            user_id: owner_id,
            ..make_collection_row(owner_id)
        };

        let mut repo = MockWorkflowCollectionRepo::new();
        repo.expect_get_collection()
            .with(eq(collection_id))
            .returning(move |_| Ok(Some(row.clone())));

        let result = get_collection(&repo, other_user_id, collection_id).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }

    #[tokio::test]
    async fn delete_collection_rejects_wrong_owner() {
        let owner_id = Uuid::new_v4();
        let attacker_id = Uuid::new_v4();
        let collection_id = Uuid::new_v4();

        let row = WorkflowCollectionRow {
            id: collection_id,
            user_id: owner_id,
            ..make_collection_row(owner_id)
        };

        let mut repo = MockWorkflowCollectionRepo::new();
        repo.expect_get_collection()
            .with(eq(collection_id))
            .returning(move |_| Ok(Some(row.clone())));
        // delete_collection should NOT be called because ownership check fails
        repo.expect_delete_collection().never();

        let result = delete_collection(&repo, attacker_id, collection_id).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }
}
