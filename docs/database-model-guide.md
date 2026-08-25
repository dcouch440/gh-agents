# Database Model Guide (Rust Layer)

This document explains how the **Rust code models the database** — where row
types live, how they're wired to Postgres, and the conventions for adding to
either. It does not enumerate columns, constraints, or migrations; for the
canonical SQL reference see [`docs/database-schema.md`](./database-schema.md).
This doc is about the layer between those SQL tables and the rest of the
application: `src/db/`.

The database has grown through 67 migrations (`migrations/0001_*.sql` through
`migrations/0067_chat_message_error.sql`). The old version of this doc was a
frozen snapshot from well before that growth and never actually named a Rust
file — this version does.

---

## 1. Directory layout

```
src/db/
├── mod.rs           # init_db(), DbPool alias, re-exports
├── types/            # Row structs — one file per domain area
│   ├── mod.rs
│   ├── agent.rs       # AgentRow, AgentGuidanceRow
│   ├── canvas.rs       # CanvasSnapshotRow, CanvasElementMapRow
│   ├── collection.rs    # WorkflowCollectionRow, CollectionWorkflowRow, ...
│   ├── document.rs      # DocumentRow, ContentVersionRow, RunTemplateRow, ...
│   ├── execution.rs     # WorkflowExecutionRow, AgentExecutionRow, TokenLedgerRow, ...
│   ├── protocol.rs      # ProtocolRow, ProtocolPortRow, ProtocolExecutionRow, ...
│   ├── room.rs         # RoomRow, RoomMemberRow, RoomSessionRow, ...
│   ├── system.rs        # OutputSchemaRow, PromptTemplateRow, SystemConfigRow, ResultRow
│   ├── system_file.rs    # SystemFileRow
│   ├── tool.rs         # ToolRow, ToolCapabilityRow, ...
│   ├── workflow.rs      # WorkflowRow, WorkflowStepRow, WorkflowStepEdgeRow, ...
│   └── workforce.rs     # TaskMissionBriefRow, TaskAgentRosterRow, BeliefRow, ...
├── traits/            # Repository trait definitions (one trait ≈ one domain)
│   ├── mod.rs
│   ├── agent.rs, collection.rs, content_version.rs, document.rs,
│   │   execution.rs, protocol.rs, room.rs, session.rs, system.rs,
│   │   system_file.rs, workflow.rs
├── pg_repo/           # The one production implementation, split by domain
│   ├── mod.rs          # PgRepo struct + SERIALIZABLE retry macro
│   ├── agent.rs, auth.rs, collection.rs, content_version.rs, cost.rs,
│   │   document.rs, execution.rs, protocol.rs, room.rs, session.rs,
│   │   system_config.rs, system_file.rs, tool.rs, tool_capability.rs,
│   │   user.rs, workflow.rs
│   └── tests.rs         # Integration tests against a real Postgres (TestDb)
├── queries/            # Legacy free-function layer (see §7)
│   ├── mod.rs           # ChatMessageRow, SessionRow + raw sqlx functions
│   └── tests.rs
├── test_utils/          # TestDb — per-test throwaway Postgres database
└── fixtures.rs          # `#[cfg(test)]` Default-based row builders
```

`src/db/types/mod.rs` and `src/db/traits/mod.rs` are flat `pub use` barrels —
every row type and every trait is available as `crate::db::TypeName`. There's
no further nesting; "domain area" is purely a file-naming convention, not a
module hierarchy.

---

## 2. The pattern: row type → trait → `PgRepo` impl

Every domain in `src/db/` follows the same three-layer shape:

1. **Row type** (`types/*.rs`) — a plain struct deriving `sqlx::FromRow`,
   `Clone`, `Debug`, usually `serde::Serialize` (for JSON API responses), and
   often a hand-written `impl Default` so tests and fixtures can build a row
   with `..Default::default()`.
2. **Repository trait** (`traits/*.rs`) — an `#[async_trait]` trait listing
   the operations available on that domain, e.g. `WorkflowRepo`. Every trait
   carries `#[cfg_attr(test, mockall::automock)]`, which generates a
   `MockWorkflowRepo` (etc.) for handler-level unit tests.
3. **`PgRepo` impl** (`pg_repo/*.rs`) — `sqlx::query_as` calls against
   Postgres. There is exactly one production struct, `PgRepo` (holds a
   `PgPool`, `src/db/pg_repo/mod.rs:78`), and it implements *all* of the
   repository traits — one `impl TraitName for PgRepo` block per domain file,
   sometimes several per file (e.g. `pg_repo/document.rs` implements
   `DocumentRepo`, `OutputSchemaRepo`, and `PromptTemplateRepo`).

### Worked example: `WorkflowStepRow`

**1. The row type** — `src/db/types/workflow.rs:23`:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct WorkflowStepRow {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub agent_id: Option<Uuid>,
    pub execution_mode: String, // "single", "workforce", "context", "input", "container"
    // ...
}
```

with a matching `impl Default for WorkflowStepRow` (`workflow.rs:179`) so
tests only set the fields they care about.

**2. The trait** — `src/db/traits/workflow.rs:84`, `WorkflowRepo`, declares
the CRUD surface:

```rust
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait WorkflowRepo: Send + Sync {
    async fn create_step(&self, step: WorkflowStepRow) -> Result<WorkflowStepRow>;
    async fn get_step(&self, id: Uuid) -> Result<Option<WorkflowStepRow>>;
    async fn list_steps(&self, workflow_id: Uuid) -> Result<Vec<WorkflowStepRow>>;
    async fn update_step(&self, step: WorkflowStepRow) -> Result<WorkflowStepRow>;
    async fn delete_step(&self, id: Uuid) -> Result<()>;
    // ...80+ more methods — see §6, this trait covers the entire workflow/
    // workforce/protocol/canvas/versioning surface, not just steps.
}
```

**3. The Postgres impl** — `src/db/pg_repo/workflow.rs:20`,
`impl WorkflowRepo for PgRepo`:

```rust
async fn create_step(&self, step: WorkflowStepRow) -> Result<WorkflowStepRow> {
    let row: WorkflowStepRow = sqlx::query_as(
        r#"INSERT INTO workflow_steps (id, workflow_id, agent_id, execution_mode, ...)
           VALUES ($1, $2, $3, $4, ...)
           RETURNING *"#,
    )
    .bind(step.id)
    .bind(step.workflow_id)
    // ...
    .fetch_one(&self.pool)
    .await?;
    Ok(row)
}

async fn get_step(&self, id: Uuid) -> Result<Option<WorkflowStepRow>> {
    let row: Option<WorkflowStepRow> =
        sqlx::query_as("SELECT * FROM workflow_steps WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
    Ok(row)
}

async fn update_step(&self, step: WorkflowStepRow) -> Result<WorkflowStepRow> {
    let row: WorkflowStepRow = sqlx::query_as(
        r#"UPDATE workflow_steps SET agent_id = $1, execution_mode = $2, ...,
               version = version + 1
           WHERE id = $24
           RETURNING *"#,
    )
    // ...
    .fetch_one(&self.pool)
    .await?;
    Ok(row)
}
```

(`create_step` at `pg_repo/workflow.rs:105`, `get_step` at `:142`,
`update_step` at `:175`, `delete_step` at `:218`.)

Notes on the query style used throughout `pg_repo/`:

- `SELECT *` / `RETURNING *` plus `sqlx::query_as` (not the compile-time
  checked `query_as!`) — `FromRow` matches columns by **name**, so field
  order in the struct doesn't need to match column order, but every column
  must have a struct field (and vice versa) or the row fails to deserialize.
- Every fallible call ends in `?` against `anyhow::Result` — no `.unwrap()`
  in a `pg_repo/*.rs` file. This matches the "no panics" rule for
  application code in `CLAUDE.md`.
- `update_step` bumps `version = version + 1` in the same statement instead
  of a separate read-modify-write — this is the general pattern for every
  row that carries a `version` column (`WorkflowRow`, `WorkflowStepRow`,
  `AgentRow`, `OutputSchemaRow`, `PromptTemplateRow`, `ToolRow`).
- Multi-statement operations that must be atomic use the `run_serializable!`
  macro (`pg_repo/mod.rs:29`): it opens a `SERIALIZABLE` transaction, runs
  the block, and retries up to `SERIALIZABLE_MAX_RETRIES` (3) times on
  Postgres error `40001` (serialization failure) before giving up.

**4. Wiring** — `PgRepo` doesn't get injected as itself; each trait is boxed
separately as `Arc<dyn WorkflowRepo>` and grouped into a `Repos` struct
(`src/server/state/repos.rs:19`) that hangs off `AppState`
(`src/server/state/mod.rs:100`). Construction clones the same pool into a
fresh `PgRepo` per trait slot:

```rust
let repos = Repos::new(
    Arc::new(PgRepo::new(db.clone())), // users
    Arc::new(PgRepo::new(db.clone())), // documents
    // ...
    Arc::new(PgRepo::new(db.clone())), // workflows
    // ...
);
```

This is cheap — `PgPool` is itself an `Arc` internally, so `db.clone()` is a
refcount bump, not a new connection pool. Handlers then call
`state.repos.workflows.create_step(...)` — they depend on the trait object,
never on `PgRepo` directly, which is what makes `MockWorkflowRepo`
substitutable in tests.

Not every trait lives in the central `Repos` registry. `WorkflowCollectionRepo`
and `WorkflowStepAgentRepo` (both in `traits/collection.rs`) are constructed
ad hoc at call sites instead — e.g.
`src/server/api/workflows/execution_handlers.rs:50`:
`let collection_repo: Arc<dyn WorkflowCollectionRepo> = Arc::new(PgRepo::new(db));`.
Same trait, same `PgRepo`, just not pre-wired into `AppState` — worth knowing
before assuming everything DB-related hangs off `state.repos`.

---

## 3. Testing conventions

Two complementary layers, both under `src/db/`:

- **`test_utils::TestDb`** (`src/db/test_utils/mod.rs`) — spins up a
  uniquely-named Postgres database (`nexor_test_{uuid}`), runs every
  migration against it, and drops it on cleanup. A shared admin pool and a
  `Semaphore` cap concurrent test databases at 4. `pg_repo/tests.rs` and
  `queries/tests.rs` use this to exercise real `PgRepo` methods against a
  real schema — these are integration tests, not mocked.
- **`fixtures` module** (`src/db/fixtures.rs`, `#[cfg(test)]`) — helper
  functions like `fixtures::step()` and `fixtures::workforce_step_with(...)`
  that build row structs via `..Default::default()`. The stated intent
  (`fixtures.rs:1`) is that adding a new field to a row struct requires zero
  changes to existing test files, since every fixture falls back to
  `Default`.
- **`mockall::automock`** on every trait generates `MockWorkflowRepo`,
  `MockAgentRepo`, etc. Handler and DAG-execution tests (e.g.
  `src/server/hub/dag/tests.rs`) construct these directly —
  `let mut wf_repo = MockWorkflowRepo::new();` then `.expect_get_step()...` —
  to test orchestration logic without touching Postgres at all.

---

## 4. Adding a new table: the typical recipe

Reverse-engineered from the pattern above, not policy — but this is what
every recent migration + type + repo triple looks like:

1. **Migration** — a new file in `migrations/` (`sqlx::migrate!()` runs them
   in order at startup, `src/db/mod.rs:38`).
2. **Row struct** in the matching `types/*.rs` file (or a new file, wired
   into `types/mod.rs`), deriving `sqlx::FromRow` + `Clone` + `Serialize`,
   plus `impl Default` if it'll be built in tests/fixtures.
3. **Trait method(s)** added to an existing domain trait, or a new trait in
   `traits/*.rs` if it's a genuinely new domain — either way,
   `#[cfg_attr(test, mockall::automock)]` stays on the trait.
4. **`PgRepo` impl** in the matching `pg_repo/*.rs` file: `sqlx::query_as`
   with `RETURNING *` / `SELECT *`, bound positionally, returning
   `anyhow::Result<T>`.
5. **Wiring**: if the new trait needs to reach handlers broadly, add a field
   to `Repos` (`server/state/repos.rs`) and thread it through
   `Repos::new(...)`; if it's only needed in one or two call sites, construct
   it ad hoc with `Arc::new(PgRepo::new(pool))` the way `WorkflowCollectionRepo`
   does.
6. **Fixtures + mocks**: add a `fixtures::` builder if the row will show up
   in tests a lot, and rely on the auto-generated `MockXxxRepo` for
   handler-level tests.

---

## 5. The workforce / workflow node model

This is the core of the "definition" layer, and the part the old doc got
most wrong: **`workforce` is not a table.** It's one value of
`WorkflowStepRow.execution_mode`. A `WorkflowStepRow` is a single DAG node
inside a `WorkflowRow`; what kind of node it is — a plain single-agent step,
a multi-agent "workforce" crew, a context/input passthrough, a container
step — is entirely determined by that one string field plus which of the
step's optional associated rows are populated.

Among the node kinds, **`workforce` is the flagship / most actively
developed archetype.** The dispatcher special-cases it explicitly and even
has a repair path: any step with a `child_workflow_id` set gets routed
through the workforce executor even if `execution_mode` is stale, because
having a mission brief + roster + child workflow is a stronger signal than
the mode string (`src/server/hub/dag/workshop/dispatch.rs:60-71`).

### `WorkflowRow` — `src/db/types/workflow.rs:6`

The workflow itself (a saved graph, reusable, versioned):

| Field | Type | Notes |
|---|---|---|
| `id`, `user_id`, `name`, `description`, `created_at` | — | unchanged from the old model |
| `execution_mode` | `String` | graph-level mode, e.g. `"dag"` (default) |
| `version` | `i32` | bumped by `PgRepo::update_workflow` |
| `container_enabled` | `bool` | whether steps in this workflow may run in Docker containers |
| `target_repo_url` / `target_branch` | `Option<String>` | the GitHub repo/branch this workflow's agents operate against |
| `vpn_enabled` | `bool` | whether container steps get a WireGuard sidecar (`hub/dag/container/`) |
| `board_overview_summary` | `String` | Haiku-distilled summary of the whole board, cached for the workflow agent / chat context |

### `WorkflowStepRow` — `src/db/types/workflow.rs:23`

The DAG node. Every field beyond the original `agent_id` / `prompt_template`
/ `output_schema_id` set exists to support either the visual canvas, the
workforce archetype, or per-step run-state caching:

| Field | Purpose |
|---|---|
| `execution_mode` | `"single"`, `"workforce"`, `"context"`, `"input"`, `"container"` — see §6 |
| `agent_execution_mode` | `"sequential"` / `"parallel"` for multi-agent steps, `None` = inherit from the workflow |
| `room_id` | links to a `RoomRow` when the step is a multi-agent "room" conversation (§8) |
| `routing_mode` / `routing_field` | label-based agent routing (paired with `StepRoutingRuleRow`) |
| `reasoning_trace` | whether to persist the agent's reasoning trace for this step |
| `verification_agent_ids` | JSON array of agent IDs used to verify this step's output |
| `position_x` / `position_y` / `width` / `height` | canvas layout — the step's own visual geometry |
| `name`, `description`, `system_prompt_suffix` | step-level display/config, distinct from the agent template's own name/prompt |
| `visible` | whether the step renders on the board (vs. hidden scaffolding) |
| `board_context_cache` / `board_context_updated_at` | Haiku-distilled awareness of the surrounding board, cached per step |
| `goal_summary` / `goal_summary_updated_at` | cached summary of the step's goal |
| `child_workflow_id` | **the workforce link** — the live child `WorkflowRow` this step spins up and owns (edited at design time, snapshotted at execution) |
| `ref_id` | stable LLM-facing identifier, e.g. `"workforce-1"`, used so agents can refer to steps by name instead of UUID |
| `pinned` | freezes the step's output — re-runs replay instead of re-executing |
| `run_results_summary` | Haiku-generated summary of the step's last run, surfaced to sibling steps via `get_run_context_for_step` |
| `designer_handoff` | free-text note the step's designer agent leaves for the next step's designer |

### Workforce support types — `src/db/types/workforce.rs`

A `workforce` step doesn't carry its configuration inline; it fans out to a
small cluster of rows keyed by `step_id`:

- **`TaskMissionBriefRow`** (`workforce.rs:6`) — one per workforce step: the
  task description, `available_capabilities`, `failure_mode`, and
  `downstream_context` the roster-design agent works from.
- **`TaskAgentRosterRow`** (`workforce.rs:19`) — one row per agent the
  workforce spins up: name, role description, capabilities,
  `execution_order`, and `child_step_id` linking it to its visual node in the
  child workflow.
- **`AgentDesignerRunRow`** / **`AgentDesignerOutputRow`** (`workforce.rs:33`,
  `:50`) — the audit trail of the LLM call(s) that *designed* the roster's
  prompts: model, token/cost accounting, and per-agent generated system +
  task prompts with the designer's reasoning. `AgentDesignerOutputRow` is
  generic across archetypes (`source_entity_id` / `source_archetype`), not
  workforce-only.
- **`BeliefExtractionPlanRow`** / **`BeliefRow`** (`workforce.rs:69`, `:82`) —
  design-time config for what a step should extract as "beliefs" (tagged,
  confidence-scored observations) from its output, and the runtime rows the
  gatekeeper actually extracts, respectively.

### Protocol types — `src/db/types/protocol.rs`

A `ProtocolRow` (`protocol.rs:6`) is a reusable execution recipe —
`protocol_type` is currently always `"workforce"` in practice, `config` is
the recipe's JSON parameters. `ProtocolPortRow` assigns agents to named
slots in the recipe. `ProtocolDocumentDefRow` (`protocol.rs:33`) defines a
deliverable document a workforce step should produce, optionally tied to a
specific roster agent via `agent_roster_entry_id`. `ProtocolExecutionRow`
(`protocol.rs:49`) is the audit trail for a protocol's hidden phases
(non-agent-visible bookkeeping steps). `WorkflowStepProtocolRow`
(`protocol.rs:77`) links a step to the protocol it was expanded from.

---

## 6. Other execution modes: backend-only vs. actively used

`WorkflowStepRow.execution_mode` also accepts `"context"`, `"input"`,
`"container"`, and (per the frontend's `ExecutionMode` type,
`frontend/src/types/workflow.ts:12`) `"manager"`. These exist in the type
system on both sides, but they are not all equally exercised in the product
today:

- **`context` / `input`** are trivial passthrough nodes — no LLM call at
  all. The dispatcher special-cases them before building any execution
  context: `if step.execution_mode == "context" || step.execution_mode ==
  "input" { return execute_passthrough(...) }`
  (`src/server/hub/dag/workshop/dispatch.rs:49`).
- **`container`** routes through `src/server/hub/dag/container/mod.rs`,
  which manages Docker container + optional WireGuard VPN sidecar lifecycle
  for isolated execution environments. It's a real, separate execution path,
  but a much smaller and less-traveled one than workforce.
- **`manager`** is checked in a handful of places
  (`hub/mod.rs:276`, `hub/board/state/fetch.rs:81`,
  `hub/execution/strategies/chat/tools.rs:40`) for board-level chat routing
  behavior, rather than having its own dedicated dispatch branch.

If you're modeling new node behavior, `workforce` is the pattern to study
(mission brief → roster → designer run → child workflow); the others are
narrower, backend-only mechanisms that haven't seen the same design
investment or frontend authoring surface.

---

## 7. The "pipeline" naming collision

**Read this before grepping for "pipeline" in this codebase.** The word
means two unrelated things depending on which layer you're in:

1. **Gone from the database.** The old `pipelines` /
   `pipeline_stages` / `pipeline_stage_members` / `pipeline_runs` tables the
   previous version of this doc described no longer exist. That whole
   concept was replaced by the collection types in
   `src/db/types/collection.rs`: `WorkflowCollectionRow` (a DAG of
   workflows), `CollectionWorkflowRow` (membership + per-workflow execution
   mode override), `CollectionWorkflowEdgeRow` (edges between workflows in
   the collection), and `CollectionRunRow` (one run of a collection). None of
   these are called "pipeline" anywhere.
2. **Alive as an unrelated service-layer term.** `src/server/services/pipeline/`
   uses "pipeline" to mean something else entirely: *the child workflow owned
   by a workforce step*. Its own doc comment is explicit about the
   disconnect (`src/server/services/pipeline/types.rs:1-7`):

   ```rust
   //! Types for the pipeline service layer.
   //!
   //! These types define the interface between callers (workforce tools,
   //! protocol apply, future pipeline creators) and the pipeline service.
   //! They are deliberately decoupled from DB row types — the service
   //! handles the mapping internally.
   ```

   Concretely, "pipeline" here is just `WorkflowStepRow.child_workflow_id`
   plus its steps/edges — the thing a workforce step is building. Types like
   `PipelineContext { parent_step_id, parent_workflow_id }`,
   `AddStepInput`, and `PipelineCreated { pipeline_id }` live entirely in the
   service layer and never touch a row type named `Pipeline*`.

There's also a third, narrower sense: `src/server/hub/dag/pipeline/` is an
internal DAG-engine module (level scheduling + output composition for agent
execution) whose own comment calls it "the legacy Pipeline" — this one is
purely an execution-engine implementation detail, not modeled in the
database or the service layer above, and shouldn't be confused with either
of the two meanings above.

**Bottom line:** if someone says "pipeline" now, they almost always mean the
service-layer concept (a workforce step's child workflow), never a database
table — that table family is `workflow_collections` /
`collection_workflows` / `collection_runs` now.

---

## 8. Execution records

- **`WorkflowExecutionRow`** (`src/db/types/execution.rs:6`) — one row per
  (sub-)workflow execution. `collection_run_id` links it to the collection
  run that triggered it (nullable — workflows can run standalone).
  `root_execution_id` and `depth` implement O(1) tree traversal for nested
  workflow executions (a workforce step's child workflow execution has the
  top-level execution as its root, `depth` = nesting level) — a Temporal-style
  pattern, per the field's doc comment.
- **`AgentExecutionRow`** (`execution.rs:26`) — one row per actual LLM
  invocation. Keyed by **`workflow_execution_id`**, not the old
  `stage_execution_id` (that concept is gone along with `pipelines`).
  `execution_type` distinguishes what kind of execution this is (e.g.
  `"dag_step"` is the default). `room_session_id` + `speaker_order` populate
  when the execution happened inside a room conversation (§9).
  `is_exemplary` flags an execution as a few-shot exemplar for future
  prompting. `trace` is a serialized dispatch trace (tokens, tool calls,
  errors) kept for persistence/debugging.
- **`ExecutionMessageRow`** / **`TokenLedgerRow`** (`execution.rs:51`, `:64`)
  — unchanged in spirit from the old doc: the LLM conversation and the cost
  ledger, respectively.
- **`TimelineRow`** (`execution.rs:77`) — not a table, a flattened
  `FromRow` target for a join across `agent_executions` +
  `execution_messages` + `workflow_steps`, used to serve the execution
  timeline view in one query instead of N+1 lookups.

---

## 9. Agent model

`AgentRow` (`src/db/types/agent.rs:6`) changed shape significantly:

- `user_id` is now `Option<Uuid>` — `None` means a **system agent**
  (paired with `is_system: bool`), not owned by any user.
- Timestamps (`created_at`/`updated_at`) were dropped from the row entirely.
- New fields: `tier` (capability/cost tier), `persona_style`, `status`,
  `output_schema_id` (agents can now declare their own default output
  shape), `version` (bumped on update, same convention as workflows/steps),
  and `default_reasoning_trace`.

`AgentGuidanceRow` (`agent.rs:26`) is new and undocumented previously:
distilled feedback/learned instructions for an agent, optionally scoped to a
specific `workflow_step_id`, versioned and toggleable via `is_active` — this
is how the system persists "what this agent learned" across runs without
mutating the agent's own `system_prompt`.

---

## 10. Domains the old doc never covered

These all exist in current `src/db/types/`, have no equivalent in the old
21-table doc, and are fully modeled through the row-type/trait/`PgRepo`
pattern from §2:

- **Room / chat** (`src/db/types/room.rs`) — `RoomRow` (a reusable
  multi-agent conversation config, optionally scoped to a
  `WorkflowCollectionRow` via `collection_id`), `RoomMemberRow` (which
  agents sit in the room), `RoomSessionRow` (one run of a room's
  conversation), `RoomTranscriptEntry` (a cross-execution join for
  rendering transcripts), `RoomExecutionOutputRow` (structured per-speaker
  output for agent-to-agent data passing), and the design-time pair
  `RoomStepConfigRow` / `RoomStepMemberRow` used when a workflow step's
  `room_id` points at a room. A separate, older chat concept —
  `ChatMessageRow` and `SessionRow` — lives in `src/db/queries/mod.rs`
  instead; see §11 for why.
- **Canvas persistence** (`src/db/types/canvas.rs`) — `CanvasSnapshotRow`
  (one upserted row per workflow, storing the full Excalidraw-style
  snapshot JSON plus the last board-submit response for debug rehydration)
  and `CanvasElementMapRow` (maps Excalidraw element IDs to
  `WorkflowStepRow`/`WorkflowStepEdgeRow` UUIDs — exactly one of `step_id`
  or `edge_id` is set per row, enforced as an XOR constraint in the DB).
- **System file store** (`src/db/types/system_file.rs`) —
  `SystemFileRow`: workflow-scoped file metadata (path, media type, tags,
  which step/agent produced it, `workflow_run_id` for run-produced vs.
  design-time files, and a `sealed` flag that goes true when the producing
  step is pinned — mirroring `WorkflowStepRow.pinned`).
- **Content versioning** (`src/db/types/document.rs`) —
  `ContentVersionRow` (immutable, hash-addressed content snapshots),
  `RunSnapshotRow` (links a run + step to a specific content version),
  `EnvelopeSnapshotRow` (a lightweight join target for reconstructing
  execution envelopes from snapshots), and `RunTemplateRow` (a frozen,
  named workflow snapshot a user can re-launch from).
- **System config** (`src/db/types/system.rs`) — `SystemConfigRow`
  (admin-controlled key/value config), plus `OutputSchemaRow`,
  `PromptTemplateRow`, and `ResultRow`, which existed in the old doc but
  each gained a `version` field.

---

## 11. The `queries/` module: an older, parallel pattern

Not everything goes through `traits/` + `pg_repo/`. `src/db/queries/mod.rs`
is a flatter, older style: free functions taking `&PgPool` directly and
returning `anyhow::Result<T>`, with row types (`ChatMessageRow`,
`SessionRow`) defined inline in the same file rather than in `types/`. This
is where global chat messages, chat sessions (including the L2/L3/L4
role-tagged sessions used by the builder and workflow-agent flows), and
basic auth-config lookups (`has_password`, `set_password`, `get_password`)
live.

It isn't fully outside the trait system, though — a couple of thin trait
shims wrap subsets of it for mockability: `ChatMessageRepo`
(`traits/session.rs:127`) and `AuthConfigRepo` are implemented in
`pg_repo/auth.rs` by simply delegating to the `queries::` free functions,
e.g.:

```rust
async fn insert_chat_message(&self, user_id: UserId, id: Uuid, role: String, content: String) -> Result<()> {
    crate::db::insert_chat_message(&self.pool, user_id, &id, &role, &content).await
}
```

But most of `queries/mod.rs` — session CRUD, `find_session_by_step_id`,
`find_manager_builder_session`, `find_workflow_agent_session`,
`check_initial_instructions_sent`, and friends — is called directly against
`&PgPool`, with no trait or mock in front of it. If you're extending chat/
session behavior, follow the existing `queries/mod.rs` convention rather
than introducing a competing pattern; if you're adding a new domain from
scratch, follow §2/§4 instead.

---

## 12. Where the "workflow agent" fits (and doesn't)

`src/server/services/workflow_agent/` (its own doc comment,
`workflow_agent/mod.rs:1-11`) is a board-level chat meta-agent that lets an
LLM edit a workflow by writing files — `topology.json` + `nodes/*.md` — in a
project-style repo, rather than calling structured tools directly. It
projects DB state to files before each agent turn, validates the agent's
file writes, and syncs the result back into `WorkflowRow` /
`WorkflowStepRow` / edges through the same `WorkflowRepo` described in §2.

That file-based editing loop is its own architecture and is documented
separately in `docs/backend-architecture.md`. What matters here is just the
boundary: the workflow agent is a *client* of the model layer described in
this document, not part of it — it reads and writes the same row types
through the same repo methods that any other handler would use.

---

## 13. Quick reference: where to look

| Question | Where |
|---|---|
| What columns does table X have, what are its constraints/indexes? | `docs/database-schema.md` |
| What Rust struct models table X? | `src/db/types/*.rs` — grep the table name |
| What operations can I perform on domain Y? | `src/db/traits/*.rs` |
| How is operation Z actually implemented against Postgres? | `src/db/pg_repo/*.rs` |
| How do I build a row for a test? | `src/db/fixtures.rs`, or `Default::default()` directly |
| How do I run a real-DB integration test? | `src/db/test_utils::TestDb`, pattern in `pg_repo/tests.rs` |
| How do I mock a repo in a handler test? | `MockXxxRepo::new()` (generated by `mockall::automock` on the trait) |
| Is "pipeline" the DB concept or the service concept? | It's not a DB concept anymore — see §7 |
