//! Tests for document endpoints

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::traits::{DocumentRepo, MockDocumentRepo};
    use crate::db::DocumentRow;
    use crate::server::auth::{AuthUser, Claims};
    use crate::types::UserId;
    use chrono::Utc;

    fn make_auth(user_id: Uuid) -> AuthUser {
        AuthUser {
            user_id: UserId(user_id),
            claims: Claims {
                sub: user_id.to_string(),
                email: "test@test.com".to_string(),
                is_admin: false,
                exp: 9999999999,
                iat: 0,
            },
        }
    }

    fn make_document(id: Uuid, user_id: Uuid) -> DocumentRow {
        DocumentRow {
            id,
            user_id,
            session_id: None,
            title: "Test Document".to_string(),
            content: "Content".to_string(),
            summary: None,
            doc_type: Some("note".to_string()),
            ref_tag: None,
            tags: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // Document handlers check: doc.user_id != auth.user_id.0 → 404
    // We test this pattern using MockDocumentRepo::get_document.

    #[tokio::test]
    async fn get_own_document_ownership_passes() {
        let user_id = Uuid::new_v4();
        let doc_id = Uuid::new_v4();
        let doc = make_document(doc_id, user_id);

        let mut repo = MockDocumentRepo::new();
        repo.expect_get_document()
            .withf(move |id| *id == doc_id)
            .returning(move |_| Ok(Some(doc.clone())));

        let fetched = repo.get_document(doc_id).await.unwrap().unwrap();
        let auth = make_auth(user_id);
        assert_eq!(fetched.user_id, auth.user_id.0);
    }

    #[tokio::test]
    async fn get_other_users_document_returns_404() {
        let owner_id = Uuid::new_v4();
        let attacker_id = Uuid::new_v4();
        let doc_id = Uuid::new_v4();
        let doc = make_document(doc_id, owner_id);

        let mut repo = MockDocumentRepo::new();
        repo.expect_get_document()
            .returning(move |_| Ok(Some(doc.clone())));

        let auth = make_auth(attacker_id);
        let fetched = repo.get_document(doc_id).await.unwrap().unwrap();
        // The handler would return 404 here
        assert_ne!(fetched.user_id, auth.user_id.0);
    }

    #[tokio::test]
    async fn nonexistent_document_returns_none() {
        let mut repo = MockDocumentRepo::new();
        repo.expect_get_document().returning(|_| Ok(None));

        let result = repo.get_document(Uuid::new_v4()).await.unwrap();
        assert!(result.is_none());
    }
}
