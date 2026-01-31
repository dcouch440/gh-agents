//! nexor: AI Agent Orchestration for GitHub Workflows

use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tracing::{debug, info};

use nexor::cli::Args;
use nexor::config::load_config;
use nexor::db::init_db;
use nexor::logging::{init_logging_with_file, LOG_DIR};
use nexor::orchestration::Scheduler;
use nexor::server::start_server;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if present (ignore errors if not found)
    let _ = dotenvy::dotenv();

    // Parse command-line arguments
    let args = Args::parse_args();

    // Validate argument combinations
    if let Err(e) = args.validate() {
        eprintln!("Error: {}", e);
        eprintln!("Run 'nexor --help' for usage information.");
        std::process::exit(1);
    }

    run_server_mode(args).await
}

/// Run in server mode (HTTP + WebSocket)
async fn run_server_mode(args: Args) -> Result<()> {
    // Initialize logging with file output
    let log_path = args
        .config
        .as_ref()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| Path::new(LOG_DIR).to_path_buf());
    let _guard = init_logging_with_file(Some(&log_path))?;

    info!("nexor server starting...");
    debug!("Debug logging enabled (verbosity: {})", args.verbose);

    // Load configuration
    let config = load_config().unwrap_or_default();

    // Initialize database
    let pool = init_db().await?;

    // Initialize scheduler
    let scheduler = Scheduler::new(pool.clone()).await?;
    let scheduler = Arc::new(RwLock::new(scheduler));

    // Server address from CLI
    let addr: SocketAddr = format!("0.0.0.0:{}", args.port).parse()?;

    // Run server
    start_server(pool, scheduler, config, addr).await?;

    info!("nexor shutting down");
    Ok(())
}
