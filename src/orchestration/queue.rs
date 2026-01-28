//! Priority task queue with persistence.
//!
//! The Task Queue is the heart of the orchestration system:
//! 1. Planner creates tasks → enqueue
//! 2. Scheduler dequeues tasks → assigns to agents
//! 3. Failed tasks → requeue with optional priority escalation

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use chrono::Utc;
use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::types::{Priority, Task, TaskId, TaskStatus};

// ============================================================================
// Slice 5.2.1: TaskQueue with Priority Ordering
// ============================================================================

/// Errors that can occur with the task queue
#[derive(Error, Debug)]
pub enum QueueError {
    #[error("task not found: {0:?}")]
    TaskNotFound(TaskId),

    #[error("database error: {0}")]
    DatabaseError(String),

    #[error("queue is empty")]
    Empty,
}

/// Wrapper for priority ordering in BinaryHeap.
/// Higher priority = dequeued first (max-heap behavior).
#[derive(Debug, Clone)]
struct PrioritizedTask {
    task: Task,
}

impl PrioritizedTask {
    fn priority_value(&self) -> u8 {
        match self.task.priority {
            Priority::Urgent => 4,
            Priority::High => 3,
            Priority::Normal => 2,
            Priority::Low => 1,
        }
    }
}

impl PartialEq for PrioritizedTask {
    fn eq(&self, other: &Self) -> bool {
        self.task.id == other.task.id
    }
}

impl Eq for PrioritizedTask {}

impl PartialOrd for PrioritizedTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PrioritizedTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority value = higher priority in queue
        // Secondary: older tasks (earlier created_at) have priority (FIFO within priority)
        match self.priority_value().cmp(&other.priority_value()) {
            Ordering::Equal => other.task.created_at.cmp(&self.task.created_at),
            ord => ord,
        }
    }
}

/// In-memory priority task queue.
pub struct TaskQueue {
    heap: BinaryHeap<PrioritizedTask>,
}

impl TaskQueue {
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.heap.len()
    }

    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    // ========================================================================
    // Slice 5.2.2: enqueue(), dequeue(), peek() Operations
    // ========================================================================

    /// Add a task to the queue.
    pub fn enqueue(&mut self, task: Task) {
        tracing::debug!(
            task_id = %task.id.0,
            priority = ?task.priority,
            "Enqueueing task"
        );
        self.heap.push(PrioritizedTask { task });
    }

    /// Remove and return the highest priority task.
    pub fn dequeue(&mut self) -> Option<Task> {
        self.heap.pop().map(|pt| {
            tracing::debug!(
                task_id = %pt.task.id.0,
                priority = ?pt.task.priority,
                "Dequeuing task"
            );
            pt.task
        })
    }

    /// Peek at the highest priority task without removing it.
    pub fn peek(&self) -> Option<&Task> {
        self.heap.peek().map(|pt| &pt.task)
    }

    /// Check if a task is in the queue.
    pub fn contains(&self, id: &TaskId) -> bool {
        self.heap.iter().any(|pt| &pt.task.id == id)
    }

    /// Remove a specific task from the queue.
    pub fn remove(&mut self, id: &TaskId) -> Option<Task> {
        let items: Vec<_> = self.heap.drain().collect();
        let mut removed = None;

        for pt in items {
            if &pt.task.id == id {
                removed = Some(pt.task);
            } else {
                self.heap.push(pt);
            }
        }

        removed
    }

    /// Get all tasks in priority order (for debugging/display).
    pub fn all_tasks(&self) -> Vec<&Task> {
        let mut tasks: Vec<_> = self.heap.iter().map(|pt| &pt.task).collect();
        tasks.sort_by(|a, b| {
            let a_val = priority_value(a.priority);
            let b_val = priority_value(b.priority);
            match b_val.cmp(&a_val) {
                Ordering::Equal => a.created_at.cmp(&b.created_at),
                ord => ord,
            }
        });
        tasks
    }

    /// Get count of tasks by priority.
    pub fn count_by_priority(&self) -> HashMap<Priority, usize> {
        let mut counts = HashMap::new();
        for pt in &self.heap {
            *counts.entry(pt.task.priority).or_insert(0) += 1;
        }
        counts
    }
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

