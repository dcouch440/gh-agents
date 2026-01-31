# Tool Routing System — Design Notes

## Current State

### Two Separate Tool Systems
1. **Execution tools** (hardcoded): 11 built-in tools in `execution_tools.rs` — read_file, write_file, edit_file, list_files, git_status, git_diff, git_add, git_commit, git_branch, run_tests, run_command
2. **DB tools** (persisted): user-defined tools in `tools` table with name, description, parameter_schema, output_schema, category, enabled. Can be assigned to agents via `agent_tools` join table.

### The Gap
DB tools exist as metadata only — they're never sent to the LLM and never dispatched. The executor always loads the hardcoded 11.

### What Exists
- `ToolRow`: id, name, description, category, parameter_schema, output_schema, enabled
- `agent_tools` join table (agent_id → tool_id)
- Tools CRUD API endpoints
- Agent-tool assignment endpoints
- Cluster system with members, conventions, shared files
- Pipeline stages can reference clusters
- Template rendering with stage output forwarding
- `allowed_tools` constraint on TaskAssignment (string-based name allowlist)

## Design Goal

Replace the hardcoded execution tools with a DB-driven system where:
- Default user gets seeded with the 11 built-in tools
- Tools can map to clusters for fulfillment
- A router agent uses `request_assistance` to delegate instead of having all tools in context
- Users can add custom tools on top of the defaults

## Architecture

### Tool → Cluster Mapping
New column on `tools` table: `cluster_id UUID REFERENCES clusters(id)`.
- If cluster_id is NULL: tool is a direct execution tool (handled by `execute_execution_tool`)
- If cluster_id is set: tool call gets routed to that cluster for fulfillment

### Seed Tools
On user creation, seed the 11 built-in tools into their `tools` table. These have `cluster_id = NULL` (direct execution). This replaces the hardcoded `execution_tools()` call.

### Dynamic Tool Loading
`executor.rs` tool loop changes:
- Instead of `execution_tools::execution_tools()`, load tools from DB for the agent
- Build `Vec<Tool>` from `ToolRow` records (parameter_schema maps to input_schema)
- For dispatch: if tool has cluster_id, route to cluster; otherwise use existing `execute_execution_tool`

### Router Agent Pattern
For agents configured with the router pattern:
- Instead of getting N tools, they get 1 meta-tool: `request_assistance`
- The router receives the tool call, inspects the request, matches to a tool by name/description
- Dispatches to the appropriate cluster
- Returns the result back to the calling agent

### request_assistance Tool
```json
{
  "name": "request_assistance",
  "description": "Request help from a specialized tool. Describe what you need.",
  "input_schema": {
    "type": "object",
    "properties": {
      "tool_name": { "type": "string", "description": "Name of the tool to invoke" },
      "request": { "type": "string", "description": "What you need done" },
      "parameters": { "type": "object", "description": "Parameters for the tool" }
    },
    "required": ["tool_name", "request"]
  }
}
```

### Execution Flow
1. Primary agent calls `request_assistance(tool_name="search_database", request="...", parameters={...})`
2. Router receives the call
3. Router looks up tool by name → finds cluster_id
4. Router dispatches task to cluster
5. Cluster agent(s) execute and return result
6. Result flows back to primary agent as tool_result

## Key Files
- `src/agents/execution_tools.rs` — current hardcoded tools
- `src/agents/executor.rs` — tool loop (lines 478-616)
- `src/agents/channels.rs` — TaskConstraints.allowed_tools
- `src/db/mod.rs` — ToolRow definition
- `src/db/pg_repo.rs` — tool DB operations
- `src/db/traits.rs` — ServerRepo trait
- `src/server/api.rs` — tool CRUD endpoints
- `src/server/orchestrator.rs` — dispatch flow
- `migrations/024_create_tools.sql` — tools schema
