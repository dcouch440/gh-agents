# Deletion Plan: Legacy Mode & Router Code

Once the router modes system is fully implemented and all data has been migrated, the following tables, columns, code, and endpoints can be removed.

---

## Phase 1: Database Migration (new migration file)

### Tables to DROP

| Table | Reason |
|-------|--------|
| `agent_modes` | Replaced by `tool_router_modes` |
| `agent_modes_versions` | Version history for dropped table |
| `tool_router_tools` | Replaced by `tool_router_mode_tools` (tools now belong to modes, not routers directly) |

### Columns to DROP

| Table | Column | Reason |
|-------|--------|--------|
| `agents` | `router_mode` | Old boolean flag replaced by `agents.router_id` FK |
| `agents_versions` | `router_mode` | Corresponding version history column |

### Migration SQL

```sql
-- 1. Drop legacy mode tables
DROP TABLE IF EXISTS agent_modes_versions CASCADE;
DROP TABLE IF EXISTS agent_modes CASCADE;

-- 2. Drop legacy router-to-tool junction (tools now on modes)
DROP TABLE IF EXISTS tool_router_tools CASCADE;

-- 3. Remove router_mode column from agents
ALTER TABLE agents DROP COLUMN IF EXISTS router_mode;
ALTER TABLE agents_versions DROP COLUMN IF EXISTS router_mode;
```

---

## Phase 2: Database Layer (`src/db/`)

### `src/db/mod.rs`
- Remove `router_mode: Option<bool>` from `AgentRow` struct (line 29)

### `src/db/queries/mod.rs`
- Remove `AgentModeRow` struct (lines 477-487)
- Remove `list_agent_modes()` function (line 491+)
- Remove `create_agent_mode()` function (line 503+)
- Remove `delete_agent_mode()` function (line 520+)

### `src/db/traits/mod.rs`
- Remove `get_agent_modes()` trait method (line 255)
- Remove `create_agent_mode()` trait method (line 258)
- Remove `delete_agent_mode()` trait method (line 261)
- Remove `get_router_tools()` trait method (line 606)
- Remove `set_router_tools()` trait method (line 608)

### `src/db/pg_repo/mod.rs`
- Remove `get_agent_modes()` implementation (lines 624-625)
- Remove `create_agent_mode()` implementation (lines 628-629)
- Remove `delete_agent_mode()` implementation (lines 632-633)
- Remove `router_mode` from all agent SQL queries (lines 355, 366, 378, 389, 404)
- Remove `router_mode` from the intermediate struct used for agent mapping (line 652, 669)
- Remove `get_router_tools()` implementation (lines 1630-1638)
- Remove `set_router_tools()` implementation (lines 1640-1658)

---

## Phase 3: API Layer (`src/server/api/`)

### `src/server/api/sessions/mod.rs`
- Remove `ModeInfo` struct (lines 20-24)
- Remove `list_modes()` endpoint handler (lines 26-53)
- Remove `AgentModeResponse` struct (lines 62-90)
- Remove `CreateAgentModeRequest` struct (lines 94-105)
- Remove `list_agent_modes()` endpoint handler (lines 118-131)
- Remove `create_agent_mode()` endpoint handler (lines 145-171)
- Remove `delete_agent_mode()` endpoint handler (lines 184-195)

### `src/server/api/mod.rs`
- Remove re-exports: `create_agent_mode`, `delete_agent_mode`, `list_agent_modes`, `list_modes`, `AgentModeResponse`, `CreateAgentModeRequest`, `ModeInfo` (lines 71-73)

### `src/server/api/tool_routers/mod.rs`
- Remove `get_router_tools()` endpoint handler (line 232+)
- Remove `set_router_tools()` endpoint handler (line 270+)

### `src/server/api/agents/mod.rs`
- Remove `router_mode: Some(false)` from agent creation (line 176)
- Remove `router_mode: existing.router_mode` from agent update (line 261)

---

## Phase 4: Routes & Constants

