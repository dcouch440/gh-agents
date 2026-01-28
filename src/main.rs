//! nexor: AI Agent Orchestration TUI for GitHub Workflows

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

use nexor::db::init_db;
use nexor::logging::{init_logging_with_file, LOG_DIR};
use nexor::orchestration::Scheduler;
use nexor::tui::{init_terminal, install_panic_hook, restore_terminal, App};

#[tokio::main]
async fn main() -> Result<()> {
    // Install panic hook first (before any TUI setup)
    install_panic_hook();

    // Initialize logging with file output
    let _guard = init_logging_with_file(Some(Path::new(LOG_DIR)))?;

    info!("nexor starting...");
    debug!("Debug logging enabled");

    // Initialize database
    let pool = init_db().await?;

    // Initialize scheduler
    let scheduler = Scheduler::new(pool.clone()).await?;
    let scheduler = Arc::new(RwLock::new(scheduler));

    // Get project root (current directory)
    let project_root = std::env::current_dir()?;

    // Create app
    let mut app = App::new(scheduler, pool, project_root);

    // Initialize terminal
    let mut terminal = init_terminal()?;

    // Run the app
    let result = app.run(&mut terminal).await;

    // Always restore terminal, even on error
    if let Err(e) = restore_terminal(&mut terminal) {
        error!("Failed to restore terminal: {}", e);
    }

    // Now handle any app error
    if let Err(e) = result {
        error!("Application error: {}", e);
        return Err(e);
    }

    info!("nexor shutting down");
    Ok(())
}
