#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use crate::db::traits::MockDocumentRepo;
    use crate::db::DocumentRow;
    use crate::server::services::documents::*;
    use crate::server::services::ServiceError;

    fn make_document(user_id: Uuid) -> DocumentRow {
        DocumentRow {
            id: Uuid::new_v4(),
            user_id,
            session_id: None,
            title: "Test Document".to_string(),
            content: "Some content".to_string(),
            summary: None,
            doc_type: Some("architecture".to_string()),
            ref_tag: None,
            tags: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            workflow_id: None,
            target_length: None,
            is_static: None,
            source_protocol_step_id: None,
        }
    }

    #[tokio::test]
    async fn create_document_rejects_empty_title() {
        let repo = MockDocumentRepo::new();

        let result = create_document(
            &repo,
            CreateDocumentInput {
                user_id: Uuid::new_v4(),
                title: "   ".to_string(),
                content: "content".to_string(),
                doc_type: None,
                session_id: None,
                tags: None,
            },
        )
        .await;
        assert!(matches!(result, Err(ServiceError::Validation(_))));
    }

    #[tokio::test]
    async fn create_document_rejects_too_long_title() {
        let repo = MockDocumentRepo::new();
        let long_title = "a".repeat(crate::constants::MAX_TITLE_LENGTH + 1);

        let result = create_document(
            &repo,
            CreateDocumentInput {
                user_id: Uuid::new_v4(),
                title: long_title,
                content: "content".to_string(),
                doc_type: None,
                session_id: None,
                tags: None,
            },
        )
        .await;
        assert!(matches!(result, Err(ServiceError::Validation(_))));
    }

    #[tokio::test]
    async fn create_document_succeeds_with_valid_input() {
        let owner = Uuid::new_v4();
        let mut repo = MockDocumentRepo::new();
        repo.expect_create_document().returning(move |input| {
            Ok(DocumentRow {
                id: Uuid::new_v4(),
                user_id: input.user_id,
                session_id: input.session_id,
                title: input.title,
                content: input.content,
                summary: None,
                doc_type: Some(input.doc_type),
                ref_tag: Some(input.ref_tag),
                tags: Some(input.tags),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                workflow_id: None,
                target_length: None,
                is_static: None,
                source_protocol_step_id: None,
            })
        });

        let result = create_document(
            &repo,
            CreateDocumentInput {
                user_id: owner,
                title: "My Document".to_string(),
                content: "Hello world".to_string(),
                doc_type: None,
                session_id: None,
                tags: None,
            },
        )
        .await;
        let doc = result.unwrap();
        assert_eq!(doc.title, "My Document");
        assert_eq!(doc.user_id, owner);
    }

    #[tokio::test]
    async fn get_document_rejects_wrong_owner() {
        let owner = Uuid::new_v4();
        let attacker = Uuid::new_v4();
        let doc = make_document(owner);
        let doc_id = doc.id;

        let mut repo = MockDocumentRepo::new();
        repo.expect_get_document()
            .returning(move |_| Ok(Some(doc.clone())));

        let result = get_document(&repo, attacker, doc_id).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }

    #[tokio::test]
    async fn delete_document_rejects_wrong_owner() {
        let owner = Uuid::new_v4();
        let attacker = Uuid::new_v4();
        let doc = make_document(owner);
        let doc_id = doc.id;

        let mut repo = MockDocumentRepo::new();
        repo.expect_get_document()
            .returning(move |_| Ok(Some(doc.clone())));

        let result = delete_document(&repo, attacker, doc_id).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }
}
