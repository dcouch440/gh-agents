# System Configuration Files

This directory contains declarative configuration for the nexor system. All files are version-controlled and synced to the database via the `sync-config` command.

## File Structure

```
config/
├── README.md                      # This file
├── capabilities.yaml              # Tool capability taxonomy
├── tool_assignments.yaml          # Tool → Capability mappings
├── constraints.yaml               # System execution constraints
├── system_agents.yaml             # Core system agents
├── routing_strategies/            # Cavernous routing configs
│   ├── research_and_docs.yaml
│   └── full_stack_impl.yaml
└── schemas/                       # JSON schemas (future)
    └── *.schema.json
```

## Configuration Files

### `capabilities.yaml`

Defines the semantic capability taxonomy used for tool selection and mode routing.

**Contains:**
- 20+ capability definitions across 8 categories
- Safety levels (safe, caution, unsafe)
- Usage notes and examples
- Category groupings

**Categories:**
- Filesystem (file_read, file_write, file_search, content_search)
- Version Control (git_read, git_write, git_history)
- System (shell_execution, process_management)
- Development (test_execution, code_analysis, code_generation, build_execution)
- Web (web_fetch, web_search, api_call)
- Data (database_query, database_schema)
- Knowledge (document_create, document_search, document_update)

### `tool_assignments.yaml`

Maps tool names to the capabilities they provide.

**Current assignments:**
- `read_file` → file_read, file_metadata
- `write_file`, `edit_file` → file_write
- `list_files` → file_search, file_metadata
- `git_status`, `git_diff`, `git_log` → git_read, git_history
- `git_branch` → git_read, git_write (dual capability)
- `git_add`, `git_commit` → git_write
- `run_command` → shell_execution, process_management
- `run_tests` → test_execution
- `web_research` → web_search, web_fetch

**Priority system:** Higher priority tools are selected first when multiple tools provide the same capability.

### `constraints.yaml`

System-wide execution constraints and safety limits.

**Constraint categories:**
- **Safety:** unsafe_operations_enabled, dangerous_tools_require_approval
- **Resource:** max_execution_time_minutes, max_cost_per_execution_usd
- **Architectural:** max_subtasks_per_cavernous_step, max_cavernous_nesting_depth
- **Usage:** max_executions_per_user_per_hour, max_api_calls_per_execution
- **Room:** max_room_turns, max_speakers_per_turn

**tenant_override:** Some constraints can be overridden by tenants, others are admin-only.

### `system_agents.yaml`

Core system agents available to all users.

**Agents:**
- **General Purpose:** General Assistant
- **Code & Development:** Code Analyzer, Code Generator, Test Engineer
- **Research & Docs:** Researcher, Technical Writer, Synthesizer
- **Specialized:** Database Architect, Security Reviewer, Frontend Specialist, Backend Specialist
- **Collaboration:** Gatekeeper (for room moderation)

### `routing_strategies/*.yaml`

Document-based routing configurations for cavernous execution (TIER 3).

**Current strategies:**
1. **research_and_docs.yaml** - Research topic → synthesize → write documentation
2. **full_stack_impl.yaml** - Database → backend → frontend → tests → validation

Each strategy defines:
- Subtasks with dependencies (DAG structure)
- Agent roles and tools
- Input/output schemas
- Aggregation mode
- Cost/time estimates

## Syncing to Database

### Command

```bash
# Sync all configurations
cargo run -- sync-config

# Sync specific file
cargo run -- sync-config --file config/capabilities.yaml

# Dry run (show what would change)
cargo run -- sync-config --dry-run

# Sync with verbose output
cargo run -- sync-config --verbose
```

### Sync Behavior

- **Idempotent:** Safe to run multiple times
- **UPSERT logic:** Creates new entries, updates existing ones
- **Tool assignments:** Matched by tool name (warns if tool not found)
- **Validation:** Checks types and required fields before applying
- **Transaction:** All changes in a single transaction (all-or-nothing)

### What Gets Synced

| File | Database Table | Sync Method |
|------|---------------|-------------|
| `capabilities.yaml` | `tool_capabilities` | UPSERT by capability_key |
| `tool_assignments.yaml` | `tool_capability_assignments` | DELETE + INSERT per tool |
| `constraints.yaml` | `system_config` (type=constraint) | UPSERT by config_key |
| `system_agents.yaml` | `agents` (is_system=true) | UPSERT by role |
| `routing_strategies/*.yaml` | `documents` (title=routing:*) | UPSERT by title |

## Editing Configurations

### Adding a New Capability

1. Add to `capabilities.yaml`:
   ```yaml
   - key: my_new_capability
     display_name: My New Capability
     category: custom
     safety_level: safe
     description: What it does
   ```

2. Assign to tools in `tool_assignments.yaml`:
   ```yaml
   my_tool:
     capabilities:
       - my_new_capability
   ```

3. Sync: `cargo run -- sync-config`

### Adding a New Tool Assignment