fn priority_value(p: Priority) -> u8 {
    match p {
        Priority::Urgent => 4,
        Priority::High => 3,
        Priority::Normal => 2,
        Priority::Low => 1,
    }
}

// ============================================================================
// Slice 5.2.3: Persistent Queue with Database
// ============================================================================

/// Persistent task queue with database backing.
pub struct PersistentTaskQueue {
    inner: TaskQueue,
    pool: SqlitePool,
}

impl PersistentTaskQueue {
    /// Create a new persistent queue and load pending tasks from database.
    pub async fn new(pool: SqlitePool) -> Result<Self, QueueError> {
        let mut queue = Self {
            inner: TaskQueue::new(),
            pool,
        };
        queue.load_from_db().await?;
        Ok(queue)
    }

    /// Load all pending tasks from database into memory queue.
    async fn load_from_db(&mut self) -> Result<(), QueueError> {
        let tasks = crate::db::list_tasks_by_status(&self.pool, TaskStatus::Pending)
            .await
            .map_err(|e| QueueError::DatabaseError(e.to_string()))?;

        for task in tasks {
            self.inner.enqueue(task);
        }

        tracing::info!(
            count = self.inner.len(),
            "Loaded pending tasks from database"
        );
        Ok(())
    }

    /// Enqueue a task (assumes task is already in database).
    pub fn enqueue(&mut self, task: Task) {
        self.inner.enqueue(task);
    }

    /// Enqueue and update task status in database.
    pub async fn enqueue_and_persist(&mut self, task: Task) -> Result<(), QueueError> {
        crate::db::update_task_status(&self.pool, &task.id, TaskStatus::Pending)
            .await
            .map_err(|e| QueueError::DatabaseError(e.to_string()))?;

        self.inner.enqueue(task);
        Ok(())
    }

    /// Dequeue highest priority task and update database status to InProgress.
    pub async fn dequeue(&mut self) -> Result<Option<Task>, QueueError> {
        match self.inner.dequeue() {
            Some(task) => {
                crate::db::update_task_status(&self.pool, &task.id, TaskStatus::InProgress)
                    .await
                    .map_err(|e| QueueError::DatabaseError(e.to_string()))?;

                Ok(Some(task))
            }
            None => Ok(None),
        }
    }

    /// Peek at highest priority task without removing.
    pub fn peek(&self) -> Option<&Task> {
        self.inner.peek()
    }

    /// Check if queue is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get queue length.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if task is in queue.
    pub fn contains(&self, id: &TaskId) -> bool {
        self.inner.contains(id)
    }

    /// Get all tasks in priority order.
    pub fn all_tasks(&self) -> Vec<&Task> {
        self.inner.all_tasks()
    }

    /// Get count by priority.
    pub fn count_by_priority(&self) -> HashMap<Priority, usize> {
        self.inner.count_by_priority()
    }

    // ========================================================================
    // Slice 5.2.4: Requeue for Failed Tasks
    // ========================================================================

    /// Requeue a task that failed or needs retry.
    pub async fn requeue(
        &mut self,
        mut task: Task,
        policy: RequeuePolicy,
    ) -> Result<(), QueueError> {
        // Apply priority policy
        task.priority = match policy {
            RequeuePolicy::SamePriority => task.priority,
            RequeuePolicy::EscalatePriority => escalate_priority(task.priority),
            RequeuePolicy::SetPriority(p) => p,
        };

        // Update task status back to Pending
        task.status = TaskStatus::Pending;
        task.updated_at = Utc::now();

        // Persist changes
        self.update_task_for_requeue(&task, &policy).await?;

        tracing::info!(
            task_id = %task.id.0,
            priority = ?task.priority,
            policy = ?policy,
            "Task requeued"
        );

        self.inner.enqueue(task);
        Ok(())
    }

    async fn update_task_for_requeue(
        &self,
        task: &Task,
        policy: &RequeuePolicy,
    ) -> Result<(), QueueError> {
        let priority_str = format!("{:?}", task.priority).to_lowercase();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE tasks
            SET status = 'pending',
                priority = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&priority_str)
        .bind(&now)
        .bind(task.id.0.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| QueueError::DatabaseError(e.to_string()))?;

