//! Tests for tools endpoints

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::traits::MockServerRepo;
    use crate::db::AgentRow;
    use crate::server::api::ownership::{require_admin, verify_agent_ownership};
    use crate::server::api::AppError;
    use crate::server::auth::{AuthUser, Claims};
    use crate::types::UserId;

    fn make_auth(user_id: Uuid, is_admin: bool) -> AuthUser {
        AuthUser {
            user_id: UserId(user_id),
            claims: Claims {
                sub: user_id.to_string(),
                email: "test@test.com".to_string(),
                is_admin,
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
            default_reasoning_trace: None,
        }
    }

    #[test]
    fn require_admin_rejects_non_admin() {
        let auth = make_auth(Uuid::new_v4(), false);
        let result = require_admin(&auth);
        assert!(matches!(result.unwrap_err(), AppError::NotFound(_)));
    }

    #[test]
    fn require_admin_accepts_admin() {
        let auth = make_auth(Uuid::new_v4(), true);
        let result = require_admin(&auth);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn agent_tools_rejects_non_owner() {
        let owner_id = Uuid::new_v4();
        let attacker_id = Uuid::new_v4();
        let agent_id = Uuid::new_v4();
        let agent = make_agent(agent_id, owner_id);

        let mut repo = MockServerRepo::new();
        repo.expect_get_persisted_agent()
            .returning(move |_| Ok(Some(agent.clone())));

        let auth = make_auth(attacker_id, false);
        let result = verify_agent_ownership(&repo, &auth, agent_id).await;
        assert!(matches!(result.unwrap_err(), AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn agent_tools_accepts_owner() {
        let user_id = Uuid::new_v4();
        let agent_id = Uuid::new_v4();
        let agent = make_agent(agent_id, user_id);

        let mut repo = MockServerRepo::new();
        repo.expect_get_persisted_agent()
            .returning(move |_| Ok(Some(agent.clone())));

        let auth = make_auth(user_id, false);
        let result = verify_agent_ownership(&repo, &auth, agent_id).await;
        assert!(result.is_ok());
    }
}
