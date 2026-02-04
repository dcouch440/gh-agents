//! Tests for database queries

use super::*;

use crate::db::test_utils::TestDb;
use crate::types::UserId;

fn test_user_id() -> UserId {
    UserId(uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap())
}

fn create_test_task() -> Task {
    Task {
        id: TaskId::new(),
        slice_id: None,
        title: "Test task".to_string(),
        description: "A test task".to_string(),
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
    }
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn can_insert_and_get_task() {
    let db = TestDb::new().await;
    let task = create_test_task();

    insert_task(&db.pool, test_user_id(), &task).await.unwrap();

    let retrieved = get_task(&db.pool, test_user_id(), &task.id).await.unwrap();
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.title, task.title);

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn can_update_task_status() {
    let db = TestDb::new().await;
    let task = create_test_task();

    insert_task(&db.pool, test_user_id(), &task).await.unwrap();
    update_task_status(&db.pool, &task.id, TaskStatus::InProgress)
        .await
        .unwrap();

    let retrieved = get_task(&db.pool, test_user_id(), &task.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.status, TaskStatus::InProgress);

    db.cleanup().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn can_list_tasks_by_status() {
    let db = TestDb::new().await;

    let task1 = create_test_task();
    let task2 = create_test_task();

    insert_task(&db.pool, test_user_id(), &task1).await.unwrap();
    insert_task(&db.pool, test_user_id(), &task2).await.unwrap();

    let pending = list_tasks_by_status(&db.pool, TaskStatus::Pending)
        .await
        .unwrap();
    assert!(pending.len() >= 2);

    db.cleanup().await;
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
