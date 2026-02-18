//! Config sync command handler

use anyhow::Result;
use sqlx::PgPool;
use std::path::Path;

use crate::config::sync_config;
use crate::db::init_db;
use crate::db::pg_repo::PgRepo;
use crate::db::traits::{ProtocolRepo, ToolRepo};

/// Run config sync command
pub async fn run_sync(config_dir: &Path, dry_run: bool, verbose: bool) -> Result<()> {
    println!("🔄 nexor Config Sync");
    println!("   Directory: {}", config_dir.display());
    if dry_run {
        println!("   Mode: DRY RUN (validation only)");
    }
    println!();

    // Initialize database
    let pool = init_db().await?;

    // Seed built-in tools and protocols (idempotent)
    if !dry_run {
        seed_builtin_tools(&pool, verbose).await?;
        seed_builtin_protocols(&pool, verbose).await?;
    }

    // Sync capabilities and tool assignments from YAML
    let stats = sync_config(&pool, config_dir, dry_run, verbose).await?;

    // Print summary
    if !dry_run {
        println!("\n✅ Sync completed successfully!");
        println!("\n📊 Summary:");
        println!(
            "   Capabilities: {} created, {} updated",
            stats.capabilities_created, stats.capabilities_updated
        );
        println!(
            "   Tool Assignments: {} updated",
            stats.tool_assignments_updated
        );

        if !stats.errors.is_empty() {
            println!("\n⚠️  {} warnings:", stats.errors.len());
            for err in &stats.errors {
                println!("   - {}", err);
            }
        }
    }

    Ok(())
}

/// Seed built-in tools (system-wide, idempotent)
async fn seed_builtin_tools(pool: &PgPool, verbose: bool) -> Result<()> {
    if verbose {
        println!("🔧 Seeding built-in tools...");
    }

    let repo = PgRepo::new(pool.clone());
    repo.seed_builtin_tools().await?;

    if verbose {
        println!("✓ Tools seeded");
    }

    Ok(())
}

/// Seed built-in protocols (system-wide, idempotent)
async fn seed_builtin_protocols(pool: &PgPool, verbose: bool) -> Result<()> {
    if verbose {
        println!("🔧 Seeding built-in protocols...");
    }

    let repo = PgRepo::new(pool.clone());
    repo.seed_builtin_protocols().await?;

    if verbose {
        println!("✓ Protocols seeded");
    }

    Ok(())
}
