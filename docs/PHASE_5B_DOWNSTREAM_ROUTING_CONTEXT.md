# Phase 5b: Downstream Routing Context Injection

## Sub-phase of Phase 5 (Port-Based DAG Executor)

**Parent Document:** [UNIFIED_WORKFLOW_SYSTEM.md](./UNIFIED_WORKFLOW_SYSTEM.md)

---

## Problem

The label routing system (Tier 2) has a critical gap: the planner agent that generates labeled data has **zero visibility into what's downstream**. It doesn't know:

- What label values the routing rules expect
- What each label means
- Which specialist agent handles each label
- What tools or capabilities that agent has

The planner is told via a hand-written prompt to "use one of: frontend, backend, database, testing" — but nothing enforces sync between that prompt text and the actual `step_routing_rules` table. If someone adds a `"devops"` routing rule but forgets to update the prompt, items that should route to devops hit the fallback agent silently.

The LLM generating labels is fuzzy. The routing is a strict string match. That mismatch needs a bridge.

## Solution

**Downstream Routing Context Injection** — when building the prompt for any step whose output feeds into a label-routing step, the system automatically:

1. Looks at outgoing edges to find downstream steps with `routing_mode = "label"`
2. Queries that step's `step_routing_rules` (now with descriptions)
3. Fetches the agent behind each rule (name, brief description, tools)
4. Injects a structured routing instruction block into the planner's prompt
5. Derives the output port's enum constraint from the routing rule labels

The planner agent doesn't need to be told what labels to use — **the system tells it**, derived from the actual routing configuration. One source of truth.

---

## What the Planner Sees

Given a downstream step with these routing rules:

| label_value | description | agent |
|-------------|-------------|-------|
| `frontend` | Front-end tasks: UI components, client-side logic, styling | Frontend Specialist |
| `backend` | Server-side tasks: API endpoints, business logic, auth | Backend Specialist |
| `database` | Data layer tasks: schema design, migrations, queries | Database Specialist |
| `testing` | Quality tasks: test planning, coverage, QA strategy | QA Specialist |

The system auto-injects into the planner's prompt:

```
## Routing Instructions

Each milestone MUST include a "category" field set to exactly one of the following values.
Pick the single best match for each milestone. Do not use any other values.

- frontend: Front-end tasks — UI components, client-side logic, styling.
  Routed to: Frontend Specialist (tools: file_write, file_read, component_generator)

- backend: Server-side tasks — API endpoints, business logic, auth.
  Routed to: Backend Specialist (tools: file_write, file_read, test_execution, shell_execution)

- database: Data layer tasks — schema design, migrations, queries.
  Routed to: Database Specialist (tools: database_query, file_write, file_read)

- testing: Quality tasks — test planning, coverage, QA strategy.
  Routed to: QA Specialist (tools: test_execution, file_read, shell_execution)

Prepare milestones by writing a comprehensive plan after reading the PRD.
Assign each milestone to exactly one category above based on what the milestone primarily requires.
```

**Key properties:**
- The label values come from `step_routing_rules.label_value` — exact match guaranteed
- The descriptions come from `step_routing_rules.description` — human-authored context
- The agent info comes from `agents.name` + the agent's assigned tools — the planner knows what each route can do
- The workflow creator's original prompt (`"Analyze the PRD and create milestones..."`) stays intact — this block is **appended** by the system

---

## Three Reinforcement Layers

This creates three layers ensuring labels are correct:

```
Layer 1: Prompt Injection (soft)
    The LLM is told what labels exist, what they mean, and what agents handle them.
    It can make an informed choice.
        ↓
Layer 2: Output Schema Validation (hard)
    The output port's json_schema has an auto-derived enum constraint:
    {"category": {"type": "string", "enum": ["frontend", "backend", "database", "testing"]}}
    Invalid labels are caught before routing.
        ↓
Layer 3: Routing Dispatch + Fallback (hard)
    step_routing_rules does the hashmap lookup.
    Unmatched labels go to the fallback agent.
    This is the existing behavior — unchanged.
```

