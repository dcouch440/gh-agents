//! Tests for PostgreSQL repository

use super::*;
use crate::db::test_utils::TestDb;
use crate::db::traits::{
    AgentExecutionRepo, DocumentRepo, RoomRepo, TokenLedgerRepo, WorkflowCollectionRepo,
    WorkflowRepo,
};
use crate::types::{Priority, Task, TaskId, TaskStatus, UserId};

// ============================================================================
// Test helpers
// ============================================================================

/// Create a user and return the raw Uuid (unwraps UserId for convenience).
async fn create_test_user(repo: &PgRepo) -> Uuid {
    let email = format!("test-{}@example.com", Uuid::new_v4().simple());
    let user = repo.create_user(&email, "hashed_password").await.unwrap();
    user.id.0
}

async fn create_test_agent(repo: &PgRepo, user_id: Uuid) -> AgentRow {
    let agent = AgentRow {
        id: Uuid::new_v4(),
        user_id: Some(user_id),
        tier: None,
        name: format!("test-agent-{}", Uuid::new_v4().simple()),
        system_prompt: "You are a test agent.".to_string(),
        persona_style: None,
        model_provider: "anthropic".to_string(),
        model_id: "claude-sonnet-4-5-20250929".to_string(),
        model_max_tokens: 4096,
        model_temperature: 0.7,
        status: Some("active".to_string()),
        router_mode: None,
        router_id: None,
        output_schema_id: None,
        version: 1,
    };
    repo.upsert_agent(UserId(user_id), agent.clone())
        .await
        .unwrap();
    agent
}

async fn create_test_workflow(repo: &PgRepo, user_id: Uuid) -> WorkflowRow {
    repo.create_workflow(
        user_id,
        format!("test-workflow-{}", Uuid::new_v4().simple()),
        "Test workflow".to_string(),
        false,
        None,
        None,
        false,
    )
    .await
    .unwrap()
}

async fn create_test_step(repo: &PgRepo, workflow_id: Uuid, agent_id: Uuid) -> WorkflowStepRow {
    let step = WorkflowStepRow {
        id: Uuid::new_v4(),
        workflow_id,
        agent_id,
        execution_mode: "single".to_string(),
        agent_execution_mode: None,
        for_each_ref: None,
        prompt_template_id: None,
        prompt_template: "Test prompt".to_string(),
        output_schema_id: None,
        output_variable_name: Some("result".to_string()),
        interactive_agent_id: None,
        for_each_label_field: None,
        room_id: None,
        routing_mode: None,
        routing_field: None,
        display_order: 0,
        version: 1,
        reasoning_trace: false,
        verification_agent_ids: None,
        position_x: None,
        position_y: None,
        name: None,
    };
    repo.create_step(step).await.unwrap()
}

/// Create a full execution chain: collection → run → workflow_execution.
/// Returns (workflow_execution_id, workflow_id).
async fn create_execution_chain(repo: &PgRepo, user_id: Uuid) -> (Uuid, Uuid) {
    let workflow = create_test_workflow(repo, user_id).await;
    let collection = repo
        .create_collection(
            user_id,
            "test-collection".to_string(),
            None,
            "sequential".to_string(),
        )
        .await
        .unwrap();
    let run = repo
        .create_collection_run(collection.id, user_id)
        .await
        .unwrap();
    let we = repo
        .create_workflow_execution(run.id, workflow.id, user_id)
        .await
        .unwrap();
    (we.id, workflow.id)
}

