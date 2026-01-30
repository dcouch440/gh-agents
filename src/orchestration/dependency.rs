//! Task dependency tracking.
//!
//! Tracks dependencies between tasks and determines which tasks are blocked.
//! Used by the scheduler to ensure work is done in the correct order.

use crate::db::pg_repo::PgRepo;
use crate::db::traits::DependencyRepo;
use crate::types::{Task, TaskId, TaskStatus};
use std::collections::HashSet;
use thiserror::Error;

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
pub struct DependencyTracker<R: DependencyRepo = PgRepo> {
    repo: R,
}

impl<R: DependencyRepo> DependencyTracker<R> {
    /// Create a new DependencyTracker
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    /// Check if a task is blocked by incomplete dependencies
    pub async fn is_blocked(&self, task: &Task) -> Result<bool, DependencyError> {
        if task.depends_on.is_empty() {
            return Ok(false);
        }

        // Check status of all dependencies
        for dep_id in &task.depends_on {
            let status = self.repo.get_task_status(dep_id.clone()).await?;

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
        let deps = self.repo.get_task_dependencies(task_id.clone()).await?;

        if deps.is_empty() {
            return Ok(false);
        }

        for dep_id in deps {
            let status = self.repo.get_task_status(dep_id.clone()).await?;

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

    /// Get all tasks that depend on the given task
    pub async fn get_blocked_by(&self, task_id: &TaskId) -> Result<Vec<TaskId>, DependencyError> {
        self.repo.get_blocked_by(task_id.clone()).await
    }

    /// Get the dependencies of a task from the database
    pub async fn get_task_dependencies(
        &self,
        task_id: &TaskId,
    ) -> Result<Vec<TaskId>, DependencyError> {
        self.repo.get_task_dependencies(task_id.clone()).await
    }

    /// Load dependencies for a task from the database into the task struct
    pub async fn load_dependencies(&self, task: &mut Task) -> Result<(), DependencyError> {
        task.depends_on = self.repo.get_task_dependencies(task.id.clone()).await?;
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
            let deps = self.repo.get_task_dependencies(current).await?;
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

            let now = chrono::Utc::now();
            self.repo
                .save_dependency(task.id.clone(), dep_id.clone(), now)
                .await?;
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

        let now = chrono::Utc::now();
        self.repo
            .save_dependency(task_id.clone(), depends_on.clone(), now)
            .await
    }

    /// Remove a dependency
    pub async fn remove_dependency(
        &self,
        task_id: &TaskId,
        depends_on: &TaskId,
    ) -> Result<(), DependencyError> {
        self.repo
            .remove_dependency(task_id.clone(), depends_on.clone())
            .await
    }

    /// Get all tasks that are ready to execute (no blocking dependencies)
    pub async fn get_ready_tasks(&self) -> Result<Vec<TaskId>, DependencyError> {
        self.repo.get_ready_task_ids().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::traits::MockDependencyRepo;
    use crate::types::{AgentTier, Priority};
    use chrono::Utc;

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
        let mock = MockDependencyRepo::new();
        let tracker = DependencyTracker::new(mock);

        let task = make_task("Task A");
        assert!(!tracker.is_blocked(&task).await.unwrap());
    }

    #[tokio::test]
    async fn task_with_incomplete_dependency_is_blocked() {
        let mut mock = MockDependencyRepo::new();

        let dep_task = make_task("Dependency");
        let dep_id = dep_task.id.clone();

        mock.expect_get_task_status()
            .withf(move |id| *id == dep_id)
            .returning(|_| Ok(Some(TaskStatus::Pending)));

        let tracker = DependencyTracker::new(mock);

        let mut task = make_task("Dependent Task");
        task.depends_on = vec![dep_task.id.clone()];

        assert!(tracker.is_blocked(&task).await.unwrap());
    }

    #[tokio::test]
    async fn task_with_completed_dependency_is_not_blocked() {
        let mut mock = MockDependencyRepo::new();

        let dep_task = make_task("Dependency");
        let dep_id = dep_task.id.clone();

        mock.expect_get_task_status()
            .withf(move |id| *id == dep_id)
            .returning(|_| Ok(Some(TaskStatus::Completed)));

        let tracker = DependencyTracker::new(mock);

        let mut task = make_task("Dependent Task");
        task.depends_on = vec![dep_task.id.clone()];

        assert!(!tracker.is_blocked(&task).await.unwrap());
    }

    #[tokio::test]
    async fn save_and_load_dependencies() {
        let mut mock = MockDependencyRepo::new();

        let task_a = make_task("Task A");
        let task_b = make_task("Task B");
        let task_a_id = task_a.id.clone();
        let task_b_id = task_b.id.clone();
        let task_a_id_ret = task_a.id.clone();

        // save_dependencies calls would_create_cycle which calls get_task_dependencies
        // then calls save_dependency
        mock.expect_get_task_dependencies().returning(move |id| {
            if id == task_b_id {
                Ok(vec![task_a_id_ret.clone()])
            } else {
                Ok(vec![])
            }
        });

        mock.expect_save_dependency().returning(|_, _, _| Ok(()));

        let tracker = DependencyTracker::new(mock);

        // Make B depend on A
        let mut task_b_with_dep = task_b.clone();
        task_b_with_dep.depends_on = vec![task_a_id.clone()];
        tracker.save_dependencies(&task_b_with_dep).await.unwrap();

        // Load dependencies
        let deps = tracker.get_task_dependencies(&task_b.id).await.unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], task_a.id);
    }

    #[tokio::test]
    async fn circular_dependency_detected() {
        let mut mock = MockDependencyRepo::new();

        let task_a = make_task("Task A");
        let task_b = make_task("Task B");
        let task_a_id = task_a.id.clone();
        let task_b_id = task_b.id.clone();

        // First add_dependency(B, A): would_create_cycle checks A's deps -> empty, no cycle, saves
        // Second add_dependency(A, B): would_create_cycle checks B's deps -> [A], then A's deps -> empty
        //   but B reaches A which == task_id (A), so cycle detected
        let a_id_for_closure = task_a_id.clone();
        let b_id_for_closure = task_b_id.clone();
        mock.expect_get_task_dependencies().returning(move |id| {
            if id == b_id_for_closure {
                Ok(vec![a_id_for_closure.clone()])
            } else {
                Ok(vec![])
            }
        });

        mock.expect_save_dependency().returning(|_, _, _| Ok(()));

        let tracker = DependencyTracker::new(mock);

        // B depends on A - should succeed
        tracker
            .add_dependency(&task_b.id, &task_a.id)
            .await
            .unwrap();

        // A depends on B - should detect cycle
        let result = tracker.add_dependency(&task_a.id, &task_b.id).await;
        assert!(matches!(
            result,
            Err(DependencyError::CircularDependency(_))
        ));
    }

    #[tokio::test]
    async fn get_blocked_by_finds_dependents() {
        let mut mock = MockDependencyRepo::new();

        let task_a = make_task("Task A");
        let task_b = make_task("Task B");
        let task_c = make_task("Task C");
        let b_id = task_b.id.clone();
        let c_id = task_c.id.clone();

        mock.expect_get_blocked_by()
            .returning(move |_| Ok(vec![b_id.clone(), c_id.clone()]));

        let tracker = DependencyTracker::new(mock);

        let blocked = tracker.get_blocked_by(&task_a.id).await.unwrap();
        assert_eq!(blocked.len(), 2);
    }

    #[tokio::test]
    async fn dependency_chain_works() {
        let mut mock = MockDependencyRepo::new();

        let task_a = make_task("Task A");
        let task_b = make_task("Task B");
        let task_c = make_task("Task C");
        let a_id = task_a.id.clone();
        let b_id = task_b.id.clone();
        let c_id = task_c.id.clone();

        // Track "completed" set via shared state
        // For simplicity, we test just the is_blocked_by_id logic with specific expectations
        let a_id2 = a_id.clone();
        let b_id2 = b_id.clone();
        let c_id2 = c_id.clone();
        let a_id3 = a_id.clone();
        let b_id3 = b_id.clone();

        mock.expect_get_task_dependencies().returning(move |id| {
            if id == b_id2 {
                Ok(vec![a_id2.clone()])
            } else if id == c_id2 {
                Ok(vec![b_id3.clone()])
            } else {
                Ok(vec![])
            }
        });

        // Use a counter to simulate status changes over multiple calls
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;
        let call_count = Arc::new(AtomicU32::new(0));
        let a_id4 = a_id3.clone();
        let b_id4 = b_id.clone();

        mock.expect_get_task_status().returning(move |id| {
            let n = call_count.fetch_add(1, Ordering::SeqCst);
            // Calls 0-2: A and B are Pending
            // Calls 3: A is Completed (after "completing" A)
            // Call 4: B is still Pending
            // Call 5: B is Completed
            if id == a_id4 {
                if n >= 2 {
                    Ok(Some(TaskStatus::Completed))
                } else {
                    Ok(Some(TaskStatus::Pending))
                }
            } else if id == b_id4 {
                if n >= 4 {
                    Ok(Some(TaskStatus::Completed))
                } else {
                    Ok(Some(TaskStatus::Pending))
                }
            } else {
                Ok(None)
            }
        });

        let tracker = DependencyTracker::new(mock);

        // A has no deps -> not blocked (no get_task_status call)
        assert!(!tracker.is_blocked_by_id(&task_a.id).await.unwrap());

        // B depends on A (Pending) -> blocked (call 0)
        assert!(tracker.is_blocked_by_id(&task_b.id).await.unwrap());

        // C depends on B (Pending) -> blocked (call 1)
        assert!(tracker.is_blocked_by_id(&task_c.id).await.unwrap());

        // B depends on A (now Completed at call >= 2) -> not blocked (call 2)
        assert!(!tracker.is_blocked_by_id(&task_b.id).await.unwrap());

        // C depends on B (still Pending) -> blocked (call 3)
        assert!(tracker.is_blocked_by_id(&task_c.id).await.unwrap());

        // C depends on B (now Completed at call >= 4) -> not blocked
        assert!(!tracker.is_blocked_by_id(&task_c.id).await.unwrap());
    }

    #[tokio::test]
    async fn get_ready_tasks_returns_unblocked() {
        let mut mock = MockDependencyRepo::new();

        let task_a = make_task("Task A");
        let a_id = task_a.id.clone();

        mock.expect_get_ready_task_ids()
            .returning(move || Ok(vec![a_id.clone()]));

        let tracker = DependencyTracker::new(mock);

        let ready = tracker.get_ready_tasks().await.unwrap();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0], task_a.id);
    }

    #[tokio::test]
    async fn remove_dependency_works() {
        let mut mock = MockDependencyRepo::new();

        let task_a = make_task("Task A");
        let task_b = make_task("Task B");
        let a_id = task_a.id.clone();

        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;
        let dep_call_count = Arc::new(AtomicU32::new(0));
        let dep_call_count2 = dep_call_count.clone();

        let a_id2 = a_id.clone();
        mock.expect_get_task_dependencies().returning(move |id| {
            let n = dep_call_count2.fetch_add(1, Ordering::SeqCst);
            // Call 0: would_create_cycle checks deps of A -> empty
            // Call 1: get_task_dependencies(B) after add -> [A]
            // Call 2: get_task_dependencies(B) after remove -> empty
            if n == 0 {
                Ok(vec![])
            } else if n == 1 {
                Ok(vec![a_id2.clone()])
            } else {
                Ok(vec![])
            }
        });

        // For add_dependency: would_create_cycle needs get_task_dependencies (already set up)
        // and save_dependency
        mock.expect_save_dependency().returning(|_, _, _| Ok(()));

        mock.expect_remove_dependency().returning(|_, _| Ok(()));

        let tracker = DependencyTracker::new(mock);

        // Add dependency
        tracker
            .add_dependency(&task_b.id, &task_a.id)
            .await
            .unwrap();
        let deps = tracker.get_task_dependencies(&task_b.id).await.unwrap();
        assert_eq!(deps.len(), 1);

        // Remove dependency
        tracker
            .remove_dependency(&task_b.id, &task_a.id)
            .await
            .unwrap();
        let deps = tracker.get_task_dependencies(&task_b.id).await.unwrap();
        assert_eq!(deps.len(), 0);
    }
}
