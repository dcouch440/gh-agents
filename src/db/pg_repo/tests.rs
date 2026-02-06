//! Tests for PostgreSQL repository

use super::*;
use crate::db::test_utils::TestDb;
use crate::types::{Priority, Task, TaskId, TaskStatus, UserId};

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn merge_queue_insert_and_get() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let id = Uuid::new_v4();
    let owner = "testowner".to_string();
    let repo_name = "testrepo".to_string();
    let pr_number = 42;
    let position = 1;
    let now = Utc::now();

    // Insert entry
    repo.insert_queue_entry(
        id,
        owner.clone(),
        repo_name.clone(),
        pr_number,
        position,
        now,
    )
    .await
    .unwrap();

    // Get entries
    let entries = repo
        .get_queue_entries(owner.clone(), repo_name.clone())
        .await
        .unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].pr_number, pr_number);
    assert_eq!(entries[0].queue_position, position);

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn merge_queue_get_next_position() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let owner = "testowner".to_string();
    let repo_name = "testrepo".to_string();
    let now = Utc::now();

    // Get next position (should be 1 for empty queue)
    let pos1 = repo
        .get_next_position(owner.clone(), repo_name.clone())
        .await
        .unwrap();
    assert_eq!(pos1, 1);

    // Insert entry at position 1
    repo.insert_queue_entry(Uuid::new_v4(), owner.clone(), repo_name.clone(), 1, 1, now)
        .await
        .unwrap();

    // Get next position (should be 2)
    let pos2 = repo
        .get_next_position(owner.clone(), repo_name.clone())
        .await
        .unwrap();
    assert_eq!(pos2, 2);

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn merge_queue_delete_entry() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let owner = "testowner".to_string();
    let repo_name = "testrepo".to_string();
    let pr_number = 42;
    let now = Utc::now();

    // Insert entry
    repo.insert_queue_entry(
        Uuid::new_v4(),
        owner.clone(),
        repo_name.clone(),
        pr_number,
        1,
        now,
    )
    .await
    .unwrap();

    // Delete entry
    let deleted = repo
        .delete_queue_entry(owner.clone(), repo_name.clone(), pr_number)
        .await
        .unwrap();
    assert!(deleted);

    // Verify deletion
    let entries = repo
        .get_queue_entries(owner.clone(), repo_name.clone())
        .await
        .unwrap();
    assert_eq!(entries.len(), 0);

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn merge_queue_update_status() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let owner = "testowner".to_string();
    let repo_name = "testrepo".to_string();
    let pr_number = 42;
    let now = Utc::now();

    // Insert entry
    repo.insert_queue_entry(
        Uuid::new_v4(),
        owner.clone(),
        repo_name.clone(),
        pr_number,
        1,
        now,
    )
    .await
    .unwrap();

    // Update status
    let updated = repo
        .update_entry_status(
            owner.clone(),
            repo_name.clone(),
            pr_number,
            "in_progress".to_string(),
            None,
            now,
        )
        .await
        .unwrap();
    assert!(updated);

    // Verify update
    let entries = repo
        .get_queue_entries(owner.clone(), repo_name.clone())
        .await
        .unwrap();
    assert_eq!(entries[0].status.to_string(), "in_progress");

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn merge_queue_reset_interrupted() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let owner = "testowner".to_string();
    let repo_name = "testrepo".to_string();
    let now = Utc::now();

    // Insert entries with in_progress status
    repo.insert_queue_entry(Uuid::new_v4(), owner.clone(), repo_name.clone(), 1, 1, now)
        .await
        .unwrap();
    repo.update_entry_status(
        owner.clone(),
        repo_name.clone(),
        1,
        "in_progress".to_string(),
        None,
        now,
    )
    .await
    .unwrap();

    // Reset interrupted
    let count = repo
        .reset_interrupted(owner.clone(), repo_name.clone(), now)
        .await
        .unwrap();
    assert_eq!(count, 1);

    // Verify reset
    let entries = repo
        .get_queue_entries(owner.clone(), repo_name.clone())
        .await
        .unwrap();
    assert_eq!(entries[0].status.to_string(), "pending");

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn user_repo_create_and_get_by_email() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let email = "test@example.com";
    let password_hash = "hashed_password";

    // Create user
    let user = repo.create_user(email, password_hash).await.unwrap();
    assert_eq!(user.email, email);
    assert_eq!(user.password_hash, Some(password_hash.to_string()));

    // Get user by email
    let fetched = repo.get_user_by_email(email).await.unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().email, email);

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn user_repo_get_by_id() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let email = "test@example.com";
    let password_hash = "hashed_password";

    // Create user
    let user = repo.create_user(email, password_hash).await.unwrap();

    // Get user by ID
    let fetched = repo.get_user_by_id(user.id).await.unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().id, user.id);

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn user_repo_create_github_user() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let email = "github@example.com";
    let github_id = 123456;
    let github_login = "testuser";
    let token = "encrypted_token";

    // Create GitHub user
    let user = repo
        .create_github_user(email, github_id, github_login, token)
        .await
        .unwrap();
    assert_eq!(user.email, email);
    assert_eq!(user.github_id, Some(github_id));
    assert_eq!(user.github_login, Some(github_login.to_string()));

    // Get by GitHub ID
    let fetched = repo.get_user_by_github_id(github_id).await.unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().github_id, Some(github_id));

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn user_repo_link_github() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    // Create regular user
    let user = repo
        .create_user("test@example.com", "password_hash")
        .await
        .unwrap();

    // Link GitHub
    let github_id = 789;
    let github_login = "linkeduser";
    let token = "encrypted_token";
    repo.link_github(user.id, github_id, github_login, token)
        .await
        .unwrap();

    // Verify link
    let fetched = repo.get_user_by_id(user.id).await.unwrap().unwrap();
    assert_eq!(fetched.github_id, Some(github_id));
    assert_eq!(fetched.github_login, Some(github_login.to_string()));

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn server_repo_health_check() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let healthy = repo.health_check().await;
    assert!(healthy);

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn server_repo_task_operations() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    // Create a user first
    let user = repo
        .create_user("taskuser@example.com", "hash")
        .await
        .unwrap();

    // Create a task
    let task = Task {
        id: TaskId(Uuid::new_v4()),
        slice_id: None,
        title: "Test Task".to_string(),
        description: "Test Description".to_string(),
        assigned_agent: None,
        status: TaskStatus::Pending,
        priority: Priority::Normal,
        context_files: vec![],
        metadata: None,
        depends_on: vec![],
        retry_count: 0,
        max_retries: 3,
        last_error: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // Insert task
    repo.insert_task(user.id, task.clone()).await.unwrap();

    // Get task by UUID
    let fetched = repo.get_task_by_uuid(user.id, task.id.0).await.unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().title, "Test Task");

    // List tasks
    let tasks = repo.list_tasks(user.id, None, None).await.unwrap();
    assert!(!tasks.is_empty());

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn document_repo_create_and_get() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let user_id = Uuid::new_v4();
    let title = "Test Document".to_string();
    let content = "This is test content".to_string();
    let doc_type = "note".to_string();
    let ref_tag = "test-ref".to_string();
    let tags = vec!["tag1".to_string(), "tag2".to_string()];

    // Create document
    let doc = repo
        .create_document(
            None,
            title.clone(),
            content.clone(),
            doc_type,
            ref_tag.clone(),
            tags,
        )
        .await
        .unwrap();

    assert_eq!(doc.title, title);
    assert_eq!(doc.content, content);

    // Get by ID
    let fetched = repo.get_document(doc.id).await.unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().title, title);

    // Get by ref_tag
    let fetched_by_ref = repo.get_document_by_ref_tag(&ref_tag).await.unwrap();
    assert!(fetched_by_ref.is_some());
    assert_eq!(fetched_by_ref.unwrap().ref_tag.unwrap_or_default(), ref_tag);

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn document_repo_update() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let user_id = Uuid::new_v4();

    // Create document
    let doc = repo
        .create_document(
            None,
            "Original Title".to_string(),
            "Original Content".to_string(),
            "note".to_string(),
            "ref".to_string(),
            vec![],
        )
        .await
        .unwrap();

    // Update document
    let updated = repo
        .update_document(
            doc.id,
            Some("Updated Content".to_string()),
            Some("Updated Title".to_string()),
            None,
        )
        .await
        .unwrap();

    assert_eq!(updated.title, "Updated Title");
    assert_eq!(updated.content, "Updated Content");

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn document_repo_delete() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let user_id = Uuid::new_v4();

    // Create document
    let doc = repo
        .create_document(
            None,
            "Title".to_string(),
            "Content".to_string(),
            "note".to_string(),
            "ref".to_string(),
            vec![],
        )
        .await
        .unwrap();

    // Delete document
    repo.delete_document(doc.id).await.unwrap();

    // Verify deletion
    let fetched = repo.get_document(doc.id).await.unwrap();
    assert!(fetched.is_none());

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn document_repo_list_by_user() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let user_id = Uuid::new_v4();

    // Create multiple documents
    for i in 1..=3 {
        repo.create_document(
            None,
            format!("Doc {}", i),
            "Content".to_string(),
            "note".to_string(),
            format!("ref-{}", i),
            vec![],
        )
        .await
        .unwrap();
    }

    // List documents
    let docs = repo.list_documents(user_id).await.unwrap();
    assert_eq!(docs.len(), 3);

    db.cleanup().await;
}

