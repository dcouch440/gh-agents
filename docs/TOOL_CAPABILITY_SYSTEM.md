# Tool Capability System - Database Records

**Generated:** 2025-02-05
**Status:** Phase 1 Complete - Migrations 067-071 applied, config sync operational

## Overview

The tool capability system enables semantic tool selection based on required capabilities rather than explicit tool lists. This document shows the current state of seeded data.

### Record Counts

| Table                        | Count | Description                                    |
|------------------------------|-------|------------------------------------------------|
| `tool_capabilities`          | 24    | Semantic capability taxonomy                   |
| `tools`                      | 15    | Built-in tools seeded from execution_tools()   |
| `tool_capability_assignments`| 22    | Tool → capability mappings                     |

---

## Tool Capabilities

Semantic capability taxonomy defining what tools can do. Used for mode-based tool resolution.

### Sample Records (5 of 24)

```sql
SELECT capability_key, display_name, category, safety_level, description
FROM tool_capabilities
LIMIT 5;
```

| capability_key | display_name   | category   | safety_level | description                                      |
|----------------|----------------|------------|--------------|--------------------------------------------------|
| file_read      | File Reading   | filesystem | safe         | Read file contents from disk, including text files, images, PDFs |
| file_write     | File Writing   | filesystem | caution      | Create new files or modify existing files on disk |
| file_search    | File Search    | filesystem | safe         | Search for files by name, pattern, or glob       |
| content_search | Content Search | filesystem | safe         | Search file contents using grep-like functionality |
| file_metadata  | File Metadata  | filesystem | safe         | Read file metadata (size, permissions, timestamps, ownership) |

### Categories

Capabilities are organized into logical categories:

- **filesystem**: file_read, file_write, file_search, content_search, file_metadata
- **version_control**: git_read, git_write, git_history
- **system**: shell_execution
- **web**: web_fetch, web_search, x_search, real_time_search
- **development**: code_analysis, test_execution
- **data**: database_query
- **integration**: api_call
- **knowledge**: document_create, document_search, document_update

### Safety Levels

- **safe**: Read-only operations, no system modification
- **caution**: Write operations, requires user awareness
- **unsafe**: Dangerous operations (shell execution), admin control required

---

## Tool Capability Assignments

Maps tools to the capabilities they provide. A tool can provide multiple capabilities.

### Sample Records (10 of 22)

```sql
SELECT t.name, tc.capability_key, tc.display_name as capability_display_name
FROM tool_capability_assignments tca
JOIN tools t ON t.id = tca.tool_id
JOIN tool_capabilities tc ON tc.id = tca.capability_id
ORDER BY t.name, tc.capability_key
LIMIT 10;
```

| tool_name   | capability_key  | capability_display_name |
|-------------|-----------------|-------------------------|
| create_doc  | document_create | Document Creation       |
| edit_file   | file_write      | File Writing            |
| git_add     | git_write       | Git Write Operations    |
| git_branch  | git_read        | Git Read Operations     |
| git_branch  | git_write       | Git Write Operations    |
| git_commit  | git_write       | Git Write Operations    |
| git_diff    | git_read        | Git Read Operations     |
| git_status  | git_read        | Git Read Operations     |
| list_files  | file_metadata   | File Metadata           |
| list_files  | file_search     | File Search             |

### Multi-Capability Tools

Some tools provide multiple capabilities:

- **git_branch**: Both `git_read` (viewing branches) and `git_write` (creating branches)
- **list_files**: Both `file_search` (finding files) and `file_metadata` (file info)
- **web_research**: `web_search`, `web_fetch`, `x_search`, `real_time_search` (Grok-powered)

---

## Tools

Built-in tools seeded from `execution_tools()` array in `/src/agents/execution_tools.rs`.

### Sample Records (5 of 15)

```sql
SELECT name, display_name, description,
       LEFT(parameters::text, 80) as parameters_preview
FROM tools
WHERE name IN ('create_doc', 'search_docs', 'read_file', 'web_research', 'run_command')
ORDER BY name;
```

| name         | display_name | description                                                                 | parameters (truncated)                           |
|--------------|--------------|-----------------------------------------------------------------------------|--------------------------------------------------|
| create_doc   | Create Doc   | Create a new document with auto-generated summary.                          | `{type: object, required: [title, content]...}` |
| read_file    | Read File    | Read the contents of a file.                                                | `{type: object, required: [path]...}`           |
| run_command  | Run Command  | Execute a shell command in a sandboxed environment.                         | `{type: object, required: [command]...}`        |
| search_docs  | Search Docs  | Full-text search across all documents.                                      | `{type: object, required: [query]...}`          |
| web_research | Web Research | Research a topic using real-time web and X/Twitter search via xAI Grok. Returns a synthesized answer with citations. Requires XAI_API_KEY environment variable. | `{type: object, required: [query]...}` |

### Complete Tool List (15 tools)

