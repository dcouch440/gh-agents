# Documenter Protocol — Implementation Tickets

## Context

The Documenter Protocol turns the workflow canvas into a document generation pipeline. A single protocol node takes a user prompt, receives upstream context through wires, and produces finished documents as first-class canvas nodes. Unlike existing protocols (decomp, review, route) which are static templates, the Documenter **thinks** — it calls an LLM to generate a strategy, then executes a hidden chain of agent-less steps to research and write documents.

Key architectural decisions:
- **New executor**: `DocumenterExecutor` — a phased pipeline (strategy -> research -> write), NOT injected into the main DAG
- **Agent-less execution**: Hidden steps don't need agents. `workflow_steps.agent_id` becomes nullable **broadly**
- **Capability-based tool resolution**: Strategy LLM requests capabilities, system resolves to tool sets directly (no agent lookup)
- **Persisted hidden state**: Each phase's input/output/status saved to `protocol_executions` table
- **All LLM calls through ExecutionEngine**: New strategy types implement `ExecutionStrategy` trait

---

## Dependency Graph

```
Part 1  ──────────────────────────────────────> Schema migrations
  |
  |---> Part 2  ─────────────────────────────-> Nullable agent_id
  |       |
  |---> Part 3  ─────────────────────────────-> Tool capabilities
  |       |
  └──-> Part 6  ─────────────────────────────-> Doc defs + executions API
          |
          v
        Part 4  ─────────────────────────────-> Documenter expander
          |
          v
        Part 5  ─────────────────────────────-> DocumenterExecutor
          |
          v
        Part 7  ─────────────────────────────-> Frontend
          |
          v
        Part 8  ─────────────────────────────-> Integration testing
```

Parts 2, 3, and 6 can proceed in parallel after Part 1.
Part 4 needs Parts 1+2.
Part 5 needs Parts 1-4.
Part 7 needs Parts 1-6.

---

## Part 1: Schema Migrations

**Migration file:** `migrations/0019_documenter_protocol.sql`

### 1A. Make `workflow_steps.agent_id` nullable

```sql
ALTER TABLE workflow_steps ALTER COLUMN agent_id DROP NOT NULL;
```

### 1B. Add `visible` column to `workflow_steps`

```sql
ALTER TABLE workflow_steps ADD COLUMN visible boolean DEFAULT true;
```

### 1C. Alter `documents` table

```sql
ALTER TABLE documents
    ADD COLUMN workflow_id uuid REFERENCES workflows(id),
    ADD COLUMN target_length integer,
    ADD COLUMN is_static boolean DEFAULT false,
    ADD COLUMN source_protocol_step_id uuid REFERENCES workflow_steps(id);

CREATE INDEX idx_documents_workflow_id ON documents(workflow_id);
CREATE INDEX idx_documents_source_protocol_step_id ON documents(source_protocol_step_id);
```

### 1D. Alter `protocol_document_defs` for protocol-scoped defs

```sql
ALTER TABLE protocol_document_defs ALTER COLUMN step_id DROP NOT NULL;
ALTER TABLE protocol_document_defs
    ADD COLUMN protocol_id uuid REFERENCES protocols(id) ON DELETE CASCADE,
    ADD COLUMN document_id uuid REFERENCES documents(id);

CREATE INDEX idx_protocol_document_defs_protocol_id ON protocol_document_defs(protocol_id);

ALTER TABLE protocol_document_defs ADD CONSTRAINT check_scope CHECK (
    (step_id IS NOT NULL AND protocol_id IS NULL) OR
    (step_id IS NULL AND protocol_id IS NOT NULL)
);
```

### 1E. Create `protocol_executions` table

```sql
CREATE TABLE protocol_executions (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    protocol_step_id uuid NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    workflow_run_id uuid,
    phase           text NOT NULL,
    document_def_id uuid REFERENCES protocol_document_defs(id),
    agent_id        uuid REFERENCES agents(id),
    input_prompt    text,
    output_content  text,
    status          text NOT NULL DEFAULT 'pending',
    error_message   text,
    tokens_in       integer,
    tokens_out      integer,
    cost_usd        double precision,
    model           text,
    capabilities_used text[],
    created_at      timestamptz DEFAULT now(),
    completed_at    timestamptz
);

CREATE INDEX idx_protocol_executions_step_id ON protocol_executions(protocol_step_id);
CREATE INDEX idx_protocol_executions_run_id ON protocol_executions(workflow_run_id);

ALTER TABLE protocol_executions ADD CONSTRAINT protocol_executions_phase_check
    CHECK (phase IN ('strategy', 'research', 'write'));
ALTER TABLE protocol_executions ADD CONSTRAINT protocol_executions_status_check
    CHECK (status IN ('pending', 'running', 'complete', 'failed'));
```

