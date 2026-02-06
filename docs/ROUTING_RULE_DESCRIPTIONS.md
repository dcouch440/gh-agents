# Routing Rule Descriptions & Downstream Context Injection

**Status:** Planned for Phase 6
**Parent Plan:** [UNIFIED_WORKFLOW_SYSTEM.md](./UNIFIED_WORKFLOW_SYSTEM.md)
**Dependencies:** Phase 1-5 complete (migrations 067-071 applied)

---

## Overview

Enable routing rules to include human-authored descriptions that automatically guide upstream planner agents via prompt injection.

**Key Insight:** Routing rule descriptions become part of the prompt that guides planner agents. They explain what each label means, bridging the gap between "fuzzy LLM label generation" and "strict string matching in routing."

## Problem Statement

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
- The workflow creator's original prompt stays intact — this block is **appended** by the system

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

## Implementation Plan

### Phase 6: Routing Rule Descriptions (2-3 days)

**Timeline:** After Phase 5 (Port-Based DAG Executor) is complete

### Migration 073: Add Description Column

**File:** `/migrations/073_routing_rule_descriptions.sql`

```sql
-- ============================================================================
-- Migration 073: Routing Rule Descriptions
-- ============================================================================

-- Add description column to existing step_routing_rules table
ALTER TABLE step_routing_rules
    ADD COLUMN description TEXT;

CREATE INDEX idx_step_routing_rules_description ON step_routing_rules(description)
    WHERE description IS NOT NULL;

COMMENT ON COLUMN step_routing_rules.description IS
    'Human-authored description of what this routing category handles.
     Used to auto-generate routing instructions in upstream planner prompts.
     Example: "Front-end tasks: UI components, client-side logic, styling"
     These descriptions become part of the prompt that guides the planner agent.';
```

**Why not in Migration 067?**
Phase 1 migrations (067-071) are already complete. This feature builds on top of the existing routing system.

**Verification:**
```bash
docker exec gh-agents-postgres-1 psql -U nexor -d nexor -c "\d step_routing_rules"
# Should show description column (nullable TEXT)
```

---

### Type Definitions

**Extend** `StepRoutingRuleRow` in `/src/types/workflow.rs`:

```rust
pub struct StepRoutingRuleRow {
    pub id: Uuid,
    pub workflow_step_id: Uuid,
    pub label_value: String,
    pub description: Option<String>,  // NEW
    pub agent_id: Uuid,
    pub display_order: i32,
    pub created_at: DateTime<Utc>,
}
```

**New types** for prompt injection in `/src/types/workflow.rs`:

```rust
/// Context about a downstream label-routing step, used to inject
/// routing instructions into the upstream planner's prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownstreamRoutingContext {
    pub downstream_step_id: Uuid,
    pub routing_field: String,           // e.g. "category"
    pub routes: Vec<RouteDescription>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDescription {
    pub label_value: String,             // "frontend"
    pub description: Option<String>,     // "Front-end tasks: UI components..."
    pub agent_name: String,              // "Frontend Specialist"
    pub agent_description: Option<String>,
    pub agent_tools: Vec<String>,        // ["file_write", "file_read", ...]
}
```

---

### Database Queries

**Add to** `/src/db/queries/workflows.rs`:

```rust
/// For a given step, find any downstream steps (via edges) that use
/// label routing, and return their routing rules with agent details.
pub async fn query_downstream_routing_context(
    step_id: Uuid,
    pool: &PgPool,
) -> Result<Vec<DownstreamRoutingContext>> {
    // 1. Find downstream steps with routing_mode = "label"
    // 2. For each, get routing rules with descriptions
    // 3. Join to agents table to get agent details
    // 4. Join to agent_tools to get tool list
    // 5. Group by downstream step
}
```

**SQL sketch:**

```sql
-- Find downstream label-routing steps
SELECT
    ws.id AS downstream_step_id,
    ws.routing_field,
    sr.label_value,
    sr.description,
    sr.display_order,
    a.id AS agent_id,
    a.name AS agent_name,
    a.description AS agent_description
FROM workflow_step_edges e
JOIN workflow_steps ws ON ws.id = e.to_step_id
JOIN step_routing_rules sr ON sr.workflow_step_id = ws.id
JOIN agents a ON a.id = sr.agent_id
WHERE e.from_step_id = $1
  AND ws.routing_mode = 'label'
ORDER BY ws.id, sr.display_order;

-- Then for each agent, get tools
SELECT t.name
FROM agent_tools at
JOIN tools t ON t.id = at.tool_id
WHERE at.agent_id = $1;
```

---

### Core Implementation: Prompt Injection

**Where it lives:** `/src/server/executors/dag/mod.rs` — in the prompt building path, after port-based inputs are resolved but before the LLM call.

