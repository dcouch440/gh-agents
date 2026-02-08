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
    let system_agents = load_system_agents(config_dir, verbose)?;
    let protocols = load_protocols(config_dir, verbose)?;

    // 2. Validate everything before touching database
    if verbose {
        println!("✓ Validating configurations...");
    }
    validate_all(&capabilities, &tool_assignments)?;

    if verbose {
        println!("✓ All validations passed");
    }

    if dry_run {
        println!(
            "🔍 DRY RUN: Would sync {} capabilities, {} tool assignments, {} system agents, and {} protocols",
            capabilities.capabilities.len(),
            tool_assignments.tool_assignments.len(),
            system_agents.system_agents.len(),
            protocols.protocols.len()
        );
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

    if verbose {
        println!("📝 Syncing system agents...");
    }
    sync_system_agents(&mut tx, &system_agents, &mut stats, verbose).await?;

    if verbose {
        println!("📝 Syncing protocols...");
    }
    sync_protocols(&mut tx, &protocols, &mut stats, verbose).await?;

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

    serde_yaml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
}

/// Load tool_assignments.yaml
fn load_tool_assignments(config_dir: &Path, verbose: bool) -> Result<ToolAssignmentsYaml> {
    let path = config_dir.join("tool_assignments.yaml");
    if verbose {
        println!("  - Loading {}", path.display());
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    serde_yaml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
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
        let tool = sqlx::query!(r#"SELECT id FROM tools WHERE name = $1"#, tool_name)
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
                    println!(
                        "  ⚠ Capability '{}' not found, skipping assignment to '{}'",
                        cap_key, tool_name
                    );
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
            println!(
                "  ✓ Updated assignments for tool: {} ({} capabilities)",
                tool_name,
                assignment.capabilities.len()
            );
        }
    }

    Ok(())
}