1. **create_doc** - Document creation with auto-summary
2. **edit_file** - Modify existing files
3. **git_add** - Stage files for commit
4. **git_branch** - Branch operations (list, create, delete)
5. **git_commit** - Create commits
6. **git_diff** - View file changes
7. **git_status** - Check repository status
8. **list_files** - List directory contents with metadata
9. **read_file** - Read file contents
10. **run_command** - Execute shell commands (sandboxed)
11. **run_tests** - Execute test suites
12. **search_docs** - Search document corpus
13. **update_doc** - Update existing documents
14. **web_research** - Grok-powered web + X/Twitter research
15. **write_file** - Create new files

---

## Configuration System

### YAML-Based Configuration

Tool capabilities and assignments are defined in YAML files under `/config/`:

- **`capabilities.yaml`**: Defines all 24 capabilities
- **`tool_assignments.yaml`**: Maps tools to capabilities

### Sync Command

```bash
cargo run -- sync-config
```

**What it does:**
1. Seeds built-in tools from `execution_tools()` (idempotent)
2. Syncs capabilities from `capabilities.yaml` (UPSERT)
3. Syncs tool assignments from `tool_assignments.yaml` (DELETE + INSERT)

**Output example:**
```
🔄 nexor Config Sync
   Directory: ./config

🔧 Seeding built-in tools...
✓ Tools seeded

✅ Sync completed successfully!

📊 Summary:
   Capabilities: 0 created, 24 updated
   Tool Assignments: 22 updated
```

---

## Schema Details

### `tool_capabilities` Table

```sql
CREATE TABLE tool_capabilities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    capability_key TEXT NOT NULL UNIQUE,  -- Snake_case identifier
    display_name TEXT NOT NULL,
    category TEXT NOT NULL,
    safety_level TEXT NOT NULL DEFAULT 'safe',  -- "safe", "caution", "unsafe"
    description TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (capability_key ~ '^[a-z][a-z0-9_]*$')
);
```

### `tool_capability_assignments` Table

```sql
CREATE TABLE tool_capability_assignments (
    tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
    capability_id UUID NOT NULL REFERENCES tool_capabilities(id) ON DELETE CASCADE,
    PRIMARY KEY (tool_id, capability_id)
);
```

### `tools` Table (Relevant Columns)

```sql
CREATE TABLE tools (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    display_name TEXT NOT NULL,
    description TEXT NOT NULL,
    parameters JSONB NOT NULL DEFAULT '{}'::jsonb,  -- JSON schema for tool inputs
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    version INTEGER NOT NULL DEFAULT 1,
    UNIQUE(user_id, name)
);
```

---

## Usage Examples

### Query Tools by Capability

```sql
-- Find all tools that can read files
SELECT t.name, t.description
FROM tools t
JOIN tool_capability_assignments tca ON t.id = tca.tool_id
JOIN tool_capabilities tc ON tc.id = tca.capability_id
WHERE tc.capability_key = 'file_read';
```

### Query Capabilities by Tool

```sql
-- Find all capabilities for web_research tool
SELECT tc.capability_key, tc.display_name, tc.safety_level
FROM tool_capabilities tc
JOIN tool_capability_assignments tca ON tc.id = tca.capability_id
JOIN tools t ON t.id = tca.tool_id
WHERE t.name = 'web_research';
```

### Mode Resolution (Capability-Based)

When a mode requires certain capabilities, the system automatically selects tools:

```rust
// Mode requires: ["file_read", "file_write", "git_write"]
// System resolves to tools: [read_file, edit_file, write_file, git_add, git_commit, git_branch]
let mode_config = mode_resolver.resolve_by_capabilities(&required_caps).await?;
```

---

## Related Documentation

- **[SYNC_CONFIG_PLAN.md](./SYNC_CONFIG_PLAN.md)** - Implementation plan for config sync command
- **[UNIFIED_WORKFLOW_SYSTEM.md](./UNIFIED_WORKFLOW_SYSTEM.md)** - Overall system architecture
- **Migration 068**: `/migrations/068_tool_capabilities.sql` - Schema definition

---

## Verification Queries

```bash
# Check record counts
docker exec gh-agents-postgres-1 psql -U nexor -d nexor -c "
SELECT
    (SELECT COUNT(*) FROM tool_capabilities) as total_capabilities,
    (SELECT COUNT(*) FROM tool_capability_assignments) as total_assignments,
    (SELECT COUNT(*) FROM tools) as total_tools;"

# View all capabilities by category
docker exec gh-agents-postgres-1 psql -U nexor -d nexor -c "
SELECT category, COUNT(*) as count
FROM tool_capabilities
GROUP BY category
ORDER BY category;"

# View tools with multiple capabilities
docker exec gh-agents-postgres-1 psql -U nexor -d nexor -c "
SELECT t.name, COUNT(*) as capability_count
FROM tools t
JOIN tool_capability_assignments tca ON t.id = tca.tool_id
GROUP BY t.name
HAVING COUNT(*) > 1
ORDER BY capability_count DESC;"
```

---

**Last Updated:** 2025-02-05
**Next Phase:** Mode resolver enhancement (capability-based tool selection)