async fn create_test_room(repo: &PgRepo, user_id: Uuid) -> RoomRow {
    repo.create_room(
        user_id,
        None,
        &format!("test-room-{}", Uuid::new_v4().simple()),
        false,
        "claude-sonnet-4-5-20250929",
        3,
        10,
        false,
    )
    .await
    .unwrap()
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn merge_queue_insert_and_get() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let user_id = create_test_user(&repo).await;
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
        user_id,
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

    let user_id = create_test_user(&repo).await;
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
    repo.insert_queue_entry(Uuid::new_v4(), owner.clone(), repo_name.clone(), 1, 1, now, user_id)
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

    let user_id = create_test_user(&repo).await;
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
        user_id,
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

    let user_id = create_test_user(&repo).await;
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
        user_id,
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

    let user_id = create_test_user(&repo).await;
    let owner = "testowner".to_string();
    let repo_name = "testrepo".to_string();
    let now = Utc::now();

    // Insert entries with in_progress status
    repo.insert_queue_entry(Uuid::new_v4(), owner.clone(), repo_name.clone(), 1, 1, now, user_id)
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

    let user_id = create_test_user(&repo).await;
    let title = "Test Document".to_string();
    let content = "This is test content".to_string();
    let doc_type = "note".to_string();
    let ref_tag = "test-ref".to_string();
    let tags = vec!["tag1".to_string(), "tag2".to_string()];

    // Create document
    let doc = repo
        .create_document(
            user_id,
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

    let user_id = create_test_user(&repo).await;

    // Create document
    let doc = repo
        .create_document(
            user_id,
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

    let user_id = create_test_user(&repo).await;

    // Create document
    let doc = repo
        .create_document(
            user_id,
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

    let user_id = create_test_user(&repo).await;

    // Create multiple documents
    for i in 1..=3 {
        repo.create_document(
            user_id,
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
        user_id,
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
    let user_id = create_test_user(&repo).await;

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
    let user_id = create_test_user(&repo).await;

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
    let user_id = create_test_user(&repo).await;

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
    let user_id = create_test_user(&repo).await;

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
    let user_id = create_test_user(&repo).await;

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
    let user_id = create_test_user(&repo).await;

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
    repo.upsert_tool(tool1.clone()).await.unwrap();

    let tool2 = ToolRow {
        id: Uuid::new_v4(),
        name: "tool_c".to_string(),
        display_name: "Tool C".to_string(),
        description: "Third".to_string(),
        parameters: serde_json::json!({}),
        created_at: Utc::now(),
        version: 1,
    };
    repo.upsert_tool(tool2.clone()).await.unwrap();

    let tool3 = ToolRow {
        id: Uuid::new_v4(),
        name: "tool_b".to_string(),
        display_name: "Tool B".to_string(),
        description: "Second".to_string(),
        parameters: serde_json::json!({}),
        created_at: Utc::now(),
        version: 1,
    };
    repo.upsert_tool(tool3.clone()).await.unwrap();

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
    let user_id = create_test_user(&repo).await;

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
    repo.upsert_tool(tool_a.clone()).await.unwrap();

    let tool_b = ToolRow {
        id: Uuid::new_v4(),
        name: "tool_b".to_string(),
        display_name: "Tool B".to_string(),
        description: "B".to_string(),
        parameters: serde_json::json!({}),
        created_at: Utc::now(),
        version: 1,
    };
    repo.upsert_tool(tool_b.clone()).await.unwrap();

    let tool_c = ToolRow {
        id: Uuid::new_v4(),
        name: "tool_c".to_string(),
        display_name: "Tool C".to_string(),
        description: "C".to_string(),
        parameters: serde_json::json!({}),
        created_at: Utc::now(),
        version: 1,
    };
    repo.upsert_tool(tool_c.clone()).await.unwrap();

    let tool_d = ToolRow {
        id: Uuid::new_v4(),
        name: "tool_d".to_string(),
        display_name: "Tool D".to_string(),
        description: "D".to_string(),
        parameters: serde_json::json!({}),
        created_at: Utc::now(),
        version: 1,
    };
    repo.upsert_tool(tool_d.clone()).await.unwrap();

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
    let user_id = create_test_user(&repo).await;

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
    repo.upsert_tool(tool.clone()).await.unwrap();

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
    let user_id = create_test_user(&repo).await;

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

// ============================================================================
// 1A: Transaction Isolation Tests
// ============================================================================

#[test]
fn serialization_failure_detection() {
    // True for code "40001"
    let db_err = sqlx::Error::Database(Box::new(TestDbError("40001")));
    assert!(super::is_serialization_failure(&db_err));

    // False for code "23505" (unique violation)
    let unique_err = sqlx::Error::Database(Box::new(TestDbError("23505")));
    assert!(!super::is_serialization_failure(&unique_err));

    // False for non-database errors
    let io_err = sqlx::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, "test"));
    assert!(!super::is_serialization_failure(&io_err));
}

/// Minimal DatabaseError implementation for unit testing error classification.
#[derive(Debug)]
struct TestDbError(&'static str);

impl std::fmt::Display for TestDbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "test db error: {}", self.0)
    }
}

impl std::error::Error for TestDbError {}

impl sqlx::error::DatabaseError for TestDbError {
    fn message(&self) -> &str {
        "test error"
    }
    fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        self
    }
    fn as_error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
        self
    }
    fn into_error(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync + 'static> {
        self
    }
    fn kind(&self) -> sqlx::error::ErrorKind {
        sqlx::error::ErrorKind::Other
    }
    fn code(&self) -> Option<std::borrow::Cow<'_, str>> {
        Some(std::borrow::Cow::Borrowed(self.0))
    }
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn concurrent_set_agent_tools_serializable() {
    let db = TestDb::new_with_connections(4).await;
    let repo = PgRepo::new(db.pool.clone());

    let user = create_test_user(&repo).await;
    let agent = create_test_agent(&repo, user).await;

    // Create 4 tools
    let mut tool_ids = Vec::new();
    for i in 0..4 {
        let tool = ToolRow {
            id: Uuid::new_v4(),
            name: format!("concurrent_tool_{}", i),
            display_name: format!("Tool {}", i),
            description: "test".to_string(),
            parameters: serde_json::json!({}),
            created_at: Utc::now(),
            version: 1,
        };
        repo.upsert_tool(tool.clone()).await.unwrap();
        tool_ids.push(tool.id);
    }

    let set_a = vec![tool_ids[0], tool_ids[1]];
    let set_b = vec![tool_ids[2], tool_ids[3]];

    // Spawn concurrent writers
    let repo_a = PgRepo::new(db.pool.clone());
    let repo_b = PgRepo::new(db.pool.clone());
    let agent_id = agent.id;
    let sa = set_a.clone();
    let sb = set_b.clone();

    let (ra, rb) = tokio::join!(
        tokio::spawn(async move { repo_a.set_agent_tools(agent_id, sa).await }),
        tokio::spawn(async move { repo_b.set_agent_tools(agent_id, sb).await }),
    );
    ra.unwrap().unwrap();
    rb.unwrap().unwrap();

    // One writer must have won cleanly — result is exactly 2 tools
    let tools = repo.get_agent_tools(agent.id).await.unwrap();
    assert_eq!(
        tools.len(),
        2,
        "expected exactly one writer's set, got {}",
        tools.len()
    );

    // The tools should be one of the two sets, not a corrupt mix
    let tool_id_set: std::collections::HashSet<Uuid> = tools.iter().map(|t| t.id).collect();
    let is_set_a = set_a.iter().all(|id| tool_id_set.contains(id));
    let is_set_b = set_b.iter().all(|id| tool_id_set.contains(id));
    assert!(
        is_set_a || is_set_b,
        "tools should be exactly set_a or set_b, not a mix"
    );

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn concurrent_set_room_members_serializable() {
    let db = TestDb::new_with_connections(4).await;
    let repo = PgRepo::new(db.pool.clone());

    let user = create_test_user(&repo).await;
    let room = create_test_room(&repo, user).await;

    // Create 3 agents
    let agent_a = create_test_agent(&repo, user).await;
    let agent_b = create_test_agent(&repo, user).await;
    let agent_c = create_test_agent(&repo, user).await;

    let set_1 = vec![
        RoomMemberInput {
            agent_id: agent_a.id,
            display_name: Some("Agent A".to_string()),
            role_description: "Role A".to_string(),
            display_order: 1,
        },
        RoomMemberInput {
            agent_id: agent_b.id,
            display_name: Some("Agent B".to_string()),
            role_description: "Role B".to_string(),
            display_order: 2,
        },
    ];
    let set_2 = vec![RoomMemberInput {
        agent_id: agent_c.id,
        display_name: Some("Agent C".to_string()),
        role_description: "Role C".to_string(),
        display_order: 1,
    }];

    let repo_1 = PgRepo::new(db.pool.clone());
    let repo_2 = PgRepo::new(db.pool.clone());
    let room_id = room.id;
    let s1 = set_1.clone();
    let s2 = set_2.clone();

    let (r1, r2) = tokio::join!(
        tokio::spawn(async move { repo_1.set_room_members(room_id, &s1).await }),
        tokio::spawn(async move { repo_2.set_room_members(room_id, &s2).await }),
    );
    r1.unwrap().unwrap();
    r2.unwrap().unwrap();

    let members = repo.list_room_members(room.id).await.unwrap();
    // One writer won — result is either 2 members (set_1) or 1 member (set_2)
    assert!(
        members.len() == 2 || members.len() == 1,
        "expected 1 or 2 members, got {}",
        members.len()
    );

    db.cleanup().await;
}

// ============================================================================
// 1B: Agent Execution Repository Tests
// ============================================================================

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn agent_execution_create_and_get() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let user = create_test_user(&repo).await;
    let agent = create_test_agent(&repo, user).await;

    let exec = repo
        .create_agent_execution(
            agent.id,
            None,  // workflow_step_id
            false, // is_interactive
            None,  // parent_agent_execution_id
            "You are a test agent.",
            "What is 2+2?",
            None, // selected_mode_id
            None, // room_session_id
            None, // speaker_order
        )
        .await
        .unwrap();

    assert_eq!(exec.agent_id, agent.id);
    assert_eq!(exec.status, "running");
    assert_eq!(exec.input, "What is 2+2?");
    assert_eq!(exec.system_prompt_rendered, "You are a test agent.");
    assert!(!exec.is_interactive);
    assert!(!exec.is_exemplary);
    assert!(exec.output.is_none());
    assert!(exec.completed_at.is_none());
    assert!(exec.workflow_step_id.is_none());

    // Get by ID
    let fetched = repo.get_agent_execution(exec.id).await.unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().id, exec.id);

    // Nonexistent → None
    let missing = repo.get_agent_execution(Uuid::new_v4()).await.unwrap();
    assert!(missing.is_none());

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn agent_execution_status_transitions() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let user = create_test_user(&repo).await;
    let agent = create_test_agent(&repo, user).await;

    // --- completed: sets completed_at + stores output ---
    let exec1 = repo
        .create_agent_execution(
            agent.id, None, false, None, "sys", "input1", None, None, None,
        )
        .await
        .unwrap();
    let updated = repo
        .update_agent_execution_status(
            exec1.id,
            "completed",
            Some("The answer is 4".to_string()),
            Some(serde_json::json!({"answer": 4})),
        )
        .await
        .unwrap();
    assert_eq!(updated.status, "completed");
    assert!(updated.completed_at.is_some());
    assert_eq!(updated.output.as_deref(), Some("The answer is 4"));
    assert_eq!(
        updated.structured_output,
        Some(serde_json::json!({"answer": 4}))
    );

    // --- failed: sets completed_at ---
    let exec2 = repo
        .create_agent_execution(
            agent.id, None, false, None, "sys", "input2", None, None, None,
        )
        .await
        .unwrap();
    let failed = repo
        .update_agent_execution_status(exec2.id, "failed", Some("error".to_string()), None)
        .await
        .unwrap();
    assert_eq!(failed.status, "failed");
    assert!(failed.completed_at.is_some());

    // --- COALESCE: passing None preserves previous output ---
    let exec3 = repo
        .create_agent_execution(
            agent.id, None, false, None, "sys", "input3", None, None, None,
        )
        .await
        .unwrap();
    // First set output
    repo.update_agent_execution_status(exec3.id, "running", Some("partial".to_string()), None)
        .await
        .unwrap();
    // Now update status without changing output (pass None)
    let preserved = repo
        .update_agent_execution_status(exec3.id, "completed", None, None)
        .await
        .unwrap();
    assert_eq!(preserved.status, "completed");
    assert_eq!(preserved.output.as_deref(), Some("partial"));

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn agent_execution_list_by_user() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let user_a = create_test_user(&repo).await;
    let user_b = create_test_user(&repo).await;
    let agent = create_test_agent(&repo, user_a).await;

    // Create execution chain for user_a
    let (we_id, _) = create_execution_chain(&repo, user_a).await;

    // Create 3 interactive executions and link to workflow_execution
    let mut exec_ids = Vec::new();
    for _ in 0..3 {
        let exec = repo
            .create_agent_execution(agent.id, None, true, None, "sys", "input", None, None, None)
            .await
            .unwrap();
        // Link to workflow_execution via raw SQL
        sqlx::query("UPDATE agent_executions SET workflow_execution_id = $1 WHERE id = $2")
            .bind(we_id)
            .bind(exec.id)
            .execute(&db.pool)
            .await
            .unwrap();
        exec_ids.push(exec.id);
    }

    // Update one to completed
    repo.update_agent_execution_status(exec_ids[0], "completed", Some("done".to_string()), None)
        .await
        .unwrap();

    // List all for user_a
    let all = repo.list_agent_executions(user_a, None).await.unwrap();
    assert_eq!(all.len(), 3);

    // List completed only
    let completed = repo
        .list_agent_executions(user_a, Some("completed".to_string()))
        .await
        .unwrap();
    assert_eq!(completed.len(), 1);

    // user_b sees nothing (tenant isolation)
    let empty = repo.list_agent_executions(user_b, None).await.unwrap();
    assert!(empty.is_empty());

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn agent_execution_completed_for_step_ids() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let user = create_test_user(&repo).await;
    let agent = create_test_agent(&repo, user).await;
    let workflow = create_test_workflow(&repo, user).await;
    let step_a = create_test_step(&repo, workflow.id, agent.id).await;
    let step_b = create_test_step(&repo, workflow.id, agent.id).await;

    // completed + non-interactive (should match)
    let e1 = repo
        .create_agent_execution(
            agent.id,
            Some(step_a.id),
            false,
            None,
            "s",
            "i",
            None,
            None,
            None,
        )
        .await
        .unwrap();
    repo.update_agent_execution_status(e1.id, "completed", Some("ok".to_string()), None)
        .await
        .unwrap();

    let e2 = repo
        .create_agent_execution(
            agent.id,
            Some(step_b.id),
            false,
            None,
            "s",
            "i",
            None,
            None,
            None,
        )
        .await
        .unwrap();
    repo.update_agent_execution_status(e2.id, "completed", Some("ok".to_string()), None)
        .await
        .unwrap();

    // running + non-interactive (should NOT match)
    repo.create_agent_execution(
        agent.id,
        Some(step_a.id),
        false,
        None,
        "s",
        "i",
        None,
        None,
        None,
    )
    .await
    .unwrap();

    // completed + interactive (should NOT match)
    let e4 = repo
        .create_agent_execution(
            agent.id,
            Some(step_a.id),
            true,
            None,
            "s",
            "i",
            None,
            None,
            None,
        )
        .await
        .unwrap();
    repo.update_agent_execution_status(e4.id, "completed", Some("ok".to_string()), None)
        .await
        .unwrap();

    let results = repo
        .list_completed_executions_for_step_ids(&[step_a.id, step_b.id])
        .await
        .unwrap();
    assert_eq!(results.len(), 2);

    // Empty array → empty
    let empty = repo
        .list_completed_executions_for_step_ids(&[])
        .await
        .unwrap();
    assert!(empty.is_empty());

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn agent_execution_routing_update() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let user = create_test_user(&repo).await;
    let agent = create_test_agent(&repo, user).await;

    let exec = repo
        .create_agent_execution(agent.id, None, false, None, "s", "i", None, None, None)
        .await
        .unwrap();

    let routing = serde_json::json!({"intent": "decompose", "confidence": 0.95});

    // Create a real document so the FK constraint is satisfied
    let doc = repo
        .create_document(
            user,
            None,
            "Routing Doc".to_string(),
            "Content".to_string(),
            "note".to_string(),
            format!("ref-{}", Uuid::new_v4().simple()),
            vec![],
        )
        .await
        .unwrap();

    repo.update_agent_execution_routing(exec.id, &routing, Some(doc.id))
        .await
        .unwrap();

    let fetched = repo.get_agent_execution(exec.id).await.unwrap().unwrap();
    assert_eq!(fetched.routing_analysis, Some(routing));
    assert_eq!(fetched.selected_routing_document_id, Some(doc.id));

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn agent_execution_exemplary_toggle() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let user = create_test_user(&repo).await;
    let agent = create_test_agent(&repo, user).await;
    let workflow = create_test_workflow(&repo, user).await;
    let step = create_test_step(&repo, workflow.id, agent.id).await;

    let exec = repo
        .create_agent_execution(
            agent.id,
            Some(step.id),
            false,
            None,
            "s",
            "i",
            None,
            None,
            None,
        )
        .await
        .unwrap();
    repo.update_agent_execution_status(exec.id, "completed", Some("ok".to_string()), None)
        .await
        .unwrap();

    // Toggle on
    let toggled = repo.set_execution_exemplary(exec.id, true).await.unwrap();
    assert!(toggled.is_exemplary);

    let exemplary = repo
        .list_exemplary_executions(agent.id, None, 10)
        .await
        .unwrap();
    assert_eq!(exemplary.len(), 1);

    // Filter by step
    let by_step = repo
        .list_exemplary_executions(agent.id, Some(step.id), 10)
        .await
        .unwrap();
    assert_eq!(by_step.len(), 1);

    // Toggle off
    repo.set_execution_exemplary(exec.id, false).await.unwrap();

    let empty = repo
        .list_exemplary_executions(agent.id, None, 10)
        .await
        .unwrap();
    assert!(empty.is_empty());

    db.cleanup().await;
}

