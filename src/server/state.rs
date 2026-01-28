//! Shared application state for HTTP handlers

use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::RwLock;

use crate::orchestration::Scheduler;
use crate::types::AppConfig;

/// Application state shared across all HTTP handlers
#[derive(Clone)]
pub struct AppState {
    /// Database connection pool
    pub db: SqlitePool,
    /// Task scheduler for orchestration
    pub scheduler: Arc<RwLock<Scheduler>>,
    /// Application configuration
    pub config: Arc<AppConfig>,
}

impl AppState {
    /// Create new application state
    pub fn new(db: SqlitePool, scheduler: Arc<RwLock<Scheduler>>, config: AppConfig) -> Self {
        Self {
            db,
            scheduler,
            config: Arc::new(config),
        }
    }
}
