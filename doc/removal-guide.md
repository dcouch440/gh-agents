# Removal Guide

Everything listed here is safe to remove. The new system (21-table schema, DAG executor with react loop, tools on agents) replaces all of it.

Run `cargo check && cargo test` after each section to catch breakage incrementally.

---

## ✅ REMOVED: Pipeline System (Replaced by Workflow Collections)

**Removal Date:** February 3, 2026

**Status:** COMPLETE - All pipeline code and tables have been removed from the system.

**What Was Removed:**
- Database tables: `pipelines`, `pipeline_stages`, `pipeline_runs`, `pipeline_stage_members`
- Backend code: Pipeline-related API endpoints, handlers, and database operations
- Frontend code: Pipeline UI components and state management
- All foreign key references to pipeline tables

**Replacement:** Workflow Collections system provides equivalent functionality with improved architecture and UX.

**Migration Notes:** Existing users were migrated to Workflow Collections. See `doc/WORKFLOW_COLLECTIONS_MIGRATION.md` for details.

---

## 1. Old DB Traits + Implementations

Delete these traits from `src/db/traits.rs` and their `PgRepo` implementations from `src/db/pg_repo.rs`:

| Trait | What it was |
|-------|-------------|
| `DependencyRepo` | Task dependency tracking |
| `TaskQueueRepo` | Persistent task queue |
| `CostRepo` | Old cost records |
| `SchedulerRepo` | Production mode toggle |
| `RefactorRepo` | Refactor sessions/changes |
| `MergeQueueRepo` | PR merge queue |

Delete these methods from `ServerRepo` trait and `PgRepo` impl:

**Task system:**
- `list_tasks`, `get_task_by_uuid`, `insert_task`

**Old global chat:**
- `insert_chat_message`, `get_chat_history`, `clear_chat_history`

**Clusters:**
- `list_persisted_clusters`, `upsert_cluster`, `delete_cluster`
- `list_cluster_members`, `add_cluster_member`, `remove_cluster_member`

**Stage side tasks:**
- `list_stage_side_tasks`, `upsert_stage_side_task`, `delete_stage_side_task`

**Schedules:**
- `list_schedules`, `upsert_schedule`, `delete_schedule`, `update_schedule_last_run`

**Triggers:**
- `list_triggers`, `upsert_trigger`, `delete_trigger`

**Old logging (replaced by execution_messages + token_ledger):**
- `insert_tool_call`
- `insert_token_usage`, `get_usage_summary`

Delete these row types from `src/db/mod.rs`:
- `ScheduleRow`
- `TriggerRow`
- `ClusterRow`
- `StageSideTaskRow`
- `RoutingEventRow`
- `UsageSummaryRow`

Delete these source files entirely:
- `src/db/queries.rs` — old query helpers, most reference dropped tables
- `src/db/prd.rs` — old PRD system
- `src/db/refactor.rs` — old refactor system

Remove `pub mod prd;`, `mod queries;`, `mod refactor;` and `pub use queries::*;`, `pub use refactor::*;` from `src/db/mod.rs`.

Remove the corresponding imports from `src/db/traits.rs`:
- `ChatMessageRow`, `SessionRow` (if only used by old chat)
- `crate::github::{PrQueueEntry, QueueError as MergeQueueError}`
- `crate::orchestration::DependencyError`
- `crate::orchestration::QueueError as TaskQueueError`
- `crate::types::{ChangeId, ChangeStatus, CostRecord, ProductionMode, RefactorChange, RefactorSession, Task, TaskId, TaskStatus}`

---

## 2. Old API Endpoints + Types

Delete these route handlers from `src/server/api.rs`:
- `/api/tasks` handlers (list, get, create)
- `/api/chat` handlers (old global chat — not session chat)
- Old `/api/sessions/*` handlers if they reference `ChatMessageRow` from old chat
- Schedule/trigger endpoints (`/api/schedules`, `/api/triggers`)
- Cluster endpoints (`/api/clusters`)
- Stage side task endpoints

Delete the corresponding request/response DTOs from `src/server/api.rs`.

Remove the route wiring from `src/server/mod.rs` for all deleted endpoints.

Delete these type files:
- `src/types/task.rs`
- `src/types/cost.rs`
- `src/types/ticket.rs`
- `src/types/prd.rs`
- `src/types/refactor.rs`

