//! Config sync command handler

use anyhow::Result;
use sqlx::PgPool;
use std::path::Path;

use crate::config::sync_config;
use crate::db::init_db;
use crate::db::pg_repo::PgRepo;
use crate::db::traits::ServerRepo;
use crate::types::UserId;

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

    // Seed built-in tools (idempotent)
    if !dry_run {
        seed_builtin_tools(&pool, verbose).await?;
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

/// Seed built-in tools for the system user (idempotent)
async fn seed_builtin_tools(pool: &PgPool, verbose: bool) -> Result<()> {
    if verbose {
        println!("🔧 Seeding built-in tools...");
    }

    // Get system user ID (the user that owns all built-in tools)
    let system_user_id = sqlx::query_scalar::<_, uuid::Uuid>(
        "SELECT id FROM users ORDER BY created_at LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;

    if let Some(user_id) = system_user_id {
        let repo = PgRepo::new(pool.clone());
        repo.seed_builtin_tools(UserId(user_id)).await?;

        if verbose {
            println!("✓ Tools seeded");
        }
    } else if verbose {
        println!("⚠ No users found, skipping tool seeding");
    }

    Ok(())
}
