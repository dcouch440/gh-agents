//! Task scheduler with pause/resume support for refactor mode.
//!
//! The scheduler controls work assignment based on production mode:
//! - Running: Normal operation, tasks are assigned
//! - RefactorMode/Paused: No new work assigned
//! - Resuming: Transitioning back to running

use anyhow::Result;
use sqlx::SqlitePool;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{Notify, RwLock};
use tokio::time::{interval, Duration};

use crate::agents::{AgentId, AgentPool};
use crate::db::{get_production_mode, set_production_mode};
use crate::types::{AgentTier, Priority, ProductionMode, Task, TaskId};

/// Task scheduler that respects production mode
pub struct Scheduler {
    pool: SqlitePool,
    /// Cached production mode (updated on state changes)
    mode_cache: Arc<RwLock<ProductionMode>>,
}

impl Scheduler {
    /// Create a new scheduler
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        let mode = get_production_mode(&pool).await?;
        Ok(Self {
            pool,
            mode_cache: Arc::new(RwLock::new(mode)),
        })
    }

    /// Get the current production mode
    pub async fn get_production_mode(&self) -> ProductionMode {
        *self.mode_cache.read().await
    }

    /// Check if the scheduler should assign new work
    pub async fn should_assign(&self) -> bool {
        self.get_production_mode().await.is_active()
    }

    /// Set the production mode
    async fn set_mode(&self, mode: ProductionMode) -> Result<()> {
        set_production_mode(&self.pool, mode).await?;
        *self.mode_cache.write().await = mode;
        tracing::info!(mode = ?mode, "Production mode changed");
        Ok(())
    }

    /// Enter refactor mode - no new work will be assigned
    pub async fn enter_refactor_mode(&self) -> Result<()> {
        let current = self.get_production_mode().await;
        if current == ProductionMode::Running {
            self.set_mode(ProductionMode::RefactorMode).await?;
        }
        Ok(())
    }

    /// Pause production for refactor (let in-progress work complete)
    ///
    /// This is a stronger signal than entering refactor mode - it indicates
    /// that the user wants to make changes that affect in-progress work.
    pub async fn pause_for_refactor(&self) -> Result<()> {
        self.set_mode(ProductionMode::Paused).await
    }

    /// Begin resuming production after refactor
    ///
    /// The resuming state allows for graceful transition back to running,
    /// including any initialization needed after plan changes.
    pub async fn begin_resume(&self) -> Result<()> {
        let current = self.get_production_mode().await;
        if current.is_refactoring() {
            self.set_mode(ProductionMode::Resuming).await?;
        }
        Ok(())
    }

    /// Resume normal operation
    pub async fn resume(&self) -> Result<()> {
        self.set_mode(ProductionMode::Running).await
    }

    /// Check if production is paused or in refactor mode
    pub async fn is_paused(&self) -> bool {
        self.get_production_mode().await.is_refactoring()
    }

    /// Refresh the cached mode from the database
    ///
    /// Call this if the mode might have been changed externally.
    pub async fn refresh_mode(&self) -> Result<()> {
        let mode = get_production_mode(&self.pool).await?;
        *self.mode_cache.write().await = mode;
        Ok(())
    }
}

// ============================================================================
// Ticket 5.5: Task Scheduler
// ============================================================================

use crate::orchestration::queue::{DependencyAwareQueue, QueueError, RequeuePolicy};
use crate::orchestration::router::Router;

/// Errors that can occur during scheduling
#[derive(Error, Debug)]
pub enum SchedulerError {
    #[error("queue error: {0}")]
    QueueError(#[from] QueueError),

    #[error("agent pool error: {0}")]
    AgentPoolError(String),

    #[error("database error: {0}")]
    DatabaseError(String),

    #[error("scheduler stopped")]
    Stopped,
}

/// Configuration for the task scheduler
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// How often to check for new tasks (ms)
    pub poll_interval_ms: u64,