Update `src/types/mod.rs` and `src/lib.rs` to remove the deleted module declarations and re-exports.

---

## 3. Old Orchestration + Agent Modules

Delete or gut these files:

**`src/agents/router_agent.rs`** — Old cluster-based routing agent. The tool dispatch is now direct (name match in `execution_tools.rs`). Delete entirely.

**`src/agents/tool_router.rs`** — Old cluster-routed tool dispatch. The `execute_request_assistance` function is still referenced by `executor.rs` for router mode. Either:
- Delete it and remove router mode from executor, or
- Keep only `request_assistance_tool()` and `execute_request_assistance()` with the simplified dispatch (no cluster routing), delete the rest

**`src/orchestration/`** — Modules tied to the old task/dispatch model:
- Dependency tracker
- Task queue
- Task router/dispatcher
- Anything that references `Task`, `TaskId`, `TaskStatus`

**`src/agents/schedule.rs`** — Old schedule manager. Delete if schedules are being removed.

**`src/agents/roles.rs`** — Old role system. Check if anything in the new system references it.

Update `AppState` in `src/server/state.rs` to remove fields for old components (schedule_manager, old dispatchers, cluster index, etc.).

---

## 4. Drop Legacy Tables

Migration `045_drop_legacy_tables.sql` (or whatever the next number is after your changes). Drop children first:

```sql
-- Task system
DROP TABLE IF EXISTS task_events CASCADE;
DROP TABLE IF EXISTS task_dependencies CASCADE;
DROP TABLE IF EXISTS tasks CASCADE;

-- Clusters (tools and agent_tools are kept)
DROP TABLE IF EXISTS cluster_members CASCADE;
DROP TABLE IF EXISTS clusters CASCADE;

-- Tickets
DROP TABLE IF EXISTS tickets CASCADE;
DROP TABLE IF EXISTS vertical_slices CASCADE;

-- Old chat
DROP TABLE IF EXISTS chat_messages CASCADE;
DROP TABLE IF EXISTS chat_sessions CASCADE;
DROP TABLE IF EXISTS messages CASCADE;

-- Old cost/usage tracking
DROP TABLE IF EXISTS cost_records CASCADE;
DROP TABLE IF EXISTS llm_calls CASCADE;
DROP TABLE IF EXISTS decisions CASCADE;
DROP TABLE IF EXISTS token_usage CASCADE;

-- Old tool call logging (tools + agent_tools are kept)
DROP TABLE IF EXISTS tool_calls CASCADE;

-- Routing/scheduling
DROP TABLE IF EXISTS routing_events CASCADE;
DROP TABLE IF EXISTS schedules CASCADE;
DROP TABLE IF EXISTS triggers CASCADE;

-- PRD/planning
DROP TABLE IF EXISTS prds CASCADE;
DROP TABLE IF EXISTS planning_sessions CASCADE;

-- Refactor
DROP TABLE IF EXISTS refactor_changes CASCADE;
DROP TABLE IF EXISTS refactor_sessions CASCADE;

-- Misc
DROP TABLE IF EXISTS pr_merge_queue CASCADE;
DROP TABLE IF EXISTS system_state CASCADE;
DROP TABLE IF EXISTS stage_side_tasks CASCADE;
DROP TABLE IF EXISTS auth_config CASCADE;
DROP TABLE IF EXISTS agent_context CASCADE;
```

---

## 5. Drop Deprecated Columns (Phase 6.2)

After the tables are gone and the code compiles clean:

**`agents`:** Drop `tier`, `persona_style`, `status`, `router_mode`, `current_task`

**`stage_executions`:** Drop old `rendered_prompt`, `output`, `structured_output`, `user_input`, `input_tokens`, `output_tokens`, `duration_ms`, old `agent_id`. Make `stage_member_id` NOT NULL.

**`documents`:** Drop `session_id`, `summary`, `doc_type`, `ref_tag`, `tags`

Remove `Option` wrappers from the corresponding row structs in `src/db/mod.rs` after columns are dropped.

---

## Verification

After each section:

```bash
cargo fmt
cargo clippy
cargo check
cargo test
```

The goal is 0 errors, 0 warnings, all tests pass. If a test references something you deleted, delete the test too.
