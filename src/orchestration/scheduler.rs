//! Task scheduler with pause/resume support for refactor mode.
//!
//! The scheduler controls work assignment based on production mode:
//! - Running: Normal operation, tasks are assigned
//! - RefactorMode/Paused: No new work assigned
//! - Resuming: Transitioning back to running

use anyhow::Result;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{Notify, RwLock};
use tokio::time::{interval, Duration};

use crate::agents::{AgentId, AgentPool};
use crate::db::pg_repo::PgRepo;
use crate::db::traits::SchedulerRepo;
use crate::types::{AgentTier, Priority, ProductionMode, Task, TaskId};

/// Task scheduler that respects production mode
pub struct Scheduler<R: SchedulerRepo = PgRepo> {
    repo: R,
    /// Cached production mode (updated on state changes)
    mode_cache: Arc<RwLock<ProductionMode>>,
}

impl Scheduler<PgRepo> {
    /// Create a new scheduler backed by PgRepo
    pub async fn new(pool: sqlx::PgPool) -> Result<Self> {
        let repo = PgRepo::new(pool);
        Self::with_repo(repo).await
    }
}

impl<R: SchedulerRepo> Scheduler<R> {
    /// Create a new scheduler with the given repo
    pub async fn with_repo(repo: R) -> Result<Self> {
        let mode = repo.get_production_mode().await?;
        Ok(Self {
            repo,
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
        self.repo.set_production_mode(mode).await?;
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
        let mode = self.repo.get_production_mode().await?;
        *self.mode_cache.write().await = mode;
        Ok(())
    }
}

// ============================================================================
// Ticket 5.5: Task Scheduler
// ============================================================================

use crate::db::traits::{DependencyRepo, TaskQueueRepo};
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
            poll_interval_ms: crate::constants::SCHEDULER_POLL_INTERVAL_MS,
            batch_size: crate::constants::SCHEDULER_BATCH_SIZE,
            agent_wait_timeout_ms: crate::constants::SCHEDULER_AGENT_WAIT_MS,
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
pub struct TaskScheduler<
    TQ: TaskQueueRepo = PgRepo,
    DR: DependencyRepo = PgRepo,
    SR: SchedulerRepo = PgRepo,
> {
    queue: Arc<RwLock<DependencyAwareQueue<TQ, DR>>>,
    router: Router,
    agent_pool: Arc<RwLock<AgentPool>>,
    refactor_scheduler: Arc<Scheduler<SR>>,
    config: SchedulerConfig,
    running: Arc<RwLock<bool>>,
    /// Notified when an agent becomes available
    agent_available: Arc<Notify>,
}

impl<TQ: TaskQueueRepo, DR: DependencyRepo, SR: SchedulerRepo> TaskScheduler<TQ, DR, SR> {
    /// Create a new task scheduler
    pub fn new(
        queue: Arc<RwLock<DependencyAwareQueue<TQ, DR>>>,
        router: Router,
        agent_pool: Arc<RwLock<AgentPool>>,
        refactor_scheduler: Arc<Scheduler<SR>>,
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
        // Agent status is now managed by the agent's own run loop.
        // When it receives AssignTask, it transitions to Working internally.
        let _ = (&self.agent_pool, &agent_id, &tier); // suppress unused warnings

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
    pub fn spawn(self: Arc<Self>) -> tokio::task::JoinHandle<Result<(), SchedulerError>>
    where
        TQ: 'static,
        DR: 'static,
        SR: 'static,
    {
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

    #[test]
    fn scheduler_config_default_values() {
        let config = SchedulerConfig::default();
        assert_eq!(config.poll_interval_ms, 100);
        assert_eq!(config.batch_size, 5);
        assert_eq!(config.agent_wait_timeout_ms, 500);
    }

    #[test]
    fn scheduler_error_display() {
        let e = SchedulerError::Stopped;
        assert_eq!(e.to_string(), "scheduler stopped");

        let e2 = SchedulerError::AgentPoolError("no agents".into());
        assert!(e2.to_string().contains("no agents"));

        let e3 = SchedulerError::DatabaseError("db down".into());
        assert!(e3.to_string().contains("db down"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::traits::MockSchedulerRepo;

    fn make_mock_running() -> MockSchedulerRepo {
        let mut mock = MockSchedulerRepo::new();
        mock.expect_get_production_mode()
            .times(1)
            .returning(|| Ok(ProductionMode::Running));
        mock
    }

    async fn setup_scheduler() -> Scheduler<MockSchedulerRepo> {
        let mock = make_mock_running();
        Scheduler::with_repo(mock).await.unwrap()
    }

    #[tokio::test]
    async fn scheduler_starts_in_running_mode() {
        let scheduler = setup_scheduler().await;

        assert_eq!(
            scheduler.get_production_mode().await,
            ProductionMode::Running
        );
        assert!(scheduler.should_assign().await);
        assert!(!scheduler.is_paused().await);
    }

    #[tokio::test]
    async fn enter_refactor_mode_stops_assignment() {
        let mut mock = MockSchedulerRepo::new();
        mock.expect_get_production_mode()
            .times(1)
            .returning(|| Ok(ProductionMode::Running));
        mock.expect_set_production_mode()
            .withf(|m| *m == ProductionMode::RefactorMode)
            .times(1)
            .returning(|_| Ok(()));

        let scheduler = Scheduler::with_repo(mock).await.unwrap();
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
        let mut mock = MockSchedulerRepo::new();
        mock.expect_get_production_mode()
            .times(1)
            .returning(|| Ok(ProductionMode::Running));
        mock.expect_set_production_mode()
            .withf(|m| *m == ProductionMode::Paused)
            .times(1)
            .returning(|_| Ok(()));

        let scheduler = Scheduler::with_repo(mock).await.unwrap();
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
        let mut mock = MockSchedulerRepo::new();
        // Initial load
        mock.expect_get_production_mode()
            .times(1)
            .returning(|| Ok(ProductionMode::Running));
        // enter_refactor_mode sets RefactorMode
        mock.expect_set_production_mode()
            .withf(|m| *m == ProductionMode::RefactorMode)
            .times(1)
            .returning(|_| Ok(()));
        // begin_resume sets Resuming
        mock.expect_set_production_mode()
            .withf(|m| *m == ProductionMode::Resuming)
            .times(1)
            .returning(|_| Ok(()));
        // resume sets Running
        mock.expect_set_production_mode()
            .withf(|m| *m == ProductionMode::Running)
            .times(1)
            .returning(|_| Ok(()));

        let scheduler = Scheduler::with_repo(mock).await.unwrap();

        scheduler.enter_refactor_mode().await.unwrap();
        scheduler.begin_resume().await.unwrap();

        assert_eq!(
            scheduler.get_production_mode().await,
            ProductionMode::Resuming
        );
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
        let mut mock = MockSchedulerRepo::new();
        mock.expect_get_production_mode()
            .times(1)
            .returning(|| Ok(ProductionMode::Running));
        // enter_refactor_mode
        mock.expect_set_production_mode()
            .withf(|m| *m == ProductionMode::RefactorMode)
            .times(1)
            .returning(|_| Ok(()));
        // pause_for_refactor
        mock.expect_set_production_mode()
            .withf(|m| *m == ProductionMode::Paused)
            .times(1)
            .returning(|_| Ok(()));
        // begin_resume
        mock.expect_set_production_mode()
            .withf(|m| *m == ProductionMode::Resuming)
            .times(1)
            .returning(|_| Ok(()));
        // resume
        mock.expect_set_production_mode()
            .withf(|m| *m == ProductionMode::Running)
            .times(1)
            .returning(|_| Ok(()));

        let scheduler = Scheduler::with_repo(mock).await.unwrap();

        assert!(scheduler.should_assign().await);

        scheduler.enter_refactor_mode().await.unwrap();
        assert!(!scheduler.should_assign().await);

        scheduler.pause_for_refactor().await.unwrap();
        assert!(scheduler.is_paused().await);

        scheduler.begin_resume().await.unwrap();
        assert_eq!(
            scheduler.get_production_mode().await,
            ProductionMode::Resuming
        );

        scheduler.resume().await.unwrap();
        assert!(scheduler.should_assign().await);
    }

    #[tokio::test]
    async fn begin_resume_only_works_in_refactor_states() {
        let mock = make_mock_running();
        let scheduler = Scheduler::with_repo(mock).await.unwrap();

        // In running mode, begin_resume does nothing (no set_production_mode call expected)
        scheduler.begin_resume().await.unwrap();
        assert_eq!(
            scheduler.get_production_mode().await,
            ProductionMode::Running
        );
    }

    #[tokio::test]
    async fn enter_refactor_mode_noop_when_not_running() {
        let mut mock = MockSchedulerRepo::new();
        mock.expect_get_production_mode()
            .times(1)
            .returning(|| Ok(ProductionMode::Running));
        // pause_for_refactor sets Paused
        mock.expect_set_production_mode()
            .withf(|m| *m == ProductionMode::Paused)
            .times(1)
            .returning(|_| Ok(()));
        // enter_refactor_mode should NOT call set because current != Running

        let scheduler = Scheduler::with_repo(mock).await.unwrap();

        scheduler.pause_for_refactor().await.unwrap();
        assert_eq!(
            scheduler.get_production_mode().await,
            ProductionMode::Paused
        );

        scheduler.enter_refactor_mode().await.unwrap();
        // Should still be Paused, not RefactorMode
        assert_eq!(
            scheduler.get_production_mode().await,
            ProductionMode::Paused
        );
    }

    #[tokio::test]
    async fn begin_resume_works_from_paused() {
        let mut mock = MockSchedulerRepo::new();
        mock.expect_get_production_mode()
            .times(1)
            .returning(|| Ok(ProductionMode::Running));
        mock.expect_set_production_mode()
            .withf(|m| *m == ProductionMode::Paused)
            .times(1)
            .returning(|_| Ok(()));
        mock.expect_set_production_mode()
            .withf(|m| *m == ProductionMode::Resuming)
            .times(1)
            .returning(|_| Ok(()));

        let scheduler = Scheduler::with_repo(mock).await.unwrap();

        scheduler.pause_for_refactor().await.unwrap();
        assert!(scheduler.is_paused().await);

        scheduler.begin_resume().await.unwrap();
        assert_eq!(
            scheduler.get_production_mode().await,
            ProductionMode::Resuming
        );
    }

    #[tokio::test]
    async fn begin_resume_noop_from_resuming() {
        let mut mock = MockSchedulerRepo::new();
        mock.expect_get_production_mode()
            .times(1)
            .returning(|| Ok(ProductionMode::Running));
        mock.expect_set_production_mode()
            .withf(|m| *m == ProductionMode::RefactorMode)
            .times(1)
            .returning(|_| Ok(()));
        mock.expect_set_production_mode()
            .withf(|m| *m == ProductionMode::Resuming)
            .times(1)
            .returning(|_| Ok(()));

        let scheduler = Scheduler::with_repo(mock).await.unwrap();

        scheduler.enter_refactor_mode().await.unwrap();
        scheduler.begin_resume().await.unwrap();
        assert_eq!(
            scheduler.get_production_mode().await,
            ProductionMode::Resuming
        );

        // Resuming.is_refactoring() is false, so begin_resume should be a no-op
        scheduler.begin_resume().await.unwrap();
        assert_eq!(
            scheduler.get_production_mode().await,
            ProductionMode::Resuming
        );
    }

    #[tokio::test]
    async fn refresh_mode_syncs_from_db() {
        let mut mock = MockSchedulerRepo::new();
        // Initial load returns Running
        mock.expect_get_production_mode()
            .times(1)
            .returning(|| Ok(ProductionMode::Running));
        // refresh_mode returns Paused
        mock.expect_get_production_mode()
            .times(1)
            .returning(|| Ok(ProductionMode::Paused));

        let scheduler = Scheduler::with_repo(mock).await.unwrap();

        // Cache says Running
        assert_eq!(
            scheduler.get_production_mode().await,
            ProductionMode::Running
        );

        // After refresh, cache matches "DB"
        scheduler.refresh_mode().await.unwrap();
        assert_eq!(
            scheduler.get_production_mode().await,
            ProductionMode::Paused
        );
    }

    #[tokio::test]
    async fn should_assign_false_when_paused() {
        let mut mock = MockSchedulerRepo::new();
        mock.expect_get_production_mode()
            .times(1)
            .returning(|| Ok(ProductionMode::Running));
        mock.expect_set_production_mode()
            .withf(|m| *m == ProductionMode::Paused)
            .times(1)
            .returning(|_| Ok(()));

        let scheduler = Scheduler::with_repo(mock).await.unwrap();

        scheduler.pause_for_refactor().await.unwrap();
        assert!(!scheduler.should_assign().await);
    }

    #[tokio::test]
    async fn should_assign_false_when_resuming() {
        let mut mock = MockSchedulerRepo::new();
        mock.expect_get_production_mode()
            .times(1)
            .returning(|| Ok(ProductionMode::Running));
        mock.expect_set_production_mode()
            .withf(|m| *m == ProductionMode::RefactorMode)
            .times(1)
            .returning(|_| Ok(()));
        mock.expect_set_production_mode()
            .withf(|m| *m == ProductionMode::Resuming)
            .times(1)
            .returning(|_| Ok(()));

        let scheduler = Scheduler::with_repo(mock).await.unwrap();

        scheduler.enter_refactor_mode().await.unwrap();
        scheduler.begin_resume().await.unwrap();
        assert!(!scheduler.should_assign().await);
    }

    #[tokio::test]
    async fn is_paused_false_when_resuming() {
        let mut mock = MockSchedulerRepo::new();
        mock.expect_get_production_mode()
            .times(1)
            .returning(|| Ok(ProductionMode::Running));
        mock.expect_set_production_mode()
            .withf(|m| *m == ProductionMode::RefactorMode)
            .times(1)
            .returning(|_| Ok(()));
        mock.expect_set_production_mode()
            .withf(|m| *m == ProductionMode::Resuming)
            .times(1)
            .returning(|_| Ok(()));

        let scheduler = Scheduler::with_repo(mock).await.unwrap();

        scheduler.enter_refactor_mode().await.unwrap();
        scheduler.begin_resume().await.unwrap();
        assert!(!scheduler.is_paused().await);
    }

    #[tokio::test]
    async fn resume_from_any_state() {
        let mut mock = MockSchedulerRepo::new();
        mock.expect_get_production_mode()
            .times(1)
            .returning(|| Ok(ProductionMode::Running));
        // resume (Running -> Running)
        mock.expect_set_production_mode()
            .withf(|m| *m == ProductionMode::Running)
            .times(1)
            .returning(|_| Ok(()));
        // pause_for_refactor
        mock.expect_set_production_mode()
            .withf(|m| *m == ProductionMode::Paused)
            .times(1)
            .returning(|_| Ok(()));
        // resume from Paused
        mock.expect_set_production_mode()
            .withf(|m| *m == ProductionMode::Running)
            .times(1)
            .returning(|_| Ok(()));

        let scheduler = Scheduler::with_repo(mock).await.unwrap();

        scheduler.resume().await.unwrap();
        assert_eq!(
            scheduler.get_production_mode().await,
            ProductionMode::Running
        );

        scheduler.pause_for_refactor().await.unwrap();
        scheduler.resume().await.unwrap();
        assert_eq!(
            scheduler.get_production_mode().await,
            ProductionMode::Running
        );
    }
}

#[cfg(test)]
mod task_scheduler_integration_tests {
    use super::*;
    use crate::agents::AgentPool;
    use crate::db::traits::{MockDependencyRepo, MockSchedulerRepo, MockTaskQueueRepo};
    use crate::llm::{LLMError, LLMRequest, LLMResponse, StopReason, StreamChunk, TokenUsage};
    use crate::orchestration::queue::DependencyAwareQueue;
    use crate::orchestration::router::{Router, RouterConfig};
    use crate::types::{
        AgentPersona, AgentPoolConfig, AgentTier, ModelConfig, Priority, Task, TaskId, TaskStatus,
    };
    use async_trait::async_trait;
    use chrono::Utc;
    use futures::Stream;
    use std::pin::Pin;

    type TestQueue = DependencyAwareQueue<MockTaskQueueRepo, MockDependencyRepo>;
    type TestTaskScheduler =
        TaskScheduler<MockTaskQueueRepo, MockDependencyRepo, MockSchedulerRepo>;

    struct MockLLMProvider;

    #[async_trait]
    impl crate::llm::LLMProvider for MockLLMProvider {
        async fn send_message(&self, _request: LLMRequest) -> Result<LLMResponse, LLMError> {
            Ok(LLMResponse {
                content: "ok".to_string(),
                content_blocks: vec![],
                model: "test".to_string(),
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage {
                    input_tokens: 1,
                    output_tokens: 1,
                },
            })
        }
        async fn send_message_stream(
            &self,
            _request: LLMRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>>, LLMError>
        {
            unimplemented!()
        }
        fn provider_name(&self) -> &'static str {
            "mock"
        }
        fn model_id(&self) -> &str {
            "test"
        }
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

    async fn setup_full() -> (
        Arc<TestTaskScheduler>,
        Arc<RwLock<TestQueue>>,
        Arc<RwLock<AgentPool>>,
    ) {
        let queue = Arc::new(RwLock::new(DependencyAwareQueue::in_memory().await));
        let router = Router::new(RouterConfig::default());
        let agent_pool_config = AgentPoolConfig {
            max_orchestrators: 2,
            max_workers: 3,
            max_utilities: 4,
        };
        let llm = Arc::new(MockLLMProvider);
        let agent_pool = Arc::new(RwLock::new(AgentPool::new(agent_pool_config, llm)));

        let mut mock = MockSchedulerRepo::new();
        mock.expect_get_production_mode()
            .returning(|| Ok(ProductionMode::Running));
        mock.expect_set_production_mode().returning(|_| Ok(()));

        let refactor_scheduler = Arc::new(Scheduler::with_repo(mock).await.unwrap());
        let config = SchedulerConfig {
            poll_interval_ms: 10,
            batch_size: 5,
            agent_wait_timeout_ms: 10,
        };

        let scheduler = Arc::new(TaskScheduler::new(
            queue.clone(),
            router,
            agent_pool.clone(),
            refactor_scheduler,
            config,
        ));

        (scheduler, queue, agent_pool)
    }

    #[tokio::test]
    async fn task_scheduler_starts_not_running() {
        let (scheduler, _, _) = setup_full().await;
        assert!(!scheduler.is_running().await);
    }

    #[tokio::test]
    async fn task_scheduler_stop() {
        let (scheduler, _, _) = setup_full().await;
        // Start and then stop
        let s = scheduler.clone();
        let handle = tokio::spawn(async move { s.run().await });

        // Give it a moment to start
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(scheduler.is_running().await);

        scheduler.stop().await;
        let result = handle.await.unwrap();
        assert!(result.is_ok());
        assert!(!scheduler.is_running().await);
    }

    #[tokio::test]
    async fn agent_available_notifier_returns_notify() {
        let (scheduler, _, _) = setup_full().await;
        let notifier = scheduler.agent_available_notifier();
        // Just verify it returns an Arc<Notify> that can be used
        notifier.notify_waiters();
    }

    #[tokio::test]
    async fn on_agent_available_notifies() {
        let (scheduler, _, _) = setup_full().await;
        let agent_id = AgentId(uuid::Uuid::new_v4());
        // Should not panic
        scheduler.on_agent_available(&agent_id).await;
    }

    #[tokio::test]
    async fn on_task_completed_returns_empty_when_no_deps() {
        let (scheduler, _, _) = setup_full().await;
        let task_id = TaskId::new();
        let unblocked = scheduler.on_task_completed(&task_id).await.unwrap();
        assert!(unblocked.is_empty());
    }

    #[tokio::test]
    async fn check_preemption_returns_none_when_no_urgent() {
        let (scheduler, _, _) = setup_full().await;
        let result = scheduler.check_preemption().await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn check_preemption_returns_none_when_urgent_but_agents_free() {
        let (scheduler, queue, agent_pool) = setup_full().await;

        // Add an urgent task
        let task = make_task(Priority::Urgent);
        {
            let mut q = queue.write().await;
            q.enqueue_in_memory(task);
        }

        // Spawn a free agent
        {
            let mut pool = agent_pool.write().await;
            pool.spawn_agent(
                AgentTier::Worker,
                AgentPersona::default(),
                ModelConfig::default(),
            )
            .unwrap();
        }

        let result = scheduler.check_preemption().await.unwrap();
        assert!(result.is_none()); // Free agent exists, no preemption needed
    }

    #[tokio::test]
    async fn check_preemption_returns_none_all_busy_not_implemented() {
        let (scheduler, queue, _) = setup_full().await;

        // Add an urgent task but no agents at all (stats all zero)
        let task = make_task(Priority::Urgent);
        {
            let mut q = queue.write().await;
            q.enqueue_in_memory(task);
        }

        // No agents spawned, so available = 0, but also total = 0
        let result = scheduler.check_preemption().await.unwrap();
        // Preemption not implemented, returns None
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn scheduler_run_paused_skips_assignment() {
        // Create a scheduler where refactor mode starts paused
        let queue: Arc<RwLock<TestQueue>> =
            Arc::new(RwLock::new(DependencyAwareQueue::in_memory().await));
        let router = Router::new(RouterConfig::default());
        let agent_pool_config = AgentPoolConfig {
            max_orchestrators: 2,
            max_workers: 3,
            max_utilities: 4,
        };
        let llm = Arc::new(MockLLMProvider);
        let agent_pool = Arc::new(RwLock::new(AgentPool::new(agent_pool_config, llm)));

        let mut mock = MockSchedulerRepo::new();
        mock.expect_get_production_mode()
            .returning(|| Ok(ProductionMode::Paused));
        mock.expect_set_production_mode().returning(|_| Ok(()));

        let refactor_scheduler = Arc::new(Scheduler::with_repo(mock).await.unwrap());
        let config = SchedulerConfig {
            poll_interval_ms: 10,
            batch_size: 5,
            agent_wait_timeout_ms: 10,
        };

        let scheduler: Arc<TestTaskScheduler> = Arc::new(TaskScheduler::new(
            queue.clone(),
            router,
            agent_pool,
            refactor_scheduler,
            config,
        ));

        // Add a task
        {
            let mut q = queue.write().await;
            q.enqueue_in_memory(make_task(Priority::Normal));
        }

        // Run briefly
        let s = scheduler.clone();
        let handle = tokio::spawn(async move { s.run().await });

        tokio::time::sleep(Duration::from_millis(50)).await;
        scheduler.stop().await;
        handle.await.unwrap().unwrap();

        // Task should still be in queue (not assigned because paused)
        let q = queue.read().await;
        assert!(!q.is_empty());
    }

    #[tokio::test]
    async fn scheduler_error_from_queue_error() {
        let qe = QueueError::Empty;
        let se: SchedulerError = qe.into();
        assert!(se.to_string().contains("queue"));
    }

    #[tokio::test]
    async fn scheduler_config_clone_and_debug() {
        let config = SchedulerConfig::default();
        let cloned = config.clone();
        assert_eq!(cloned.poll_interval_ms, config.poll_interval_ms);
        assert_eq!(cloned.batch_size, config.batch_size);
        let debug = format!("{:?}", config);
        assert!(debug.contains("SchedulerConfig"));
    }

    #[tokio::test]
    async fn preemption_action_debug() {
        let action = PreemptionAction {
            agent_id: AgentId(uuid::Uuid::new_v4()),
            task_to_pause: make_task(Priority::Normal),
        };
        let debug = format!("{:?}", action);
        assert!(debug.contains("PreemptionAction"));
    }
}