**Modify existing prompt building:**

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
    if !inputs.is_empty() {
        prompt.push_str("\n\n## Inputs\n\n");
        prompt.push_str(&serde_json::to_string_pretty(inputs)?);
    }

    // 3. Downstream routing context (NEW — Phase 6)
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

        block.push_str("\n");

        // Agent context — what the planner is routing TO
        let tools_str = route.agent_tools.join(", ");
        block.push_str(&format!(
            "  Routed to: {} (tools: {})\n\n",
            route.agent_name, tools_str
        ));
    }

    block
}
```

**Where to hook it in:**
- Look for where `execute_step()` or similar builds the user prompt
- This is after port resolution but before calling the LLM
- Inject routing context at that point

---

### API Endpoints

**Extend routing rule CRUD** in `/src/server/api/workflow_ports.rs`:

```rust
// POST /api/steps/{id}/routing-rules
#[derive(Deserialize)]
struct CreateRoutingRule {
    label_value: String,
    description: Option<String>,  // NEW
    agent_id: Uuid,
    display_order: Option<i32>,
}

// PUT /api/routing-rules/{id}
#[derive(Deserialize)]
struct UpdateRoutingRule {
    description: Option<String>,  // NEW
    agent_id: Option<Uuid>,
    display_order: Option<i32>,
}

