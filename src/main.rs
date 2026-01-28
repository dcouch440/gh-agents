//! nexor: AI Agent Orchestration TUI for GitHub Workflows

use std::path::Path;

use anyhow::Result;
use tracing::{debug, info};

use nexor::logging::{init_logging_with_file, LOG_DIR};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging with file output
    let _guard = init_logging_with_file(Some(Path::new(LOG_DIR)))?;

    info!("nexor starting...");
    debug!("Debug logging enabled");

    // TODO: Initialize application

    info!("nexor shutting down");
    Ok(())
}