// ============================================================================
// Router Modes Tests

// ============================================================================
// Router Modes Tests
// ============================================================================

async fn create_test_router(repo: &PgRepo, user_id: Uuid) -> ToolRouterRow {
    repo.create_tool_router(
        "Test Router",
        Some("Test description".to_string()),
        "You are a test router",
        "claude-3-5-sonnet-20241022",
    )
    .await
    .unwrap()
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn test_create_and_get_router_mode() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());
    let user_id = Uuid::new_v4();

    let router = create_test_router(&repo, user_id).await;

    let mode = repo
        .create_router_mode(
            router.id,
            "coding",
            "Coding Mode",
            "For software engineering tasks",
            "You are a precise coding assistant",
            0.3,
            8192,
            false, // append_to_agent_system_prompt
            true,  // append_to_agent_tools
            10,
        )
        .await
        .unwrap();

    assert_eq!(mode.mode_key, "coding");
    assert_eq!(mode.display_name, "Coding Mode");
    assert_eq!(mode.temperature, 0.3);
    assert!(!mode.append_to_agent_system_prompt);
    assert!(mode.append_to_agent_tools);

    let retrieved = repo.get_router_mode(mode.id).await.unwrap();
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.mode_key, mode.mode_key);

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn test_get_router_mode_by_key() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());
    let user_id = Uuid::new_v4();

    let router = create_test_router(&repo, user_id).await;

    let mode = repo
        .create_router_mode(
            router.id,
            "research",
            "Research Mode",
            "For research tasks",
            "You are a research assistant",
            0.7,
            4096,
            true,
            true,
            5,
        )
        .await
        .unwrap();

    // Get by key
    let found = repo
        .get_router_mode_by_key(router.id, "research")
        .await
        .unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, mode.id);

    // Non-existent key
    let not_found = repo
        .get_router_mode_by_key(router.id, "nonexistent")
        .await
        .unwrap();
    assert!(not_found.is_none());

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn test_list_router_modes_ordering() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());
    let user_id = Uuid::new_v4();

    let router = create_test_router(&repo, user_id).await;

    // Create modes with different display_order
    repo.create_router_mode(
        router.id, "mode_c", "Mode C", "Third", "Prompt C", 0.7, 4096, false, true, 15,
    )
    .await
    .unwrap();

    repo.create_router_mode(
        router.id, "mode_a", "Mode A", "First", "Prompt A", 0.7, 4096, false, true, 5,
    )
    .await
    .unwrap();

    repo.create_router_mode(
        router.id, "mode_b", "Mode B", "Second", "Prompt B", 0.7, 4096, false, true, 10,
    )
    .await
    .unwrap();

    let modes = repo.list_router_modes(router.id).await.unwrap();
    assert_eq!(modes.len(), 3);
    // Should be ordered by display_order: 5, 10, 15
    assert_eq!(modes[0].display_order, 5);
    assert_eq!(modes[1].display_order, 10);
    assert_eq!(modes[2].display_order, 15);

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn test_update_router_mode_partial() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());
    let user_id = Uuid::new_v4();

    let router = create_test_router(&repo, user_id).await;

    let mode = repo
        .create_router_mode(
            router.id,
            "original",
            "Original Name",
            "Original description",
            "Original prompt",
            0.5,
            2048,
            false,
            false,
            20,
        )
        .await
        .unwrap();

    // Update only display_name and temperature
    let updated = repo
        .update_router_mode(
            mode.id,
            None,                             // mode_key unchanged
            Some("Updated Name".to_string()), // display_name changed
            None,                             // description unchanged
            None,                             // system_prompt unchanged
            Some(0.9),                        // temperature changed
            None,                             // max_tokens unchanged
            None,                             // append_to_agent_system_prompt unchanged
            None,                             // append_to_agent_tools unchanged
            None,                             // display_order unchanged
        )
        .await
        .unwrap();

    assert_eq!(updated.mode_key, "original"); // Unchanged
    assert_eq!(updated.display_name, "Updated Name"); // Changed
    assert_eq!(updated.description, "Original description"); // Unchanged
    assert_eq!(updated.temperature, 0.9); // Changed
    assert_eq!(updated.max_tokens, 2048); // Unchanged

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn test_delete_router_mode() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());
    let user_id = Uuid::new_v4();

    let router = create_test_router(&repo, user_id).await;

    let mode = repo
        .create_router_mode(
            router.id,
            "delete_me",
            "Delete Me",
            "Will be deleted",
            "Prompt",
            0.7,
            4096,
            false,
            true,
            1,
        )
        .await
        .unwrap();

    // Verify exists
    let found = repo.get_router_mode(mode.id).await.unwrap();
    assert!(found.is_some());

    // Delete
    repo.delete_router_mode(mode.id).await.unwrap();

    // Verify deleted
    let not_found = repo.get_router_mode(mode.id).await.unwrap();
    assert!(not_found.is_none());

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn test_get_mode_tools() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());
    let user_id = Uuid::new_v4();

    let router = create_test_router(&repo, user_id).await;

    // Create 3 tools
    let tool1 = ToolRow {
        id: Uuid::new_v4(),
        name: "tool_a".to_string(),
        display_name: "Tool A".to_string(),
        description: "First".to_string(),
        parameters: serde_json::json!({}),
        created_at: Utc::now(),
        version: 1,
    };
    repo.upsert_tool(tool1.clone())
        .await
        .unwrap();

    let tool2 = ToolRow {
        id: Uuid::new_v4(),
        name: "tool_c".to_string(),
        display_name: "Tool C".to_string(),
        description: "Third".to_string(),
        parameters: serde_json::json!({}),
        created_at: Utc::now(),
        version: 1,
    };
    repo.upsert_tool(tool2.clone())
        .await
        .unwrap();

    let tool3 = ToolRow {
        id: Uuid::new_v4(),
        name: "tool_b".to_string(),
        display_name: "Tool B".to_string(),
        description: "Second".to_string(),
        parameters: serde_json::json!({}),
        created_at: Utc::now(),
        version: 1,
    };
    repo.upsert_tool(tool3.clone())
        .await
        .unwrap();

    // Create mode
    let mode = repo
        .create_router_mode(
            router.id,
            "tool_test",
            "Tool Test",
            "Test mode tools",
            "Prompt",
            0.7,
            4096,
            false,
            true,
            1,
        )
        .await
        .unwrap();

    // Associate tools
    repo.set_mode_tools(mode.id, &[tool1.id, tool2.id, tool3.id])
        .await
        .unwrap();

    // Get tools
    let tools = repo.get_mode_tools(mode.id).await.unwrap();
    assert_eq!(tools.len(), 3);
    // Should be ordered by name: tool_a, tool_b, tool_c
    assert_eq!(tools[0].name, "tool_a");
    assert_eq!(tools[1].name, "tool_b");
    assert_eq!(tools[2].name, "tool_c");

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn test_set_mode_tools_replaces() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());
    let user_id = Uuid::new_v4();

    let router = create_test_router(&repo, user_id).await;

    // Create tools
    let tool_a = ToolRow {
        id: Uuid::new_v4(),
        name: "tool_a".to_string(),
        display_name: "Tool A".to_string(),
        description: "A".to_string(),
        parameters: serde_json::json!({}),
        created_at: Utc::now(),
        version: 1,
    };
    repo.upsert_tool(tool_a.clone())
        .await
        .unwrap();

    let tool_b = ToolRow {
        id: Uuid::new_v4(),
        name: "tool_b".to_string(),
        display_name: "Tool B".to_string(),
        description: "B".to_string(),
        parameters: serde_json::json!({}),
        created_at: Utc::now(),
        version: 1,
    };
    repo.upsert_tool(tool_b.clone())
        .await
        .unwrap();

    let tool_c = ToolRow {
        id: Uuid::new_v4(),
        name: "tool_c".to_string(),
        display_name: "Tool C".to_string(),
        description: "C".to_string(),
        parameters: serde_json::json!({}),
        created_at: Utc::now(),
        version: 1,
    };
    repo.upsert_tool(tool_c.clone())
        .await
        .unwrap();

    let tool_d = ToolRow {
        id: Uuid::new_v4(),
        name: "tool_d".to_string(),
        display_name: "Tool D".to_string(),
        description: "D".to_string(),
        parameters: serde_json::json!({}),
        created_at: Utc::now(),
        version: 1,
    };
    repo.upsert_tool(tool_d.clone())
        .await
        .unwrap();

    // Create mode
    let mode = repo
        .create_router_mode(
            router.id,
            "replace_test",
            "Replace Test",
            "Test replacement",
            "Prompt",
            0.7,
            4096,
            false,
            true,
            1,
        )
        .await
        .unwrap();

    // Set tools [A, B]
    repo.set_mode_tools(mode.id, &[tool_a.id, tool_b.id])
        .await
        .unwrap();

    let tools = repo.get_mode_tools(mode.id).await.unwrap();
    assert_eq!(tools.len(), 2);

    // Set tools [C, D] - should replace
    repo.set_mode_tools(mode.id, &[tool_c.id, tool_d.id])
        .await
        .unwrap();

    let tools = repo.get_mode_tools(mode.id).await.unwrap();
    assert_eq!(tools.len(), 2);
    assert_eq!(tools[0].name, "tool_c");
    assert_eq!(tools[1].name, "tool_d");

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn test_set_mode_tools_empty() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());
    let user_id = Uuid::new_v4();

    let router = create_test_router(&repo, user_id).await;

    // Create tool
    let tool = ToolRow {
        id: Uuid::new_v4(),
        name: "tool".to_string(),
        display_name: "Tool".to_string(),
        description: "Test".to_string(),
        parameters: serde_json::json!({}),
        created_at: Utc::now(),
        version: 1,
    };
    repo.upsert_tool(tool.clone())
        .await
        .unwrap();

    // Create mode
    let mode = repo
        .create_router_mode(
            router.id,
            "empty_test",
            "Empty Test",
            "Test empty tools",
            "Prompt",
            0.7,
            4096,
            false,
            true,
            1,
        )
        .await
        .unwrap();

    // Set one tool
    repo.set_mode_tools(mode.id, &[tool.id]).await.unwrap();
    let tools = repo.get_mode_tools(mode.id).await.unwrap();
    assert_eq!(tools.len(), 1);

    // Set empty array
    repo.set_mode_tools(mode.id, &[]).await.unwrap();
    let tools = repo.get_mode_tools(mode.id).await.unwrap();
    assert_eq!(tools.len(), 0);

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn test_mode_boolean_flags() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());
    let user_id = Uuid::new_v4();

    let router = create_test_router(&repo, user_id).await;

    // Create mode with append_to_agent_system_prompt=true, append_to_agent_tools=false
    let mode = repo
        .create_router_mode(
            router.id,
            "bool_test",
            "Boolean Test",
            "Test boolean storage",
            "Prompt",
            0.7,
            4096,
            true,  // append_to_agent_system_prompt
            false, // append_to_agent_tools
            1,
        )
        .await
        .unwrap();

    // Retrieve and verify
    let retrieved = repo.get_router_mode(mode.id).await.unwrap().unwrap();
    assert!(retrieved.append_to_agent_system_prompt);
    assert!(!retrieved.append_to_agent_tools);

    // Verify not stored as NULL by querying database directly
    let (append_prompt, append_tools): (bool, bool) = sqlx::query_as(
        "SELECT append_to_agent_system_prompt, append_to_agent_tools FROM tool_router_modes WHERE id = $1",
    )
    .bind(mode.id)
    .fetch_one(&db.pool)
    .await
    .unwrap();

    assert!(append_prompt);
    assert!(!append_tools);

    db.cleanup().await;
}
