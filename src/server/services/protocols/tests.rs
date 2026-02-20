#[cfg(test)]
mod tests {
    use mockall::predicate::*;
    use uuid::Uuid;

    use crate::db::fixtures::fixtures::*;
    use crate::db::traits::MockProtocolRepo;
    use crate::db::ProtocolRow;
    use crate::server::services::protocols::*;
    use crate::server::services::ServiceError;

    fn make_protocol_row() -> ProtocolRow {
        protocol_row()
    }

    // ====================================================================
    // validate_port_name tests
    // ====================================================================

    #[test]
    fn validate_port_name_accepts_valid_names() {
        assert!(validate_port_name("researcher").is_ok());
        assert!(validate_port_name("a").is_ok());
        assert!(validate_port_name("my_port_1").is_ok());
        assert!(validate_port_name("x99").is_ok());
    }

    #[test]
    fn validate_port_name_rejects_empty() {
        assert!(matches!(
            validate_port_name(""),
            Err(ServiceError::Validation(_))
        ));
    }

    #[test]
    fn validate_port_name_rejects_leading_digit() {
        assert!(matches!(
            validate_port_name("1port"),
            Err(ServiceError::Validation(_))
        ));
    }

    #[test]
    fn validate_port_name_rejects_uppercase() {
        assert!(matches!(
            validate_port_name("MyPort"),
            Err(ServiceError::Validation(_))
        ));
    }

    #[test]
    fn validate_port_name_rejects_too_long() {
        let long_name = "a".repeat(51);
        assert!(matches!(
            validate_port_name(&long_name),
            Err(ServiceError::Validation(_))
        ));
    }

    // ====================================================================
    // create_port tests
    // ====================================================================

    #[tokio::test]
    async fn create_port_rejects_empty_name() {
        let mut repo = MockProtocolRepo::new();
        // Validation fails before any repo call, so no expectations needed
        repo.expect_get_protocol().never();
        repo.expect_create_protocol_port().never();

        let result = create_port(
            &repo,
            Uuid::new_v4(),
            "".to_string(),
            None,
            Uuid::new_v4(),
            None,
        )
        .await;
        assert!(matches!(result, Err(ServiceError::Validation(_))));
    }

    #[tokio::test]
    async fn create_port_returns_not_found_for_missing_protocol() {
        let protocol_id = Uuid::new_v4();

        let mut repo = MockProtocolRepo::new();
        repo.expect_get_protocol()
            .with(eq(protocol_id))
            .returning(|_| Ok(None));
        repo.expect_create_protocol_port().never();

        let result = create_port(
            &repo,
            protocol_id,
            "researcher".to_string(),
            None,
            Uuid::new_v4(),
            None,
        )
        .await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }

    // ====================================================================
    // list_protocols tests
    // ====================================================================

    #[tokio::test]
    async fn list_protocols_returns_all() {
        let p1 = make_protocol_row();
        let p2 = make_protocol_row();
        let expected = vec![p1.clone(), p2.clone()];
        let expected_clone = expected.clone();

        let mut repo = MockProtocolRepo::new();
        repo.expect_list_protocols()
            .returning(move || Ok(expected_clone.clone()));

        let result = list_protocols(&repo).await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, expected[0].id);
        assert_eq!(result[1].id, expected[1].id);
    }

    #[tokio::test]
    async fn list_protocols_returns_empty_when_none() {
        let mut repo = MockProtocolRepo::new();
        repo.expect_list_protocols().returning(|| Ok(Vec::new()));

        let result = list_protocols(&repo).await.unwrap();
        assert!(result.is_empty());
    }
}
