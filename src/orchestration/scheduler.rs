//! Task scheduler with pause/resume support for refactor mode.
//!
//! The scheduler controls work assignment based on production mode:
//! - Running: Normal operation, tasks are assigned
//! - RefactorMode/Paused: No new work assigned
//! - Resuming: Transitioning back to running

use anyhow::Result;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::db::{get_production_mode, set_production_mode};
use crate::types::ProductionMode;

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

        assert_eq!(scheduler.get_production_mode().await, ProductionMode::Running);
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

        assert_eq!(scheduler.get_production_mode().await, ProductionMode::Paused);
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

        assert_eq!(scheduler.get_production_mode().await, ProductionMode::Running);
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
        assert_eq!(scheduler.get_production_mode().await, ProductionMode::Running);
    }
}