### Verification

```bash
docker exec gh-agents-postgres-1 psql -U nexor -d nexor -c "\d workflow_steps"
docker exec gh-agents-postgres-1 psql -U nexor -d nexor -c "\d documents"
docker exec gh-agents-postgres-1 psql -U nexor -d nexor -c "\d protocol_document_defs"
docker exec gh-agents-postgres-1 psql -U nexor -d nexor -c "\d protocol_executions"
```

---

## Part 2: Nullable `agent_id` Ripple Effects

Making `agent_id` optional touches every layer. This is the highest-risk change.

### 2A. Rust Row Types

**`WorkflowStepRow`** — `src/db/mod.rs` ~line 141
- `agent_id: Uuid` -> `agent_id: Option<Uuid>`
- Add field: `visible: bool`

**`DocumentRow`** — `src/db/mod.rs` ~line 77
- Add: `workflow_id: Option<Uuid>`
- Add: `target_length: Option<i32>`
- Add: `is_static: Option<bool>`
- Add: `source_protocol_step_id: Option<Uuid>`

**`ProtocolDocumentDefRow`** — `src/db/mod.rs` ~line 37
- `step_id: Uuid` -> `step_id: Option<Uuid>`
- Add: `protocol_id: Option<Uuid>`
- Add: `document_id: Option<Uuid>`

**New `ProtocolExecutionRow`** — `src/db/mod.rs`
```rust
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ProtocolExecutionRow {
    pub id: Uuid,
    pub protocol_step_id: Uuid,
    pub workflow_run_id: Option<Uuid>,
    pub phase: String,
    pub document_def_id: Option<Uuid>,
    pub agent_id: Option<Uuid>,
    pub input_prompt: Option<String>,
    pub output_content: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub tokens_in: Option<i32>,
    pub tokens_out: Option<i32>,
    pub cost_usd: Option<f64>,
    pub model: Option<String>,
    pub capabilities_used: Option<Vec<String>>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}
```

**`WorkflowStepResponse`** — `src/server/api/workflows/mod.rs` ~line 67
- `agent_id: Uuid` -> `agent_id: Option<Uuid>`
- Add: `visible: bool`

**`StepDefinition`** — `src/server/hub/protocols/types.rs` ~line 70
- `agent_id: Uuid` -> `agent_id: Option<Uuid>`

**`HubError::AgentNotFound`** — `src/server/hub/error/mod.rs` ~line 45
- `agent_id: Uuid` -> `agent_id: Option<Uuid>`

### 2B. DB Query Updates

**`src/db/pg_repo/mod.rs`:**
- `create_step` (~line 1376): Add `visible` to INSERT columns + bind
- `update_step` (~line 1426): Add `visible` to SET clause + bind
- `list_document_defs` (~line 1569): Add `document_id`, `protocol_id` to SELECT
- `create_document_def` (~line 1579): Add `document_id`, `protocol_id` to INSERT
- All `DocumentRow` queries: Add 4 new columns to SELECT/INSERT where explicit column lists are used

