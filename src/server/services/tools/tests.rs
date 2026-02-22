#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::fixtures::fixtures::*;
    use crate::db::traits::{MockAgentRepo, MockToolRepo};
    use crate::server::services::tools::*;
    use crate::server::services::ServiceError;

    #[tokio::test]
    async fn create_rejects_non_admin() {
        let repo = MockToolRepo::new();
        let result = create_tool(
            &repo,
            CreateToolInput {
                is_admin: false,
                name: "test_tool".to_string(),
                display_name: None,
                description: None,
                parameters: None,
            },
        )
        .await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }

    #[tokio::test]
    async fn create_rejects_empty_name() {
        let repo = MockToolRepo::new();
        let result = create_tool(
            &repo,
            CreateToolInput {
                is_admin: true,
                name: "  ".to_string(),
                display_name: None,
                description: None,
                parameters: None,
            },
        )
        .await;
        assert!(matches!(result, Err(ServiceError::Validation(_))));
    }

    #[tokio::test]
    async fn create_succeeds_for_admin() {
        let mut repo = MockToolRepo::new();
        repo.expect_upsert_tool().returning(|_| Ok(()));

        let result = create_tool(
            &repo,
            CreateToolInput {
                is_admin: true,
                name: "content_search".to_string(),
                display_name: Some("Content Search".to_string()),
                description: Some("Search file contents".to_string()),
                parameters: None,
            },
        )
        .await;
        let tool = result.unwrap();
        assert_eq!(tool.name, "content_search");
        assert_eq!(tool.display_name, "Content Search");
        assert_eq!(tool.description, "Search file contents");
    }

    #[tokio::test]
    async fn create_defaults_display_name_to_name() {
        let mut repo = MockToolRepo::new();
        repo.expect_upsert_tool().returning(|_| Ok(()));

        let result = create_tool(
            &repo,
            CreateToolInput {
                is_admin: true,
                name: "content_search".to_string(),
                display_name: None,
                description: None,
                parameters: None,
            },
        )
        .await;
        let tool = result.unwrap();
        assert_eq!(tool.display_name, "content_search");
    }

    #[tokio::test]
    async fn update_rejects_non_admin() {
        let repo = MockToolRepo::new();
        let result = update_tool(
            &repo,
            UpdateToolInput {
                is_admin: false,
                tool_id: Uuid::new_v4(),
                name: Some("new_name".to_string()),
                display_name: None,
                description: None,
                parameters: None,
            },
        )
        .await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }

    #[tokio::test]
    async fn update_merges_partial_fields() {
        let tool = tool_row("original");
        let tool_id = tool.id;
        let tool_clone = tool.clone();

        let mut repo = MockToolRepo::new();
        repo.expect_get_tool()
            .returning(move |_| Ok(Some(tool_clone.clone())));
        repo.expect_upsert_tool().returning(|_| Ok(()));

        let result = update_tool(
            &repo,
            UpdateToolInput {
                is_admin: true,
                tool_id,
                name: Some("updated".to_string()),
                display_name: None,
                description: None,
                parameters: None,
            },
        )
        .await;
        let updated = result.unwrap();
        assert_eq!(updated.name, "updated");
        assert_eq!(updated.display_name, "original"); // unchanged
    }

    #[tokio::test]
    async fn delete_rejects_non_admin() {
        let repo = MockToolRepo::new();
        let result = delete_tool(&repo, false, Uuid::new_v4()).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }

    #[tokio::test]
    async fn get_agent_tools_rejects_non_owner() {
        let owner_id = Uuid::new_v4();
        let attacker_id = Uuid::new_v4();
        let agent_id = Uuid::new_v4();
        let agent = agent_owned(agent_id, owner_id);

        let tool_repo = MockToolRepo::new();
        let mut agent_repo = MockAgentRepo::new();
        agent_repo
            .expect_get_persisted_agent()
            .returning(move |_| Ok(Some(agent.clone())));

        let result = get_agent_tools(&tool_repo, &agent_repo, attacker_id, agent_id).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }

    #[tokio::test]
    async fn get_agent_tools_succeeds_for_owner() {
        let user_id = Uuid::new_v4();
        let agent_id = Uuid::new_v4();
        let agent = agent_owned(agent_id, user_id);
        let tool = tool_row("search");

        let mut tool_repo = MockToolRepo::new();
        tool_repo
            .expect_get_agent_tools()
            .returning(move |_| Ok(vec![tool.clone()]));

        let mut agent_repo = MockAgentRepo::new();
        agent_repo
            .expect_get_persisted_agent()
            .returning(move |_| Ok(Some(agent.clone())));

        let result = get_agent_tools(&tool_repo, &agent_repo, user_id, agent_id).await;
        let tools = result.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "search");
    }

    #[tokio::test]
    async fn set_agent_tools_rejects_invalid_uuid() {
        let user_id = Uuid::new_v4();
        let agent_id = Uuid::new_v4();
        let agent = agent_owned(agent_id, user_id);

        let tool_repo = MockToolRepo::new();
        let mut agent_repo = MockAgentRepo::new();
        agent_repo
            .expect_get_persisted_agent()
            .returning(move |_| Ok(Some(agent.clone())));

        let result = set_agent_tools(
            &tool_repo,
            &agent_repo,
            SetAgentToolsInput {
                user_id,
                agent_id,
                tool_ids: vec!["not-a-uuid".to_string()],
            },
        )
        .await;
        assert!(matches!(result, Err(ServiceError::Validation(_))));
    }
}
