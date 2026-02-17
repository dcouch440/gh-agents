#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use crate::db::traits::MockOutputSchemaRepo;
    use crate::db::OutputSchemaRow;
    use crate::server::services::output_schemas::*;
    use crate::server::services::ServiceError;

    fn make_schema(user_id: Option<Uuid>) -> OutputSchemaRow {
        OutputSchemaRow {
            id: Uuid::new_v4(),
            user_id,
            name: "Test Schema".to_string(),
            schema: serde_json::json!({"type": "object"}),
            created_at: Utc::now(),
            version: 1,
        }
    }

    #[tokio::test]
    async fn create_output_schema_rejects_empty_name() {
        let repo = MockOutputSchemaRepo::new();

        let result = create_output_schema(
            &repo,
            Uuid::new_v4(),
            "   ".to_string(),
            serde_json::json!({}),
        )
        .await;
        assert!(matches!(result, Err(ServiceError::Validation(_))));
    }

    #[tokio::test]
    async fn create_output_schema_succeeds_with_valid_name() {
        let owner = Uuid::new_v4();
        let mut repo = MockOutputSchemaRepo::new();
        repo.expect_create_output_schema()
            .returning(|uid, name, schema| {
                Ok(OutputSchemaRow {
                    id: Uuid::new_v4(),
                    user_id: uid,
                    name,
                    schema,
                    created_at: Utc::now(),
                    version: 1,
                })
            });

        let result = create_output_schema(
            &repo,
            owner,
            "My Schema".to_string(),
            serde_json::json!({"type": "object"}),
        )
        .await;
        let row = result.unwrap();
        assert_eq!(row.name, "My Schema");
        assert_eq!(row.user_id, Some(owner));
    }

    #[tokio::test]
    async fn get_output_schema_returns_system_schema_to_any_user() {
        let system_schema = make_schema(None);
        let schema_id = system_schema.id;

        let mut repo = MockOutputSchemaRepo::new();
        repo.expect_get_output_schema()
            .returning(move |_| Ok(Some(system_schema.clone())));

        let result = get_output_schema(&repo, Uuid::new_v4(), schema_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().user_id.is_none());
    }

    #[tokio::test]
    async fn get_output_schema_rejects_wrong_owner() {
        let owner = Uuid::new_v4();
        let attacker = Uuid::new_v4();
        let schema = make_schema(Some(owner));
        let schema_id = schema.id;

        let mut repo = MockOutputSchemaRepo::new();
        repo.expect_get_output_schema()
            .returning(move |_| Ok(Some(schema.clone())));

        let result = get_output_schema(&repo, attacker, schema_id).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }

    #[tokio::test]
    async fn update_output_schema_rejects_system_schema() {
        let system_schema = make_schema(None);
        let schema_id = system_schema.id;

        let mut repo = MockOutputSchemaRepo::new();
        repo.expect_get_output_schema()
            .returning(move |_| Ok(Some(system_schema.clone())));

        let result = update_output_schema(
            &repo,
            Uuid::new_v4(),
            schema_id,
            Some("New Name".to_string()),
            None,
        )
        .await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }
}