        // Log task event for audit trail
        sqlx::query(
            r#"
            INSERT INTO task_events (id, task_id, event_type, details, timestamp)
            VALUES (?, ?, 'requeued', ?, ?)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(task.id.0.to_string())
        .bind(format!("Requeued with policy {:?}", policy))
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| QueueError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}

/// Policy for requeueing failed tasks.
#[derive(Debug, Clone, Copy)]
pub enum RequeuePolicy {
    /// Keep same priority
    SamePriority,
    /// Escalate priority by one level (Low→Normal→High→Urgent)
    EscalatePriority,
    /// Set specific priority
    SetPriority(Priority),
}

/// Escalate priority by one level.
fn escalate_priority(current: Priority) -> Priority {
    match current {
        Priority::Low => Priority::Normal,
        Priority::Normal => Priority::High,
        Priority::High => Priority::Urgent,
        Priority::Urgent => Priority::Urgent, // Already max
    }
}

// ============================================================================
// Slice 5.4.3: Dependency-Aware Queue
// ============================================================================

use crate::orchestration::dependency::DependencyTracker;

/// Queue statistics including blocked/unblocked counts
#[derive(Debug, Clone)]
pub struct QueueStats {
    pub blocked: usize,
    pub unblocked: usize,
    pub total: usize,
}

/// A task queue that respects dependencies between tasks.
///
/// Wraps PersistentTaskQueue and filters out blocked tasks during dequeue.
pub struct DependencyAwareQueue {
    queue: PersistentTaskQueue,
    dependency_tracker: DependencyTracker,
}

impl DependencyAwareQueue {
    /// Create a new dependency-aware queue
    pub async fn new(pool: SqlitePool) -> Result<Self, QueueError> {
        let queue = PersistentTaskQueue::new(pool.clone()).await?;
        let dependency_tracker = DependencyTracker::new(pool);

        Ok(Self {
            queue,
            dependency_tracker,
        })
    }

    /// Enqueue a task and save its dependencies
    pub async fn enqueue(&mut self, task: Task) -> Result<(), QueueError> {
        // Save dependencies first
        self.dependency_tracker
            .save_dependencies(&task)
            .await
            .map_err(|e| QueueError::DatabaseError(e.to_string()))?;

        self.queue.enqueue_and_persist(task).await
    }

    /// Enqueue a task without persisting (task already in DB)
    pub fn enqueue_in_memory(&mut self, task: Task) {
        self.queue.enqueue(task);
    }

    /// Dequeue the highest priority task that is not blocked
    pub async fn dequeue_unblocked(&mut self) -> Result<Option<Task>, QueueError> {
        let mut blocked_tasks = vec![];

        loop {
            match self.queue.dequeue().await? {
                Some(mut task) => {
                    // Load dependencies from database
                    self.dependency_tracker
                        .load_dependencies(&mut task)
                        .await
                        .map_err(|e| QueueError::DatabaseError(e.to_string()))?;

                    let is_blocked = self
                        .dependency_tracker
                        .is_blocked(&task)
                        .await
                        .map_err(|e| QueueError::DatabaseError(e.to_string()))?;

                    if is_blocked {
                        tracing::debug!(
                            task_id = %task.id,
                            "Task blocked by dependencies, skipping"
                        );
                        blocked_tasks.push(task);
                    } else {
                        // Found an unblocked task - re-enqueue blocked ones
                        for blocked in blocked_tasks {
                            self.queue.enqueue(blocked);
                        }
                        return Ok(Some(task));
                    }
                }
                None => {
                    // Queue exhausted - re-add blocked tasks
                    for blocked in blocked_tasks {
                        self.queue.enqueue(blocked);
                    }
                    return Ok(None);
                }
            }
        }
    }

    /// Simple dequeue without dependency checking
    pub async fn dequeue(&mut self) -> Result<Option<Task>, QueueError> {
        self.queue.dequeue().await
    }

