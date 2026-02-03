//! Tests for test database utilities

use super::*;

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn test_db_creates_and_cleans_up() {
    let db = TestDb::new().await;
    let row: (i32,) = sqlx::query_as("SELECT 1").fetch_one(&db.pool).await.unwrap();
    assert_eq!(row.0, 1);
    let name = db.db_name.clone();
    db.cleanup().await;

    // Verify the database was dropped
    let admin_pool = PgPoolOptions::new().max_connections(1).connect(&admin_url()).await.unwrap();
    let exists: (bool,) = sqlx::query_as(&format!("SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname = '{}')", name))
        .fetch_one(&admin_pool)
        .await
        .unwrap();
    assert!(!exists.0);
    admin_pool.close().await;
}

#[tokio::test]
#[ignore = "requires running Postgres"]
async fn multiple_test_dbs_coexist() {
    let db1 = TestDb::new().await;
    let db2 = TestDb::new().await;
    assert_ne!(db1.db_name, db2.db_name);
    db1.cleanup().await;
    db2.cleanup().await;
}

#[test]
fn replace_db_name_works() {
    assert_eq!(replace_db_name("postgres://user:pass@localhost:5432/mydb", "newdb"), "postgres://user:pass@localhost:5432/newdb");
    assert_eq!(
        replace_db_name("postgres://user:pass@localhost:5432/mydb?sslmode=require", "newdb"),
        "postgres://user:pass@localhost:5432/newdb?sslmode=require"
    );
}
