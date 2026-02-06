//! Configuration sync from YAML files to database

mod types;
mod validators;

pub use types::*;
pub use validators::*;

use anyhow::{Context, Result};
use sqlx::{PgPool, Postgres, Transaction};
use std::path::Path;

/// Main sync function - syncs all config files to database
pub async fn sync_config(
    pool: &PgPool,
    config_dir: &Path,
    dry_run: bool,
    verbose: bool,
) -> Result<SyncStats> {
    let mut stats = SyncStats::default();

    // 1. Load all YAML files
    if verbose {
        println!("📖 Loading config files from {}", config_dir.display());
    }

    let capabilities = load_capabilities(config_dir, verbose)?;
    let tool_assignments = load_tool_assignments(config_dir, verbose)?;

    // 2. Validate everything before touching database
    if verbose {
        println!("✓ Validating configurations...");
    }
    validate_all(&capabilities, &tool_assignments)?;

    if verbose {
        println!("✓ All validations passed");
    }

    if dry_run {
        println!("🔍 DRY RUN: Would sync {} capabilities and {} tool assignments",
            capabilities.capabilities.len(),
            tool_assignments.tool_assignments.len());
        return Ok(stats);
    }

    // 3. Begin transaction
    let mut tx = pool.begin().await?;

    // 4. Sync each table
    if verbose {
        println!("📝 Syncing capabilities...");
    }
    sync_capabilities(&mut tx, &capabilities, &mut stats, verbose).await?;

    if verbose {
        println!("📝 Syncing tool assignments...");
    }
    sync_tool_assignments(&mut tx, &tool_assignments, &mut stats, verbose).await?;

    // 5. Commit transaction
    tx.commit().await?;

    if verbose {
        println!("✅ Sync complete!");
        print_stats(&stats);
    }

    Ok(stats)
}

/// Load capabilities.yaml
fn load_capabilities(config_dir: &Path, verbose: bool) -> Result<CapabilitiesYaml> {
    let path = config_dir.join("capabilities.yaml");
    if verbose {
        println!("  - Loading {}", path.display());
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))
}

/// Load tool_assignments.yaml
fn load_tool_assignments(config_dir: &Path, verbose: bool) -> Result<ToolAssignmentsYaml> {
    let path = config_dir.join("tool_assignments.yaml");
    if verbose {
        println!("  - Loading {}", path.display());
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))
}

/// Sync capabilities to tool_capabilities table
async fn sync_capabilities(
    tx: &mut Transaction<'_, Postgres>,
    capabilities: &CapabilitiesYaml,
    stats: &mut SyncStats,
    verbose: bool,
) -> Result<()> {
    for cap in &capabilities.capabilities {
        // UPSERT on capability_key
        let result = sqlx::query!(
            r#"
            INSERT INTO tool_capabilities (
                capability_key, display_name, category, safety_level, description
            )
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (capability_key) DO UPDATE
            SET
                display_name = EXCLUDED.display_name,
                category = EXCLUDED.category,
                safety_level = EXCLUDED.safety_level,
                description = EXCLUDED.description
            RETURNING (xmax = 0) AS created
            "#,
            cap.key,
            cap.display_name,
            cap.category,
            cap.safety_level,
            cap.description
        )
        .fetch_one(&mut **tx)
        .await
        .with_context(|| format!("Failed to sync capability '{}'", cap.key))?;

        if result.created.unwrap_or(false) {
            stats.capabilities_created += 1;
            if verbose {
                println!("  ✓ Created capability: {}", cap.key);
            }
        } else {
            stats.capabilities_updated += 1;
            if verbose {
                println!("  ↻ Updated capability: {}", cap.key);
            }
        }
    }

    Ok(())
}

/// Sync tool assignments to tool_capability_assignments table
async fn sync_tool_assignments(
    tx: &mut Transaction<'_, Postgres>,
    assignments: &ToolAssignmentsYaml,
    stats: &mut SyncStats,
    verbose: bool,
) -> Result<()> {
    for (tool_name, assignment) in &assignments.tool_assignments {
        // Get tool ID
        let tool = sqlx::query!(
            r#"SELECT id FROM tools WHERE name = $1"#,
            tool_name
        )
        .fetch_optional(&mut **tx)
        .await?;

        let Some(tool) = tool else {
            if verbose {
                println!("  ⚠ Tool '{}' not found in database, skipping", tool_name);
            }
            stats.add_error(format!("Tool '{}' not found", tool_name));
            continue;
        };

        // Delete existing assignments for this tool
        sqlx::query!(
            r#"DELETE FROM tool_capability_assignments WHERE tool_id = $1"#,
            tool.id
        )
        .execute(&mut **tx)
        .await?;

        // Insert new assignments
        for cap_key in &assignment.capabilities {
            // Get capability ID
            let cap = sqlx::query!(
                r#"SELECT id FROM tool_capabilities WHERE capability_key = $1"#,
                cap_key
            )
            .fetch_optional(&mut **tx)
            .await?;

            let Some(cap) = cap else {
                if verbose {
                    println!("  ⚠ Capability '{}' not found, skipping assignment to '{}'",
                        cap_key, tool_name);
                }
                stats.add_error(format!(
                    "Capability '{}' not found for tool '{}'",
                    cap_key, tool_name
                ));
                continue;
            };

            sqlx::query!(
                r#"
                INSERT INTO tool_capability_assignments (tool_id, capability_id)
                VALUES ($1, $2)
                ON CONFLICT (tool_id, capability_id) DO NOTHING
                "#,
                tool.id,
                cap.id
            )
            .execute(&mut **tx)
            .await?;
        }

        stats.tool_assignments_updated += 1;
        if verbose {
            println!("  ✓ Updated assignments for tool: {} ({} capabilities)",
                tool_name, assignment.capabilities.len());
        }
    }

    Ok(())
}

/// Print sync statistics
fn print_stats(stats: &SyncStats) {
    println!("\n📊 Sync Statistics:");
    println!("  Capabilities: {} created, {} updated",
        stats.capabilities_created, stats.capabilities_updated);
    println!("  Tool Assignments: {} updated", stats.tool_assignments_updated);

    if !stats.errors.is_empty() {
        println!("\n⚠️  {} warnings:", stats.errors.len());
        for err in &stats.errors {
            println!("  - {}", err);
        }
    }
}