    /// Get count of blocked vs unblocked tasks
    pub async fn get_queue_stats(&self) -> Result<QueueStats, QueueError> {
        let mut blocked = 0;
        let mut unblocked = 0;

        for task in self.queue.all_tasks() {
            let is_blocked = self
                .dependency_tracker
                .is_blocked(task)
                .await
                .map_err(|e| QueueError::DatabaseError(e.to_string()))?;

            if is_blocked {
                blocked += 1;
            } else {
                unblocked += 1;
            }
        }

        Ok(QueueStats {
            blocked,
            unblocked,
            total: blocked + unblocked,
        })
    }

    /// Get queue length
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Peek at highest priority task
    pub fn peek(&self) -> Option<&Task> {
        self.queue.peek()
    }

    /// Check if a task is in the queue
    pub fn contains(&self, id: &TaskId) -> bool {
        self.queue.contains(id)
    }

    /// Get all tasks in priority order
    pub fn all_tasks(&self) -> Vec<&Task> {
        self.queue.all_tasks()
    }

    /// Requeue a failed task
    pub async fn requeue(&mut self, task: Task, policy: RequeuePolicy) -> Result<(), QueueError> {
        self.queue.requeue(task, policy).await
    }

    /// Notify that a task completed - returns IDs of tasks that may be unblocked
    pub async fn on_task_completed(&self, task_id: &TaskId) -> Result<Vec<TaskId>, QueueError> {
        let unblocked = self
            .dependency_tracker
            .get_blocked_by(task_id)
            .await
            .map_err(|e| QueueError::DatabaseError(e.to_string()))?;

        if !unblocked.is_empty() {
            tracing::info!(
                completed_task = %task_id,
                unblocked_count = unblocked.len(),
                "Tasks unblocked by completion"
            );
        }

        Ok(unblocked)
    }

