//! nexor: AI Agent Orchestration for GitHub Workflows
//!
//! Main entry point - parses CLI arguments and dispatches to command handlers.

use anyhow::Result;

use nexor::cli::Args;
use nexor::commands;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file (if present)
    let _ = dotenvy::dotenv();

    // Parse and validate command-line arguments
    let args = Args::parse_args();
    if let Err(e) = args.validate() {
        eprintln!("Error: {}", e);
        eprintln!("Run 'nexor --help' for usage information.");
        std::process::exit(1);
    }

    // Dispatch to command handler
    commands::dispatch(args).await
}