    /// Maximum tasks to assign per tick
    pub batch_size: usize,

    /// How long to wait when no agents available (ms)
    pub agent_wait_timeout_ms: u64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            poll_interval_ms: 100,
            batch_size: 5,
            agent_wait_timeout_ms: 500,
        }
    }
}

/// Result of attempting to assign a single task
#[derive(Debug)]
enum AssignResult {
    /// Task was assigned to an agent
    Assigned,
    /// No tasks available in the queue
    NoTasks,
    /// No agents available for the required tier
    NoAgents(AgentTier),
}

/// Preemption action to take
#[derive(Debug)]
pub struct PreemptionAction {
    pub agent_id: AgentId,
    pub task_to_pause: Task,
}

/// Task scheduler that assigns work to agents.
///
/// Integrates:
/// - DependencyAwareQueue: Provides tasks in priority order, respecting dependencies
/// - Router: Determines which agent tier handles each task
/// - AgentPool: Provides available agents
/// - Scheduler (refactor mode): Respects pause/resume state
pub struct TaskScheduler {
    queue: Arc<RwLock<DependencyAwareQueue>>,
    router: Router,
    agent_pool: Arc<RwLock<AgentPool>>,
    refactor_scheduler: Arc<Scheduler>,
    config: SchedulerConfig,
    running: Arc<RwLock<bool>>,
    /// Notified when an agent becomes available
    agent_available: Arc<Notify>,
}

impl TaskScheduler {
    /// Create a new task scheduler
    pub fn new(
        queue: Arc<RwLock<DependencyAwareQueue>>,
        router: Router,
        agent_pool: Arc<RwLock<AgentPool>>,
        refactor_scheduler: Arc<Scheduler>,
        config: SchedulerConfig,
    ) -> Self {
        Self {
            queue,
            router,
            agent_pool,
            refactor_scheduler,
            config,
            running: Arc::new(RwLock::new(false)),
            agent_available: Arc::new(Notify::new()),
        }
    }

    /// Get a handle to notify when agents become available
    pub fn agent_available_notifier(&self) -> Arc<Notify> {
        self.agent_available.clone()
    }

    /// Start the scheduler loop
    pub async fn run(&self) -> Result<(), SchedulerError> {
        *self.running.write().await = true;

        let mut ticker = interval(Duration::from_millis(self.config.poll_interval_ms));

        tracing::info!("Task scheduler started");

        loop {
            ticker.tick().await;

            if !*self.running.read().await {
                tracing::info!("Task scheduler stopping");
                break;
            }

            // Check if we're paused for refactor
            if !self.refactor_scheduler.should_assign().await {
                tracing::trace!("Scheduler paused for refactor mode");
                continue;
            }

            // Try to assign tasks this tick
            match self.tick().await {
                Ok(assigned) => {
                    if assigned > 0 {
                        tracing::debug!(count = assigned, "Assigned tasks this tick");
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "Scheduler tick error");
                    // Continue running despite errors
                }
            }
        }

        Ok(())
    }

    /// Stop the scheduler loop
    pub async fn stop(&self) {
        *self.running.write().await = false;
    }

    /// Check if the scheduler is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Single tick of the scheduler: try to assign up to batch_size tasks
    async fn tick(&self) -> Result<usize, SchedulerError> {
        let mut assigned = 0;

        for _ in 0..self.config.batch_size {
            match self.try_assign_one().await? {
                AssignResult::Assigned => assigned += 1,
                AssignResult::NoTasks => break,
                AssignResult::NoAgents(tier) => {
                    tracing::debug!(
                        tier = ?tier,
                        "Waiting for available agent"
                    );

                    // Wait for agent or timeout
                    tokio::select! {
                        _ = self.agent_available.notified() => {
                            tracing::debug!("Agent became available, resuming");
                        }
                        _ = tokio::time::sleep(Duration::from_millis(self.config.agent_wait_timeout_ms)) => {
                            // Timeout, will retry on next tick
                        }
                    }
                    break;
                }
            }
        }

        Ok(assigned)
    }

