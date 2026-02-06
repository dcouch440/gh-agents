//! nexor: AI Agent Orchestration for GitHub Workflows

use anyhow::Result;
use std::net::SocketAddr;
use std::path::Path;
use tracing::{debug, info};

use nexor::cli::{Args, Commands};
use nexor::config::{load_config, sync_config};
use nexor::db::init_db;
use nexor::logging::{init_logging_with_file, LOG_DIR};
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

    // Handle subcommands
    match &args.command {
        Some(Commands::SyncConfig { config_dir, dry_run }) => {
            let verbose = args.verbose > 0;
            run_sync_config(config_dir, *dry_run, verbose).await
        }
        Some(Commands::Serve { .. }) | None => {
            // Default to server mode if no command specified
            run_server_mode(args).await
        }
    }
}

/// Run in server mode (HTTP + WebSocket)
async fn run_server_mode(args: Args) -> Result<()> {
    // Initialize logging with file output
    let log_path = args
        .config()
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

    // Server address from CLI
    let addr: SocketAddr = format!("0.0.0.0:{}", args.port()).parse()?;

    // Run server
    start_server(pool, config, addr).await?;

    info!("nexor shutting down");
    Ok(())
}

/// Run config sync command
async fn run_sync_config(config_dir: &Path, dry_run: bool, verbose: bool) -> Result<()> {
    println!("🔄 nexor Config Sync");
    println!("   Directory: {}", config_dir.display());
    if dry_run {
        println!("   Mode: DRY RUN (validation only)");
    }
    println!();

    // Initialize database
    let pool = init_db().await?;

    // Seed built-in tools for system user (idempotent)
    if !dry_run {
        if verbose {
            println!("🔧 Seeding built-in tools...");
        }

        // Get system user ID (the user that owns all built-in tools)
        let system_user_id = sqlx::query_scalar::<_, uuid::Uuid>(
            "SELECT DISTINCT user_id FROM tools LIMIT 1"
        )
        .fetch_optional(&pool)
        .await?;

        if let Some(user_id) = system_user_id {
            use nexor::db::pg_repo::PgRepo;
            use nexor::db::traits::ServerRepo;
            use nexor::types::UserId;

            let repo = PgRepo::new(pool.clone());
            repo.seed_builtin_tools(UserId(user_id)).await?;

            if verbose {
                println!("✓ Tools seeded");
            }
        } else if verbose {
            println!("⚠ No existing tools found, skipping tool seeding");
        }
    }

    // Run sync
    let stats = sync_config(&pool, config_dir, dry_run, verbose).await?;

    // Print summary
    if !dry_run {
        println!("\n✅ Sync completed successfully!");
        println!("\n📊 Summary:");
        println!("   Capabilities: {} created, {} updated",
            stats.capabilities_created, stats.capabilities_updated);
        println!("   Tool Assignments: {} updated", stats.tool_assignments_updated);

        if !stats.errors.is_empty() {
            println!("\n⚠️  {} warnings:", stats.errors.len());
            for err in &stats.errors {
                println!("   - {}", err);
            }
        }
    }

    Ok(())
}