### `src/server/mod.rs`
- Remove route: `routes::AGENT_MODES` -> `get(api::list_agent_modes).post(api::create_agent_mode)` (line 170)
- Remove route: `routes::AGENT_MODE` -> `delete(api::delete_agent_mode)` (line 172)
- Remove route: `routes::MODES` -> `get(api::list_modes)` (line 201)
- Remove route: router tools `get(api::get_router_tools).put(api::set_router_tools)` (line 327)

### `src/constants.rs`
- Remove `AGENT_MODES` constant (line 199)
- Remove `AGENT_MODE` constant (line 200)
- Remove `MODES` constant (line 231)

### `src/server/openapi.rs`
- Remove `super::api::sessions::list_modes` (line 80)
- Remove `super::api::sessions::ModeInfo` (line 175)
- Remove `super::api::tool_routers::get_router_tools` (line 145)
- Remove `super::api::tool_routers::set_router_tools` (line 146)

---

## Phase 5: Execution Layer (`src/server/hub/`)

### `src/server/hub/mod.rs`
- Remove mode loading and classification block (lines 71-82):
  ```rust
  let modes = state.repo.get_agent_modes(agent_id).await?;
  let active_mode = if modes.is_empty() { ... } else { classify_mode(...) };
  ```
- Remove mode overlay application (lines 94-97):
  ```rust
  if let Some(mode) = &active_mode { apply_mode_overlay(...); }
  ```
- Remove `classify_mode()` function (lines 117-175+)
- Remove `apply_mode_overlay()` function (lines 185-199)

### `src/server/hub/tests.rs`
- Remove all `apply_mode_overlay_*` tests (lines 24-92)
- Remove `make_mode()` helper function

---

## Phase 6: Dead Code

### `src/agents/tool_router.rs` (entire file)
- `request_assistance_tool()` is never called outside its own file/tests
- Comments already say "LEGACY CODE REMOVED" and "Tool routing now handled by RouterStrategy"
- Also remove `pub mod tool_router;` from `src/agents/mod.rs` (line 7)

### `src/server/router_service/mod.rs`
- Remove usage of `get_router_tools()` (line 105) - replace with mode-level tool loading

---

## Phase 7: Test Mock Implementations

### `src/server/mod.rs` (mock repo)
- Remove `get_agent_modes()` mock (lines 783-788)
- Remove `create_agent_mode()` mock (lines 789-791)
- Remove `delete_agent_mode()` mock (lines 792-794)

### `src/server/chat_consumer/tests.rs` (mock repo)
- Remove `get_agent_modes()` mock (lines 192-197)
- Remove `create_agent_mode()` mock (lines 198-200)
- Remove `delete_agent_mode()` mock (lines 201-203)

### `src/server/room_executor/tests.rs`
- Remove `router_mode: None` from test fixture (line 20)

---

## Verification

After all deletions:

```bash
# Must pass with zero errors
~/.cargo/bin/cargo check
~/.cargo/bin/cargo test
~/.cargo/bin/cargo clippy

# Verify dropped tables
docker exec gh-agents-postgres-1 psql -U nexor -d nexor -c "\dt agent_modes*"
docker exec gh-agents-postgres-1 psql -U nexor -d nexor -c "\dt tool_router_tools"
docker exec gh-agents-postgres-1 psql -U nexor -d nexor -c "SELECT column_name FROM information_schema.columns WHERE table_name = 'agents' AND column_name = 'router_mode';"
```

All three queries should return empty results.

---

## Summary

| Category | Count |
|----------|-------|
| Tables dropped | 3 (`agent_modes`, `agent_modes_versions`, `tool_router_tools`) |
| Columns dropped | 2 (`agents.router_mode`, `agents_versions.router_mode`) |
| Files deleted | 1 (`src/agents/tool_router.rs`) |
| Endpoints removed | 6 |
| Route constants removed | 3 |
| Structs removed | 4 (`AgentModeRow`, `AgentModeResponse`, `CreateAgentModeRequest`, `ModeInfo`) |
| Functions removed | 5 (`classify_mode`, `apply_mode_overlay`, `list_agent_modes`, `create_agent_mode`, `delete_agent_mode`) |
| Trait methods removed | 5 |
| Test functions removed | 5+ overlay tests, mock impls across 3 files |