Without this feature, only Layer 3 exists, and it fails silently (fallback agent gets work it wasn't designed for). With this feature, Layer 1 prevents most mismatches, Layer 2 catches the rest, and Layer 3 is a safety net that rarely fires.

---

## Changes Required Per Phase

### Phase 1 Addition: Schema Change

**Migration 067** (`step_routing_rules` table) — add `description` column:

```sql
CREATE TABLE step_routing_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_step_id UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    label_value TEXT NOT NULL,
    description TEXT,                    -- NEW: human-authored context for this route
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    display_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(workflow_step_id, label_value)
);
```

Single column addition. No migration complexity.

### Phase 2 Addition: Type Definitions

**Extend `StepRoutingRuleRow`** (`src/types/workflow.rs`):

```rust
pub struct StepRoutingRuleRow {
    pub id: Uuid,
    pub workflow_step_id: Uuid,
    pub label_value: String,
    pub description: Option<String>,     // NEW
    pub agent_id: Uuid,
    pub display_order: i32,
    pub created_at: DateTime<Utc>,
}
```

**New type — `DownstreamRoutingContext`**:

```rust
/// Context about a downstream label-routing step, used to inject
/// routing instructions into the upstream planner's prompt.
pub struct DownstreamRoutingContext {
    pub downstream_step_id: Uuid,
    pub routing_field: String,           // e.g. "category"
    pub routes: Vec<RouteDescription>,
}

pub struct RouteDescription {
    pub label_value: String,             // "frontend"
    pub description: Option<String>,     // "Front-end tasks: UI components..."
    pub agent_name: String,              // "Frontend Specialist"
    pub agent_description: Option<String>,
    pub agent_tools: Vec<String>,        // ["file_write", "file_read", ...]
}
```

### Phase 3 Addition: Database Query

**New query** in `src/db/queries/workflows.rs`:

```rust
/// For a given step, find any downstream steps (via edges) that use
/// label routing, and return their routing rules with agent details.
async fn query_downstream_routing_context(
    step_id: Uuid,
    pool: &PgPool,
) -> Result<Vec<DownstreamRoutingContext>>
```

**SQL sketch:**

```sql
SELECT
    sr.label_value,
    sr.description,
    a.name AS agent_name,
    a.description AS agent_description,
    ws.routing_field,
    ws.id AS downstream_step_id
FROM workflow_step_edges e
JOIN workflow_steps ws ON ws.id = e.to_step_id
JOIN step_routing_rules sr ON sr.workflow_step_id = ws.id
JOIN agents a ON a.id = sr.agent_id
WHERE e.from_step_id = $1
  AND ws.routing_mode = 'label'
ORDER BY sr.display_order;
```

Then a second query (or join) to get each agent's tools:

```sql
SELECT t.name
FROM agent_tools at
JOIN tools t ON t.id = at.tool_id
WHERE at.agent_id = $1;
```

### Phase 5b: Prompt Injection Logic (THE CORE)

**Where it lives:** `src/server/executors/dag/mod.rs` — in the prompt building path, after port-based inputs are resolved but before the LLM call.

**Pseudocode:**

```rust
async fn build_step_prompt(
    step: &WorkflowStepRow,
    inputs: &HashMap<String, JsonValue>,
    pool: &PgPool,
) -> Result<String> {
    let mut prompt = String::new();

    // 1. User-authored prompt (existing)
    prompt.push_str(&step.prompt);

    // 2. Port-based inputs (Phase 5 existing)
    prompt.push_str(&format_inputs_as_json(inputs));

    // 3. Downstream routing context (NEW — Phase 5b)
    let downstream_contexts = query_downstream_routing_context(step.id, pool).await?;

    for ctx in &downstream_contexts {
        prompt.push_str(&build_routing_instruction_block(ctx));
    }

    Ok(prompt)
}

fn build_routing_instruction_block(ctx: &DownstreamRoutingContext) -> String {
    let mut block = String::new();

    block.push_str("\n\n## Routing Instructions\n\n");
    block.push_str(&format!(
        "Each item MUST include a \"{}\" field set to exactly one of the following values.\n",
        ctx.routing_field
    ));
    block.push_str("Pick the single best match for each item. Do not use any other values.\n\n");

    for route in &ctx.routes {
        block.push_str(&format!("- {}", route.label_value));

        if let Some(desc) = &route.description {
            block.push_str(&format!(": {}", desc));
        }

        // Agent context — what the planner is routing TO
        let tools_str = route.agent_tools.join(", ");
        block.push_str(&format!(
            "\n  Routed to: {} (tools: {})\n",
            route.agent_name, tools_str
        ));
    }

    block
}
```

### Phase 5b Addition: Output Schema Enum Derivation

When a step has a downstream label-routing connection, the system can **auto-derive** the enum constraint for the routing field in the output port's `json_schema`:

```rust
/// After downstream routing context is resolved, verify or inject
/// the enum constraint into the output port's schema.
fn ensure_routing_enum_in_schema(
    output_port: &mut StepOutputRow,
    ctx: &DownstreamRoutingContext,
) {
    let valid_labels: Vec<String> = ctx.routes.iter()
        .map(|r| r.label_value.clone())
        .collect();

    // If the port has a json_schema, verify the routing field has matching enum
    // If not, inject it
    // This is validation — warn or error if schema enum doesn't match routing rules
}
```

This is **defensive** — the prompt injection (Layer 1) should handle most cases, but schema validation (Layer 2) catches the rest.

### Phase 8 Addition: API Changes

**Extend routing rule CRUD** to include `description`:

```
POST /api/steps/{id}/routing-rules
Body: { "label_value": "frontend", "description": "Front-end tasks: UI...", "agent_id": "uuid" }

PUT /api/routing-rules/{id}
Body: { "description": "Updated description..." }
```

**New read endpoint** — get downstream routing context for a step:

```
GET /api/steps/{id}/downstream-routing-context
Response: {
    "contexts": [
        {
            "downstream_step_id": "uuid",
            "routing_field": "category",
            "routes": [
                {
                    "label_value": "frontend",
                    "description": "Front-end tasks...",
                    "agent_name": "Frontend Specialist",
                    "agent_tools": ["file_write", "file_read"]
                }
            ]
        }
    ]
}
```

This endpoint is useful for the UI — when editing a planner step, the UI can show what routing rules exist downstream and preview the auto-injected prompt block.

---

## Where This Fits in the Roadmap

```
Phase 1: Foundation (schema)
    └── Add `description` column to step_routing_rules in migration 067
         (one line, no extra time)

Phase 2: Type Definitions
    └── Add DownstreamRoutingContext, RouteDescription types
         Add description field to StepRoutingRuleRow
         (+0.5 days)

Phase 3: Database Queries
    └── Add query_downstream_routing_context()
         Join across edges → steps → routing_rules → agents → agent_tools
         (+0.5 days)

Phase 5: Port-Based DAG Executor
    └── Phase 5a: Core port resolution, envelope wrapping, label routing dispatch
                   (existing 4-5 days)
    └── Phase 5b: Downstream routing context injection    ← THIS SUB-PHASE
                   - build_routing_instruction_block()
                   - Inject into prompt building path
                   - Output schema enum derivation/validation
                   (+1-2 days)

Phase 8: API Endpoints
    └── Extend routing rule CRUD with description field
         Add GET /steps/{id}/downstream-routing-context
         (+0.5 days)

Phase 9: Integration Testing
    └── Test case: planner step → label-routing step
         Verify prompt contains routing instructions
         Verify schema enum matches routing rules
         Verify mismatched labels are caught
         (+0.5 days)
```

**Total added time: 3-4 days spread across existing phases, with Phase 5b as the concentrated sub-phase (1-2 days).**

---

## Verification Plan

### 1. Prompt Injection Verification

Create a two-step workflow:
- Step 1: Planner (outputs `milestones` array)
- Step 2: For-each with label routing on `category` field

**Routing rules on Step 2:**
| label | description | agent |
|-------|-------------|-------|
| frontend | UI components and client-side logic | Frontend Specialist |
| backend | API endpoints and server logic | Backend Specialist |

**Verify Step 1's prompt includes:**
```
## Routing Instructions

Each item MUST include a "category" field set to exactly one of the following values.

- frontend: UI components and client-side logic
  Routed to: Frontend Specialist (tools: file_write, file_read)

- backend: API endpoints and server logic
  Routed to: Backend Specialist (tools: file_write, test_execution)
```

### 2. Schema Validation Verification

- Output port on Step 1 has `json_schema` with `category` enum
- Verify enum values match routing rule `label_value`s exactly
- Test: add a routing rule for `"devops"` → verify schema enum updates or warns

### 3. End-to-End Verification

- Execute the workflow
- Verify planner outputs only valid label values
- Verify routing dispatches to correct agents
- Verify no items hit the fallback agent (unless intentionally unlabeled)

### 4. Edge Cases

- Step with no downstream label routing → no injection (existing behavior)
- Step with multiple downstream label-routing steps → inject context for each
- Routing rule with no description → label still appears, just without description text
- Agent with no tools assigned → show "(no tools)" or omit tools line

---

## Design Decisions

1. **Append, don't replace** — The routing instruction block is appended after the user's prompt, not injected into the middle. The user's creative prompt stays intact. System concerns go at the end.

2. **Description is optional** — `step_routing_rules.description` is nullable. If omitted, the label still appears in the prompt but without explanation. Descriptions are strongly encouraged but not enforced.

3. **Agent context is read-only** — The planner sees agent names and tools but can't modify them. This is informational context to help the LLM make better routing decisions.

4. **Schema derivation is defensive** — The system warns if the output schema enum doesn't match routing rules, but doesn't block execution. The prompt injection is the primary mechanism; schema validation is a safety net.

5. **One source of truth** — The routing rules table is the single source. The prompt text, schema enum, and routing dispatch all derive from it. No manual sync needed.
