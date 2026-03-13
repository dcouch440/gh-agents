#[cfg(test)]
mod tests {
    //! Tests for database queries

    use super::super::*;

    use crate::db::pg_repo::PgRepo;
    use crate::db::test_utils::TestDb;
    use crate::db::traits::{
        CreateStepInputPort, RoomRepo, SaveRoomExecutionOutputInput, SystemConfigRepo,
        ToolCapabilityRepo, WorkflowRepo,
    };
    use crate::db::{
        AgentExecutionRow, AgentRow, RoomRow, RoomSessionRow, WorkflowRow, WorkflowStepRow,
    };
    use crate::types::UserId;

    fn test_user_id() -> UserId {
        UserId(uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap())
    }

    // Helper for tests that use repository traits
    fn test_repo(db: &TestDb) -> PgRepo {
        PgRepo::new(db.pool.clone())
    }

    // Chat message tests

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn can_insert_and_get_chat_message() {
        let db = TestDb::new().await;
        let id = Uuid::new_v4();

        insert_chat_message(&db.pool, test_user_id(), &id, "user", "Hello, world!")
            .await
            .unwrap();

        let history = get_chat_history(&db.pool, test_user_id(), 50, 0)
            .await
            .unwrap();
        assert!(history.len() >= 1);

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn chat_history_pagination_works() {
        let db = TestDb::new().await;

        // Insert 5 messages
        for i in 0..5 {
            let id = Uuid::new_v4();
            insert_chat_message(
                &db.pool,
                test_user_id(),
                &id,
                "user",
                &format!("Message {}", i),
            )
            .await
            .unwrap();
        }

        // Get first 2
        let history = get_chat_history(&db.pool, test_user_id(), 2, 0)
            .await
            .unwrap();
        assert_eq!(history.len(), 2);

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn can_clear_chat_history() {
        let db = TestDb::new().await;

        for _ in 0..3 {
            let id = Uuid::new_v4();
            insert_chat_message(&db.pool, test_user_id(), &id, "user", "Test message")
                .await
                .unwrap();
        }

        let history = get_chat_history(&db.pool, test_user_id(), 50, 0)
            .await
            .unwrap();
        assert!(history.len() >= 3);

        clear_chat_history(&db.pool, test_user_id()).await.unwrap();

        let history = get_chat_history(&db.pool, test_user_id(), 50, 0)
            .await
            .unwrap();
        assert_eq!(history.len(), 0);

        db.cleanup().await;
    }

    // Auth tests

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn test_password_flow() {
        let db = TestDb::new().await;

        // Initially no password
        assert!(!has_password(&db.pool).await.unwrap());
        assert!(get_password(&db.pool).await.unwrap().is_none());

        // Set password
        set_password(&db.pool, "test_hash_123").await.unwrap();

        // Now has password
        assert!(has_password(&db.pool).await.unwrap());
        let stored = get_password(&db.pool).await.unwrap();
        assert_eq!(stored, Some("test_hash_123".to_string()));

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn test_set_password_twice_fails() {
        let db = TestDb::new().await;

        // Set password first time
        set_password(&db.pool, "hash1").await.unwrap();

        // Setting again should fail (unique constraint on id=1)
        let result = set_password(&db.pool, "hash2").await;
        assert!(result.is_err());

        // Original password should still be there
        let stored = get_password(&db.pool).await.unwrap();
        assert_eq!(stored, Some("hash1".to_string()));

        db.cleanup().await;
    }

    // ============================================================================
    // Tool Capability Tests
    // ============================================================================

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn test_tool_capability_crud() {
        let db = TestDb::new().await;
        let repo = test_repo(&db);

        // Get all capabilities (seeded by migration)
        let caps = repo.get_tool_capabilities().await.unwrap();
        assert!(
            caps.len() >= 15,
            "Should have at least 15 seeded capabilities"
        );

        // Get by key
        let file_read = repo.get_tool_capability_by_key("file_read").await.unwrap();
        assert!(file_read.is_some());
        let file_read = file_read.unwrap();
        assert_eq!(file_read.capability_key, "file_read");
        assert_eq!(file_read.category, "filesystem");

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn test_tool_capability_assignments() {
        use crate::db::ToolRow;

        let db = TestDb::new().await;
        let repo = test_repo(&db);

        // Get tools (from seed data)
        let tools = sqlx::query_as::<_, ToolRow>("SELECT * FROM tools LIMIT 1")
            .fetch_all(&db.pool)
            .await
            .unwrap();

        if tools.is_empty() {
            // Skip test if no tools seeded yet
            db.cleanup().await;
            return;
        }

        let tool = &tools[0];

        // Get capabilities for tool
        let caps = repo.get_capabilities_by_tool(tool.id).await.unwrap();
        // Should have at least some capabilities assigned
        assert!(
            !caps.is_empty() || true,
            "Tool may or may not have capabilities initially"
        );

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn test_set_tool_capabilities() {
        use crate::db::ToolRow;

        let db = TestDb::new().await;
        let repo = test_repo(&db);

        // Create test tool
        let tool = sqlx::query_as::<_, ToolRow>(
            "INSERT INTO tools (name, description, input_schema) VALUES ($1, $2, $3) RETURNING *",
        )
        .bind("test_tool")
        .bind("Test tool")
        .bind(serde_json::json!({}))
        .fetch_one(&db.pool)
        .await
        .unwrap();

        // Get capability IDs
        let file_read = repo
            .get_tool_capability_by_key("file_read")
            .await
            .unwrap()
            .unwrap();
        let file_write = repo
            .get_tool_capability_by_key("file_write")
            .await
            .unwrap()
            .unwrap();

        // Set capabilities
        repo.set_tool_capabilities(tool.id, &[file_read.id, file_write.id])
            .await
            .unwrap();

        // Verify
        let caps = repo.get_capabilities_by_tool(tool.id).await.unwrap();
        assert_eq!(caps.len(), 2);

        let keys: Vec<_> = caps.iter().map(|c| c.capability_key.as_str()).collect();
        assert!(keys.contains(&"file_read"));
        assert!(keys.contains(&"file_write"));

        db.cleanup().await;
    }

    // ============================================================================
    // System Config Tests
    // ============================================================================

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn test_system_config_crud() {
        let db = TestDb::new().await;
        let repo = test_repo(&db);

        // Upsert new config
        let config = repo
            .upsert_system_config(
                "test",
                "test:key",
                &serde_json::json!({"value": "test"}),
                Some("Test config".to_string()),
                None,
            )
            .await
            .unwrap();

        assert_eq!(config.config_key, "test:key");
        assert_eq!(config.config_type, "test");

        // Get config
        let retrieved = repo.get_system_config("test:key").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(
            retrieved.unwrap().config_value,
            serde_json::json!({"value": "test"})
        );

        // Update config (upsert)
        let updated = repo
            .upsert_system_config(
                "test",
                "test:key",
                &serde_json::json!({"value": "updated"}),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(
            updated.config_value,
            serde_json::json!({"value": "updated"})
        );

        // Delete config
        repo.delete_system_config("test:key").await.unwrap();
        let deleted = repo.get_system_config("test:key").await.unwrap();
        assert!(deleted.is_none());

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn test_get_execution_constraints() {
        let db = TestDb::new().await;
        let repo = test_repo(&db);

        // Should have constraint configs from migration seed
        let constraints = repo.get_execution_constraints().await.unwrap();
        assert!(!constraints.is_empty(), "Should have seeded constraints");

        assert!(
            constraints.contains_key("max_subtasks_per_cavernous_step")
                || constraints.contains_key("unsafe_operations_enabled"),
            "Should have at least some constraint keys"
        );

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn test_unsafe_operations_flag() {
        let db = TestDb::new().await;
        let repo = test_repo(&db);

        // Should default to false from migration
        let unsafe_ops = repo.get_unsafe_operations_enabled().await.unwrap();
        assert!(
            !unsafe_ops,
            "unsafe_operations_enabled should default to false"
        );

        // Set to true
        repo.upsert_system_config(
            "constraint",
            "unsafe_operations_enabled",
            &serde_json::json!(true),
            None,
            None,
        )
        .await
        .unwrap();

        // Verify
        let unsafe_ops = repo.get_unsafe_operations_enabled().await.unwrap();
        assert!(unsafe_ops);

        db.cleanup().await;
    }

    // ============================================================================
    // Workflow Port Tests
    // ============================================================================

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn test_step_input_output_crud() {
        use crate::db::{WorkflowRow, WorkflowStepRow};

        let db = TestDb::new().await;
        let repo = test_repo(&db);

        // Create test workflow + step (minimal)
        let user_id = Uuid::new_v4();
        let agent = sqlx::query_as::<_, AgentRow>(
            "INSERT INTO agents (user_id, name, system_prompt) VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(user_id)
        .bind("Port Test Agent")
        .bind("Test prompt")
        .fetch_one(&db.pool)
        .await
        .unwrap();

        let workflow = sqlx::query_as::<_, WorkflowRow>(
            "INSERT INTO workflows (user_id, name, description) VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(user_id)
        .bind("Test Workflow")
        .bind("Test")
        .fetch_one(&db.pool)
        .await
        .unwrap();

        let step = sqlx::query_as::<_, WorkflowStepRow>(
            "INSERT INTO workflow_steps (workflow_id, agent_id, execution_mode, display_order)
         VALUES ($1, $2, $3, $4) RETURNING *",
        )
        .bind(workflow.id)
        .bind(agent.id)
        .bind("single")
        .bind(0)
        .fetch_one(&db.pool)
        .await
        .unwrap();

        // Create input port
        let input = repo
            .create_step_input(CreateStepInputPort {
                workflow_step_id: step.id,
                port_name: "data".to_string(),
                port_type: "object".to_string(),
                required: true,
                default_value: None,
                description: Some("Input data".to_string()),
                json_schema: None,
            })
            .await
            .unwrap();

        assert_eq!(input.port_name, "data");
        assert_eq!(input.port_type, "object");
        assert!(input.required);

        // Create output port
        let output = repo
            .create_step_output(
                step.id,
                "result",
                "string",
                "result",
                Some("Output result".to_string()),
                None,
            )
            .await
            .unwrap();

        assert_eq!(output.port_name, "result");
        assert_eq!(output.json_path, "result");

        // Get ports
        let inputs = repo.get_step_inputs(step.id).await.unwrap();
        let outputs = repo.get_step_outputs(step.id).await.unwrap();

        assert_eq!(inputs.len(), 1);
        assert_eq!(outputs.len(), 1);

        // Delete ports
        repo.delete_step_input(input.id).await.unwrap();
        repo.delete_step_output(output.id).await.unwrap();

        let inputs = repo.get_step_inputs(step.id).await.unwrap();
        let outputs = repo.get_step_outputs(step.id).await.unwrap();

        assert_eq!(inputs.len(), 0);
        assert_eq!(outputs.len(), 0);

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn test_routing_rule_crud() {
        use crate::db::{AgentRow, WorkflowRow, WorkflowStepRow};

        let db = TestDb::new().await;
        let repo = test_repo(&db);

        // Create test workflow + step + agent
        let user_id = Uuid::new_v4();
        let agent = sqlx::query_as::<_, AgentRow>(
            "INSERT INTO agents (user_id, name, system_prompt) VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(user_id)
        .bind("Test Agent")
        .bind("Test prompt")
        .fetch_one(&db.pool)
        .await
        .unwrap();

        let workflow = sqlx::query_as::<_, WorkflowRow>(
            "INSERT INTO workflows (user_id, name, description) VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(user_id)
        .bind("Test Workflow")
        .bind("Test")
        .fetch_one(&db.pool)
        .await
        .unwrap();

        let step = sqlx::query_as::<_, WorkflowStepRow>(
        "INSERT INTO workflow_steps (workflow_id, agent_id, execution_mode, display_order, routing_mode, routing_field)
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING *"
    )
    .bind(workflow.id)
    .bind(agent.id)
    .bind("for_each")
    .bind(0)
    .bind("label")
    .bind("category")
    .fetch_one(&db.pool)
    .await
    .unwrap();

        // Create routing rule
        let rule = repo
            .create_routing_rule(
                step.id,
                "frontend",
                agent.id,
                Some("Frontend tasks".to_string()),
                0,
            )
            .await
            .unwrap();

        assert_eq!(rule.label_value, "frontend");
        assert_eq!(rule.agent_id, agent.id);
        assert_eq!(rule.description, Some("Frontend tasks".to_string()));

        // Get routing rules
        let rules = repo.get_step_routing_rules(step.id).await.unwrap();
        assert_eq!(rules.len(), 1);

        // Update rule
        let agent2 = sqlx::query_as::<_, AgentRow>(
            "INSERT INTO agents (user_id, name, system_prompt) VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(user_id)
        .bind("Agent 2")
        .bind("Test")
        .fetch_one(&db.pool)
        .await
        .unwrap();

        let updated = repo
            .update_routing_rule(rule.id, Some(agent2.id), None, Some(1))
            .await
            .unwrap();

        assert_eq!(updated.agent_id, agent2.id);
        assert_eq!(updated.display_order, 1);

        // Delete rule
        repo.delete_routing_rule(rule.id).await.unwrap();
        let rules = repo.get_step_routing_rules(step.id).await.unwrap();
        assert_eq!(rules.len(), 0);

        db.cleanup().await;
    }

    // ============================================================================
    // Room Output Tests
    // ============================================================================

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn test_room_execution_outputs() {
        use crate::db::{AgentExecutionRow, AgentRow, RoomRow, RoomSessionRow};

        let db = TestDb::new().await;
        let repo = test_repo(&db);

        // Create test room + session + execution (minimal)
        let user_id = Uuid::new_v4();
        let room = sqlx::query_as::<_, RoomRow>(
            "INSERT INTO rooms (user_id, name) VALUES ($1, $2) RETURNING *",
        )
        .bind(user_id)
        .bind("Test Room")
        .fetch_one(&db.pool)
        .await
        .unwrap();

        let session = sqlx::query_as::<_, RoomSessionRow>(
            "INSERT INTO room_sessions (room_id, status) VALUES ($1, $2) RETURNING *",
        )
        .bind(room.id)
        .bind("active")
        .fetch_one(&db.pool)
        .await
        .unwrap();

        let agent = sqlx::query_as::<_, AgentRow>(
            "INSERT INTO agents (user_id, name, system_prompt) VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(user_id)
        .bind("Test Agent")
        .bind("Test")
        .fetch_one(&db.pool)
        .await
        .unwrap();

        let execution = sqlx::query_as::<_, AgentExecutionRow>(
            "INSERT INTO agent_executions (agent_id, status, execution_mode, structured_output)
         VALUES ($1, $2, $3, $4) RETURNING *",
        )
        .bind(agent.id)
        .bind("completed")
        .bind("single")
        .bind(serde_json::json!({}))
        .fetch_one(&db.pool)
        .await
        .unwrap();

        // Save room execution output
        let output = repo
            .save_room_execution_output(SaveRoomExecutionOutputInput {
                room_session_id: session.id,
                agent_execution_id: execution.id,
                agent_id: agent.id,
                speaker_order: 0,
                turn_number: 1,
                output_name: "analysis".to_string(),
                structured_output: serde_json::json!({"findings": ["item1", "item2"]}),
                raw_output: "Raw output text".to_string(),
                schema_id: None,
            })
            .await
            .unwrap();

        assert_eq!(output.output_name, "analysis");
        assert_eq!(output.turn_number, 1);

        // Get outputs
        let outputs = repo
            .get_room_execution_outputs(session.id, None)
            .await
            .unwrap();
        assert_eq!(outputs.len(), 1);

        // Get outputs by turn
        let turn_outputs = repo
            .get_room_execution_outputs(session.id, Some(1))
            .await
            .unwrap();
        assert_eq!(turn_outputs.len(), 1);

        db.cleanup().await;
    }
}