// ============================================================================
// 1C: Token Ledger Tests
// ============================================================================

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn token_ledger_insert_and_retrieve() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let user = create_test_user(&repo).await;

    let entry = repo
        .insert_ledger_entry(user, None, "claude-sonnet-4-5-20250929", 1000, 500, 0.015)
        .await
        .unwrap();

    assert_eq!(entry.user_id, user);
    assert_eq!(entry.model_id, "claude-sonnet-4-5-20250929");
    assert_eq!(entry.input_tokens, 1000);
    assert_eq!(entry.output_tokens, 500);
    assert!((entry.cost_usd - 0.015).abs() < 1e-6);

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn token_ledger_user_spend_aggregation() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let user = create_test_user(&repo).await;
    let other_user = create_test_user(&repo).await;

    // Insert 3 entries
    repo.insert_ledger_entry(user, None, "model-a", 100, 50, 0.010)
        .await
        .unwrap();
    repo.insert_ledger_entry(user, None, "model-a", 200, 100, 0.020)
        .await
        .unwrap();

    // Small sleep to ensure different timestamps for the `since` filter
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let midpoint = Utc::now();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    repo.insert_ledger_entry(user, None, "model-b", 300, 150, 0.030)
        .await
        .unwrap();

    // Total: 0.060
    let total = repo.get_user_spend(user, None).await.unwrap();
    assert!(
        (total - 0.060).abs() < 1e-4,
        "expected ~0.060, got {}",
        total
    );

    // Since midpoint: only the last entry (0.030)
    let recent = repo.get_user_spend(user, Some(midpoint)).await.unwrap();
    assert!(
        (recent - 0.030).abs() < 1e-4,
        "expected ~0.030, got {}",
        recent
    );

    // Different user: 0.0
    let other = repo.get_user_spend(other_user, None).await.unwrap();
    assert!((other).abs() < 1e-6);

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn token_ledger_model_breakdown() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let user = create_test_user(&repo).await;

    // sonnet: 2 entries
    repo.insert_ledger_entry(user, None, "claude-sonnet", 1000, 500, 0.01)
        .await
        .unwrap();
    repo.insert_ledger_entry(user, None, "claude-sonnet", 2000, 800, 0.02)
        .await
        .unwrap();

    // haiku: 1 entry
    repo.insert_ledger_entry(user, None, "claude-haiku", 500, 200, 0.001)
        .await
        .unwrap();

    // gpt-4: 1 entry (highest cost)
    repo.insert_ledger_entry(user, None, "gpt-4", 3000, 1000, 0.05)
        .await
        .unwrap();

    let breakdown = repo.get_model_breakdown(user, None).await.unwrap();
    assert_eq!(breakdown.len(), 3);

    // Ordered by total_cost_usd DESC: gpt-4 (0.05), sonnet (0.03), haiku (0.001)
    assert_eq!(breakdown[0].model_id, "gpt-4");
    assert_eq!(breakdown[0].call_count, 1);

    assert_eq!(breakdown[1].model_id, "claude-sonnet");
    assert_eq!(breakdown[1].call_count, 2);
    assert_eq!(breakdown[1].total_input_tokens, 3000);
    assert_eq!(breakdown[1].total_output_tokens, 1300);

    assert_eq!(breakdown[2].model_id, "claude-haiku");
    assert_eq!(breakdown[2].call_count, 1);

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn token_ledger_concurrent_inserts() {
    let db = TestDb::new_with_connections(4).await;
    let repo = PgRepo::new(db.pool.clone());

    let user = create_test_user(&repo).await;
    let user_id = user;

    // Spawn 10 concurrent inserts of 0.001 each
    let mut handles = Vec::new();
    for _ in 0..10 {
        let r = PgRepo::new(db.pool.clone());
        handles.push(tokio::spawn(async move {
            r.insert_ledger_entry(user_id, None, "model", 100, 50, 0.001)
                .await
                .unwrap();
        }));
    }
    for h in handles {
        h.await.unwrap();
    }

    let total = repo.get_user_spend(user_id, None).await.unwrap();
    assert!(
        (total - 0.010).abs() < 1e-4,
        "expected ~0.010, got {}",
        total
    );

    db.cleanup().await;
}

