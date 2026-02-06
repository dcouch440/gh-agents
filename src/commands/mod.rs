//! Command-line command handlers

mod serve;
mod sync;

pub use serve::run_serve;
pub use sync::run_sync;

use anyhow::Result;

use crate::cli::{Args, Commands};

/// Dispatch CLI command to appropriate handler
pub async fn dispatch(args: Args) -> Result<()> {
    match args.command {
        Some(Commands::SyncConfig { config_dir, dry_run }) => {
            let verbose = args.verbose > 0;
            run_sync(&config_dir, dry_run, verbose).await
        }
        Some(Commands::Serve { .. }) | None => {
            // Default to server mode if no command specified
            run_serve(args).await
        }
    }
}
