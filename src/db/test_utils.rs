//! Per-test Postgres database helper.
//!
//! Each `TestDb` creates a uniquely-named database (`nexor_test_{uuid}`),
//! runs all migrations on it, and drops it on cleanup.

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;
use tokio::sync::{OnceCell, OwnedSemaphorePermit, Semaphore};
use uuid::Uuid;

fn admin_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://nexor:nexor@localhost:5432/nexor".to_string())
}

/// Shared admin pool across all tests to avoid connection exhaustion.
static ADMIN_POOL: OnceCell<PgPool> = OnceCell::const_new();

/// Limit concurrent test DBs to stay under Postgres max_connections (100).
/// Each test holds 1 connection + the shared admin pool uses up to 5,
/// so we allow up to 80 concurrent tests as a safe margin.
static DB_SEMAPHORE: once_cell::sync::Lazy<std::sync::Arc<Semaphore>> =
    once_cell::sync::Lazy::new(|| std::sync::Arc::new(Semaphore::new(80)));

async fn shared_admin_pool() -> &'static PgPool {
    ADMIN_POOL
        .get_or_init(|| async {
            PgPoolOptions::new()
                .max_connections(10)
                .acquire_timeout(Duration::from_secs(60))
                .idle_timeout(Duration::from_secs(120))
                .connect(&admin_url())
                .await
                .expect("Failed to connect shared admin pool")
        })
        .await
}

/// An isolated Postgres database for a single test.
pub struct TestDb {
    pub pool: PgPool,
    db_name: String,
    /// Held for the lifetime of the TestDb to limit total open connections.
    _permit: OwnedSemaphorePermit,
}

impl TestDb {
    /// Create a fresh test database with all migrations applied.
    pub async fn new() -> Self {
        let permit = std::sync::Arc::clone(&DB_SEMAPHORE)
            .acquire_owned()
            .await
            .unwrap();

        let admin_pool = shared_admin_pool().await;
        let db_name = format!("nexor_test_{}", Uuid::new_v4().simple());

        sqlx::query(&format!("CREATE DATABASE \"{}\"", db_name))
            .execute(admin_pool)
            .await
            .unwrap();

        let test_url = replace_db_name(&admin_url(), &db_name);

        let pool = PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Duration::from_secs(5))
            .connect(&test_url)
            .await
            .unwrap();

        sqlx::migrate!().run(&pool).await.unwrap();

        Self {
            pool,
            db_name,
            _permit: permit,
        }
    }

    /// Drop the test database. Must be called at the end of each test.
    pub async fn cleanup(self) {
        self.pool.close().await;

        let admin_pool = shared_admin_pool().await;

        // Terminate any remaining connections to the test database
        sqlx::query(&format!(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}'",
            self.db_name
        ))
        .execute(admin_pool)
        .await
        .ok();

        sqlx::query(&format!("DROP DATABASE IF EXISTS \"{}\"", self.db_name))
            .execute(admin_pool)
            .await
            .ok();

        // _permit is dropped here, releasing a slot for another test
    }
}

/// Replace the database name component in a Postgres URL.
fn replace_db_name(url: &str, new_db: &str) -> String {
    // URL format: postgres://user:pass@host:port/dbname
    if let Some(pos) = url.rfind('/') {
        let base = &url[..pos];
        let after_db = url[pos + 1..]
            .find('?')
            .map(|q| &url[pos + 1 + q..])
            .unwrap_or("");
        if after_db.is_empty() {
            format!("{}/{}", base, new_db)
        } else {
            format!("{}/{}{}", base, new_db, after_db)
        }
    } else {
        url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn test_db_creates_and_cleans_up() {
        let db = TestDb::new().await;
        let row: (i32,) = sqlx::query_as("SELECT 1")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(row.0, 1);
        let name = db.db_name.clone();
        db.cleanup().await;

        // Verify the database was dropped
        let admin_pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&admin_url())
            .await
            .unwrap();
        let exists: (bool,) = sqlx::query_as(&format!(
            "SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname = '{}')",
            name
        ))
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
        assert_eq!(
            replace_db_name("postgres://user:pass@localhost:5432/mydb", "newdb"),
            "postgres://user:pass@localhost:5432/newdb"
        );
        assert_eq!(
            replace_db_name(
                "postgres://user:pass@localhost:5432/mydb?sslmode=require",
                "newdb"
            ),
            "postgres://user:pass@localhost:5432/newdb?sslmode=require"
        );
    }
}