// ============================================================================
// 1D: Workflow & Room Tests
// ============================================================================

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn workflow_set_edges_atomic() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let user = create_test_user(&repo).await;
    let workflow = create_test_workflow(&repo, user).await;
    let agent = create_test_agent(&repo, user).await;

    let step_a = create_test_step(&repo, workflow.id, agent.id).await;
    let step_b = create_test_step(&repo, workflow.id, agent.id).await;
    let step_c = create_test_step(&repo, workflow.id, agent.id).await;

    // Set A→B, B→C
    repo.set_edges(
        workflow.id,
        vec![
            WorkflowStepEdgeRow {
                id: Uuid::new_v4(),
                from_step_id: step_a.id,
                to_step_id: step_b.id,
                from_output_port: None,
                to_input_port: None,
                transform_jsonpath: None,
                condition_type: None,
                condition_value: None,
                edge_label: None,
                workflow_id: workflow.id,
            },
            WorkflowStepEdgeRow {
                id: Uuid::new_v4(),
                from_step_id: step_b.id,
                to_step_id: step_c.id,
                from_output_port: None,
                to_input_port: None,
                transform_jsonpath: None,
                condition_type: None,
                condition_value: None,
                edge_label: None,
                workflow_id: workflow.id,
            },
        ],
    )
    .await
    .unwrap();

    let edges = repo.list_edges(workflow.id).await.unwrap();
    assert_eq!(edges.len(), 2);

    // Replace with A→C
    repo.set_edges(
        workflow.id,
        vec![WorkflowStepEdgeRow {
            id: Uuid::new_v4(),
            from_step_id: step_a.id,
            to_step_id: step_c.id,
            from_output_port: None,
            to_input_port: None,
            transform_jsonpath: None,
            condition_type: None,
            condition_value: None,
            edge_label: None,
            workflow_id: workflow.id,
        }],
    )
    .await
    .unwrap();

    let edges = repo.list_edges(workflow.id).await.unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].from_step_id, step_a.id);
    assert_eq!(edges[0].to_step_id, step_c.id);

    // Clear all
    repo.set_edges(workflow.id, vec![]).await.unwrap();
    let edges = repo.list_edges(workflow.id).await.unwrap();
    assert!(edges.is_empty());

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn step_document_add_remove_idempotent() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let user = create_test_user(&repo).await;
    let workflow = create_test_workflow(&repo, user).await;
    let agent = create_test_agent(&repo, user).await;
    let step = create_test_step(&repo, workflow.id, agent.id).await;

    let doc1 = repo
        .create_document(
            user,
            None,
            "Doc 1".to_string(),
            "Content".to_string(),
            "note".to_string(),
            format!("ref-{}", Uuid::new_v4().simple()),
            vec![],
        )
        .await
        .unwrap();
    let doc2 = repo
        .create_document(
            user,
            None,
            "Doc 2".to_string(),
            "Content".to_string(),
            "note".to_string(),
            format!("ref-{}", Uuid::new_v4().simple()),
            vec![],
        )
        .await
        .unwrap();

    // Add doc1 twice → idempotent (ON CONFLICT DO NOTHING)
    repo.add_step_document(step.id, doc1.id).await.unwrap();
    repo.add_step_document(step.id, doc1.id).await.unwrap();
    let docs = repo.list_step_documents(step.id).await.unwrap();
    assert_eq!(docs.len(), 1);

    // Add doc2
    repo.add_step_document(step.id, doc2.id).await.unwrap();
    let docs = repo.list_step_documents(step.id).await.unwrap();
    assert_eq!(docs.len(), 2);

    // Remove doc1, then again (idempotent)
    repo.remove_step_document(step.id, doc1.id).await.unwrap();
    repo.remove_step_document(step.id, doc1.id).await.unwrap();
    let docs = repo.list_step_documents(step.id).await.unwrap();
    assert_eq!(docs.len(), 1);

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn room_session_lifecycle() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let user = create_test_user(&repo).await;
    let room = create_test_room(&repo, user).await;

    // Create session
    let session = repo.create_room_session(room.id).await.unwrap();
    assert_eq!(session.status, "active");
    assert_eq!(session.current_turn, 0);
    assert!(session.transcript_summary.is_none());
    assert!(session.completed_at.is_none());

    // Increment turn
    let turn1 = repo.increment_room_session_turn(session.id).await.unwrap();
    assert_eq!(turn1, 1);
    let turn2 = repo.increment_room_session_turn(session.id).await.unwrap();
    assert_eq!(turn2, 2);

    // Set summary
    repo.set_transcript_summary(session.id, "Discussion about testing.")
        .await
        .unwrap();
    let fetched = repo.get_room_session(session.id).await.unwrap().unwrap();
    assert_eq!(
        fetched.transcript_summary.as_deref(),
        Some("Discussion about testing.")
    );

    // Complete session
    repo.update_room_session_status(session.id, "completed")
        .await
        .unwrap();
    let done = repo.get_room_session(session.id).await.unwrap().unwrap();
    assert_eq!(done.status, "completed");
    assert!(done.completed_at.is_some());

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn room_transcript_join_ordering() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let user = create_test_user(&repo).await;
    let room = create_test_room(&repo, user).await;
    let agent_a = create_test_agent(&repo, user).await;
    let agent_b = create_test_agent(&repo, user).await;

    // Set room members with display names
    repo.set_room_members(
        room.id,
        &[
            RoomMemberInput {
                agent_id: agent_a.id,
                display_name: Some("Alice".to_string()),
                role_description: "Architect".to_string(),
                display_order: 1,
            },
            RoomMemberInput {
                agent_id: agent_b.id,
                display_name: Some("Bob".to_string()),
                role_description: "Reviewer".to_string(),
                display_order: 2,
            },
        ],
    )
    .await
    .unwrap();

    let session = repo.create_room_session(room.id).await.unwrap();

    // Create executions per agent
    let exec_a = repo
        .create_agent_execution(
            agent_a.id,
            None,
            false,
            None,
            "sys",
            "input",
            None,
            Some(session.id),
            Some(1),
        )
        .await
        .unwrap();
    let exec_b = repo
        .create_agent_execution(
            agent_b.id,
            None,
            false,
            None,
            "sys",
            "input",
            None,
            Some(session.id),
            Some(2),
        )
        .await
        .unwrap();

    // Create messages (agent_a first, then agent_b)
    repo.create_execution_message(exec_a.id, "assistant", "Hello from Alice", None, 10, 5)
        .await
        .unwrap();

    // Small delay to ensure ordering
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    repo.create_execution_message(exec_b.id, "assistant", "Hello from Bob", None, 10, 5)
        .await
        .unwrap();

    let transcript = repo.get_room_transcript(session.id).await.unwrap();
    assert_eq!(transcript.len(), 2);

    // Ordered by created_at ASC
    assert_eq!(transcript[0].agent_name, "Alice");
    assert_eq!(transcript[0].content, "Hello from Alice");
    assert_eq!(transcript[0].role_description, "Architect");

    assert_eq!(transcript[1].agent_name, "Bob");
    assert_eq!(transcript[1].content, "Hello from Bob");
    assert_eq!(transcript[1].role_description, "Reviewer");

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn delete_workflow_cascades() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let user = create_test_user(&repo).await;
    let workflow = create_test_workflow(&repo, user).await;
    let agent = create_test_agent(&repo, user).await;
    let step_a = create_test_step(&repo, workflow.id, agent.id).await;
    let step_b = create_test_step(&repo, workflow.id, agent.id).await;

    // Add edges
    repo.set_edges(
        workflow.id,
        vec![WorkflowStepEdgeRow {
            id: Uuid::new_v4(),
            from_step_id: step_a.id,
            to_step_id: step_b.id,
            from_output_port: None,
            to_input_port: None,
            transform_jsonpath: None,
            condition_type: None,
            condition_value: None,
            edge_label: None,
            workflow_id: workflow.id,
        }],
    )
    .await
    .unwrap();

    // Add step document
    let doc = repo
        .create_document(
            user,
            None,
            "Cascade Doc".to_string(),
            "Content".to_string(),
            "note".to_string(),
            format!("ref-{}", Uuid::new_v4().simple()),
            vec![],
        )
        .await
        .unwrap();
    repo.add_step_document(step_a.id, doc.id).await.unwrap();

    // Verify everything exists
    assert!(repo.get_workflow(workflow.id).await.unwrap().is_some());
    assert_eq!(repo.list_steps(workflow.id).await.unwrap().len(), 2);
    assert_eq!(repo.list_edges(workflow.id).await.unwrap().len(), 1);

    // Delete workflow
    repo.delete_workflow(workflow.id).await.unwrap();

    // Everything cascaded
    assert!(repo.get_workflow(workflow.id).await.unwrap().is_none());
    assert!(repo.list_steps(workflow.id).await.unwrap().is_empty());
    assert!(repo.list_edges(workflow.id).await.unwrap().is_empty());

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn set_room_members_replaces_all() {
    let db = TestDb::new().await;
    let repo = PgRepo::new(db.pool.clone());

    let user = create_test_user(&repo).await;
    let room = create_test_room(&repo, user).await;
    let agent_a = create_test_agent(&repo, user).await;
    let agent_b = create_test_agent(&repo, user).await;
    let agent_c = create_test_agent(&repo, user).await;

    // Set [A, B]
    repo.set_room_members(
        room.id,
        &[
            RoomMemberInput {
                agent_id: agent_a.id,
                display_name: Some("A".to_string()),
                role_description: "Role A".to_string(),
                display_order: 1,
            },
            RoomMemberInput {
                agent_id: agent_b.id,
                display_name: Some("B".to_string()),
                role_description: "Role B".to_string(),
                display_order: 2,
            },
        ],
    )
    .await
    .unwrap();
    let members = repo.list_room_members(room.id).await.unwrap();
    assert_eq!(members.len(), 2);
    assert_eq!(members[0].display_order, 1);
    assert_eq!(members[1].display_order, 2);

    // Replace with [C]
    repo.set_room_members(
        room.id,
        &[RoomMemberInput {
            agent_id: agent_c.id,
            display_name: Some("C".to_string()),
            role_description: "Role C".to_string(),
            display_order: 1,
        }],
    )
    .await
    .unwrap();
    let members = repo.list_room_members(room.id).await.unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].agent_id, agent_c.id);
    assert_eq!(members[0].role_description, "Role C");

    // Clear all
    repo.set_room_members(room.id, &[]).await.unwrap();
    let members = repo.list_room_members(room.id).await.unwrap();
    assert!(members.is_empty());

    db.cleanup().await;
}