// New endpoint for preview
// GET /api/steps/{id}/downstream-routing-context
async fn get_downstream_routing_context(
    Path(step_id): Path<Uuid>,
    Extension(state): Extension<AppState>,
) -> Result<Json<DownstreamRoutingContextResponse>> {
    let contexts = state.repo()
        .query_downstream_routing_context(step_id)
        .await?;

    Ok(Json(DownstreamRoutingContextResponse { contexts }))
}
```

**Response format:**
```json
{
  "contexts": [
    {
      "downstream_step_id": "uuid",
      "routing_field": "category",
      "routes": [
        {
          "label_value": "frontend",
          "description": "Front-end tasks: UI components...",
          "agent_name": "Frontend Specialist",
          "agent_tools": ["file_write", "file_read"]
        }
      ]
    }
  ]
}
```

This endpoint is useful for the UI — when editing a planner step, the UI can preview the auto-injected prompt block.

---

### UI Design

**Routing rule creation modal** (enhanced):

```
┌──────────────────────────────────────────────────────┐
│  Assign agents to categories                        │
│                                                      │
│  frontend   → [Frontend Specialist ▾]               │
│  Description: [Front-end tasks: UI components,_____ │
│                client-side logic, styling__________ ]│
│                                                      │
│  backend    → [Backend Specialist ▾]                │
│  Description: [Server-side tasks: API endpoints,___ │
│                business logic, auth________________ ]│
│                                                      │
│  database   → [Database Specialist ▾]               │
│  Description: [Data layer tasks: schema design,____ │
│                migrations, queries_________________ ]│
│                                                      │
│  testing    → [QA Specialist ▾]                     │
│  Description: [Quality tasks: test planning,_______ │
│                coverage, QA strategy_______________ ]│
│                                                      │
│  Fallback (for unmatched categories):               │
│  [General Implementation Agent ▾]                   │
│                                                      │
│  💡 These descriptions guide the upstream planner   │
│     on which category to choose for each item       │
│                                                      │
│  [Preview Injected Prompt] [Create Node]            │
└──────────────────────────────────────────────────────┘
```

**Key UI features:**
- Multi-line text input for descriptions (expandable textarea)
- Descriptions are optional but recommended
- Preview button shows what the planner will see
- Tooltip explains: "This text will be added to the upstream planner's prompt"

**Preview modal** (when user clicks "Preview Injected Prompt"):

```
┌─────────────────────────────────────────────────┐
│  Preview: Auto-Injected Routing Instructions   │
│                                                 │
│  This will be appended to the planner's prompt:│
│                                                 │
│  ┌───────────────────────────────────────────┐ │
│  │ ## Routing Instructions                   │ │
│  │                                           │ │
│  │ Each item MUST include a "category"      │ │
│  │ field set to exactly one of:             │ │
│  │                                           │ │
│  │ - frontend: Front-end tasks — UI         │ │
│  │   components, client-side logic          │ │
│  │   Routed to: Frontend Specialist         │ │
│  │   (tools: file_write, file_read)         │ │
│  │                                           │ │
│  │ - backend: Server-side tasks — API       │ │
│  │   endpoints, business logic              │ │
│  │   Routed to: Backend Specialist          │ │
│  │   (tools: file_write, test_execution)    │ │
│  │                                           │ │
│  │ ...                                       │ │
│  └───────────────────────────────────────────┘ │
│                                                 │
│  [Close]                                        │
└─────────────────────────────────────────────────┘
```

**Properties panel** (when editing an existing routing node):

```
┌─────────────────────────────────────────────┐
│  Properties: Process Milestones            │
├─────────────────────────────────────────────┤
│  Execution Mode: For Each                  │
│  Parallel: Yes                             │
│  Routing: By Label                         │
│                                            │
│  Routing Field: category                   │
│                                            │
│  Routing Rules:                            │
│  ┌─────────────────────────────────────┐  │
│  │ frontend  → Frontend Spec       [×] │  │
│  │ "Front-end tasks: UI components..." │  │
│  │ [Edit Description]                  │  │
│  └─────────────────────────────────────┘  │
│  ┌─────────────────────────────────────┐  │
│  │ backend   → Backend Spec        [×] │  │
│  │ "Server-side tasks: API..."         │  │
│  │ [Edit Description]                  │  │
│  └─────────────────────────────────────┘  │
│  [+ Add Rule]                              │
│                                            │
│  Fallback Agent:                           │
│  [General Implementation ▾]                │
│                                            │
│  [Preview Prompt Injection]                │
└─────────────────────────────────────────────┘
```

**Interaction:**
- Click routing rule to expand and edit description inline
- Description truncated with "..." in collapsed view
- [Preview Prompt Injection] fetches from `/api/steps/{id}/downstream-routing-context` and displays

---

### Testing

**1. Prompt Injection Test**

Create a two-step workflow:
- Step 1: Planner (outputs `milestones` array)
- Step 2: For-each with label routing on `category` field

Routing rules on Step 2:

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

**Test code:**
```rust
#[tokio::test]
async fn test_downstream_routing_context_injection() {
    let pool = setup_test_db().await;

    // Create workflow with planner → router
    let workflow_id = create_test_workflow(&pool).await;
    let step1 = create_planner_step(&pool, workflow_id).await;
    let step2 = create_label_routing_step(&pool, workflow_id).await;
    create_edge(&pool, step1.id, step2.id).await;

    // Add routing rules with descriptions
    create_routing_rule(&pool, step2.id, "frontend",
        "UI components and client-side logic", frontend_agent_id).await;
    create_routing_rule(&pool, step2.id, "backend",
        "API endpoints and server logic", backend_agent_id).await;

    // Build prompt for step 1
    let prompt = build_step_prompt(&step1, &HashMap::new(), &pool).await.unwrap();

    // Verify routing instructions are included
    assert!(prompt.contains("## Routing Instructions"));
    assert!(prompt.contains("frontend: UI components"));
    assert!(prompt.contains("Routed to: Frontend Specialist"));
}
```

**2. Schema Validation Test**

- Output port on Step 1 has `json_schema` with `category` enum
- Verify enum values match routing rule `label_value`s exactly
- Test: add a routing rule for `"devops"` → verify schema enum updates or warns

**3. End-to-End Test**

- Execute the workflow
- Verify planner outputs only valid label values
- Verify routing dispatches to correct agents
- Verify no items hit the fallback agent (unless intentionally unlabeled)

**4. Edge Cases**

- Step with no downstream label routing → no injection (existing behavior)
- Step with multiple downstream label-routing steps → inject context for each
- Routing rule with no description → label still appears, just without description text
- Agent with no tools assigned → show "(no tools)" or omit tools line

---

## Benefits Summary

**Three-layer reinforcement:**

1. **Prompt injection (soft)** - LLM knows what labels exist and what they mean
2. **Schema validation (hard)** - Enum constraint catches invalid labels
3. **Routing dispatch (existing)** - Fallback agent is safety net

**Single source of truth:** Routing rules table defines labels, descriptions, and agents. Prompt text, schema enum, and routing dispatch all derive from this - no manual sync needed.

**Developer experience:** Workflow designers author routing semantics once (in routing rules), and they automatically become prompt semantics that guide the planner.

**Reduced errors:** Eliminates the silent failure mode where items hit the fallback agent because the planner didn't know what labels were valid.

---

## Implementation Checklist

- [ ] **Migration 073** - Add `description` column to `step_routing_rules`
- [ ] **Types** - Add `DownstreamRoutingContext` and `RouteDescription`
- [ ] **Queries** - Implement `query_downstream_routing_context()`
- [ ] **Prompt injection** - Add `build_routing_instruction_block()` and hook into prompt building
- [ ] **API endpoints** - Extend routing rule CRUD to include `description`
- [ ] **GET endpoint** - Add `/api/steps/{id}/downstream-routing-context` for UI preview
- [ ] **Tests** - Prompt injection test, end-to-end test, edge cases
- [ ] **UI components** - Description fields in routing rule modals (future)
- [ ] **UI preview** - Preview modal showing injected prompt (future)
- [ ] **Documentation** - Update main plan to reference this feature

---

## Critical Files

**New:**
- `/migrations/073_routing_rule_descriptions.sql`

**Modified:**
- `/src/types/workflow.rs` - Add DownstreamRoutingContext types
- `/src/db/queries/workflows.rs` - Add query_downstream_routing_context()
- `/src/server/executors/dag/mod.rs` - Add prompt injection logic
- `/src/server/api/workflow_ports.rs` - Extend routing rule endpoints

**Future (UI):**
- Frontend routing rule creation modal (add description fields)
- Frontend preview modal (show injected prompt)
- Frontend properties panel (edit descriptions)

---

## Related Documents

- **[UNIFIED_WORKFLOW_SYSTEM.md](./UNIFIED_WORKFLOW_SYSTEM.md)** - Parent plan with full system architecture
- **[PHASE_5B_DOWNSTREAM_ROUTING_CONTEXT.md](./PHASE_5B_DOWNSTREAM_ROUTING_CONTEXT.md)** - Original detailed design document (this extracts the implementation plan from that)

---

**Last Updated:** 2025-02-05
**Status:** Ready for implementation after Phase 5 complete
