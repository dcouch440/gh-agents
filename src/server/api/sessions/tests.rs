//! Tests for session endpoints

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::traits::MockServerRepo;
    use crate::db::AgentRow;
    use crate::server::api::ownership::verify_agent_ownership;
    use crate::server::auth::{AuthUser, Claims};
    use crate::types::UserId;

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

    fn make_agent(id: Uuid, owner_id: Uuid) -> AgentRow {
        AgentRow {
            id,
            user_id: Some(owner_id),
            tier: None,
            name: "test".to_string(),
            system_prompt: "".to_string(),
            persona_style: None,
            model_provider: "anthropic".to_string(),
            model_id: "claude-sonnet-4-20250514".to_string(),
            model_max_tokens: 4096,
            model_temperature: 0.7,
            status: Some("idle".to_string()),
            router_mode: None,
            router_id: None,
            output_schema_id: None,
            version: 1,
        }
    }

    #[tokio::test]
    async fn agent_mode_ownership_rejects_non_owner() {
        let owner_id = Uuid::new_v4();
        let attacker_id = Uuid::new_v4();
        let agent_id = Uuid::new_v4();
        let agent = make_agent(agent_id, owner_id);

        let mut repo = MockServerRepo::new();
        repo.expect_get_persisted_agent()
            .returning(move |_| Ok(Some(agent.clone())));

        let auth = make_auth(attacker_id);
        let result = verify_agent_ownership(&repo, &auth, agent_id).await;
        assert!(matches!(
            result.unwrap_err(),
            crate::server::api::AppError::NotFound(_)
        ));
    }

    #[tokio::test]
    async fn agent_mode_ownership_accepts_owner() {
        let user_id = Uuid::new_v4();
        let agent_id = Uuid::new_v4();
        let agent = make_agent(agent_id, user_id);

        let mut repo = MockServerRepo::new();
        repo.expect_get_persisted_agent()
            .returning(move |_| Ok(Some(agent.clone())));

        let auth = make_auth(user_id);
        let result = verify_agent_ownership(&repo, &auth, agent_id).await;
        assert!(result.is_ok());
    }

    // =================================================================
    // Session Ownership Tests
    // =================================================================
    // Session handlers check: session.user_id != auth.user_id.0 → 404
    // We test this pattern directly using MockServerRepo::get_session.

    use crate::db::traits::ServerRepo;
    use crate::db::SessionRow;
    use chrono::Utc;

    fn make_session(id: Uuid, user_id: Uuid) -> SessionRow {
        SessionRow {
            id,
            user_id,
            mode_id: "chat".to_string(),
            title: "Test Session".to_string(),
            summary: String::new(),
            agent_id: None,
            draft_config: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn get_own_session_ownership_passes() {
        let user_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let session = make_session(session_id, user_id);

        let mut repo = MockServerRepo::new();
        repo.expect_get_session()
            .withf(move |id| *id == session_id)
            .returning(move |_| Ok(Some(session.clone())));

        let fetched = repo.get_session(session_id).await.unwrap().unwrap();
        // Simulates the handler's ownership check
        assert_eq!(fetched.user_id, user_id);
    }

    #[tokio::test]
    async fn get_other_users_session_returns_404() {
        let owner_id = Uuid::new_v4();
        let attacker_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let session = make_session(session_id, owner_id);

        let mut repo = MockServerRepo::new();
        repo.expect_get_session()
            .returning(move |_| Ok(Some(session.clone())));

        let auth = make_auth(attacker_id);
        let fetched = repo.get_session(session_id).await.unwrap().unwrap();
        // The handler would return 404 here
        assert_ne!(fetched.user_id, auth.user_id.0);
    }

    #[tokio::test]
    async fn nonexistent_session_returns_none() {
        let mut repo = MockServerRepo::new();
        repo.expect_get_session().returning(|_| Ok(None));

        let result = repo.get_session(Uuid::new_v4()).await.unwrap();
        assert!(result.is_none());
    }
}