    /// Try to assign a single task to an agent
    async fn try_assign_one(&self) -> Result<AssignResult, SchedulerError> {
        // Get next unblocked task
        let task = {
            let mut queue = self.queue.write().await;
            match queue.dequeue_unblocked().await? {
                Some(t) => t,
                None => return Ok(AssignResult::NoTasks),
            }
        };

        // Route to determine tier
        let tier = self.router.route(&task);

        // Find available agent in that tier
        let agent_id = {
            let pool = self.agent_pool.read().await;
            pool.get_available_agent_id(tier)
        };

        match agent_id {
            Some(id) => {
                self.assign_task(task, id, tier).await?;
                Ok(AssignResult::Assigned)
            }
            None => {
                // Put task back and report no agents
                let mut queue = self.queue.write().await;
                queue.requeue(task, RequeuePolicy::SamePriority).await?;
                Ok(AssignResult::NoAgents(tier))
            }
        }
    }

    /// Assign a task to an agent
    async fn assign_task(
        &self,
        task: Task,
        agent_id: AgentId,
        tier: AgentTier,
    ) -> Result<(), SchedulerError> {
        // Mark agent as working on this task
        {
            let mut pool = self.agent_pool.write().await;
            if let Some(agent) = pool.get_agent_mut(&agent_id) {
                agent
                    .start_task(task.id.0)
                    .map_err(|e| SchedulerError::AgentPoolError(e.to_string()))?;
            }
        }

        // Update task status in database via queue's dependency tracker
        {
            let _queue = self.queue.read().await;
            // The actual task status update would happen through the db module
            // For now, we log the assignment
            tracing::info!(
                task_id = %task.id,
                task_title = %task.title,
                agent_id = %agent_id.0,
                tier = ?tier,
                "Task assigned to agent"
            );
        }

        Ok(())
    }

    /// Call this when an agent completes work and becomes available
    pub async fn on_agent_available(&self, agent_id: &AgentId) {
        tracing::debug!(agent_id = %agent_id.0, "Agent became available");
        self.agent_available.notify_waiters();
    }

    /// Call this when a task completes
    pub async fn on_task_completed(&self, task_id: &TaskId) -> Result<Vec<TaskId>, SchedulerError> {
        let queue = self.queue.read().await;
        let unblocked = queue.on_task_completed(task_id).await?;

        if !unblocked.is_empty() {
            tracing::info!(
                task_id = %task_id,
                unblocked_count = unblocked.len(),
                "Task completion unblocked dependent tasks"
            );
        }

        Ok(unblocked)
    }

    /// Check if we should preempt running work for an urgent task
    pub async fn check_preemption(&self) -> Result<Option<PreemptionAction>, SchedulerError> {
        // Check if there's an urgent task waiting
        let has_urgent = {
            let queue = self.queue.read().await;
            queue
                .peek()
                .map(|t| t.priority == Priority::Urgent)
                .unwrap_or(false)
        };

        if !has_urgent {
            return Ok(None);
        }

        // Check if there are any free agents
        let stats = {
            let pool = self.agent_pool.read().await;
            pool.stats()
        };

        let has_free_worker = stats.workers.available > 0;
        let has_free_orchestrator = stats.orchestrators.available > 0;
        let has_free_utility = stats.utilities.available > 0;

        if has_free_worker || has_free_orchestrator || has_free_utility {
            // There's a free agent, no need to preempt
            return Ok(None);
        }

        // All agents are busy - would need to preempt
        // For now, just return None - preemption can be implemented later
        tracing::debug!("Urgent task waiting but all agents busy - preemption not yet implemented");
        Ok(None)
    }

    /// Spawn scheduler as background task
    pub fn spawn(self: Arc<Self>) -> tokio::task::JoinHandle<Result<(), SchedulerError>> {
        tokio::spawn(async move { self.run().await })
    }
}