1. Edit `tool_assignments.yaml`:
   ```yaml
   my_new_tool:
     capabilities:
       - file_read
       - content_search
     notes: What this tool does
     priority: 10
   ```

2. Sync: `cargo run -- sync-config`

### Modifying Constraints

1. Edit `constraints.yaml`:
   ```yaml
   max_execution_time_minutes:
     value: 90  # Changed from 60
     type: integer
     # ... rest of fields
   ```

2. Sync: `cargo run -- sync-config`

3. ⚠️ **Warning:** Constraint changes affect all running executions immediately

### Creating a New Routing Strategy

1. Create `config/routing_strategies/my_strategy.yaml`:
   ```yaml
   strategy_name: my_strategy
   description: What this strategy does
   capabilities_required: [...]
   subtasks:
     - id: task1
       # ... subtask definition
   # ... rest of config
   ```

2. Sync: `cargo run -- sync-config`

3. Strategy becomes available as `routing:my_strategy` document

## Validation

### Pre-Sync Validation

The sync command validates:
- ✅ YAML syntax
- ✅ Required fields present
- ✅ Type correctness (integer, float, boolean, string)
- ✅ Enum values (safety_level, config_type, etc.)
- ✅ References (capability_key exists, tool names valid)
- ✅ JSON schema format for routing strategies

### Runtime Validation

The execution engine validates:
- ✅ Constraints are within allowed ranges
- ✅ Required capabilities are available
- ✅ Tool assignments match agent capabilities
- ✅ Routing strategy subtasks form valid DAG

## Best Practices

### Capabilities

- ✅ Use descriptive, specific capability keys
- ✅ Group related capabilities in same category
- ✅ Document examples and use cases
- ✅ Set appropriate safety_level
- ❌ Don't create overly broad capabilities (e.g., "do_everything")
- ❌ Don't duplicate existing capabilities

### Tool Assignments

- ✅ Assign all relevant capabilities to tools
- ✅ Use priority to prefer safer/specialized tools
- ✅ Document why a tool has a specific capability
- ✅ Mark dangerous tools with requires_approval: true
- ❌ Don't assign capabilities a tool doesn't actually provide
- ❌ Don't set priority=0 unless tool is truly last resort

### Constraints

- ✅ Set conservative defaults (can be increased if needed)
- ✅ Document rationale for each constraint
- ✅ Use tenant_override: false for safety-critical constraints
- ✅ Provide recommended_range for adjustable constraints
- ❌ Don't set unrealistic limits (too low or too high)
- ❌ Don't make safety constraints overridable by tenants

### Routing Strategies

- ✅ Keep subtasks focused and single-purpose
- ✅ Use clear, semantic subtask IDs
- ✅ Define comprehensive output schemas
- ✅ Provide example inputs and outputs
- ✅ Estimate time and cost accurately
- ❌ Don't create circular dependencies
- ❌ Don't make subtasks too granular (limit: 10 subtasks)
- ❌ Don't exceed nesting depth limits

## Migration from Seed Migrations

**Current state:** Migrations 068 and 071 contain hardcoded seed data.

**Future state:** Remove seed data from migrations, use config files exclusively.

**Migration path:**
1. ✅ Config files created with current seed data
2. ⏸️ Test sync command thoroughly
3. ⏸️ Update migrations to only create tables (remove INSERT statements)
4. ⏸️ Update setup docs to require `cargo run -- sync-config` after migrations

## Troubleshooting

### "Tool not found" warning during sync

**Problem:** Tool assignment references a tool that doesn't exist in database.

**Solution:**
- Check tool name matches database exactly (case-sensitive)
- Create the tool first, then sync assignments
- Remove assignment if tool is no longer needed

### "Constraint validation failed"

**Problem:** Constraint value doesn't match expected type or range.

**Solution:**
- Check type field matches value (integer, float, boolean)
- Ensure value is within recommended_range if specified
- Review error message for specific validation failure

### "Circular dependency in routing strategy"

**Problem:** Subtasks form a circular dependency graph.

**Solution:**
- Review depends_on fields in subtasks
- Ensure dependencies form a DAG (no cycles)
- Use `--dry-run` to see dependency graph

### "Capability not found"

**Problem:** Tool assignment references capability that doesn't exist.

**Solution:**
- Check capability_key in capabilities.yaml
- Sync capabilities before tool assignments
- Fix typo in capability reference

## Future Enhancements

- [ ] JSON schema files in `schemas/` directory
- [ ] Validation command: `cargo run -- validate-config`
- [ ] Config diff command: `cargo run -- config-diff`
- [ ] Export command: `cargo run -- export-config` (database → YAML)
- [ ] Config versioning and rollback
- [ ] Web UI for config management
- [ ] Config change approval workflow
- [ ] Automated config testing in CI

## See Also

- [Phase 1 Plan](/Users/davidcouch/.claude/plans/floofy-honking-thompson.md) - Database schema and architecture
- [CLAUDE.md](/CLAUDE.md) - Development guide and conventions
- Migrations 067-071 - Database schema for these configurations
