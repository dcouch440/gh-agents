//! Task dependency tracking.
//!
//! Tracks dependencies between tasks and determines which tasks are blocked.
//! Used by the scheduler to ensure work is done in the correct order.

use crate::types::{Task, TaskId, TaskStatus};
use sqlx::SqlitePool;
use std::collections::HashSet;
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur during dependency tracking
#[derive(Error, Debug)]
pub enum DependencyError {
    #[error("database error: {0}")]
    DatabaseError(String),

    #[error("circular dependency detected: {0}")]
    CircularDependency(String),

    #[error("task not found: {0}")]
    TaskNotFound(TaskId),
}

/// Tracks task dependencies and determines blocked status
pub struct DependencyTracker {
    pool: SqlitePool,
}

impl DependencyTracker {
    /// Create a new DependencyTracker
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Check if a task is blocked by incomplete dependencies
    pub async fn is_blocked(&self, task: &Task) -> Result<bool, DependencyError> {
        if task.depends_on.is_empty() {
            return Ok(false);
        }

        // Check status of all dependencies
        for dep_id in &task.depends_on {
            let status = self.get_task_status(dep_id).await?;

            match status {
                Some(TaskStatus::Completed) => {
                    // Dependency satisfied
                    continue;
                }
                Some(_) => {
                    // Dependency not complete
                    tracing::debug!(
                        task_id = %task.id,
                        blocked_by = %dep_id,
                        "Task is blocked"
                    );
                    return Ok(true);
                }
                None => {
                    // Dependency task not found - treat as blocked
                    tracing::warn!(
                        task_id = %task.id,
                        missing_dep = %dep_id,
                        "Dependency task not found"
                    );
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Check if a task is blocked, loading dependencies from database
    pub async fn is_blocked_by_id(&self, task_id: &TaskId) -> Result<bool, DependencyError> {
        let deps = self.get_task_dependencies(task_id).await?;

        if deps.is_empty() {
            return Ok(false);
        }

        for dep_id in deps {
            let status = self.get_task_status(&dep_id).await?;

            match status {
                Some(TaskStatus::Completed) => continue,
                Some(_) | None => {
                    tracing::debug!(
                        task_id = %task_id,
                        blocked_by = %dep_id,
                        "Task is blocked"
                    );
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Get the current status of a task by ID
    async fn get_task_status(&self, id: &TaskId) -> Result<Option<TaskStatus>, DependencyError> {
        let row: Option<(String,)> = sqlx::query_as("SELECT status FROM tasks WHERE id = ?")
            .bind(id.0.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DependencyError::DatabaseError(e.to_string()))?;

        Ok(row.map(|r| match r.0.as_str() {
            "pending" => TaskStatus::Pending,
            "inprogress" | "in_progress" => TaskStatus::InProgress,
            "review" => TaskStatus::Review,
            "completed" => TaskStatus::Completed,
            "failed" => TaskStatus::Failed,
            _ => TaskStatus::Pending,
        }))
    }

    /// Get all tasks that depend on the given task
    pub async fn get_blocked_by(&self, task_id: &TaskId) -> Result<Vec<TaskId>, DependencyError> {
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT task_id FROM task_dependencies WHERE depends_on_id = ?")
                .bind(task_id.0.to_string())
                .fetch_all(&self.pool)
                .await
                .map_err(|e| DependencyError::DatabaseError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .filter_map(|r| Uuid::from_str(&r.0).ok())
            .map(TaskId)
            .collect())
    }

    /// Get the dependencies of a task from the database
    pub async fn get_task_dependencies(
        &self,
        task_id: &TaskId,
    ) -> Result<Vec<TaskId>, DependencyError> {
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT depends_on_id FROM task_dependencies WHERE task_id = ?")
                .bind(task_id.0.to_string())
                .fetch_all(&self.pool)
                .await
                .map_err(|e| DependencyError::DatabaseError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .filter_map(|r| Uuid::from_str(&r.0).ok())
            .map(TaskId)
            .collect())
    }

    /// Load dependencies for a task from the database into the task struct
    pub async fn load_dependencies(&self, task: &mut Task) -> Result<(), DependencyError> {
        task.depends_on = self.get_task_dependencies(&task.id).await?;
        Ok(())
    }

    /// Check for circular dependencies when adding a new dependency
    pub async fn would_create_cycle(
        &self,
        task_id: &TaskId,
        new_dep_id: &TaskId,
    ) -> Result<bool, DependencyError> {
        // DFS to check if new_dep_id can reach task_id through the dependency graph
        let mut visited = HashSet::new();
        let mut stack = vec![new_dep_id.clone()];

        while let Some(current) = stack.pop() {
            if current == *task_id {
                return Ok(true); // Cycle detected
            }

            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());

            // Get dependencies of current task
            let deps = self.get_task_dependencies(&current).await?;
            stack.extend(deps);
        }

        Ok(false)
    }

    /// Save task dependencies to database
    pub async fn save_dependencies(&self, task: &Task) -> Result<(), DependencyError> {
        for dep_id in &task.depends_on {
            // Check for cycle first
            if self.would_create_cycle(&task.id, dep_id).await? {
                return Err(DependencyError::CircularDependency(format!(
                    "{} -> {}",
                    task.id, dep_id
                )));
            }

            let now = chrono::Utc::now().to_rfc3339();
            sqlx::query(
                r#"
                INSERT OR IGNORE INTO task_dependencies (task_id, depends_on_id, created_at)
                VALUES (?, ?, ?)
                "#,
            )
            .bind(task.id.0.to_string())
            .bind(dep_id.0.to_string())
            .bind(&now)
            .execute(&self.pool)
            .await
            .map_err(|e| DependencyError::DatabaseError(e.to_string()))?;
        }

        Ok(())
    }

    /// Add a single dependency
    pub async fn add_dependency(
        &self,
        task_id: &TaskId,
        depends_on: &TaskId,
    ) -> Result<(), DependencyError> {
        // Check for cycle first
        if self.would_create_cycle(task_id, depends_on).await? {
            return Err(DependencyError::CircularDependency(format!(
                "{} -> {}",
                task_id, depends_on
            )));
        }

        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO task_dependencies (task_id, depends_on_id, created_at)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(task_id.0.to_string())
        .bind(depends_on.0.to_string())
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| DependencyError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Remove a dependency
    pub async fn remove_dependency(
        &self,
        task_id: &TaskId,
        depends_on: &TaskId,
    ) -> Result<(), DependencyError> {
        sqlx::query("DELETE FROM task_dependencies WHERE task_id = ? AND depends_on_id = ?")
            .bind(task_id.0.to_string())
            .bind(depends_on.0.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| DependencyError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Get all tasks that are ready to execute (no blocking dependencies)
    pub async fn get_ready_tasks(&self) -> Result<Vec<TaskId>, DependencyError> {
        // Get all pending tasks that either have no dependencies
        // or all their dependencies are completed
        let rows: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT t.id FROM tasks t
            WHERE t.status = 'pending'
            AND NOT EXISTS (
                SELECT 1 FROM task_dependencies td
                JOIN tasks dep ON td.depends_on_id = dep.id
                WHERE td.task_id = t.id
                AND dep.status != 'completed'
            )
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DependencyError::DatabaseError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .filter_map(|r| Uuid::from_str(&r.0).ok())
            .map(TaskId)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AgentTier, Priority};
    use chrono::Utc;
    use tempfile::TempDir;

    async fn setup_test_db() -> (SqlitePool, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = crate::db::init_db_at(db_path.to_str().unwrap())
            .await
            .unwrap();
        (pool, temp_dir)
    }

    fn make_task(title: &str) -> Task {
        Task {
            id: TaskId::new(),
            slice_id: None,
            title: title.to_string(),
            description: String::new(),
            assigned_tier: AgentTier::Worker,
            assigned_agent: None,
            status: TaskStatus::Pending,
            priority: Priority::Normal,
            context_files: vec![],
            metadata: None,
            depends_on: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn task_without_dependencies_is_not_blocked() {
        let (pool, _temp_dir) = setup_test_db().await;
        let tracker = DependencyTracker::new(pool.clone());

        let task = make_task("Task A");
        assert!(!tracker.is_blocked(&task).await.unwrap());

        pool.close().await;
    }

    #[tokio::test]
    async fn task_with_incomplete_dependency_is_blocked() {
        let (pool, _temp_dir) = setup_test_db().await;
        let tracker = DependencyTracker::new(pool.clone());

        // Create dependency task
        let dep_task = make_task("Dependency");
        crate::db::insert_task(&pool, &dep_task).await.unwrap();

        // Create task that depends on it
        let mut task = make_task("Dependent Task");
        task.depends_on = vec![dep_task.id.clone()];

        assert!(tracker.is_blocked(&task).await.unwrap());

        pool.close().await;
    }

    #[tokio::test]
    async fn task_with_completed_dependency_is_not_blocked() {
        let (pool, _temp_dir) = setup_test_db().await;
        let tracker = DependencyTracker::new(pool.clone());

        // Create and complete dependency task
        let mut dep_task = make_task("Dependency");
        dep_task.status = TaskStatus::Completed;
        crate::db::insert_task(&pool, &dep_task).await.unwrap();

        // Create task that depends on it
        let mut task = make_task("Dependent Task");
        task.depends_on = vec![dep_task.id.clone()];

        assert!(!tracker.is_blocked(&task).await.unwrap());

        pool.close().await;
    }

    #[tokio::test]
    async fn save_and_load_dependencies() {
        let (pool, _temp_dir) = setup_test_db().await;
        let tracker = DependencyTracker::new(pool.clone());

        // Create two tasks
        let task_a = make_task("Task A");
        let task_b = make_task("Task B");
        crate::db::insert_task(&pool, &task_a).await.unwrap();
        crate::db::insert_task(&pool, &task_b).await.unwrap();

        // Make B depend on A
        let mut task_b_with_dep = task_b.clone();
        task_b_with_dep.depends_on = vec![task_a.id.clone()];
        tracker.save_dependencies(&task_b_with_dep).await.unwrap();

        // Load dependencies
        let deps = tracker.get_task_dependencies(&task_b.id).await.unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], task_a.id);

        pool.close().await;
    }

    #[tokio::test]
    async fn circular_dependency_detected() {
        let (pool, _temp_dir) = setup_test_db().await;
        let tracker = DependencyTracker::new(pool.clone());

        // Create tasks A -> B
        let task_a = make_task("Task A");
        let task_b = make_task("Task B");
        crate::db::insert_task(&pool, &task_a).await.unwrap();
        crate::db::insert_task(&pool, &task_b).await.unwrap();

        // Make B depend on A
        tracker
            .add_dependency(&task_b.id, &task_a.id)
            .await
            .unwrap();

        // Try to make A depend on B - should detect cycle
        let result = tracker.add_dependency(&task_a.id, &task_b.id).await;
        assert!(matches!(
            result,
            Err(DependencyError::CircularDependency(_))
        ));

        pool.close().await;
    }

    #[tokio::test]
    async fn get_blocked_by_finds_dependents() {
        let (pool, _temp_dir) = setup_test_db().await;
        let tracker = DependencyTracker::new(pool.clone());

        // Create tasks
        let task_a = make_task("Task A");
        let task_b = make_task("Task B");
        let task_c = make_task("Task C");
        crate::db::insert_task(&pool, &task_a).await.unwrap();
        crate::db::insert_task(&pool, &task_b).await.unwrap();
        crate::db::insert_task(&pool, &task_c).await.unwrap();

        // B and C depend on A
        tracker
            .add_dependency(&task_b.id, &task_a.id)
            .await
            .unwrap();
        tracker
            .add_dependency(&task_c.id, &task_a.id)
            .await
            .unwrap();

        // Get tasks blocked by A
        let blocked = tracker.get_blocked_by(&task_a.id).await.unwrap();
        assert_eq!(blocked.len(), 2);

        pool.close().await;
    }

    #[tokio::test]
    async fn dependency_chain_works() {
        let (pool, _temp_dir) = setup_test_db().await;
        let tracker = DependencyTracker::new(pool.clone());

        // Create chain: A -> B -> C
        let task_a = make_task("Task A");
        let task_b = make_task("Task B");
        let task_c = make_task("Task C");
        crate::db::insert_task(&pool, &task_a).await.unwrap();
        crate::db::insert_task(&pool, &task_b).await.unwrap();
        crate::db::insert_task(&pool, &task_c).await.unwrap();

        // B depends on A, C depends on B
        tracker
            .add_dependency(&task_b.id, &task_a.id)
            .await
            .unwrap();
        tracker
            .add_dependency(&task_c.id, &task_b.id)
            .await
            .unwrap();

        // A is not blocked
        assert!(!tracker.is_blocked_by_id(&task_a.id).await.unwrap());

        // B is blocked (A not complete)
        assert!(tracker.is_blocked_by_id(&task_b.id).await.unwrap());

        // C is blocked (B not complete)
        assert!(tracker.is_blocked_by_id(&task_c.id).await.unwrap());

        // Complete A
        crate::db::update_task_status(&pool, &task_a.id, TaskStatus::Completed)
            .await
            .unwrap();

        // B is now not blocked
        assert!(!tracker.is_blocked_by_id(&task_b.id).await.unwrap());

        // C is still blocked (B not complete)
        assert!(tracker.is_blocked_by_id(&task_c.id).await.unwrap());

        // Complete B
        crate::db::update_task_status(&pool, &task_b.id, TaskStatus::Completed)
            .await
            .unwrap();

        // C is now not blocked
        assert!(!tracker.is_blocked_by_id(&task_c.id).await.unwrap());

        pool.close().await;
    }

    #[tokio::test]
    async fn get_ready_tasks_returns_unblocked() {
        let (pool, _temp_dir) = setup_test_db().await;
        let tracker = DependencyTracker::new(pool.clone());

        // Create chain: A -> B -> C
        let task_a = make_task("Task A");
        let task_b = make_task("Task B");
        let task_c = make_task("Task C");
        crate::db::insert_task(&pool, &task_a).await.unwrap();
        crate::db::insert_task(&pool, &task_b).await.unwrap();
        crate::db::insert_task(&pool, &task_c).await.unwrap();

        tracker
            .add_dependency(&task_b.id, &task_a.id)
            .await
            .unwrap();
        tracker
            .add_dependency(&task_c.id, &task_b.id)
            .await
            .unwrap();

        // Only A should be ready
        let ready = tracker.get_ready_tasks().await.unwrap();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0], task_a.id);

        pool.close().await;
    }

    #[tokio::test]
    async fn remove_dependency_works() {
        let (pool, _temp_dir) = setup_test_db().await;
        let tracker = DependencyTracker::new(pool.clone());

        let task_a = make_task("Task A");
        let task_b = make_task("Task B");
        crate::db::insert_task(&pool, &task_a).await.unwrap();
        crate::db::insert_task(&pool, &task_b).await.unwrap();

        // Add then remove dependency
        tracker
            .add_dependency(&task_b.id, &task_a.id)
            .await
            .unwrap();
        let deps = tracker.get_task_dependencies(&task_b.id).await.unwrap();
        assert_eq!(deps.len(), 1);

        tracker
            .remove_dependency(&task_b.id, &task_a.id)
            .await
            .unwrap();
        let deps = tracker.get_task_dependencies(&task_b.id).await.unwrap();
        assert_eq!(deps.len(), 0);

        pool.close().await;
    }
}