#[cfg(test)]
mod task_scheduler_tests {
    use super::*;

    #[test]
    fn scheduler_config_defaults() {
        let config = SchedulerConfig::default();
        assert!(config.poll_interval_ms > 0);
        assert!(config.batch_size > 0);
        assert!(config.agent_wait_timeout_ms > 0);
    }

    #[test]
    fn scheduler_config_custom() {
        let config = SchedulerConfig {
            poll_interval_ms: 200,
            batch_size: 10,
            agent_wait_timeout_ms: 1000,
        };
        assert_eq!(config.poll_interval_ms, 200);
        assert_eq!(config.batch_size, 10);
        assert_eq!(config.agent_wait_timeout_ms, 1000);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn setup_scheduler() -> (Scheduler, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = crate::db::init_db_at(db_path.to_str().unwrap())
            .await
            .unwrap();
        let scheduler = Scheduler::new(pool).await.unwrap();
        (scheduler, temp_dir)
    }

    #[tokio::test]
    async fn scheduler_starts_in_running_mode() {
        let (scheduler, _temp_dir) = setup_scheduler().await;

        assert_eq!(
            scheduler.get_production_mode().await,
            ProductionMode::Running
        );
        assert!(scheduler.should_assign().await);
        assert!(!scheduler.is_paused().await);
    }

    #[tokio::test]
    async fn enter_refactor_mode_stops_assignment() {
        let (scheduler, _temp_dir) = setup_scheduler().await;

        scheduler.enter_refactor_mode().await.unwrap();

        assert_eq!(
            scheduler.get_production_mode().await,
            ProductionMode::RefactorMode
        );
        assert!(!scheduler.should_assign().await);
        assert!(scheduler.is_paused().await);
    }

    #[tokio::test]
    async fn pause_for_refactor() {
        let (scheduler, _temp_dir) = setup_scheduler().await;

        scheduler.pause_for_refactor().await.unwrap();

        assert_eq!(
            scheduler.get_production_mode().await,
            ProductionMode::Paused
        );
        assert!(!scheduler.should_assign().await);
        assert!(scheduler.is_paused().await);
    }

    #[tokio::test]
    async fn resume_after_refactor() {
        let (scheduler, _temp_dir) = setup_scheduler().await;

        scheduler.enter_refactor_mode().await.unwrap();
        scheduler.begin_resume().await.unwrap();

        assert_eq!(
            scheduler.get_production_mode().await,
            ProductionMode::Resuming
        );
        // Resuming is not "active" - still need to call resume()
        assert!(!scheduler.should_assign().await);

        scheduler.resume().await.unwrap();

        assert_eq!(
            scheduler.get_production_mode().await,
            ProductionMode::Running
        );
        assert!(scheduler.should_assign().await);
    }

    #[tokio::test]
    async fn full_refactor_cycle() {
        let (scheduler, _temp_dir) = setup_scheduler().await;

        // 1. Start in running mode
        assert!(scheduler.should_assign().await);

        // 2. Enter refactor mode
        scheduler.enter_refactor_mode().await.unwrap();
        assert!(!scheduler.should_assign().await);

        // 3. Pause for more serious changes
        scheduler.pause_for_refactor().await.unwrap();
        assert!(scheduler.is_paused().await);

        // 4. Begin resume
        scheduler.begin_resume().await.unwrap();
        assert_eq!(
            scheduler.get_production_mode().await,
            ProductionMode::Resuming
        );

        // 5. Complete resume
        scheduler.resume().await.unwrap();
        assert!(scheduler.should_assign().await);
    }

    #[tokio::test]
    async fn begin_resume_only_works_in_refactor_states() {
        let (scheduler, _temp_dir) = setup_scheduler().await;

        // In running mode, begin_resume does nothing
        scheduler.begin_resume().await.unwrap();
        assert_eq!(
            scheduler.get_production_mode().await,
            ProductionMode::Running
        );
    }
}