/// Load system_agents.yaml
fn load_system_agents(config_dir: &Path, verbose: bool) -> Result<SystemAgentsYaml> {
    let path = config_dir.join("system_agents.yaml");
    if verbose {
        println!("  - Loading {}", path.display());
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    serde_yaml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
}

/// Sync system agents to agents table
async fn sync_system_agents(
    tx: &mut Transaction<'_, Postgres>,
    agents: &SystemAgentsYaml,
    stats: &mut SyncStats,
    verbose: bool,
) -> Result<()> {
    use uuid::Uuid;

    for agent in &agents.system_agents {
        // Generate stable UUID from agent role (deterministic)
        let agent_id = Uuid::new_v5(&Uuid::NAMESPACE_DNS, agent.role.as_bytes());

        // UPSERT system agent (user_id = NULL for system agents)
        let result = sqlx::query!(
            r#"
            INSERT INTO agents (
                id, user_id, name, system_prompt, persona_style,
                model_provider, model_id, model_max_tokens, model_temperature,
                status, router_mode, output_schema_id, version
            )
            VALUES ($1, NULL, $2, $3, NULL, 'anthropic', 'claude-sonnet-4-20250514', 4096, 0.7, 'idle', false, NULL, 1)
            ON CONFLICT (id) DO UPDATE
            SET
                name = EXCLUDED.name,
                system_prompt = EXCLUDED.system_prompt,
                version = agents.version + 1
            RETURNING (xmax = 0) AS "created!"
            "#,
            agent_id,
            agent.name,
            agent.system_prompt,
        )
        .fetch_one(&mut **tx)
        .await?;

        if result.created {
            stats.system_agents_created += 1;
        } else {
            stats.system_agents_updated += 1;
        }

        if verbose {
            let action = if result.created { "Created" } else { "Updated" };
            println!("  ✓ {}: {} ({})", action, agent.name, agent.role);
        }
    }

    Ok(())
}

/// Load protocols.yaml
fn load_protocols(config_dir: &Path, verbose: bool) -> Result<ProtocolsYaml> {
    let path = config_dir.join("protocols.yaml");
    if verbose {
        println!("  - Loading {}", path.display());
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    serde_yaml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
}

/// UUID namespaces for deterministic protocol-related ID generation.
/// Each namespace is distinct to avoid collisions across entity types.
mod protocol_ns {
    use uuid::Uuid;

    pub const PROTOCOLS: Uuid = Uuid::from_bytes([
        0x70, 0x72, 0x6f, 0x74, 0x6f, 0x63, 0x6f, 0x6c, 0x73, 0x2d, 0x6e, 0x65, 0x78, 0x6f, 0x72,
        0x21,
    ]);
    pub const AGENTS: Uuid = Uuid::from_bytes([
        0x70, 0x72, 0x6f, 0x74, 0x6f, 0x2d, 0x61, 0x67, 0x65, 0x6e, 0x74, 0x73, 0x2d, 0x6e, 0x78,
        0x21,
    ]);
    pub const SCHEMAS: Uuid = Uuid::from_bytes([
        0x70, 0x72, 0x6f, 0x74, 0x6f, 0x2d, 0x73, 0x63, 0x68, 0x65, 0x6d, 0x61, 0x73, 0x2d, 0x6e,
        0x78,
    ]);
    pub const TEMPLATES: Uuid = Uuid::from_bytes([
        0x70, 0x72, 0x6f, 0x74, 0x6f, 0x2d, 0x74, 0x6d, 0x70, 0x6c, 0x74, 0x73, 0x2d, 0x6e, 0x78,
        0x21,
    ]);
}

/// Sync protocols from protocols.yaml to database.
///
/// For each protocol, upserts in FK-dependency order:
/// 1. Agent (system-owned, user_id = NULL)
/// 2. Output schema (system-owned, user_id = NULL)
/// 3. Prompt template (if present)
/// 4. Protocol row with FK references
async fn sync_protocols(
    tx: &mut Transaction<'_, Postgres>,
    protocols: &ProtocolsYaml,
    stats: &mut SyncStats,
    verbose: bool,
) -> Result<()> {
    use uuid::Uuid;

    for proto in &protocols.protocols {
        let protocol_id = Uuid::new_v5(&protocol_ns::PROTOCOLS, proto.name.as_bytes());
        let agent_id = Uuid::new_v5(&protocol_ns::AGENTS, proto.name.as_bytes());
        let schema_id = Uuid::new_v5(&protocol_ns::SCHEMAS, proto.name.as_bytes());

        // 1. Upsert dedicated agent (system-owned)
        sqlx::query(
            r#"
            INSERT INTO agents (
                id, user_id, name, system_prompt, persona_style,
                model_provider, model_id, model_max_tokens, model_temperature,
                status, router_mode, output_schema_id, version
            )
            VALUES ($1, NULL, $2, $3, NULL, $4, $5, $6, $7, 'active', false, NULL, 1)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                system_prompt = EXCLUDED.system_prompt,
                model_provider = EXCLUDED.model_provider,
                model_id = EXCLUDED.model_id,
                model_max_tokens = EXCLUDED.model_max_tokens,
                model_temperature = EXCLUDED.model_temperature,
                version = agents.version + 1
            "#,
        )
        .bind(agent_id)
        .bind(&proto.agent.name)
        .bind(&proto.agent.system_prompt)
        .bind(&proto.agent.model_provider)
        .bind(&proto.agent.model_id)
        .bind(proto.agent.model_max_tokens)
        .bind(proto.agent.model_temperature)
        .execute(&mut **tx)
        .await
        .with_context(|| format!("Failed to upsert agent for protocol '{}'", proto.name))?;

        // 2. Upsert output schema (system-owned)
        sqlx::query(
            r#"
            INSERT INTO output_schemas (id, user_id, name, schema, version)
            VALUES ($1, NULL, $2, $3, 1)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                schema = EXCLUDED.schema,
                version = output_schemas.version + 1
            "#,
        )
        .bind(schema_id)
        .bind(&proto.output_schema.name)
        .bind(&proto.output_schema.schema)
        .execute(&mut **tx)
        .await
        .with_context(|| {
            format!(
                "Failed to upsert output schema for protocol '{}'",
                proto.name
            )
        })?;

        // 3. Upsert prompt template if present
        let template_id = if let Some(ref tmpl) = proto.prompt_template {
            let tid = Uuid::new_v5(&protocol_ns::TEMPLATES, proto.name.as_bytes());
            sqlx::query(
                r#"
                INSERT INTO prompt_templates (id, user_id, name, content, version)
                VALUES ($1, NULL, $2, $3, 1)
                ON CONFLICT (id) DO UPDATE SET
                    name = EXCLUDED.name,
                    content = EXCLUDED.content,
                    version = prompt_templates.version + 1
                "#,
            )
            .bind(tid)
            .bind(&tmpl.name)
            .bind(&tmpl.content)
            .execute(&mut **tx)
            .await
            .with_context(|| {
                format!(
                    "Failed to upsert prompt template for protocol '{}'",
                    proto.name
                )
            })?;
            Some(tid)
        } else {
            None
        };

        // 4. Upsert protocol row with FK references
        let config = proto
            .config
            .clone()
            .unwrap_or_else(|| serde_json::json!({}));

        let row: (bool,) = sqlx::query_as(
            r#"
            INSERT INTO protocols (id, name, description, protocol_type, config,
                                   agent_id, output_schema_id, prompt_template_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (name) DO UPDATE SET
                description = EXCLUDED.description,
                protocol_type = EXCLUDED.protocol_type,
                config = EXCLUDED.config,
                agent_id = EXCLUDED.agent_id,
                output_schema_id = EXCLUDED.output_schema_id,
                prompt_template_id = EXCLUDED.prompt_template_id,
                version = protocols.version + 1,
                updated_at = now()
            RETURNING (xmax = 0)
            "#,
        )
        .bind(protocol_id)
        .bind(&proto.name)
        .bind(&proto.description)
        .bind(&proto.protocol_type)
        .bind(&config)
        .bind(agent_id)
        .bind(schema_id)
        .bind(template_id)
        .fetch_one(&mut **tx)
        .await
        .with_context(|| format!("Failed to upsert protocol '{}'", proto.name))?;

        let created = row.0;
        if created {
            stats.protocols_created += 1;
        } else {
            stats.protocols_updated += 1;
        }

        if verbose {
            let action = if created { "Created" } else { "Updated" };
            println!("  ✓ {}: {} ({})", action, proto.name, proto.protocol_type);
        }
    }

    Ok(())
}

/// Print sync statistics
fn print_stats(stats: &SyncStats) {
    println!("\n📊 Sync Statistics:");
    println!(
        "  Capabilities: {} created, {} updated",
        stats.capabilities_created, stats.capabilities_updated
    );
    println!(
        "  Tool Assignments: {} updated",
        stats.tool_assignments_updated
    );
    println!(
        "  System Agents: {} created, {} updated",
        stats.system_agents_created, stats.system_agents_updated
    );
    println!(
        "  Protocols: {} created, {} updated",
        stats.protocols_created, stats.protocols_updated
    );

    if !stats.errors.is_empty() {
        println!("\n⚠️  {} warnings:", stats.errors.len());
        for err in &stats.errors {
            println!("  - {}", err);
        }
    }
}
