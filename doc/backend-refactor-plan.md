# Backend Database Refactoring Plan

6 phases, 20 steps. Each step is one commit/PR. Additive first, remove last.

Reference: `doc/database-model-guide.md` (19-table target schema)

---

## Phase 1: New Definition-Layer Tables (additive, no behavior change)

### Step 1.1: `output_schemas` table + CRUD
- **Migration** `033_create_output_schemas.sql`
- **Types**: `OutputSchemaRow` in `src/db/mod.rs`
- **Trait**: `OutputSchemaRepo` in `src/db/traits.rs` (create, get, list, update, delete)
- **Impl**: `PgRepo` in `src/db/pg_repo.rs`
- **API**: `GET/POST /api/output-schemas`, `GET/PUT/DELETE /api/output-schemas/:id`
- **Routes**: Wire in `src/server/mod.rs`
- **Tests**: DB round-trip, API handler tests
- **Verify**: `cargo fmt && cargo clippy && cargo check && cargo test`

### Step 1.2: `prompt_templates` table + CRUD
- Same pattern as 1.1
- **Migration** `034_create_prompt_templates.sql`
- Table: id, user_id, name, content, created_at. Unique (user_id, name)
- **Trait**: `PromptTemplateRepo`
- **API**: `/api/prompt-templates`

### Step 1.3: `workflows` + `workflow_steps` + `workflow_step_edges` + `step_documents`
- **Migration** `035_create_workflows.sql` (all 4 tables in one migration)
- **Types**: `WorkflowRow`, `WorkflowStepRow`, `WorkflowStepEdgeRow`, `StepDocumentRow`
- **Trait**: `WorkflowRepo` — workflow CRUD, step CRUD, edge set/get, step document set/get
- **API**:
  - `GET/POST /api/workflows`, `GET/PUT/DELETE /api/workflows/:id`
  - `POST /api/workflows/:id/steps`, `PUT/DELETE /api/workflows/:wid/steps/:sid`
  - `POST/DELETE /api/workflows/:id/edges`
  - `POST/DELETE /api/workflows/:wid/steps/:sid/documents`
- **Tests**: Full CRUD, DAG cycle detection test
- **Verify**: `cargo fmt && cargo clippy && cargo check && cargo test`

### Step 1.4: `pipeline_stage_members` table + CRUD
- **Migration** `036_create_pipeline_stage_members.sql`
- **Types**: `PipelineStageMemberRow`
- **Trait**: Add to existing pipeline repo — list, add, remove, reorder members
- **API**: `GET/POST /api/pipelines/:pid/stages/:num/members`, `PUT/DELETE .../members/:mid`
- **Tests**: CRUD round-trip

---

## Phase 2: New Execution-Layer Tables (additive)

### Step 2.1: `agent_executions` + `execution_messages`
- **Migration** `037_create_agent_executions.sql` (both tables)
- **Types**: `AgentExecutionRow`, `ExecutionMessageRow`
- **Trait**: `AgentExecutionRepo` — create/update/get/list agent_executions, create/list execution_messages
- **API** (read-only for now):
  - `GET /api/agent-executions/:id`
  - `GET /api/agent-executions/:id/messages`
- **Tests**: Insert/query, parent-child interactive relationship

### Step 2.2: `token_ledger`
- **Migration** `038_create_token_ledger.sql`
- **Types**: `TokenLedgerRow`
- **Trait**: `TokenLedgerRepo` — insert, get_user_spend, get_run_spend, get_model_breakdown
- **API**: `GET /api/costs` (with date range params)
- **Tests**: Insert and aggregation

### Step 2.3: `results`
- **Migration** `039_create_results.sql`
- **Types**: `ResultRow`
- **Trait**: `ResultRepo` — save, get, list, list_by_schema, delete
- **API**: `GET /api/results`, `GET/DELETE /api/results/:id`
- **Tests**: CRUD

---

## Phase 3: Modify Existing Tables (make old columns nullable, add new columns)

### Step 3.1: Simplify `agents`
- **Migration** `040_simplify_agents.sql`: Rename persona_name→name, persona_prompt→system_prompt. Make tier, persona_style, current_task, status, router_mode nullable with defaults.
- **Types**: Update `AgentRow` field names, old fields become `Option`
- **DB/API**: Update queries to use new names. Old API still works.
- **Tests**: Existing agent tests pass

### Step 3.2: Simplify `pipeline_stages`
- **Migration** `041_simplify_pipeline_stages.sql`: Make agent_id, cluster_id, role, approval_required, fan_out, input_definitions, output_description, output_schema nullable.
- **Types**: `PipelineStageRow` fields become `Option`
- **Tests**: Existing pipeline tests pass

### Step 3.3: Simplify `stage_executions`
- **Migration** `042_simplify_stage_executions.sql`: Add stage_member_id (nullable FK), pipeline_id, stage_number columns.
- **Types**: New fields as `Option` on `StageExecutionRow`
- **Tests**: Existing tests pass

### Step 3.4: Simplify `pipeline_runs`
- **Migration** `043_simplify_pipeline_runs.sql`: Make stage_outputs, total_input_tokens, total_output_tokens nullable. Add alias initial_task→initial_input.
- **Types**: Update `PipelineRunRow`
- **Tests**: Existing tests pass

