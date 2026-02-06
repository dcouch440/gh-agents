# Config Sync Command - Implementation Plan

**Goal:** Build production-ready `cargo run -- sync-config` command to sync YAML configs to database

## Overview

Sync configuration files from `/config/*.yaml` to the database using idempotent UPSERT operations with full transaction safety.

## Dependencies

Add to `Cargo.toml`:
```toml
[dependencies]
serde_yaml = "0.9"
```

## Module Structure (Following CLAUDE.md conventions)

```
src/config/
├── mod.rs              # Add: pub mod sync; pub use sync::*;
└── sync/
    ├── mod.rs          # Main sync implementation
    ├── tests.rs        # Tests (separate file per CLAUDE.md)
    ├── types.rs        # YAML deserialization types
    └── validators.rs   # Validation logic
```

## Key Types (src/config/sync/types.rs)

```rust
// Capability deserialization
#[derive(Debug, Clone, Deserialize)]
pub struct CapabilitiesYaml {
    pub capabilities: Vec<Capability>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Capability {
    pub key: String,          // Must match ^[a-z][a-z0-9_]*$
    pub display_name: String,
    pub category: String,
    pub safety_level: String, // safe|caution|unsafe
    pub description: String,
}

// Tool assignment deserialization
#[derive(Debug, Clone, Deserialize)]
pub struct ToolAssignmentsYaml {
    pub tool_assignments: HashMap<String, ToolAssignment>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolAssignment {
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub priority: i32,
}

// Additional types:
// - ConstraintsYaml
// - SystemAgentsYaml
// - RoutingStrategyYaml
```

## Sync Logic (src/config/sync/mod.rs)

**Main function:**
```rust
pub async fn sync_config(
    pool: &PgPool,
    config_dir: &Path,
    dry_run: bool,
    verbose: bool,
) -> Result<SyncStats> {
    // 1. Load all YAML files
    // 2. Validate (fail fast if invalid)
    // 3. Begin transaction
    // 4. Sync each table with UPSERT
    // 5. Commit transaction
    // 6. Return stats
}
```

**UPSERT strategies:**
- `tool_capabilities`: UPSERT on `capability_key`
- `tool_capability_assignments`: DELETE + INSERT per tool
- `system_config`: UPSERT on `config_key`
- `agents`: UPSERT on deterministic UUID from role (UUID v5)
- `documents`: UPSERT on `ref_tag` for routing strategies

**Transaction pattern:**
```rust
let mut tx = pool.begin().await?;
sync_capabilities(&mut tx, &capabilities, &mut stats).await?;
sync_tool_assignments(&mut tx, &tool_assignments, &mut stats).await?;
// ... more syncs
tx.commit().await?; // All or nothing
```

## CLI Integration (src/cli.rs)

```rust
#[derive(Subcommand, Debug)]
pub enum Commands {
    SyncConfig {
        #[arg(short='d', long, default_value="./config")]
        config_dir: PathBuf,

        #[arg(long)]
        dry_run: bool,

        #[arg(short, long)]
        verbose: bool,
    },
}
```

## Main Handler (src/main.rs)

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse_args();

    if let Some(Commands::SyncConfig { config_dir, dry_run, verbose }) = args.command {
        return run_sync_config(config_dir, dry_run, verbose).await;
    }

    run_server_mode(args).await
}

