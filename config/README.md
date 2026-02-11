# System Configuration Files

This directory contains declarative configuration for the nexor system.

## File Structure

```
config/
├── README.md                      # This file
├── capabilities.yaml              # Tool capability taxonomy
├── tool_assignments.yaml          # Tool → Capability mappings
├── constraints.yaml               # System execution constraints
├── protocols/                     # Protocol agent prompts & model configs
│   ├── documenter/
│   │   ├── config.yaml            # Model, temperature, max_tokens per role
│   │   ├── strategist.md          # Plans research + writing per document
│   │   ├── researcher.md          # Gathers information with tools
│   │   ├── writer.md              # Produces final document content
│   │   └── assistant.md           # Configures the step via chat
│   └── meeting/
│       ├── config.yaml            # Model config for gatekeeper
│       └── gatekeeper.md          # Speaker selection and moderation
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

**Priority system:** Higher priority tools are selected first when multiple tools provide the same capability.

### `constraints.yaml`

System-wide execution constraints and safety limits.

**Constraint categories:**
- **Safety:** unsafe_operations_enabled, dangerous_tools_require_approval
- **Resource:** max_execution_time_minutes, max_cost_per_execution_usd
- **Architectural:** max_subtasks_per_cavernous_step, max_cavernous_nesting_depth
- **Usage:** max_executions_per_user_per_hour, max_api_calls_per_execution
- **Room:** max_room_turns, max_speakers_per_turn

### `protocols/`

Protocol agent prompts and model configurations. Each protocol gets a directory containing:
- `config.yaml` — model ID, temperature, max_tokens, max_rounds, context_budget per agent role
- `<role>.md` — system prompt for each agent role

Loaded at compile time via `include_str!()` (prompts) and `once_cell::sync::Lazy` (YAML configs). No database reads, no runtime file I/O.

### `routing_strategies/*.yaml`

Document-based routing configurations for cavernous execution (TIER 3).

## Syncing to Database

### Command

```bash
# Sync all configurations
cargo run -- sync-config

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

**Not synced (compile-time only):** `protocols/` — loaded via `include_str!()`, no DB round-trip.

## Validation

### Pre-Sync Validation

The sync command validates:
- YAML syntax
- Required fields present
- Type correctness (integer, float, boolean, string)
- Enum values (safety_level, config_type, etc.)
- References (capability_key exists, tool names valid)

### Runtime Validation

The execution engine validates:
- Constraints are within allowed ranges
- Required capabilities are available
- Tool assignments match agent capabilities
- Routing strategy subtasks form valid DAG

## See Also

- [CLAUDE.md](/CLAUDE.md) - Development guide and conventions
- Migrations 067-071 - Database schema for these configurations