### Step 3.5: Simplify `documents`
- **Migration** `044_simplify_documents.sql`: Make session_id, summary, doc_type, ref_tag, tags nullable with defaults.
- **Types**: Those fields become `Option` on `DocumentRow`
- **Tests**: Existing document tests pass

---

## Phase 4: Migrate Runtime Logic to New Tables

### Step 4.1: Dual-write pipeline execution
- Update pipeline execution in `src/server/orchestrator.rs` to write `agent_executions`, `execution_messages`, `token_ledger` rows alongside existing writes.
- Both old and new tables populated.
- **Tests**: Run pipeline, verify both old and new tables have data

### Step 4.2: Read from new tables
- Update `/api/pipelines/:pid/runs/:rid/tree` endpoint to read from `agent_executions` + `execution_messages`
- Update cost endpoint to read from `token_ledger`
- **Tests**: API returns correct data from new tables

### Step 4.3: Stage members + workflow DAG execution
- Pipeline runner resolves work from `pipeline_stage_members` instead of `pipeline_stages.agent_id/cluster_id`
- Implement workflow DAG executor: topological sort, entry node detection, fan-out/fan-in via edges, for_each expansion, `{variable}` resolution from prior step outputs
- Handle `interactive_agent_id` — two agent_execution rows per step
- **API**: `POST /api/agent-executions/:id/messages` (send chat), `POST /api/agent-executions/:id/approve` (approve interactive)
- **WebSocket**: `agent_execution_update`, `for_each_spawned`, `execution_message` events
- **Tests**: Linear workflow, parallel fan-out, for_each, interactive approval, variable resolution

---

## Phase 5: Remove Old Code

### Step 5.1: Remove old DB traits + impls
- Delete: `DependencyRepo`, `TaskQueueRepo`, `CostRepo`, `SchedulerRepo`, `RefactorRepo`, `MergeQueueRepo`
- Delete: PgRepo impls, `src/db/queries.rs`, `src/db/prd.rs`, `src/db/refactor.rs`
- Remove old methods from `ServerRepo`: task CRUD, old chat, cluster CRUD, schedule/trigger, tool_calls, token_usage, stage_side_tasks
- Remove old row types from `src/db/mod.rs`
- **Verify**: `cargo check && cargo test`

### Step 5.2: Remove old API endpoints + types
- Delete routes: `/api/tasks`, `/api/chat` (old), old `/api/sessions/*`, schedule/trigger endpoints
- Delete handlers, DTOs, route constants
- Delete `src/types/task.rs`, `src/types/cost.rs`, `src/types/ticket.rs`, `src/types/prd.rs`, `src/types/refactor.rs`
- Update `src/types/mod.rs`, `src/lib.rs` exports
- **Verify**: `cargo check && cargo test`

### Step 5.3: Remove old orchestration/agent modules
- Remove `src/orchestration/` modules tied to old task/dispatch model
- Remove `src/agents/` modules tied to old execution model
- Update `AppState` to remove old component references
- **Verify**: `cargo check && cargo test`

---

## Phase 6: Drop Old Tables + Columns

### Step 6.1: Drop legacy tables
- **Migration** `045_drop_legacy_tables.sql` (children first):
  - task_events, task_dependencies, tasks
  - cluster_members, clusters
  - tickets, vertical_slices
  - chat_messages, chat_sessions (old)
  - messages
  - cost_records, llm_calls, decisions, token_usage
  - tool_calls (tools and agent_tools are kept — recreated in migration 046)
  - routing_events, schedules, triggers
  - prds, planning_sessions
  - refactor_changes, refactor_sessions
  - pr_merge_queue, system_state, stage_side_tasks
  - auth_config, agent_context
- **Tests**: Full test suite passes against new schema

### Step 6.2: Drop deprecated columns
- **Migration** `046_drop_deprecated_columns.sql`:
  - `agents`: Drop tier, persona_style, status, router_mode, current_task
  - `pipeline_stages`: Drop agent_id, cluster_id, role, approval_required, fan_out, input_definitions, output_description, output_schema
  - `stage_executions`: Drop old output/prompt fields, make stage_member_id NOT NULL
  - `pipeline_runs`: Drop stage_outputs, total_input_tokens, total_output_tokens
  - `documents`: Drop session_id, summary, doc_type, ref_tag, tags
- **Types**: Remove `Option` wrappers, clean up row structs
- **Tests**: Full suite passes
- **Verify**: `cargo fmt && cargo clippy && cargo check && cargo test`

---

## Critical Files

| File | Every step touches |
|------|--------------------|
| `src/db/mod.rs` | Row types |
| `src/db/traits.rs` | Repository traits |
| `src/db/pg_repo.rs` | Trait implementations (2,384 lines) |
| `src/server/mod.rs` | Route wiring |
| `src/server/api.rs` | API handlers |
| `migrations/` | SQL migrations |

## Verification (every step)

```bash
cargo fmt
cargo clippy
cargo check
cargo test
# For DB steps: cargo test -- --ignored (requires running Postgres)
```