    /// Get the dependency tracker for direct access
    pub fn dependency_tracker(&self) -> &DependencyTracker {
        &self.dependency_tracker
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AgentTier;

    fn make_task(priority: Priority) -> Task {
        Task {
            id: TaskId::new(),
            slice_id: None,
            title: format!("{:?} task", priority),
            description: String::new(),
            assigned_tier: AgentTier::Worker,
            assigned_agent: None,
            status: TaskStatus::Pending,
            priority,
            context_files: vec![],
            metadata: None,
            depends_on: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_task_with_time(priority: Priority, secs_ago: i64) -> Task {
        let mut task = make_task(priority);
        task.created_at = Utc::now() - chrono::Duration::seconds(secs_ago);
        task
    }

    // Basic queue tests

    #[test]
    fn empty_queue_is_empty() {
        let queue = TaskQueue::new();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn enqueue_increases_length() {
        let mut queue = TaskQueue::new();
        queue.enqueue(make_task(Priority::Normal));
        assert_eq!(queue.len(), 1);
        assert!(!queue.is_empty());
    }

    #[test]
    fn dequeue_returns_highest_priority_first() {
        let mut queue = TaskQueue::new();

        queue.enqueue(make_task(Priority::Low));
        queue.enqueue(make_task(Priority::Urgent));
        queue.enqueue(make_task(Priority::Normal));
        queue.enqueue(make_task(Priority::High));

        assert_eq!(queue.dequeue().unwrap().priority, Priority::Urgent);
        assert_eq!(queue.dequeue().unwrap().priority, Priority::High);
        assert_eq!(queue.dequeue().unwrap().priority, Priority::Normal);
        assert_eq!(queue.dequeue().unwrap().priority, Priority::Low);
        assert!(queue.dequeue().is_none());
    }

    #[test]
    fn same_priority_fifo_order() {
        let mut queue = TaskQueue::new();

        // Older task should come first
        let older = make_task_with_time(Priority::Normal, 100);
        let newer = make_task_with_time(Priority::Normal, 10);

        let older_id = older.id.clone();

        queue.enqueue(newer);
        queue.enqueue(older);

        let first = queue.dequeue().unwrap();
        assert_eq!(first.id, older_id);
    }

    #[test]
    fn peek_does_not_remove() {
        let mut queue = TaskQueue::new();
        queue.enqueue(make_task(Priority::High));

        assert!(queue.peek().is_some());
        assert!(queue.peek().is_some()); // Still there
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn peek_returns_highest_priority() {
        let mut queue = TaskQueue::new();
        queue.enqueue(make_task(Priority::Low));
        queue.enqueue(make_task(Priority::Urgent));

        assert_eq!(queue.peek().unwrap().priority, Priority::Urgent);
    }

    #[test]
    fn contains_finds_task() {
        let mut queue = TaskQueue::new();
        let task = make_task(Priority::Normal);
        let id = task.id.clone();

        queue.enqueue(task);

        assert!(queue.contains(&id));
        assert!(!queue.contains(&TaskId::new()));
    }

    #[test]
    fn remove_extracts_task() {
        let mut queue = TaskQueue::new();
        let task1 = make_task(Priority::Normal);
        let task2 = make_task(Priority::High);
        let id1 = task1.id.clone();

        queue.enqueue(task1);
        queue.enqueue(task2);

        let removed = queue.remove(&id1);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().id, id1);
        assert_eq!(queue.len(), 1);
        assert!(!queue.contains(&id1));
    }

    #[test]
    fn remove_nonexistent_returns_none() {
        let mut queue = TaskQueue::new();
        queue.enqueue(make_task(Priority::Normal));

        let removed = queue.remove(&TaskId::new());
        assert!(removed.is_none());
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn all_tasks_returns_priority_order() {
        let mut queue = TaskQueue::new();
        queue.enqueue(make_task(Priority::Low));
        queue.enqueue(make_task(Priority::High));
        queue.enqueue(make_task(Priority::Normal));

        let tasks = queue.all_tasks();
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].priority, Priority::High);
        assert_eq!(tasks[1].priority, Priority::Normal);
        assert_eq!(tasks[2].priority, Priority::Low);
    }

    #[test]
    fn count_by_priority_works() {
        let mut queue = TaskQueue::new();
        queue.enqueue(make_task(Priority::High));
        queue.enqueue(make_task(Priority::High));
        queue.enqueue(make_task(Priority::Normal));

        let counts = queue.count_by_priority();
        assert_eq!(counts.get(&Priority::High), Some(&2));
        assert_eq!(counts.get(&Priority::Normal), Some(&1));
        assert_eq!(counts.get(&Priority::Low), None);
    }

    // Priority escalation tests

    #[test]
    fn escalate_priority_works() {
        assert_eq!(escalate_priority(Priority::Low), Priority::Normal);
        assert_eq!(escalate_priority(Priority::Normal), Priority::High);
        assert_eq!(escalate_priority(Priority::High), Priority::Urgent);
        assert_eq!(escalate_priority(Priority::Urgent), Priority::Urgent);
    }

    // Queue error display tests

    #[test]
    fn queue_error_display() {
        let err = QueueError::Empty;
        assert!(err.to_string().contains("empty"));

        let err = QueueError::TaskNotFound(TaskId::new());
        assert!(err.to_string().contains("not found"));

        let err = QueueError::DatabaseError("connection failed".to_string());
        assert!(err.to_string().contains("connection failed"));
    }

    // RequeuePolicy tests

    #[test]
    fn requeue_policy_debug() {
        let policy = RequeuePolicy::SamePriority;
        assert!(format!("{:?}", policy).contains("SamePriority"));

        let policy = RequeuePolicy::EscalatePriority;
        assert!(format!("{:?}", policy).contains("EscalatePriority"));

        let policy = RequeuePolicy::SetPriority(Priority::Urgent);
        assert!(format!("{:?}", policy).contains("Urgent"));
    }
}

#[cfg(test)]
mod persistent_queue_tests {
    use super::*;
    use crate::types::AgentTier;
    use tempfile::TempDir;

    async fn setup_test_db() -> (SqlitePool, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = crate::db::init_db_at(db_path.to_str().unwrap())
            .await
            .unwrap();
        (pool, temp_dir)
    }

    fn make_task(priority: Priority) -> Task {
        Task {
            id: TaskId::new(),
            slice_id: None,
            title: format!("{:?} task", priority),
            description: String::new(),
            assigned_tier: AgentTier::Worker,
            assigned_agent: None,
            status: TaskStatus::Pending,
            priority,
            context_files: vec![],
            metadata: None,
            depends_on: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn persistent_queue_loads_pending_tasks() {
        let (pool, _temp_dir) = setup_test_db().await;

        // Insert a pending task
        let task = make_task(Priority::Normal);
        crate::db::insert_task(&pool, &task).await.unwrap();

        // Create queue - should load the task
        let queue = PersistentTaskQueue::new(pool.clone()).await.unwrap();
        assert_eq!(queue.len(), 1);

        pool.close().await;
    }

    #[tokio::test]
    async fn persistent_queue_ignores_non_pending() {
        let (pool, _temp_dir) = setup_test_db().await;

        // Insert a completed task
        let mut task = make_task(Priority::Normal);
        task.status = TaskStatus::Completed;
        crate::db::insert_task(&pool, &task).await.unwrap();

        // Create queue - should not load completed task
        let queue = PersistentTaskQueue::new(pool.clone()).await.unwrap();
        assert!(queue.is_empty());

        pool.close().await;
    }

    #[tokio::test]
    async fn dequeue_updates_status_to_in_progress() {
        let (pool, _temp_dir) = setup_test_db().await;

        let task = make_task(Priority::Normal);
        let task_id = task.id.clone();
        crate::db::insert_task(&pool, &task).await.unwrap();

        let mut queue = PersistentTaskQueue::new(pool.clone()).await.unwrap();
        let dequeued = queue.dequeue().await.unwrap();

        assert!(dequeued.is_some());

        // Check database was updated
        let db_task = crate::db::get_task(&pool, &task_id).await.unwrap().unwrap();
        assert_eq!(db_task.status, TaskStatus::InProgress);

        pool.close().await;
    }

    #[tokio::test]
    async fn requeue_updates_priority_and_status() {
        let (pool, _temp_dir) = setup_test_db().await;

        let task = make_task(Priority::Normal);
        let task_id = task.id.clone();
        crate::db::insert_task(&pool, &task).await.unwrap();

        let mut queue = PersistentTaskQueue::new(pool.clone()).await.unwrap();

        // Dequeue the task
        let task = queue.dequeue().await.unwrap().unwrap();
        assert!(queue.is_empty());

        // Requeue with escalation
        queue
            .requeue(task, RequeuePolicy::EscalatePriority)
            .await
            .unwrap();

        assert_eq!(queue.len(), 1);
        assert_eq!(queue.peek().unwrap().priority, Priority::High);

        // Check database
        let db_task = crate::db::get_task(&pool, &task_id).await.unwrap().unwrap();
        assert_eq!(db_task.status, TaskStatus::Pending);
        assert_eq!(db_task.priority, Priority::High);

        pool.close().await;
    }

    #[tokio::test]
    async fn requeue_with_set_priority() {
        let (pool, _temp_dir) = setup_test_db().await;

        let task = make_task(Priority::Low);
        crate::db::insert_task(&pool, &task).await.unwrap();

        let mut queue = PersistentTaskQueue::new(pool.clone()).await.unwrap();
        let task = queue.dequeue().await.unwrap().unwrap();

        queue
            .requeue(task, RequeuePolicy::SetPriority(Priority::Urgent))
            .await
            .unwrap();

        assert_eq!(queue.peek().unwrap().priority, Priority::Urgent);

        pool.close().await;
    }

    #[tokio::test]
    async fn requeue_creates_event() {
        let (pool, _temp_dir) = setup_test_db().await;

        let task = make_task(Priority::Normal);
        let task_id = task.id.clone();
        crate::db::insert_task(&pool, &task).await.unwrap();

        let mut queue = PersistentTaskQueue::new(pool.clone()).await.unwrap();
        let task = queue.dequeue().await.unwrap().unwrap();
        queue
            .requeue(task, RequeuePolicy::SamePriority)
            .await
            .unwrap();

        // Check event was created
        let event_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM task_events WHERE task_id = ? AND event_type = 'requeued'",
        )
        .bind(task_id.0.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(event_count.0, 1);

        pool.close().await;
    }
}
