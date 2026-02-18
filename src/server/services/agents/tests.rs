#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::traits::MockAgentRepo;
    use crate::db::AgentRow;
    use crate::server::services::agents::*;
    use crate::server::services::ServiceError;

    fn make_agent(id: Uuid, owner_id: Uuid) -> AgentRow {
        AgentRow {
            id,
            user_id: Some(owner_id),
            tier: None,
            name: "test-agent".to_string(),
            system_prompt: String::new(),
            persona_style: Some("casual".to_string()),
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

    fn make_system_agent(id: Uuid) -> AgentRow {
        AgentRow {
            user_id: None,
            is_system: true,
            ..make_agent(id, Uuid::new_v4())
        }
    }

    // ── create_agent ──────────────────────────────────────────────────

    #[tokio::test]
    async fn create_rejects_empty_model_id() {
        let repo = MockAgentRepo::new();
        let result = create_agent(
            &repo,
            CreateAgentInput {
                user_id: Uuid::new_v4(),
                name: "Agent".to_string(),
                system_prompt: None,
                persona_style: None,
                model_provider: None,
                model_id: "   ".to_string(),
                model_max_tokens: None,
                model_temperature: None,
                output_schema_id: None,
            },
        )
        .await;
        assert!(matches!(result, Err(ServiceError::Validation(_))));
    }

    #[tokio::test]
    async fn create_applies_defaults() {
        let mut repo = MockAgentRepo::new();
        repo.expect_upsert_agent().returning(|_| Ok(()));

        let result = create_agent(
            &repo,
            CreateAgentInput {
                user_id: Uuid::new_v4(),
                name: "My Agent".to_string(),
                system_prompt: None,
                persona_style: None,
                model_provider: None,
                model_id: "claude-sonnet-4-20250514".to_string(),
                model_max_tokens: None,
                model_temperature: None,
                output_schema_id: None,
            },
        )
        .await;
        let agent = result.unwrap();
        assert_eq!(agent.name, "My Agent");
        assert_eq!(agent.model_provider, "anthropic");
        assert_eq!(agent.persona_style, Some("casual".to_string()));
        assert_eq!(agent.model_max_tokens, 4096);
        assert!((agent.model_temperature - 0.7).abs() < f32::EPSILON);
        assert!(!agent.is_system);
    }

    // ── get_agent ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn get_rejects_wrong_owner() {
        let owner = Uuid::new_v4();
        let attacker = Uuid::new_v4();
        let agent_id = Uuid::new_v4();
        let agent = make_agent(agent_id, owner);

        let mut repo = MockAgentRepo::new();
        repo.expect_get_persisted_agent()
            .returning(move |_| Ok(Some(agent.clone())));

        let result = get_agent(&repo, attacker, agent_id).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }

    #[tokio::test]
    async fn get_allows_system_agent_for_any_user() {
        let agent_id = Uuid::new_v4();
        let system_agent = make_system_agent(agent_id);

        let mut repo = MockAgentRepo::new();
        repo.expect_get_persisted_agent()
            .returning(move |_| Ok(Some(system_agent.clone())));

        let random_user = Uuid::new_v4();
        let result = get_agent(&repo, random_user, agent_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, agent_id);
    }

    // ── delete_agent ──────────────────────────────────────────────────

    #[tokio::test]
    async fn delete_rejects_system_agent() {
        let user_id = Uuid::new_v4();
        let agent_id = Uuid::new_v4();
        let system_agent = AgentRow {
            user_id: Some(user_id),
            is_system: true,
            ..make_agent(agent_id, user_id)
        };

        let mut repo = MockAgentRepo::new();
        repo.expect_get_persisted_agent()
            .returning(move |_| Ok(Some(system_agent.clone())));

        let result = delete_agent(&repo, user_id, agent_id).await;
        assert!(matches!(result, Err(ServiceError::Validation(_))));
    }

    #[tokio::test]
    async fn delete_rejects_wrong_owner() {
        let owner = Uuid::new_v4();
        let attacker = Uuid::new_v4();
        let agent_id = Uuid::new_v4();
        let agent = make_agent(agent_id, owner);

        let mut repo = MockAgentRepo::new();
        repo.expect_get_persisted_agent()
            .returning(move |_| Ok(Some(agent.clone())));

        let result = delete_agent(&repo, attacker, agent_id).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }
}