async fn run_sync_config(...) -> Result<()> {
    let pool = init_db().await?;
    let stats = sync_config(&pool, &config_dir, dry_run, verbose).await?;
    println!("Capabilities: {} created, {} updated", ...);
    Ok(())
}
```

## Validation Logic (src/config/sync/validators.rs)

**Pre-sync validation (fail fast):**
1. Capability keys match regex `^[a-z][a-z0-9_]*$`
2. Safety levels are valid enums (safe|caution|unsafe)
3. Tool assignments reference existing capabilities
4. System agent roles are unique
5. Routing strategy DAGs have no cycles (Kahn's algorithm)
6. Cross-file references valid (tools exist, capabilities exist)

```rust
pub fn validate_all(...) -> ValidationResult {
    let mut errors = Vec::new();

    // Validate each config type
    // Validate cross-file references
    // Validate DAG cycles

    ValidationResult { valid: errors.is_empty(), errors }
}
```

## Error Handling

**Use thiserror for library errors:**
```rust
#[derive(Error, Debug)]
pub enum SyncError {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("YAML parse error in {file}: {message}")]
    YamlParse { file: String, message: String },

    #[error("tool '{0}' not found in database")]
    ToolNotFound(String),
}
```

**Convert to anyhow in application code:**
```rust
.context("Failed to sync capabilities")?
```

## Testing (src/config/sync/tests.rs)

**Test coverage:**
- Capability key validation (valid/invalid patterns)
- YAML deserialization (each type)
- Idempotency (run sync twice, check stats)
- Transaction rollback (error → no changes committed)
- DAG validation (valid DAG, cycle detection)
- Cross-file validation (missing tools, capabilities)

```rust
#[tokio::test]
async fn sync_capabilities_idempotent() {
    let pool = setup_test_db().await;
    // First sync: creates
    // Second sync: updates
    assert_eq!(stats1.created, 1);
    assert_eq!(stats2.updated, 1);
}
```

## Implementation Order

1. ✅ Add serde_yaml to Cargo.toml
2. Create src/config/sync/types.rs (basic structs)
3. Create src/config/sync/validators.rs (validation logic)
4. Write validation tests (TDD)
5. Implement sync functions one table at a time:
   - sync_capabilities
   - sync_tool_assignments
   - sync_constraints
   - sync_system_agents
   - sync_routing_strategies
6. Write integration tests for each sync function
7. Update CLI (src/cli.rs)
8. Update main (src/main.rs)
9. Manual end-to-end testing
10. Update README with usage

## Verification Steps

```bash
# 1. Dry run (validation only)
cargo run -- sync-config --dry-run --verbose

# 2. Actual sync
cargo run -- sync-config

# 3. Verify in database
docker exec gh-agents-postgres-1 psql -U nexor -d nexor -c \
  "SELECT capability_key FROM tool_capabilities LIMIT 5;"

# 4. Test idempotency (run twice)
cargo run -- sync-config --verbose
# Check stats: second run should show updates not creates

# 5. Test invalid config (should fail gracefully)
# Edit config/capabilities.yaml: add "File-Read" (invalid)
cargo run -- sync-config  # Should error with validation message

# 6. Run unit tests
cargo test config::sync::

# 7. Verify web_research has new capabilities
docker exec gh-agents-postgres-1 psql -U nexor -d nexor -c \
  "SELECT t.name, tc.capability_key FROM tools t
   JOIN tool_capability_assignments tca ON t.id = tca.tool_id
   JOIN tool_capabilities tc ON tc.id = tca.capability_id
   WHERE t.name = 'web_research';"
# Should show: web_search, web_fetch, x_search, real_time_search
```

## Production-Ready Features

1. **Idempotent** - Safe to run multiple times (UPSERT patterns)
2. **Transactional** - All changes or none (single transaction)
3. **Validated** - Fail fast before any DB changes
4. **Dry run** - Preview changes without applying
5. **Verbose mode** - Detailed logging for debugging
6. **Statistics** - Clear summary of what changed
7. **Deterministic** - System agents use UUID v5 for consistency
8. **Error messages** - Clear, actionable with file/field context

## Critical Files

- `/Users/davidcouch/Dev/gh-agents/src/config/sync/types.rs` - YAML types
- `/Users/davidcouch/Dev/gh-agents/src/config/sync/mod.rs` - Sync implementation
- `/Users/davidcouch/Dev/gh-agents/src/config/sync/validators.rs` - Validation
- `/Users/davidcouch/Dev/gh-agents/src/cli.rs` - CLI subcommand
- `/Users/davidcouch/Dev/gh-agents/src/main.rs` - Command handler

## Success Criteria

- ✅ `cargo check` passes
- ✅ All tests pass (`cargo test config::sync::`)
- ✅ Dry run validates all config files
- ✅ Actual sync applies all changes
- ✅ Idempotent (run twice, second shows updates not creates)
- ✅ Transaction rollback on error
- ✅ web_research gets x_search + real_time_search capabilities
- ✅ Orchestrator tools (create_doc, search_docs, update_doc) added to DB
