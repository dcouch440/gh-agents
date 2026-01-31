# Agent Dashboard Feature - Planning Notes

## Date: 2026-01-31

## Problem Statement

The pre-defined agents and chat views aren't working well. Too many hardcoded tools and calls. The user wants a **self-service dashboard** to design, configure, and manage agent teams from the UI — not from code.

## Current State Summary

### What Exists (Backend)

- **3-tier agent hierarchy**: Orchestrator → Worker → Utility (hardcoded in `AgentTier` enum)
- **Agents persisted in DB** (`migrations/003_create_agents.sql`): tier, persona, model config, status
- **11 execution tools hardcoded** in `src/agents/execution_tools.rs`: file ops, git ops, run tests, run command
- **Role system** (`src/agents/roles.rs`): RoleLibrary with predefined roles, categories, delegation rules, required reading
- **Dispatcher + Pool + Scheduler** manage agent lifecycle and task routing
- **Task dependencies** tracked in DB, dependency-aware queue
- **Channel-based async runtime**: each agent is a tokio task with command/response channels

### What Exists (Frontend)

- **AgentsPage** (`ui/src/pages/AgentsPage/`): read-only overview with tier gauges and status dots
- **TasksPage** (`ui/src/pages/TasksPage/`): Kanban board with priority and status filtering
- **ChatPage** (`ui/src/pages/ChatPage/`): mode-based chat with SSE streaming
- **SettingsPage** (`ui/src/pages/SettingsPage/`): model config per tier, pool limits, autonomy level
- **Zustand stores** for agents, tasks, config, sessions

### What's Hardcoded (Needs to Become Dynamic)

1. **Tool definitions** — 11 tools baked into Rust code, no DB registry
2. **Agent tiers** — Only 3 tiers, no custom hierarchies
3. **Role library** — Roles defined in code, not user-configurable
4. **Delegation rules** — `can_delegate_to` set in role definitions, not editable
5. **Routing rules** — Router has hardcoded priority rules in `src/orchestration/router.rs`
6. **Tool allowlists** — Set per-task via `TaskConstraints.allowed_tools`, but no UI for it

## Desired Features

### 1. Code Editor (with bells and whistles)
- Full code editor in the UI for viewing/editing project files
- Syntax highlighting, file tree, multi-tab
- Likely Monaco Editor (VS Code engine)

### 2. Agent Creation & Persistence via UI
- Create agents from the dashboard, not just from system startup
- Configure: tier, persona (name, system prompt, style), model, tools
- Agents saved to DB and available across sessions
- CRUD operations for agents

### 3. Custom Hierarchies / Team Designer
- Move beyond fixed Orchestrator → Worker → Utility
- Let users define their own team structures
- Custom delegation chains (who can assign to whom)
- Visual team builder (drag-and-drop or graph editor)

### 4. Tool Selection Dashboard
- Registry of available tools (persisted in DB)
- Per-agent tool assignment
- Possibly custom tool creation (name, description, schema, handler)
- Tool categories and search

### 5. Task Management Dashboard
- Start tasks from the UI and assign to specific agents/teams
- Monitor progress in real-time
- View agent activity, tool calls, costs
- All from one unified dashboard

## Architecture Gaps to Address

| Gap | Current | Needed |
|-----|---------|--------|
| Tool registry | Hardcoded in Rust | DB-backed, CRUD API |
| Agent creation | Pool spawns at startup | On-demand via API |
| Hierarchies | 3 fixed tiers | User-defined graphs |
| Role config | Code-defined RoleLibrary | DB-backed, UI-editable |
| Delegation | Static `can_delegate_to` | Dynamic, per-team rules |
| Routing | Hardcoded rules in Router | Configurable rule engine |
| Code editor | None (FilesPage is a stub) | Monaco-based editor |
| Team designer | None | New UI component |

## Decisions Made

1. **Custom tiers**: Undecided — keep flexible. Design the schema so tiers are just a label string rather than an enum. The 3 defaults (Orchestrator, Worker, Utility) remain as presets.
2. **Team graphs**: DAG — agents can receive work from multiple parents, but with cycle prevention. Keep it practical, don't over-engineer.
3. **Tool extensibility**: Select from existing registry for now. Schema should allow custom tools later.
4. **Code editor**: Read-write. Full Monaco editor that saves to disk.
5. **Teams scope**: TBD — revisit when we get to team designer.

## Completed Work

### Part 1: Agent CRUD — Server (2026-01-31)

**Files modified:**
- `src/db/mod.rs` — Expanded `AgentRow` with all DB fields (persona_prompt, persona_style, model_provider, model_max_tokens, model_temperature)
- `src/db/pg_repo.rs` — Expanded list/upsert queries, added `get_persisted_agent`
- `src/db/traits.rs` — Added `get_persisted_agent` to `ServerRepo` trait
- `src/server/api.rs` — Added `CreateAgentRequest`, `UpdateAgentRequest`, expanded `AgentResponse`, added create/get/update/delete handlers, updated list_agents to query DB, added 7 integration tests
- `src/server/mod.rs` — Added routes: POST /agents, GET/PATCH/DELETE /agents/:id
- `src/server/tools.rs` — Updated AgentRow construction to include new fields
- `src/server/orchestrator.rs` — Added `get_persisted_agent` to test mock

**Endpoints added:**
- `POST /api/agents` — Create agent with full config
- `GET /api/agents` — List agents from DB (was returning empty vec)
- `GET /api/agents/:id` — Get single agent
- `PATCH /api/agents/:id` — Partial update
- `DELETE /api/agents/:id` — Delete agent

**Tests:** All 1,970 tests pass.

## Open Questions (remaining)

1. Should teams be scoped per-project or global?
