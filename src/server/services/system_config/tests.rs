#[cfg(test)]
mod tests {
    use crate::db::traits::MockSystemConfigRepo;
    use crate::server::services::system_config::*;
    use crate::server::services::ServiceError;

    #[tokio::test]
    async fn upsert_rejects_empty_key() {
        let repo = MockSystemConfigRepo::new();
        let result = upsert_system_config(
            &repo,
            UpsertSystemConfigInput {
                config_type: "llm".to_string(),
                config_key: "  ".to_string(),
                config_value: serde_json::json!("value"),
                description: None,
                created_by: None,
            },
        )
        .await;
        assert!(matches!(result, Err(ServiceError::Validation(_))));
    }

    #[tokio::test]
    async fn upsert_rejects_empty_type() {
        let repo = MockSystemConfigRepo::new();
        let result = upsert_system_config(
            &repo,
            UpsertSystemConfigInput {
                config_type: "".to_string(),
                config_key: "model".to_string(),
                config_value: serde_json::json!("value"),
                description: None,
                created_by: None,
            },
        )
        .await;
        assert!(matches!(result, Err(ServiceError::Validation(_))));
    }
}
