//! Tests for PostgreSQL repository

use super::*;
use crate::db::test_utils::TestDb;
use crate::types::{Priority, Task, TaskId, TaskStatus};

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
    repo.insert_queue_entry(id, owner.clone(), repo_name.clone(), pr_number, position, now).await.unwrap();

    // Get entries
    let entries = repo.get_queue_entries(owner.clone(), repo_name.clone()).await.unwrap();
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
    let pos1 = repo.get_next_position(owner.clone(), repo_name.clone()).await.unwrap();
    assert_eq!(pos1, 1);

    // Insert entry at position 1
    repo.insert_queue_entry(Uuid::new_v4(), owner.clone(), repo_name.clone(), 1, 1, now).await.unwrap();

    // Get next position (should be 2)
    let pos2 = repo.get_next_position(owner.clone(), repo_name.clone()).await.unwrap();
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
    repo.insert_queue_entry(Uuid::new_v4(), owner.clone(), repo_name.clone(), pr_number, 1, now).await.unwrap();

    // Delete entry
    let deleted = repo.delete_queue_entry(owner.clone(), repo_name.clone(), pr_number).await.unwrap();
    assert!(deleted);

    // Verify deletion
    let entries = repo.get_queue_entries(owner.clone(), repo_name.clone()).await.unwrap();
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
    repo.insert_queue_entry(Uuid::new_v4(), owner.clone(), repo_name.clone(), pr_number, 1, now).await.unwrap();

    // Update status
    let updated = repo
        .update_entry_status(owner.clone(), repo_name.clone(), pr_number, "in_progress".to_string(), None, now)
        .await
        .unwrap();
    assert!(updated);

    // Verify update
    let entries = repo.get_queue_entries(owner.clone(), repo_name.clone()).await.unwrap();
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
    repo.insert_queue_entry(Uuid::new_v4(), owner.clone(), repo_name.clone(), 1, 1, now).await.unwrap();
    repo.update_entry_status(owner.clone(), repo_name.clone(), 1, "in_progress".to_string(), None, now).await.unwrap();

    // Reset interrupted
    let count = repo.reset_interrupted(owner.clone(), repo_name.clone(), now).await.unwrap();
    assert_eq!(count, 1);

    // Verify reset
    let entries = repo.get_queue_entries(owner.clone(), repo_name.clone()).await.unwrap();
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
    let user = repo.create_github_user(email, github_id, github_login, token).await.unwrap();
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
    let user = repo.create_user("test@example.com", "password_hash").await.unwrap();

    // Link GitHub
    let github_id = 789;
    let github_login = "linkeduser";
    let token = "encrypted_token";
    repo.link_github(user.id, github_id, github_login, token).await.unwrap();

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
    let user = repo.create_user("taskuser@example.com", "hash").await.unwrap();

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
    let doc = repo.create_document(user_id, None, title.clone(), content.clone(), doc_type, ref_tag.clone(), tags).await.unwrap();

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
        .update_document(doc.id, Some("Updated Content".to_string()), Some("Updated Title".to_string()), None)
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
        .create_document(user_id, None, "Title".to_string(), "Content".to_string(), "note".to_string(), "ref".to_string(), vec![])
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
        repo.create_document(user_id, None, format!("Doc {}", i), "Content".to_string(), "note".to_string(), format!("ref-{}", i), vec![])
            .await
            .unwrap();
    }

    // List documents
    let docs = repo.list_documents(user_id).await.unwrap();
    assert_eq!(docs.len(), 3);

    db.cleanup().await;
}
