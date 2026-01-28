//! nexor: AI Agent Orchestration TUI for GitHub Workflows

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

use nexor::cli::Args;
use nexor::db::init_db;
use nexor::headless::HeadlessRunner;
use nexor::logging::{init_logging_with_file, LOG_DIR};
use nexor::orchestration::Scheduler;
use nexor::tui::{init_terminal, install_panic_hook, restore_terminal, App};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments
    let args = Args::parse_args();

    // Validate argument combinations
    if let Err(e) = args.validate() {
        eprintln!("Error: {}", e);
        eprintln!("Run 'nexor --help' for usage information.");
        std::process::exit(1);
    }

    if args.is_headless() {
        // Headless mode - no TUI
        run_headless(args).await
    } else {
        // Interactive TUI mode
        run_tui(args).await
    }
}

/// Run in headless mode (no TUI)
async fn run_headless(args: Args) -> Result<()> {
    // Initialize logging based on verbosity (to stderr so stdout is clean for output)
    let log_level = args.log_level();
    let filter = tracing_subscriber::EnvFilter::new(format!(
        "nexor={},sqlx=warn",
        log_level
    ));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    info!("nexor headless mode starting...");

    // Run headless session
    let runner = HeadlessRunner::new(args)?;
    runner.run().await?;

    info!("nexor headless mode complete");
    Ok(())
}

/// Run in interactive TUI mode
async fn run_tui(args: Args) -> Result<()> {
    // Install panic hook first (before any TUI setup)
    install_panic_hook();

    // Initialize logging with file output
    let log_path = args.config.as_ref()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| Path::new(LOG_DIR).to_path_buf());
    let _guard = init_logging_with_file(Some(&log_path))?;

    info!("nexor starting...");
    debug!("Debug logging enabled (verbosity: {})", args.verbose);

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
