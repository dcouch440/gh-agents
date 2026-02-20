#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::fixtures::fixtures::*;
    use crate::db::traits::{MockAgentRepo, MockSessionRepo};
    use crate::server::services::sessions::*;
    use crate::server::services::ServiceError;
    use crate::types::UserId;

    fn make_session(user_id: Uuid) -> SessionRow {
        session(user_id)
    }

    #[tokio::test]
    async fn create_session_applies_defaults() {
        let user_id = Uuid::new_v4();
        let session = make_session(user_id);
        let session_clone = session.clone();

        let mut repo = MockSessionRepo::new();
        repo.expect_create_session()
            .returning(|_, _, _, _, _, _| Ok(()));
        repo.expect_get_session()
            .returning(move |_| Ok(Some(session_clone.clone())));

        let agent_repo = MockAgentRepo::new();

        let result = create_session(
            &repo,
            &agent_repo,
            CreateSessionInput {
                user_id: UserId(user_id),
                mode_id: String::new(),
                agent_id: None,
                title: String::new(),
                draft_config: None,
            },
        )
        .await;
        let row = result.unwrap();
        assert_eq!(row.user_id, user_id);
    }

    #[tokio::test]
    async fn create_session_validates_agent_exists() {
        let user_id = Uuid::new_v4();
        let fake_agent = Uuid::new_v4();

        let repo = MockSessionRepo::new();
        let mut agent_repo = MockAgentRepo::new();
        agent_repo
            .expect_get_persisted_agent()
            .returning(|_| Ok(None));

        let result = create_session(
            &repo,
            &agent_repo,
            CreateSessionInput {
                user_id: UserId(user_id),
                mode_id: "home".to_string(),
                agent_id: Some(fake_agent),
                title: "Test".to_string(),
                draft_config: None,
            },
        )
        .await;
        assert!(matches!(result, Err(ServiceError::Validation(_))));
    }

    #[tokio::test]
    async fn get_session_rejects_non_owner() {
        let owner_id = Uuid::new_v4();
        let attacker_id = Uuid::new_v4();
        let session = make_session(owner_id);
        let session_id = session.id;
        let session_clone = session.clone();

        let mut repo = MockSessionRepo::new();
        repo.expect_get_session()
            .returning(move |_| Ok(Some(session_clone.clone())));

        let result = get_session(&repo, attacker_id, session_id).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }

    #[tokio::test]
    async fn verify_session_chat_rejects_empty_message() {
        let user_id = Uuid::new_v4();
        let repo = MockSessionRepo::new();

        let result = verify_session_chat(&repo, user_id, Uuid::new_v4(), "   ").await;
        assert!(matches!(result, Err(ServiceError::Validation(_))));
    }

    #[tokio::test]
    async fn verify_session_chat_rejects_wrong_owner() {
        let owner_id = Uuid::new_v4();
        let attacker_id = Uuid::new_v4();
        let session = make_session(owner_id);
        let session_id = session.id;
        let session_clone = session.clone();

        let mut repo = MockSessionRepo::new();
        repo.expect_get_session()
            .returning(move |_| Ok(Some(session_clone.clone())));

        let result = verify_session_chat(&repo, attacker_id, session_id, "Hello").await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }
}