**Fix pre-existing bug in `get_tools_by_capability`** (~line 3419):
- Wrong column names (`input_schema` should be `parameters`, `updated_at` doesn't exist)
- Missing columns (`display_name`, `version`)

### 2C. DAG Executor — Critical Sites

All in `src/server/hub/dag/mod.rs`:

**Site 1** (~line 542): Main workflow loop agent load
```rust
// Before: unconditional agent load
// After: unwrap with error, skip for documenter/entry/document modes
let agent_id = step.agent_id.ok_or_else(|| HubError::Internal(
    anyhow!("step {} requires agent_id for mode {}", step_id, step.execution_mode)
))?;
```

**Site 2** (~line 2501): Collection variant — same fix

**Site 3** (~line 1978): Chain pipeline agent load — same fix

**Site 4** — `compose_prompt` in `src/server/hub/dag/utils/mod.rs` (~line 566):
```rust
// Before: get_agent_context(step.agent_id)
// After: gate behind if let Some(agent_id)
```

### 2D. API Handler Updates

**`create_workflow_step`** — `src/server/api/workflows/mod.rs` (~line 478):
- `resolved_agent_id` becomes `Option<Uuid>`
- For `"entry"` / `"document"` / `"documenter"` modes: `agent_id = None`
- Add `visible: true` default

**`update_workflow_step`** — same file (~line 613):
- `req.agent_id.or(existing.agent_id)` (both Option now)
- Preserve `visible` from existing

**`step_response` helper** — same file (~line 171):
- Add `visible: r.visible`

### 2E. Test Helpers to Update

Every `WorkflowStepRow` construction in tests needs `agent_id: Some(Uuid::new_v4())` and `visible: true`:
- `src/server/hub/dag/tests.rs` — `make_step`, `make_for_each_step`
- `src/server/hub/dag/utils/tests.rs` — `make_step`
- `src/server/hub/engine/filters/documenter_prompt/tests.rs` — `make_def`

---

## Part 3: Wire Up Tool Capabilities

The `tool_capabilities`, `tool_capability_assignments`, and `mode_required_capabilities` tables exist but are completely unwired.

### 3A. Fix `get_tools_by_capability`

**File:** `src/db/pg_repo/mod.rs` (~line 3419)

The existing SQL is broken (wrong column names). Fix to:
```sql
SELECT t.id, t.name, t.display_name, t.description, t.parameters,
       t.created_at, t.version
FROM tools t
JOIN tool_capability_assignments tca ON t.id = tca.tool_id
JOIN tool_capabilities tc ON tc.id = tca.capability_id
WHERE tc.capability_key = $1
ORDER BY t.name
```

### 3B. Add `get_tools_by_capabilities` (multi-key)

**File:** `src/db/traits/mod.rs` — add to `ToolCapabilityRepo`:
```rust
async fn get_tools_by_capabilities(&self, capability_keys: &[String]) -> Result<Vec<ToolRow>>;
```

**Implementation** in `src/db/pg_repo/mod.rs`:
```sql
SELECT t.id, t.name, t.display_name, t.description, t.parameters, t.created_at, t.version
FROM tools t
WHERE NOT EXISTS (
    SELECT 1 FROM unnest($1::text[]) AS required_key
    WHERE NOT EXISTS (
        SELECT 1 FROM tool_capability_assignments tca
        JOIN tool_capabilities tc ON tc.id = tca.capability_id
        WHERE tca.tool_id = t.id AND tc.capability_key = required_key
    )
)
ORDER BY t.name
```

### 3C. Extract Capability Resolver Utility

**New module:** `src/server/hub/capability_resolver/mod.rs`

Extract the capability -> tool resolution pattern from `ModeResolver` (~lines 170-193 of `src/server/hub/mode_resolver/mod.rs`):

```rust
pub async fn resolve_capabilities_to_tools(
    capability_keys: &[String],
    repo: &dyn ToolCapabilityRepo,
) -> Result<(Vec<Tool>, Vec<String>), HubError> {
    let mut tools = Vec::new();
    let mut seen = HashSet::new();
    for cap_key in capability_keys {
        let tool_rows = repo.get_tools_by_capability(cap_key).await?;
        for row in &tool_rows {
            if seen.insert(row.name.clone()) {
                if let Some(tool_def) = registry::get_tool_definition(&row.name) {
                    tools.push(tool_def);
                }
            }
        }
    }
    let tool_names = tools.iter().map(|t| t.name.clone()).collect();
    Ok((tools, tool_names))
}
```

### 3D. Seed Capability Data

Create a migration or seed script to populate `tool_capabilities` and `tool_capability_assignments` with baseline entries:
- `web_search` — tools: web_search, web_fetch
- `code_analysis` — tools: file_read, git_diff, ast_parse
- `file_operations` — tools: file_read, file_write, directory_list

---

## Part 4: Documenter Protocol Expander

### 4A. Builtin Seed

**File:** `src/server/hub/protocols/builtins.rs`

Add 6th protocol:
```rust
BuiltinProtocol {
    id: Uuid::new_v5(&PROTOCOLS_NS, b"Documenter"),
    name: "Documenter".into(),
    description: "Generate structured documents using specialist agents...".into(),
    protocol_type: "documenter".into(),
    config: serde_json::json!({}),
},
```

Update test assertions from 5 -> 6.

### 4B. Schema Generation

**File:** `src/server/hub/protocols/schema_gen.rs`

Add `documenter_schema(doc_defs: &[Value]) -> Value`:
- Output: `{ documents: [{ name: enum, research_strategy, required_capabilities, writer_prompt }] }`
- `name` enum populated from doc def names

### 4C. Prompt Generation

**File:** `src/server/hub/protocols/prompt_gen.rs`

Add `documenter_prompt(doc_defs: &[Value]) -> String`:
- Role: "Document Strategist"
- Lists requested documents with names, target lengths, descriptions
- Lists available capabilities from the registry
- Instructs: plan research strategy per document, specify required capabilities

### 4D. DocumenterExpander

**New file:** `src/server/hub/protocols/expanders/documenter.rs`

```rust
pub struct DocumenterExpander;

impl ProtocolExpander for DocumenterExpander {
    fn protocol_type(&self) -> &str { "documenter" }

    fn validate(&self, config: &ProtocolConfig) -> Result<(), ProtocolError> {
        // config.config must have "document_defs" array, each with name + target_length
    }

    fn generate_schema(&self, config: &ProtocolConfig) -> Result<Value, ProtocolError> {
        let defs = config.config["document_defs"].as_array().unwrap();
        Ok(schema_gen::documenter_schema(defs))
    }

    fn generate_prompt_injection(&self, config: &ProtocolConfig) -> Result<String, ProtocolError> {
        let defs = config.config["document_defs"].as_array().unwrap();
        Ok(prompt_gen::documenter_prompt(defs))
    }

    fn expand(&self, config: &ProtocolConfig) -> Result<ProtocolExpansion, ProtocolError> {
        // Returns expansion with schema + prompt but NO steps or edges
        // (DocumenterExecutor handles hidden execution at runtime)
        Ok(ProtocolExpansion {
            output_schema: self.generate_schema(config)?,
            prompt_injection: self.generate_prompt_injection(config)?,
            steps: vec![],
            edges: vec![],
            output_ports: vec![],
            input_ports: vec![],
        })
    }
}
```

### 4E. Register Expander

**`src/server/hub/protocols/expanders/mod.rs`:** Add `mod documenter; pub use documenter::DocumenterExpander;`

**`src/server/hub/protocols/mod.rs`:** Register in `register_builtins()`

### 4F. Apply Endpoint Changes

**File:** `src/server/api/protocols/mod.rs`

In `apply_protocol` handler:
1. Before calling `engine.expand()`, inject doc defs from DB into `ProtocolConfig.config`
2. After expansion, documenter-specific branch: create blank document rows, link via `step_documents`, update doc def rows with `document_id`

---

## Part 5: DocumenterExecutor

This is the core new system — a phased pipeline executor.

### 5A. Strategy Types

Three new `ExecutionStrategy` implementations:

**`src/server/hub/strategies/documenter_strategy/mod.rs`** — Phase 1: Strategy Planning
- System prompt: auto-generated from doc defs + available capabilities
- Tools: none
- Max rounds: 1 (single-turn structured output)
- Temperature: 0.3

**`src/server/hub/strategies/documenter_research/mod.rs`** — Phase 2: Research
- System prompt: research-oriented
- Tools: resolved from capability registry
- Max rounds: 15 (tool use loops)
- Temperature: 0.2
- `execute_tool()`: delegates to `execution_tools::execute_execution_tool()` (mirrors DagStepStrategy)

**`src/server/hub/strategies/documenter_writer/mod.rs`** — Phase 3: Write
- System prompt: writer-oriented with length guidance
- Tools: none
- Max rounds: 1
- Temperature: 0.5

### 5B. Strategy Output Schema

Phase 1 LLM returns structured JSON:
```json
{
  "document_plans": [
    {
      "document_name": "API Reference",
      "research_strategy": "Analyze the API routes in src/server/api/...",
      "required_capabilities": ["code_analysis"],
      "writer_prompt": "Write a comprehensive REST API reference..."
    }
  ]
}
```

Rust types:
```rust
#[derive(Debug, Deserialize)]
pub struct StrategyOutput {
    pub document_plans: Vec<DocumentPlan>,
}

#[derive(Debug, Deserialize)]
pub struct DocumentPlan {
    pub document_name: String,
    pub research_strategy: String,
    pub required_capabilities: Vec<String>,
    pub writer_prompt: String,
}
```

### 5C. DocumenterExecutor

**New module:** `src/server/hub/dag/documenter/mod.rs`

```rust
pub struct DocumenterExecutor<'a> {
    engine: &'a ExecutionEngine,
    state: &'a AppState,
    ctx: &'a WorkflowExecutionContext,
    step: &'a WorkflowStepRow,
    cancel: Option<&'a CancellationToken>,
}
```

**`execute()` method flow:**

```
1. Load document definitions from protocol_document_defs
2. Build Phase 1 system prompt (doc defs + available capabilities)
3. Compose user prompt (step prompt + upstream context)

4. PHASE 1 — Strategy:
   - Create protocol_execution row (phase="strategy", status="running")
   - engine.execute(DocumenterStrategyStrategy) -> structured JSON
   - Parse into StrategyOutput
   - Update protocol_execution (status="complete", output_content)
   - Broadcast progress

5. PHASE 2 — Research (parallel via JoinSet):
   For each document_plan:
     - Create protocol_execution row (phase="research", status="running")
     - resolve_capabilities_to_tools(plan.required_capabilities)
     - engine.clone_with_provider().execute(DocumenterResearchStrategy)
     - Update protocol_execution (status="complete" or "failed")
     - Broadcast progress
   Collect results. Continue on partial failure.

6. PHASE 3 — Write (parallel via JoinSet):
   For each successful research result:
     - Create protocol_execution row (phase="write", status="running")
     - Build writer prompt = plan.writer_prompt + research output
     - engine.clone_with_provider().execute(DocumenterWriterStrategy)
     - Save content to documents table
     - Update protocol_execution (status="complete")
     - Broadcast via WebSocket (document populated)

7. Return aggregated tokens/cost
```

### 5D. DAG Executor Integration

**File:** `src/server/hub/dag/mod.rs`

Add dispatch branch at ~line 576 (before the `for_each` check):
```rust
} else if step.execution_mode == "documenter" {
    execute_documenter_step(engine, state, ctx, step, /* ... */).await
}
```

The `execute_documenter_step` wrapper:
- Broadcasts StepStarted
- Resolves port inputs (upstream context)
- Composes prompt
- Constructs `DocumenterExecutor` and calls `execute()`
- Accumulates tokens/cost into workflow totals
- Stores output in var_outputs/completed/completed_envelopes
- Broadcasts StepCompleted

### 5E. Error Handling

- **Phase 1 failure:** Fatal — step fails, error propagated via DAG broadcaster
- **Phase 2 partial failure:** Log warning, continue with successful docs. If ALL fail -> step fails
- **Phase 3 partial failure:** Same pattern. Failed docs get no content but don't block others
- **Cost tracking:** Accumulate across all phases and all parallel tasks

### 5F. WebSocket Events

**New variant** in `src/server/ws/events.rs`:
```rust
DocumenterPhaseProgress {
    step_id: Uuid,
    phase: String,         // "strategy", "research", "write"
    completed: usize,
    total: usize,
    document_name: Option<String>,
}
```

---

## Part 6: Protocol Document Defs API

### 6A. New Handlers

**New file:** `src/server/api/protocols/documents.rs`

| Method | Path | Handler |
|--------|------|---------|
| POST | `/api/protocols/:id/documents` | `create_protocol_document` |
| GET | `/api/protocols/:id/documents` | `list_protocol_documents` |
| PUT | `/api/protocols/:pid/documents/:did` | `update_protocol_document` |
| DELETE | `/api/protocols/:pid/documents/:did` | `delete_protocol_document` |

These operate on protocol-scoped defs (`protocol_id` set, `step_id` null). At apply time, defs are copied to step-scoped entries.

### 6B. Protocol Executions API

| Method | Path | Handler |
|--------|------|---------|
| GET | `/api/protocols/:id/executions` | `list_protocol_executions` |

Returns the `protocol_executions` rows for debugging/observability.

### 6C. Route Registration

**`src/constants.rs`:** Add route constants
**`src/server/mod.rs`:** Register routes in `build_protected_routes`

### 6D. Repository Methods

**`src/db/traits/mod.rs`:**
```rust
// Protocol execution CRUD:
async fn create_protocol_execution(&self, row: ProtocolExecutionRow) -> Result<ProtocolExecutionRow>;
async fn update_protocol_execution_status(&self, id: Uuid, status: &str, ...) -> Result<ProtocolExecutionRow>;
async fn list_protocol_executions_by_step(&self, step_id: Uuid) -> Result<Vec<ProtocolExecutionRow>>;
async fn list_protocol_executions_by_run(&self, run_id: Uuid) -> Result<Vec<ProtocolExecutionRow>>;

// Protocol-scoped doc defs:
async fn list_protocol_document_defs(&self, protocol_id: Uuid) -> Result<Vec<ProtocolDocumentDefRow>>;
async fn create_protocol_document_def(&self, def: ProtocolDocumentDefRow) -> Result<ProtocolDocumentDefRow>;
async fn update_protocol_document_def(&self, id: Uuid, ...) -> Result<ProtocolDocumentDefRow>;
async fn delete_protocol_document_def(&self, id: Uuid) -> Result<()>;
```

---

## Part 7: Frontend

### 7A. Type Updates

**`frontend/src/types/workflow.ts`:**
- `agent_id: string` -> `agent_id: string | null`
- Add `visible: boolean`

### 7B. Canvas Filtering

**`frontend/src/components/canvas/mappers.ts`:**
- Filter `visible === false` steps from canvas rendering
- Existing null guards on `agent_id` should already handle `null`

### 7C. Document Canvas Node

**New component:** `frontend/src/components/canvas/nodes/DocumentNode.tsx`
- Renders document name, description, content preview
- Output port for wiring to downstream steps
- States: blank (pending) -> populated (content written)

### 7D. Documenter Protocol UI

**New component:** `frontend/src/components/panels/DocumenterPanel.tsx`
- Document definition manager (add/remove/edit docs with name, length, description)
- Prompt text area
- No system prompt field (auto-generated)

### 7E. Real-time Document Updates

- WebSocket subscription for `DocumenterPhaseProgress` events
- Canvas document nodes transition blank -> populated during execution

---

## Part 8: Integration Testing

### 8A. Unit Tests Per Component

| Component | Test File | Key Tests |
|-----------|-----------|-----------|
| DocumenterExpander | `protocols/expanders/documenter.rs` | validate, expand, schema gen, prompt gen |
| Schema gen | `protocols/schema_gen.rs` | documenter_schema output shape |
| Prompt gen | `protocols/prompt_gen.rs` | doc listing, capability listing |
| DocumenterStrategyStrategy | `strategies/documenter_strategy/tests.rs` | system_prompt, tools, build_messages |
| DocumenterResearchStrategy | `strategies/documenter_research/tests.rs` | tool resolution, execute_tool |
| DocumenterWriterStrategy | `strategies/documenter_writer/tests.rs` | prompt composition, length guidance |
| DocumenterExecutor | `dag/documenter/tests.rs` | 3-phase flow, partial failure, token accumulation |
| Capability resolver | `capability_resolver/tests.rs` | single/multi capability resolution |

### 8B. Integration Tests

- End-to-end flow with mock LLM provider through all 3 phases
- Apply -> execute cycle with document population verification
- Partial research failure handling
- WebSocket event broadcasting

### 8C. Verification Commands

```bash
cargo check                    # Type check (catches nullable agent_id breakage)
cargo test                     # All tests
cargo clippy                   # Lint
cargo fmt                      # Format

# Frontend
cd frontend && npx tsc --noEmit  # Type check
cd frontend && npx eslint .      # Lint
```

---

## Critical Files Reference

| File | What Changes |
|------|-------------|
| `src/db/mod.rs` | WorkflowStepRow, DocumentRow, ProtocolDocumentDefRow, new ProtocolExecutionRow |
| `src/db/pg_repo/mod.rs` | create_step, update_step, document queries, fix get_tools_by_capability, new CRUD |
| `src/db/traits/mod.rs` | New trait methods for protocol executions + protocol doc defs |
| `src/server/hub/dag/mod.rs` | Add "documenter" dispatch branch, nullable agent_id handling |
| `src/server/hub/dag/utils/mod.rs` | compose_prompt agent_id guard |
| `src/server/hub/dag/documenter/mod.rs` | **NEW** — DocumenterExecutor |
| `src/server/hub/strategies/documenter_*/mod.rs` | **NEW** — 3 strategy types |
| `src/server/hub/capability_resolver/mod.rs` | **NEW** — shared capability->tool resolver |
| `src/server/hub/protocols/expanders/documenter.rs` | **NEW** — DocumenterExpander |
| `src/server/hub/protocols/builtins.rs` | Add 6th builtin |
| `src/server/hub/protocols/schema_gen.rs` | Add documenter_schema() |
| `src/server/hub/protocols/prompt_gen.rs` | Add documenter_prompt() |
| `src/server/api/protocols/mod.rs` | Apply endpoint documenter branch |
| `src/server/api/protocols/documents.rs` | **NEW** — protocol doc def CRUD |
| `src/server/ws/events.rs` | New DocumenterPhaseProgress variant |
| `src/server/mod.rs` | Route registration |
| `src/constants.rs` | New route constants |
| `src/server/hub/error/mod.rs` | AgentNotFound variant |
| `frontend/src/types/workflow.ts` | agent_id nullable, visible field |
