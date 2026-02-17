#[cfg(test)]
mod tests {
    //! Tests for agents endpoints

    use super::super::*;

    fn test_agent_response() -> AgentResponse {
        AgentResponse {
            id: "agent-1".to_string(),
            name: "Test Agent".to_string(),
            system_prompt: "You are a test agent".to_string(),
            persona_style: "casual".to_string(),
            model_provider: "anthropic".to_string(),
            model_id: "claude-sonnet-4-20250514".to_string(),
            model_max_tokens: 4096,
            model_temperature: 0.7,
            status: "idle".to_string(),
            output_schema_id: None,
            version: 1,
            default_reasoning_trace: false,
            is_system: false,
        }
    }

    #[test]
    fn agent_pool_stats_serializes() {
        let stats = AgentPoolStats {
            total: 6,
            available: 5,
            max: 12,
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"total\""));
        assert!(json.contains("\"available\""));
        assert!(json.contains("\"max\""));
    }

    #[test]
    fn agents_list_response_serializes() {
        let response = AgentsListResponse {
            agents: vec![test_agent_response()],
            stats: AgentPoolStats {
                total: 1,
                available: 1,
                max: 12,
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"agent-1\""));
        assert!(json.contains("\"total\""));
    }

    #[test]
    fn agent_response_serializes_all_fields() {
        let response = test_agent_response();
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"name\":\"Test Agent\""));
        assert!(json.contains("\"model_provider\":\"anthropic\""));
        assert!(json.contains("\"model_max_tokens\":4096"));
    }

    mod ownership_tests {
        use uuid::Uuid;

        use crate::db::traits::MockServerRepo;
        use crate::db::AgentRow;
        use crate::server::api::ownership::verify_agent_ownership;
        use crate::server::api::AppError;
        use crate::server::auth::{AuthUser, Claims};
        use crate::types::UserId;

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

                output_schema_id: None,
                version: 1,
                default_reasoning_trace: None,
                is_system: false,
            }
        }

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

        #[tokio::test]
        async fn get_own_agent_succeeds() {
            let user_id = Uuid::new_v4();
            let agent_id = Uuid::new_v4();
            let agent = make_agent(agent_id, user_id);

            let mut repo = MockServerRepo::new();
            repo.expect_get_persisted_agent()
                .withf(move |id| *id == agent_id)
                .returning(move |_| Ok(Some(agent.clone())));

            let auth = make_auth(user_id);
            let result = verify_agent_ownership(&repo, &auth, agent_id).await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap().id, agent_id);
        }

        #[tokio::test]
        async fn get_other_users_agent_returns_404() {
            let owner_id = Uuid::new_v4();
            let attacker_id = Uuid::new_v4();
            let agent_id = Uuid::new_v4();
            let agent = make_agent(agent_id, owner_id);

            let mut repo = MockServerRepo::new();
            repo.expect_get_persisted_agent()
                .returning(move |_| Ok(Some(agent.clone())));

            let auth = make_auth(attacker_id);
            let result = verify_agent_ownership(&repo, &auth, agent_id).await;
            assert!(matches!(result.unwrap_err(), AppError::NotFound(_)));
        }

        #[tokio::test]
        async fn nonexistent_agent_returns_404() {
            let mut repo = MockServerRepo::new();
            repo.expect_get_persisted_agent().returning(|_| Ok(None));

            let auth = make_auth(Uuid::new_v4());
            let result = verify_agent_ownership(&repo, &auth, Uuid::new_v4()).await;
            assert!(matches!(result.unwrap_err(), AppError::NotFound(_)));
        }

        #[tokio::test]
        async fn system_agent_accessible_by_any_user() {
            let agent_id = Uuid::new_v4();
            let system_agent = AgentRow {
                id: agent_id,
                user_id: None, // System agent — no owner
                tier: None,
                name: "system".to_string(),
                system_prompt: "".to_string(),
                persona_style: None,
                model_provider: "anthropic".to_string(),
                model_id: "claude-sonnet-4-20250514".to_string(),
                model_max_tokens: 4096,
                model_temperature: 0.7,
                status: Some("idle".to_string()),

                output_schema_id: None,
                version: 1,
                default_reasoning_trace: None,
                is_system: true,
            };

            let mut repo = MockServerRepo::new();
            repo.expect_get_persisted_agent()
                .returning(move |_| Ok(Some(system_agent.clone())));

            // Any random user should be able to access system agents
            let auth = make_auth(Uuid::new_v4());
            let result = verify_agent_ownership(&repo, &auth, agent_id).await;
            assert!(result.is_ok());
        }
    }
}
