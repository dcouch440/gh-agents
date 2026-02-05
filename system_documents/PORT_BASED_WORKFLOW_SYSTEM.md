# Plan: Port-Based Workflow System with Label Routing

## Executive Summary

**Goal:** Transform nexor from a variable-based workflow system into a visual-first, port-based pipeline builder with semantic agent routing.

**Scope:** Backend data model, execution engine, and API. UI vision documented for future implementation.

**Strategy:** Clean break from variable system. Refactor existing DAG executor with port-based data flow, consistent output envelopes, and label-based routing for dynamic multi-agent execution.

**Key Innovation:** Label-based routing enables dynamic array sizes (4-8 items) with semantic agent assignment (frontend → Frontend Specialist, backend → Backend Specialist) - no fixed slot configuration required.

**Application Status:** Has not run in production. No backwards compatibility concerns. Clean slate implementation.

---

## System Overview

### High-Level Architecture

```
User Request
    ↓
Workflow Definition (nodes + edges + ports)
    ↓
DAG Executor (topological execution)
    ↓
Step Execution (port-based inputs)
    ├─ Single Agent Execution
    ├─ For-Each Sequential
    ├─ For-Each Parallel (same agent)
    ├─ For-Each Label Routing (specialist agents)
    └─ Interactive Review Room (human-in-loop)
    ↓
Output Envelope (status, data, metadata, error)
    ↓
Next Step (reads from upstream ports)
```

### Core Concepts

**1. Ports** - Explicit input/output definitions on steps
- Steps declare what they produce (output ports)
- Steps declare what they need (input ports)
- Edges connect output port → input port

**2. Envelopes** - Consistent wrapper for all outputs
```json
{
  "status": "success" | "error" | "partial",
  "data": <actual output>,
  "metadata": {execution_id, timing, cost, agent_id, routing_label},
  "error": <error details if failed>
}
```

**3. Label Routing** - Semantic agent assignment for arrays
- Array items declare category/type field
- Routing rules map category → specialist agent
- Dynamic size support (4 items or 8 items)
- Fallback agent for unmatched categories

**4. Review Rooms** - Interactive human checkpoints
- Workflow pauses at review step
- Agent joins room with full context (via input ports)
- Human discusses, approves, or requests changes
- Agent outputs decision, workflow resumes

---

## Migration Strategy: Clean Break

**Decision:** Remove variable system entirely. Application has not run in production - no backwards compatibility needed.

### What Gets Removed

**Database:**
- `execution_variables` table (drop completely)
- `output_variable_name` column from `workflow_steps` (deprecated, can remove)

**Code:**
- Variable interpolation in prompts: `{variable_name}` → removed
- `resolve_variable()` functions
- Variable storage/retrieval logic
- `ExecutionVariableRow` types

**Benefits:**
- Simpler codebase
- One data flow model
- No technical debt
- Cleaner mental model

### What Gets Refactored

**DAG Executor (`src/server/executors/dag/mod.rs`):**
- Input resolution: Variables → Port connections
- Output handling: Raw values → Envelopes
- For-each execution: Add label routing mode
- Error handling: Silent failures → Preserved in envelopes

**Collection Executor (`src/server/executors/collection_dag/mod.rs`):**
- Update to work with envelope outputs
- No major logic changes

**Unchanged:**
- Room executor (`src/server/executors/room/mod.rs`) - Used by review steps
- Chat executor (`src/server/executors/chat/mod.rs`) - Different use case
- Topological sort logic - Core algorithm stays
- Edge traversal - Same DAG structure

---

## Overview (Detailed)

Complete redesign of workflow execution with:
1. **Port-based data flow** - Direct output-to-input wiring, no variable abstraction
2. **Consistent output envelopes** - All executions return standard structure
3. **Label-based routing** - Dynamic arrays route to specialist agents by category
4. **Automatic envelope unwrapping** - System reads `.data` field automatically
5. **Interactive review rooms** - Human-in-loop with agent conversation
6. **Proper error tracking** - Failed iterations preserved in aggregate outputs

## Current State Analysis

### What Already Exists
- ✅ DAG execution via `workflow_steps` + `workflow_step_edges`
- ✅ Multi-tier DAGs via `workflow_collections`
- ✅ For-each iteration support (`execution_mode: "for_each"`)
- ✅ Output storage in `agent_executions.structured_output` (JSONB)
- ✅ Human-in-loop via interactive steps
- ✅ Multi-agent collaboration via rooms

### Current Limitations
- ❌ No visual positioning data for canvas-based UI
- ❌ Inconsistent output format - varies by LLM response
- ❌ No explicit port definitions for visual wiring
- ❌ **Critical Bug:** For-each iterations fail silently - errors are logged but not tracked
- ❌ No per-iteration metadata (index, label, timing)
- ❌ Variable system (`execution_variables`) adds unnecessary abstraction
- ❌ No standard error structure in outputs

### Key Files
- `/Users/davidcouch/Dev/gh-agents/src/server/executors/dag/mod.rs` - DAG execution engine
- `/Users/davidcouch/Dev/gh-agents/migrations/035_create_workflows.sql` - Workflow schema
- `/Users/davidcouch/Dev/gh-agents/migrations/037_create_agent_executions.sql` - Execution storage

## Design: Standard Output Envelope

### Single Step Execution Output

All step executions will produce a consistent envelope structure:

```json
{
  "status": "success" | "error",
  "data": {
    // Actual step output (LLM response parsed as JSON)
    "sections": ["intro", "features"],
    "requirements": [...]
  },
  "metadata": {
    "execution_id": "uuid",
    "execution_time_ms": 1234,
    "tokens_in": 100,
    "tokens_out": 200,
    "cost_usd": 0.05,
    "model": "claude-opus-4"
  },
  "error": null
}
```

### For-Each Aggregated Output

When `execution_mode: "for_each"`, aggregate all iteration envelopes:

```json
{
  "status": "success" | "partial" | "error",
  "data": [
    {
      "status": "success",
      "data": {/* iteration 0 result */},
      "metadata": {
        "execution_id": "uuid-0",
        "iteration_index": 0,
        "iteration_label": "Feature A",
        "execution_time_ms": 800,
        "tokens_in": 50,
        "tokens_out": 100,
        "cost_usd": 0.02
      },
      "error": null
    },
    {
      "status": "error",
      "data": null,
      "metadata": {
        "execution_id": "uuid-1",
        "iteration_index": 1,
        "iteration_label": "Feature B",
        "execution_time_ms": 200
      },
      "error": {
        "message": "Rate limit exceeded",
        "type": "RateLimitError",
        "retryable": true
      }
    }
  ],
  "metadata": {
    "total_iterations": 2,
    "successful_iterations": 1,
    "failed_iterations": 1,
    "execution_time_ms": 1000,
    "total_tokens_in": 50,
    "total_tokens_out": 100,
    "total_cost_usd": 0.02
  },
  "errors": [
    {
      "iteration_index": 1,
      "iteration_label": "Feature B",
      "message": "Rate limit exceeded",
      "type": "RateLimitError"
    }
  ]
}
```

### Benefits

1. **Consistent Access Pattern** - Always read from `.data` field
2. **Error Tracking** - Failed iterations preserved with error details
3. **Partial Success** - `status: "partial"` when some iterations succeed
4. **Metadata Per Step** - Timing, cost, token usage always available
5. **Array Mapping** - `data[*].data` for iteration results, `data[*].metadata.iteration_index` for ordering

## Design: Port-Based Data Flow

### Schema Changes

```sql
-- 1. Add canvas positioning + routing config to workflow_steps
ALTER TABLE workflow_steps
  ADD COLUMN position_x FLOAT,
  ADD COLUMN position_y FLOAT,
  ADD COLUMN width FLOAT DEFAULT 200,
  ADD COLUMN height FLOAT DEFAULT 100,
  ADD COLUMN routing_mode TEXT,              -- NULL (same agent), "label" (route by field)
  ADD COLUMN routing_field TEXT;             -- For routing_mode="label": which field to read (e.g., "category")

-- 2. Define output ports (what this step produces)
CREATE TABLE step_outputs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workflow_step_id UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
  port_name TEXT NOT NULL,              -- "result", "items", "count"
  port_type TEXT NOT NULL,              -- "string", "array", "object", "number"
  json_path TEXT NOT NULL,              -- Path in .data: "sections", "requirements"
  description TEXT,
  UNIQUE(workflow_step_id, port_name)
);

-- 3. Define input ports (what this step expects)
CREATE TABLE step_inputs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workflow_step_id UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
  port_name TEXT NOT NULL,              -- "sections", "tasks", "config"
  port_type TEXT NOT NULL,
  required BOOLEAN NOT NULL DEFAULT false,
  default_value JSONB,
  description TEXT,
  UNIQUE(workflow_step_id, port_name)
);

-- 4. Enhance edges to connect ports directly
ALTER TABLE workflow_step_edges
  DROP CONSTRAINT IF EXISTS workflow_step_edges_pkey;

ALTER TABLE workflow_step_edges
  ADD COLUMN id UUID DEFAULT gen_random_uuid(),
  ADD COLUMN from_output_port TEXT,     -- "result" from upstream step
  ADD COLUMN to_input_port TEXT,        -- "data" on downstream step
  ADD COLUMN transform_jsonpath TEXT,   -- Optional: "$.items[*].name"
  ADD COLUMN condition_type TEXT,       -- NULL, "if_true", "if_false", "if_equals"
  ADD COLUMN condition_value JSONB,
  ADD COLUMN edge_label TEXT;           -- Visual label for UI

ALTER TABLE workflow_step_edges
  ADD CONSTRAINT workflow_step_edges_pkey PRIMARY KEY (id);

-- 5. Keep unique constraint on workflow + from + to
ALTER TABLE workflow_step_edges
  ADD CONSTRAINT workflow_step_edges_workflow_from_to_unique
    UNIQUE(workflow_id, from_step_id, to_step_id);

-- 6. Routing rules for label-based agent assignment
CREATE TABLE step_routing_rules (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workflow_step_id UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
  label_value TEXT NOT NULL,                -- "frontend", "backend", "database", "testing"
  agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  display_order INTEGER NOT NULL DEFAULT 0,  -- UI ordering
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE(workflow_step_id, label_value)
);

CREATE INDEX idx_step_routing_rules_step ON step_routing_rules(workflow_step_id);
```

## Design: For-Each Parallelization Modes

### Three Execution Strategies

**1. Sequential**
```sql
execution_mode: "for_each"
agent_execution_mode: "sequential"
```
- One agent processes array elements one-by-one
- Total time: `N * avg_item_time`
- Use case: Order matters, or expensive agent config

**2. Parallel (Same Agent)**
```sql
execution_mode: "for_each"
agent_execution_mode: "parallel"
routing_mode: NULL  -- Same agent for all items
```
- System counts array elements at runtime
- Spawns N identical agent instances in parallel
- Each agent gets one element (automatically indexed)
- Total time: `max(item_times)` + orchestration overhead
- Use case: Independent items, homogeneous processing

**3. Parallel (Label-Based Routing)**
```sql
execution_mode: "for_each"
agent_execution_mode: "parallel"
routing_mode: "label"  -- NEW: Route by item label/category
routing_field: "category"  -- Which field to read for routing
```
- **Dynamic array size:** Handles 4 items or 8 items at runtime
- **Semantic routing:** Each item declares its category/type
- **Specific agents:** Category maps to configured agent
- **Fallback:** Unmatched categories use default agent
- Total time: `max(category_times)`
- Use case: Heterogeneous items (Frontend, Backend, Database, Testing)

### New Schema for Label-Based Routing

```sql
-- For routing_mode="label": map categories to agents
CREATE TABLE step_routing_rules (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workflow_step_id UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
  label_value TEXT NOT NULL,                -- "frontend", "backend", "database", "testing"
  agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  display_order INTEGER NOT NULL DEFAULT 0,  -- UI ordering
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE(workflow_step_id, label_value)
);

CREATE INDEX idx_step_routing_rules_step ON step_routing_rules(workflow_step_id);
```

### Data Flow Example

**Workflow:** PRD → Decomposition → Parallel Implementation

```
Step 1: PRD Analyzer
  Agent: "Requirements Analyzer"
  Outputs:
    - port "sections" (array)
    - port "requirements" (array)

Step 2: Milestone Decomposer
  Agent: "Strategic Planner"
  Inputs:
    - port "sections" ← step-1.sections
    - port "requirements" ← step-1.requirements
  Prompt: "Create 4-8 milestones based on complexity. Each milestone should have a category: 'frontend', 'backend', 'database', or 'testing'."

  Outputs:
    - port "milestones" (array, dynamic size: 4-8)
      Schema: [{
        name: string,
        category: "frontend" | "backend" | "database" | "testing",
        description: string,
        tasks: array
      }]

Step 3: Milestone Implementation (for_each parallel with label routing)
  Execution Mode: for_each
  Agent Execution Mode: parallel
  Routing Mode: label
  Routing Field: "category"  -- Read item.category to determine agent

  Routing Rules:
    "frontend" → Frontend Specialist Agent
    "backend" → Backend Specialist Agent
    "database" → Database Specialist Agent
    "testing" → QA Specialist Agent
    (fallback) → General Implementation Agent

  Inputs:
    - port "milestone" ← step-2.milestones (routed by category)

  Outputs:
    - port "implementation" (object)

Edges (visual wiring):
  1. step-1 → step-2 (sections + requirements connected)
  2. step-2 → step-3 (milestones connected)
     - System sees routing_mode="label"
     - Reads each item's "category" field
     - Routes to appropriate agent
```

**Execution Flow (Label-Based Routing):**

1. **Step 1 executes** → produces envelope:
   ```json
   {
     "status": "success",
     "data": {
       "sections": ["intro", "features", "constraints"],
       "requirements": [...]
     },
     "metadata": {...}
   }
   ```

2. **Step 2 reads from Step 1:**
   - Wire: `step-1.sections → step-2.sections` automatically reads `envelope.data.sections`
   - Wire: `step-1.requirements → step-2.requirements` reads `envelope.data.requirements`
   - Build input object: `{"sections": [...], "requirements": [...]}`
   - Execute Strategic Planner agent with inputs

3. **Step 2 executes** → produces envelope:
   ```json
   {
     "status": "success",
     "data": {
       "milestones": [
         {"name": "Auth System", "category": "backend", "description": "...", "tasks": [...]},
         {"name": "User Dashboard", "category": "frontend", "description": "...", "tasks": [...]},
         {"name": "Database Schema", "category": "database", "description": "...", "tasks": [...]},
         {"name": "API Layer", "category": "backend", "description": "...", "tasks": [...]},
         {"name": "Test Suite", "category": "testing", "description": "...", "tasks": [...]},
         {"name": "UI Components", "category": "frontend", "description": "...", "tasks": [...]}
       ]
     },
     "metadata": {...}
   }
   ```
   Note: 6 milestones (dynamic size) with 2 backend, 2 frontend, 1 database, 1 testing

4. **Step 3 (parallel label-based routing):**
   - Wire: `step-2.milestones → step-3.milestone`
   - System detects `routing_mode: "label"` + `routing_field: "category"`
   - Extract array: `step-2-envelope.data.milestones` (6 elements)
   - Read each item's `category` field
   - Route to agents:
     ```
     Item 0 (backend) → Backend Specialist Agent + {"milestone": milestones[0]}
     Item 1 (frontend) → Frontend Specialist Agent + {"milestone": milestones[1]}
     Item 2 (database) → Database Specialist Agent + {"milestone": milestones[2]}
     Item 3 (backend) → Backend Specialist Agent + {"milestone": milestones[3]}
     Item 4 (testing) → QA Specialist Agent + {"milestone": milestones[4]}
     Item 5 (frontend) → Frontend Specialist Agent + {"milestone": milestones[5]}
     ```
   - Spawn 6 agents in parallel (2 backend, 2 frontend, 1 db, 1 testing)
   - Wait for all to complete
   - Aggregate envelopes into array (preserving original order)

5. **Step 3 aggregate output:**
   ```json
   {
     "status": "success",  // or "partial" if any failed
     "data": [
       {
         "status": "success",
         "data": {"implementation": "..."},
         "metadata": {
           "execution_id": "uuid-0",
           "iteration_index": 0,
           "iteration_label": "Auth System",
           "routing_label": "backend",
           "agent_id": "backend-specialist-agent-id"
         }
       },
       {
         "status": "success",
         "metadata": {
           "iteration_index": 1,
           "iteration_label": "User Dashboard",
           "routing_label": "frontend",
           "agent_id": "frontend-specialist-agent-id"
         }
       },
       // ... 4 more items
     ],
     "metadata": {
       "total_iterations": 6,
       "successful_iterations": 6,
       "routing_mode": "label",
       "routing_distribution": {
         "backend": 2,
         "frontend": 2,
         "database": 1,
         "testing": 1
       }
     }
   }
   ```

**Key Insight:** User never writes `features[0]` or `data.features` - wires are semantic connections, system handles indexing and envelope unwrapping automatically.

## UI/UX Design for Label-Based Routing

### Fluid Workflow for Creating Multi-Agent Pipelines

**User Goal:** "Take PRD → Decompose into 4-8 milestones → Route each to specialist agent"

**Step-by-Step UI Flow:**

#### 1. Create "Decompose PRD" Node

User drags "Agent" node onto canvas, configures:
```
Name: Decompose into Milestones
Agent: Strategic Planner
Prompt: "Analyze the PRD and create 4-8 milestones. Each milestone should have:
         - name
         - category (one of: 'frontend', 'backend', 'database', 'testing')
         - description
         - tasks"

Output Ports: [+ Add Output]
  Port Name: milestones
  Type: Array
  Item Schema: {
    name: string,
    category: "frontend" | "backend" | "database" | "testing",
    description: string,
    tasks: array
  }
```

#### 2. Wire Output to Next Step

User drags from "milestones" output port and releases on empty canvas area.

**System detects:** Array output with `category` field in schema

**Modal appears:**
```
┌─────────────────────────────────────────────────┐
│  How should we process the milestones?         │
│                                                 │
│  ○ Sequential                                  │
│    One agent processes all milestones in order │
│                                                 │
│  ○ Parallel (same agent)                       │
│    Same agent processes all milestones at once │
│                                                 │
│  ● Parallel (route by category) [Recommended]  │
│    Route each milestone to a specialist agent  │
│    ↓                                            │
│    Detected field: "category"                  │
│    Possible values: frontend, backend,         │
│                     database, testing          │
│                                                 │
│  [Continue]                                     │
└─────────────────────────────────────────────────┘
```

User selects "Parallel (route by category)" → [Continue]

#### 3. Configure Routing Rules

**New modal:**
```
┌─────────────────────────────────────────────────┐
│  Assign agents to categories                   │
│                                                 │
│  ┌─────────────────────────────────────────┐   │
│  │ frontend   → [Frontend Specialist ▾]    │   │
│  │ backend    → [Backend Specialist ▾]     │   │
│  │ database   → [Database Specialist ▾]    │   │
│  │ testing    → [QA Specialist ▾]          │   │
│  └─────────────────────────────────────────┘   │
│                                                 │
│  Fallback (for unmatched categories):          │
│  [General Implementation Agent ▾]              │
│                                                 │
│  [Create Node]                                 │
└─────────────────────────────────────────────────┘
```

User selects agents from dropdowns → [Create Node]

#### 4. Visual Node Representation

**New node appears on canvas:**
```
┌───────────────────────────────────┐
│  Process Milestones               │
│  (Route by category)              │
│                                   │
│  ┌─────────────────────────────┐ │
│  │ ▶ frontend  → Frontend Spec │ │
│  │ ▶ backend   → Backend Spec  │ │
│  │ ▶ database  → Database Spec │ │
│  │ ▶ testing   → QA Spec       │ │
│  │ ▶ (other)   → General Impl  │ │
│  └─────────────────────────────┘ │
│                                   │
│  Output: implementation           │
└───────────────────────────────────┘
```

**Compact view (collapsed):**
```
┌──────────────────────┐
│  Process Milestones  │
│  ┌─┬─┬─┬─┐           │
│  │F│B│D│T│  +1       │
│  └─┴─┴─┴─┘           │
└──────────────────────┘
```
Hovering shows tooltip: "F=Frontend Specialist, B=Backend Specialist, D=Database Specialist, T=QA Specialist, +1=Fallback"

#### 5. Editing Routing Rules

User clicks node → Properties panel shows:
```
┌─────────────────────────────────┐
│  Properties: Process Milestones │
├─────────────────────────────────┤
│  Execution Mode: For Each       │
│  Parallel: Yes                  │
│  Routing: By Label              │
│                                 │
│  Routing Field: category        │
│                                 │
│  Routing Rules:                 │
│  frontend  → Frontend Spec  [×] │
│  backend   → Backend Spec   [×] │
│  database  → Database Spec  [×] │
│  testing   → QA Spec        [×] │
│  [+ Add Rule]                   │
│                                 │
│  Fallback Agent:                │
│  [General Implementation ▾]     │
└─────────────────────────────────┘
```

Can add/remove/edit rules inline.

#### 6. Execution Visualization

During execution, node shows real-time routing:
```
┌──────────────────────┐
│  Process Milestones  │
│  ┌─┬─┬─┬─┐           │
│  │✓│⚙│⚙│⚙│  ⚙       │
│  └─┴─┴─┴─┘           │
│  2/6 completed       │
└──────────────────────┘
```
- ✓ = Completed (green)
- ⚙ = In progress (blue spinning)
- ✗ = Failed (red)

Clicking shows breakdown:
```
Milestone "Auth" (backend) → Backend Specialist → ✓ Completed (1.2s)
Milestone "Dashboard" (frontend) → Frontend Specialist → ✓ Completed (2.1s)
Milestone "Schema" (database) → Database Specialist → ⚙ Running...
Milestone "API" (backend) → Backend Specialist → ⚙ Running...
Milestone "Tests" (testing) → QA Specialist → ⚙ Running...
Milestone "UI Components" (frontend) → Frontend Specialist → ⚙ Running...
```

### Key UX Principles

1. **Schema-Driven Intelligence:** System detects category fields and suggests routing
2. **Guided Setup:** Modal walks user through routing configuration
3. **Visual Clarity:** Node shows routing rules at a glance
4. **Flexibility:** Works with 4, 6, 8+ items dynamically
5. **Transparent Execution:** Real-time routing visualization

## Implementation Plan

### Phase 1: Database Schema

**Files to create:**
- `/migrations/XXX_add_visual_workflow_support.sql`

**Changes:**
1. Add positioning columns to `workflow_steps`
2. Create `step_outputs` table
3. Create `step_inputs` table
4. Alter `workflow_step_edges` with port columns

### Phase 2: Output Envelope System

**Files to modify:**
- `/src/server/executors/dag/mod.rs`
- `/src/types/execution.rs` (create if doesn't exist)

**Changes:**

1. Define envelope types:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecutionEnvelope {
    pub status: ExecutionStatus,
    pub data: Option<JsonValue>,
    pub metadata: ExecutionMetadata,
    pub error: Option<ExecutionError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Success,
    Error,
    Partial,  // For for_each with some failures
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    pub execution_id: Uuid,
    pub execution_time_ms: u64,
    pub tokens_in: Option<i32>,
    pub tokens_out: Option<i32>,
    pub cost_usd: Option<f64>,
    pub model: String,
    pub agent_id: Option<Uuid>,  // Which agent executed this
    // For for_each iterations
    pub iteration_index: Option<usize>,
    pub iteration_label: Option<String>,
    pub routing_label: Option<String>,  // Category/label used for routing
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionError {
    pub message: String,
    pub error_type: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForEachAggregateEnvelope {
    pub status: ExecutionStatus,
    pub data: Vec<StepExecutionEnvelope>,
    pub metadata: ForEachMetadata,
    pub errors: Vec<IterationError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForEachMetadata {
    pub total_iterations: usize,
    pub successful_iterations: usize,
    pub failed_iterations: usize,
    pub execution_time_ms: u64,
    pub total_tokens_in: i32,
    pub total_tokens_out: i32,
    pub total_cost_usd: f64,
    pub routing_mode: Option<String>,  // NULL, "label"
    pub routing_distribution: Option<HashMap<String, usize>>,  // {"frontend": 2, "backend": 2, ...}
}
```

2. Wrap all execution outputs in `execute_step()`:
   - After LLM response, parse structured output
   - Create `StepExecutionEnvelope` with status, data, metadata, error
   - Store envelope as `agent_executions.structured_output`

3. Fix for_each aggregation (lines 898-1017 in dag/mod.rs):
   - Collect ALL iteration envelopes (including errors)
   - Build `ForEachAggregateEnvelope` with:
     - `data`: Array of all iteration envelopes
     - `metadata`: Aggregate stats
     - `errors`: List of failed iteration details
   - Set `status: "partial"` if any failures, `"error"` if all fail

4. Add label-based routing for_each execution:
```rust
// In execute_dag() for for_each steps
if step.execution_mode == "for_each" {
    let array = resolve_input_array(&step, &inputs)?;

    if step.agent_execution_mode == "sequential" {
        // Sequential: one agent, one-by-one
        sequential_for_each(step, array).await
    } else if step.routing_mode.as_deref() == Some("label") {
        // Label-based routing: read item field, route to specific agent
        let routing_field = step.routing_field
            .as_ref()
            .ok_or_else(|| anyhow!("routing_field required for label routing"))?;

        // Load routing rules (label → agent_id mappings)
        let routing_rules = query_step_routing_rules(step.id, pool).await?;
        let default_agent_id = step.agent_id; // Fallback for unmatched labels

        // Build label → agent_id lookup
        let agent_map: HashMap<String, Uuid> = routing_rules.iter()
            .map(|r| (r.label_value.clone(), r.agent_id))
            .collect();

        // Spawn agents in parallel, routing by label
        let futures: Vec<_> = array.iter().enumerate()
            .map(|(idx, elem)| {
                // Extract label from item
                let label = elem.get(routing_field)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Item {} missing field '{}'", idx, routing_field))?;

                // Route to agent (or fallback)
                let agent_id = agent_map.get(label)
                    .copied()
                    .unwrap_or(default_agent_id);

                Ok(execute_step_iteration(
                    agent_id,       // Different agent per category!
                    elem,
                    idx,
                    Some(label.to_string()),  // Pass routing label for metadata
                    workflow_execution_id
                ))
            })
            .collect::<Result<Vec<_>>>()?;

        let envelopes = futures::future::join_all(futures).await;
        aggregate_envelopes(envelopes)
    } else {
        // Parallel (same agent): N copies of same agent
        let futures: Vec<_> = array.iter().enumerate()
            .map(|(idx, elem)| {
                execute_step_iteration(
                    step.agent_id,  // Same agent for all
                    elem,
                    idx,
                    None,  // No routing label
                    workflow_execution_id
                )
            })
            .collect();

        let envelopes = futures::future::join_all(futures).await;
        aggregate_envelopes(envelopes)
    }
}
```

### Phase 3: Port-Based Input Resolution

**Files to modify:**
- `/src/server/executors/dag/mod.rs`
- `/src/db/queries/workflows.rs` (or similar)

**Changes:**

1. Query step inputs/outputs when loading workflow:
```rust
pub async fn get_workflow_with_ports(workflow_id: Uuid) -> Result<WorkflowWithPorts> {
    let steps = query_workflow_steps(workflow_id);
    let edges = query_workflow_edges(workflow_id);

    // NEW: Load port definitions
    let inputs = query_step_inputs(workflow_id);
    let outputs = query_step_outputs(workflow_id);

    Ok(WorkflowWithPorts { steps, edges, inputs, outputs })
}
```

2. Build step inputs from edges:
```rust
async fn build_step_inputs(
    step_id: Uuid,
    workflow_execution_id: Uuid,
    edges: &[EdgeWithPorts],
    pool: &PgPool,
) -> Result<HashMap<String, JsonValue>> {
    let mut inputs = HashMap::new();

    for edge in edges.iter().filter(|e| e.to_step_id == step_id) {
        // 1. Get source execution
        let source_exec = get_step_execution(
            workflow_execution_id,
            edge.from_step_id,
            pool
        ).await?;

        // 2. AUTOMATIC ENVELOPE UNWRAPPING
        // User wires: step-a.items → step-b.data
        // System reads: step-a-envelope.data.items (automatic .data prefix)
        let envelope: StepExecutionEnvelope =
            serde_json::from_value(source_exec.structured_output)?;

        let json_path = format!("$.{}", edge.from_output_port);
        let value = jsonpath::select(&envelope.data, &json_path)?;

        // 3. Apply optional transformation
        let transformed = if let Some(transform) = &edge.transform_jsonpath {
            jsonpath::select(&value, transform)?
        } else {
            value
        };

        // 4. Map to input port
        inputs.insert(edge.to_input_port.clone(), transformed);
    }

    // 5. Fill in defaults for missing optional inputs
    let input_defs = get_step_inputs(step_id, pool).await?;
    for input_def in input_defs {
        if !inputs.contains_key(&input_def.port_name) {
            if let Some(default) = input_def.default_value {
                inputs.insert(input_def.port_name, default);
            } else if input_def.required {
                return Err(anyhow!("Missing required input: {}", input_def.port_name));
            }
        }
    }

    Ok(inputs)
}
```

3. For-each array resolution:
```rust
// When a for_each step has an incoming edge with array data
async fn resolve_for_each_array(
    step: &WorkflowStepRow,
    inputs: &HashMap<String, JsonValue>,
) -> Result<Vec<JsonValue>> {
    // Find the array input port
    // Convention: for_each steps should have exactly one array-type input
    let array_input = inputs.values()
        .find(|v| v.is_array())
        .ok_or_else(|| anyhow!("No array input found for for_each step"))?;

    // Extract elements
    let elements = array_input.as_array()
        .ok_or_else(|| anyhow!("Input is not an array"))?;

    // For parallelism_mode="fixed", verify size
    if let Some(expected_size) = step.expected_array_size {
        if elements.len() != expected_size as usize {
            return Err(anyhow!(
                "Array size mismatch: expected {}, got {}",
                expected_size,
                elements.len()
            ));
        }
    }

    Ok(elements.clone())
}
```

4. Update `execute_step()` to use input object:
   - Replace variable interpolation with structured input
   - Pass inputs as JSON in system message or user message
   - Example: `"You will receive inputs as JSON: {inputs}"`

### Phase 4: Remove Variable System

**Files to modify:**
- `/src/server/executors/dag/mod.rs`
- `/src/db/queries/executions.rs`
- Database migration to drop table

**Changes:**

1. Drop `execution_variables` table:
```sql
DROP TABLE IF EXISTS execution_variables;
```

2. Remove variable interpolation logic from DAG executor
3. Remove `output_variable_name` column from `workflow_steps` (optional cleanup)

### Phase 5: API Endpoints for Visual Builder

**Files to create:**
- `/src/server/api/workflow_ports.rs`

**Endpoints:**

```rust
// Port management
GET    /api/workflows/{id}/ports           // Get all inputs/outputs for workflow
POST   /api/steps/{id}/inputs              // Define input port
PUT    /api/step-inputs/{id}               // Update input port
DELETE /api/step-inputs/{id}               // Remove input port
POST   /api/steps/{id}/outputs             // Define output port
PUT    /api/step-outputs/{id}              // Update output port
DELETE /api/step-outputs/{id}              // Remove output port

// Edge management with ports
POST   /api/workflows/{id}/edges           // Create edge with port mapping
PUT    /api/edges/{id}                     // Update edge port mapping

// Visual positioning
PATCH  /api/steps/{id}/position            // Update x, y, width, height

// Routing rules (for label-based agent assignment)
GET    /api/steps/{id}/routing-rules       // List all routing rules for a step
POST   /api/steps/{id}/routing-rules       // Create routing rule (label_value, agent_id)
PUT    /api/routing-rules/{id}             // Update routing rule
DELETE /api/routing-rules/{id}             // Remove routing rule

// Routing configuration
PATCH  /api/steps/{id}/routing             // Set routing_mode and routing_field
```

## Verification Plan

### 1. Database Migration
```bash
docker exec -it gh-agents-postgres-1 psql -U nexor -d nexor
\d workflow_steps          -- Should show position_x, position_y, width, height, routing_mode, routing_field
\d step_outputs            -- Should exist
\d step_inputs             -- Should exist
\d step_routing_rules      -- Should exist with label_value, agent_id, display_order
\d workflow_step_edges     -- Should show port columns (from_output_port, to_input_port, etc.)
\d execution_variables     -- Should not exist
```

### 2. Output Envelope Testing

Create test workflow with:
- Step A: Simple LLM call → should produce envelope with status, data, metadata
- Step B: For-each over array → should produce aggregate envelope
- Step C (iteration): Intentionally fail some iterations → verify partial status

**Expected:**
```json
// Step A output
{
  "status": "success",
  "data": {"result": "..."},
  "metadata": {"execution_id": "...", "tokens_in": 100, ...},
  "error": null
}

// Step B output (for_each with partial failure)
{
  "status": "partial",
  "data": [
    {"status": "success", "data": {...}, ...},
    {"status": "error", "data": null, "error": {...}, ...}
  ],
  "metadata": {
    "total_iterations": 2,
    "successful_iterations": 1,
    "failed_iterations": 1
  },
  "errors": [{"iteration_index": 1, ...}]
}
```

### 3. Port-Based Wiring Testing

Create workflow:
```
Step 1 (outputs: "list" → data.items)
  ↓
Step 2 (inputs: "items", outputs: "count" → data.count)
  ↓
Step 3 (for_each over "items", inputs: "item")
```

**Verify:**
- Step 2 receives `{"items": [...]}` from Step 1's envelope
- Step 3 iterates over each element from Step 2's `data.count`
- Each iteration receives `{"item": <element>}`

### 4. Array Mapping Testing

Test JSONPath transformations:
```
Edge: step-1.list → step-2.names
  with transform: "$.items[*].name"
```

**Verify:**
- Step 2 receives only the `name` field from each item
- Array structure is preserved

### 5. Error Tracking Testing

Create for_each workflow that will partially fail:
- Use intentional errors (e.g., malformed JSON schema)
- Verify failed iterations appear in aggregated output
- Verify `errors` array contains details
- Verify `status: "partial"` when some succeed

### 6. Label-Based Routing Testing

Create workflow with label-based for_each routing:

**Setup:**
```sql
-- Step 1: Generate milestones (dynamic size: 4-8)
INSERT INTO workflow_steps (workflow_id, agent_id, execution_mode)
VALUES (workflow_id, 'strategic-planner-agent', 'single');

-- Step 2: Parallel implementation with label routing
INSERT INTO workflow_steps (
  workflow_id,
  execution_mode,
  agent_execution_mode,
  routing_mode,
  routing_field,
  agent_id  -- Fallback agent
)
VALUES (workflow_id, 'for_each', 'parallel', 'label', 'category', 'general-implementation-agent');

-- Define routing rules (category → agent mappings)
INSERT INTO step_routing_rules (workflow_step_id, label_value, agent_id, display_order) VALUES
  (step2_id, 'frontend', 'frontend-specialist-agent', 0),
  (step2_id, 'backend', 'backend-specialist-agent', 1),
  (step2_id, 'database', 'database-specialist-agent', 2),
  (step2_id, 'testing', 'qa-specialist-agent', 3);

-- Define ports and edge
INSERT INTO step_outputs (workflow_step_id, port_name, port_type, json_path)
VALUES (step1_id, 'milestones', 'array', 'milestones');

INSERT INTO step_inputs (workflow_step_id, port_name, port_type, required)
VALUES (step2_id, 'milestone', 'object', true);

INSERT INTO workflow_step_edges (workflow_id, from_step_id, to_step_id, from_output_port, to_input_port)
VALUES (workflow_id, step1_id, step2_id, 'milestones', 'milestone');
```

**Execute and Verify:**

1. **Step 1 output** (6 milestones, dynamic):
```json
{
  "status": "success",
  "data": {
    "milestones": [
      {"name": "Auth", "category": "backend", "tasks": [...]},
      {"name": "Dashboard", "category": "frontend", "tasks": [...]},
      {"name": "Schema", "category": "database", "tasks": [...]},
      {"name": "API", "category": "backend", "tasks": [...]},
      {"name": "Tests", "category": "testing", "tasks": [...]},
      {"name": "UI Components", "category": "frontend", "tasks": [...]}
    ]
  }
}
```

2. **Step 2 routing verification:**
   - Item 0 (category: "backend") → Backend Specialist Agent
   - Item 1 (category: "frontend") → Frontend Specialist Agent
   - Item 2 (category: "database") → Database Specialist Agent
   - Item 3 (category: "backend") → Backend Specialist Agent (reused)
   - Item 4 (category: "testing") → QA Specialist Agent
   - Item 5 (category: "frontend") → Frontend Specialist Agent (reused)

3. **Verify parallel execution:**
   - All 6 agents spawn in parallel (timestamps should overlap)
   - Same agent can process multiple items concurrently

4. **Verify metadata:**
   - Each iteration has `routing_label: "backend"`, `"frontend"`, etc.
   - Each iteration has `agent_id` matching the routing rule
   - Aggregate has `routing_distribution: {"backend": 2, "frontend": 2, "database": 1, "testing": 1}`

**Expected aggregate output:**
```json
{
  "status": "success",
  "data": [
    {
      "status": "success",
      "data": {...},
      "metadata": {
        "iteration_index": 0,
        "iteration_label": "Auth",
        "routing_label": "backend",
        "agent_id": "backend-specialist-agent",
        "execution_time_ms": 1200
      }
    },
    // ... 5 more iterations
  ],
  "metadata": {
    "total_iterations": 6,
    "successful_iterations": 6,
    "routing_mode": "label",
    "routing_distribution": {
      "backend": 2,
      "frontend": 2,
      "database": 1,
      "testing": 1
    }
  }
}
```

5. **Test fallback agent:**
   - Add milestone with `category: "infrastructure"` (no routing rule)
   - Verify it routes to `general-implementation-agent` (fallback)
   - Verify metadata includes `routing_label: "infrastructure"` with fallback agent_id

### 7. Automatic Envelope Unwrapping Testing

Create simple two-step workflow:

**Setup:**
```
Step 1 output: {"data": {"result": "hello", "count": 5}}
Step 2 expects input port "message" connected to step-1.result
```

**Verify:**
- User creates edge: `step-1.result → step-2.message`
- System automatically reads `step-1-envelope.data.result` (not `step-1-envelope.result`)
- Step 2 receives: `{"message": "hello"}` (automatic unwrapping)
- No need for user to specify `.data` prefix

---

## Implementation Roadmap

### Phase 1: Database Schema (1-2 days)

**Goal:** Add port system, routing rules, remove variables

**Tasks:**
1. Create migration: Add columns to `workflow_steps`
   - `position_x`, `position_y`, `width`, `height` (visual layout)
   - `routing_mode`, `routing_field` (label routing config)
2. Create `step_inputs` table (port definitions)
3. Create `step_outputs` table (port definitions)
4. Create `step_routing_rules` table (label → agent mappings)
5. Alter `workflow_step_edges` table (add port columns)
6. Create migration: Drop `execution_variables` table
7. Run migrations, verify schema

**Verification:**
```bash
docker exec -it gh-agents-postgres-1 psql -U nexor -d nexor
\d workflow_steps
\d step_inputs
\d step_outputs
\d step_routing_rules
\d workflow_step_edges
SELECT * FROM execution_variables;  -- Should error (table dropped)
```

### Phase 2: Type Definitions (1 day)

**Goal:** Define envelope and port types

**Tasks:**
1. Create `/src/types/execution.rs` (or add to existing types file):
   - `StepExecutionEnvelope`
   - `ExecutionStatus` enum
   - `ExecutionMetadata`
   - `ExecutionError`
   - `ForEachAggregateEnvelope`
   - `ForEachMetadata`
2. Create port types:
   - `StepInputRow`
   - `StepOutputRow`
   - `RoutingRuleRow`
   - `EdgeWithPorts`
3. Update existing types:
   - `WorkflowStepRow` with new columns
   - `AgentExecutionRow` (ensure `structured_output` field exists)

**Verification:**
- `cargo check` passes
- No compilation errors

### Phase 3: Database Queries (2-3 days)

**Goal:** CRUD operations for ports and routing

**Tasks:**
1. Add to `/src/db/queries/workflows.rs`:
   - `query_step_inputs(workflow_id)` → Vec<StepInputRow>
   - `query_step_outputs(workflow_id)` → Vec<StepOutputRow>
   - `query_step_routing_rules(step_id)` → Vec<RoutingRuleRow>
   - `create_step_input(...)`
   - `create_step_output(...)`
   - `create_routing_rule(...)`
   - `update_edge_with_ports(...)`
2. Update existing queries:
   - `get_workflow_with_ports(workflow_id)` - Load workflow + ports + routing
   - `get_workflow_step(step_id)` - Include new columns

**Verification:**
- Write unit tests for each query
- Test with sample data
- `cargo test db::queries::workflows`

### Phase 4: Refactor DAG Executor - Core (3-5 days)

**Goal:** Replace variable system with port-based flow

**Tasks:**
1. **Remove variable code:**
   - Delete `resolve_variable()` functions
   - Remove `execution_variables` table access
   - Remove `{variable_name}` interpolation from prompt rendering

2. **Add envelope wrapping:**
   ```rust
   fn wrap_in_envelope(
       execution_id: Uuid,
       agent_id: Uuid,
       output: Option<JsonValue>,
       error: Option<anyhow::Error>,
       timing: ExecutionTiming,
       tokens: TokenUsage,
   ) -> StepExecutionEnvelope {
       // Build envelope with status, data, metadata, error
   }
   ```

3. **Implement port-based input resolution:**
   ```rust
   async fn build_step_inputs(
       step_id: Uuid,
       workflow_execution_id: Uuid,
       edges: &[EdgeWithPorts],
       pool: &PgPool,
   ) -> Result<HashMap<String, JsonValue>> {
       // For each incoming edge:
       //   1. Get source execution
       //   2. Parse envelope
       //   3. Extract data from .data.<output_port>
       //   4. Apply optional JSONPath transform
       //   5. Map to input port
       // Fill in defaults for missing optional inputs
   }
   ```

4. **Update `execute_step()` signature:**
   - Input: `HashMap<String, JsonValue>` (from ports)
   - Output: `StepExecutionEnvelope`
   - Store envelope in `agent_executions.structured_output`

**Verification:**
- Single-step workflow executes
- Output wrapped in envelope
- `cargo test executors::dag::single_step`

### Phase 5: Refactor DAG Executor - For-Each (3-4 days)

**Goal:** Add label-based routing

**Tasks:**
1. **Refactor for-each input resolution:**
   ```rust
   async fn resolve_for_each_array(
       step: &WorkflowStepRow,
       inputs: &HashMap<String, JsonValue>,
   ) -> Result<Vec<JsonValue>> {
       // Find array input (should be only one for for_each steps)
       // Extract elements
       // Return vector
   }
   ```

2. **Implement label routing:**
   ```rust
   async fn execute_for_each_label_routing(
       step: &WorkflowStepRow,
       array: Vec<JsonValue>,
       routing_field: &str,
       routing_rules: &HashMap<String, Uuid>,
       default_agent_id: Uuid,
       workflow_execution_id: Uuid,
   ) -> Result<ForEachAggregateEnvelope> {
       // For each element:
       //   1. Read routing_field value (category)
       //   2. Look up agent_id from routing_rules
       //   3. Use default_agent_id if not found
       //   4. Spawn execution (in parallel)
       // Aggregate all envelopes
       // Build ForEachAggregateEnvelope with stats
   }
   ```

3. **Update for-each aggregation:**
   - Collect ALL iteration envelopes (including errors)
   - Set `status: "partial"` if any failures
   - Set `status: "error"` if all failures
   - Include `routing_distribution` in metadata

4. **Error handling:**
   - Failed iterations preserved in aggregate
   - `errors` array populated with details

**Verification:**
- For-each sequential works
- For-each parallel (same agent) works
- For-each label routing works
- Failed iterations tracked correctly
- `cargo test executors::dag::for_each`

### Phase 6: API Endpoints (2-3 days)

**Goal:** CRUD APIs for ports and routing

**Tasks:**
1. Create `/src/server/api/workflow_ports.rs`:
   ```rust
   // Port management
   GET    /api/workflows/{id}/ports
   POST   /api/steps/{id}/inputs
   PUT    /api/step-inputs/{id}
   DELETE /api/step-inputs/{id}
   POST   /api/steps/{id}/outputs
   PUT    /api/step-outputs/{id}
   DELETE /api/step-outputs/{id}

   // Routing rules
   GET    /api/steps/{id}/routing-rules
   POST   /api/steps/{id}/routing-rules
   PUT    /api/routing-rules/{id}
   DELETE /api/routing-rules/{id}

   // Configuration
   PATCH  /api/steps/{id}/routing
   PATCH  /api/steps/{id}/position
   ```

2. Update `/src/server/api/workflows.rs`:
   - Return ports when getting workflow
   - Update edge creation to include port mapping

3. Add validation:
   - Port names must be unique per step
   - Edge port references must exist
   - Routing field must exist in output schema
   - Label values in routing rules must match schema enum

**Verification:**
- Test all endpoints with curl/Postman
- Create workflow with ports via API
- Update routing rules
- `cargo test api::workflow_ports`

### Phase 7: Interactive Review Rooms (2 days)

**Goal:** Human-in-loop review with agent conversation

**Tasks:**
1. Add review step type detection:
   ```rust
   if step.is_interactive && step.agent_execution_mode == "room" {
       // Open review room
   }
   ```

2. Implement review room flow:
   - Create room session
   - Add review agent to room
   - Agent receives step inputs via ports (context)
   - Agent presents data, asks for feedback
   - User approves/rejects/modifies via chat
   - Agent outputs decision to port
   - Workflow resumes

3. Integration with existing room executor:
   - Use `/src/server/executors/room/mod.rs`
   - Pass input data as room context
   - Capture final decision as output

**Verification:**
- Create workflow with review step
- Execute, verify room opens
- Chat with agent, approve
- Verify workflow continues
- Check output contains decision

### Phase 8: Collection Executor Update (1 day)

**Goal:** Update multi-workflow executor for envelopes

**Tasks:**
1. Update `/src/server/executors/collection_dag/mod.rs`:
   - Read workflow outputs from envelopes
   - Pass data between workflows via ports
   - Handle envelope status checks

**Verification:**
- Create collection with 2 workflows
- Execute, verify data flows between them
- `cargo test executors::collection_dag`

### Phase 9: Integration Testing (2-3 days)

**Goal:** End-to-end workflow testing

**Tasks:**
1. **Test Case 1: Simple Pipeline**
   - PRD Analyzer → Summarizer
   - Verify port connections
   - Verify envelope structure

2. **Test Case 2: Label Routing**
   - Decomposer → Label-routed implementation (4-8 items)
   - Verify routing to correct agents
   - Verify routing_distribution in output

3. **Test Case 3: Interactive Review**
   - Analysis → Review Room → Implementation
   - Test approval flow
   - Test rejection/modification flow

4. **Test Case 4: Error Handling**
   - Intentional failures in for-each
   - Verify partial status
   - Verify errors array populated

5. **Test Case 5: Complex Multi-Step**
   - Full PRD → Decompose → Route → Review → Implement workflow
   - Verify all features together

**Verification:**
- All test cases pass
- No panics or unwraps
- Error messages clear and actionable

### Phase 10: Documentation & Cleanup (1-2 days)

**Goal:** Polish and document

**Tasks:**
1. Update README with new architecture
2. Add code comments to complex functions
3. Remove dead code (old variable system remnants)
4. Run `cargo fmt` and `cargo clippy`
5. Fix all clippy warnings
6. Add API documentation (OpenAPI/Swagger)

**Verification:**
- `cargo clippy` clean
- `cargo test` all passing
- Documentation readable

---

## Total Estimated Timeline

**Breakdown:**
- Phase 1 (Schema): 1-2 days
- Phase 2 (Types): 1 day
- Phase 3 (Queries): 2-3 days
- Phase 4 (DAG Core): 3-5 days
- Phase 5 (For-Each): 3-4 days
- Phase 6 (API): 2-3 days
- Phase 7 (Review Rooms): 2 days
- Phase 8 (Collections): 1 day
- Phase 9 (Integration): 2-3 days
- Phase 10 (Docs): 1-2 days

**Total:** 18-27 days (3.5-5.5 weeks)

**Recommendation:** Work in order, complete each phase before moving to next. Each phase builds on previous work.

---

## Success Criteria

1. ✅ All tests passing (`cargo test`)
2. ✅ No clippy warnings (`cargo clippy`)
3. ✅ Can create workflow with ports via API
4. ✅ Can execute workflow with label routing
5. ✅ Failed iterations preserved in output
6. ✅ Interactive review rooms functional
7. ✅ Variable system completely removed
8. ✅ Integration tests cover all features
9. ✅ Code is clean and well-documented
10. ✅ Ready for visual UI implementation (backend complete)

---

## Notes

- **Application has not run:** Clean slate, no backwards compatibility concerns
- **One executor:** DAG executor handles all workflow execution, refactored in place
- **Backend only:** UI vision documented for future reference
- **Future-ready:** Design supports AI-generated workflows and self-scheduling
- **Review rooms:** Already integrated, enables human-in-loop collaboration
- **Label routing:** Core innovation enabling dynamic multi-agent pipelines

## Future Extensibility: AI-Generated Workflows

**Vision:** AI agents design workflows, schedule review checkpoints, and adapt execution based on feedback.

### Phase 1: Human-Designed Workflows (Current Plan)
- User creates workflow in UI (nodes, edges, ports, routing)
- AI executes predefined workflow
- Human reviews at configured checkpoints

### Phase 2: AI-Generated Workflows (Future)

**User Input:** High-level goal
```
"Build an authentication system with frontend, backend, database, and tests"
```

**AI Workflow Generator Agent:**
1. Analyzes requirements
2. Generates workflow structure:
   ```json
   {
     "steps": [
       {"name": "Gather Requirements", "agent": "Requirements Analyst"},
       {"name": "Review Requirements", "type": "interactive_review"},
       {"name": "Design Architecture", "agent": "System Architect"},
       {"name": "Review Architecture", "type": "interactive_review"},
       {"name": "Implementation", "type": "for_each_label_routing",
        "routing_field": "component", "routing_rules": {
          "frontend": "Frontend Specialist",
          "backend": "Backend Specialist",
          "database": "Database Specialist",
          "testing": "QA Specialist"
        }},
       {"name": "Integration Testing", "agent": "Test Engineer"},
       {"name": "Final Review", "type": "interactive_review"}
     ]
   }
   ```
3. Auto-schedules review checkpoints based on:
   - Task complexity
   - Uncertainty level
   - Dependency criticality
4. Creates workflow in database
5. Executes workflow

**Self-Scheduling Reviews:**
- AI determines: "I'm uncertain about architecture decision → schedule review"
- AI detects: "All components complete → checkpoint before integration"
- User gets notifications: "Requirements complete. 4 milestones identified. Review?"

**Adaptive Execution:**
- Human feedback: "Change milestone 2 to focus on API security"
- AI regenerates affected downstream steps
- Workflow continues with updated plan

**Design Principles That Support This:**
1. **Workflows are JSON** - Can be generated programmatically
2. **Ports are contracts** - AI can reason about data dependencies
3. **Routing is semantic** - AI assigns specialists based on category
4. **Envelopes are inspectable** - AI can check status, analyze results
5. **Review rooms exist** - AI can request human input when needed

**Future Implementation Needs:**
- Workflow generation LLM (meta-agent)
- Checkpoint planner (uncertainty-based scheduling)
- Workflow modification API (mid-execution changes)
- Meta-execution orchestrator

---

## UI Vision: Visual Workflow Builder (Future Reference)

**Note:** This plan focuses on backend. This section documents UI vision for future implementation.

### Technology Stack (Recommended)
- **Canvas:** React Flow (visual node editor)
- **State:** Zustand or Jotai (lightweight)
- **API Client:** Existing `/frontend/src/api/` typed endpoints
- **Real-time:** WebSocket for execution updates

### Core UI Components

**1. Workflow Canvas**
```
┌─────────────────────────────────────────────────────┐
│ [+ Node] [▶ Run] [💾 Save]          Workflow: PRD   │
├─────────────────────────────────────────────────────┤
│                                                     │
│   ┌─────────────┐                                  │
│   │ PRD Analyze │                                  │
│   │  ○ sections │────────────┐                     │
│   │  ○ requirements │────┐   │                     │
│   └─────────────┘      │   │                     │
│                        │   │                     │
│                        ↓   ↓                     │
│                 ┌──────────────┐                  │
│                 │ Decompose    │                  │
│                 │  ● sections  │                  │
│                 │  ● requirements                │
│                 │  ○ milestones │────┐           │
│                 └──────────────┘    │           │
│                                     │           │
│                                     ↓           │
│                         ┌────────────────────┐   │
│                         │ Process Milestones │   │
│                         │  ● milestone       │   │
│                         │  (route by category)│  │
│                         │  ┌─┬─┬─┬─┐         │   │
│                         │  │F│B│D│T│  +1     │   │
│                         │  └─┴─┴─┴─┘         │   │
│                         └────────────────────┘   │
│                                                     │
└─────────────────────────────────────────────────────┘
```

**2. Node Configuration Panel**
```
┌─────────────────────────────────┐
│  Process Milestones             │
├─────────────────────────────────┤
│  Agent: [Not set - uses routing]│
│  Execution: For Each            │
│  Parallel: Yes                  │
│  Routing: By Label              │
│                                 │
│  Input Ports:                   │
│  ● milestone (object, required) │
│  [+ Add Input]                  │
│                                 │
│  Output Ports:                  │
│  ○ implementation (object)      │
│  [+ Add Output]                 │
│                                 │
│  Routing Configuration:         │
│  Field: category                │
│                                 │
│  Rules:                         │
│  frontend  → [Frontend Spec ▾]  │
│  backend   → [Backend Spec ▾]   │
│  database  → [Database Spec ▾]  │
│  testing   → [QA Spec ▾]        │
│  [+ Add Rule]                   │
│                                 │
│  Fallback: [General Agent ▾]    │
└─────────────────────────────────┘
```

**3. Edge Creation Flow**

**Step 1:** User drags from output port "milestones"

**Step 2:** System detects array with category field → Modal appears:
```
┌─────────────────────────────────────────────────┐
│  How should we process the milestones?         │
│                                                 │
│  ○ Sequential                                  │
│  ○ Parallel (same agent)                       │
│  ● Parallel (route by category) [Recommended]  │
│                                                 │
│  Detected field: "category"                    │
│  Values: frontend, backend, database, testing  │
│                                                 │
│  [Continue]                                     │
└─────────────────────────────────────────────────┘
```

**Step 3:** Routing configuration modal:
```
┌─────────────────────────────────────────────────┐
│  Assign agents to categories                   │
│                                                 │
│  frontend   → [Frontend Specialist ▾]          │
│  backend    → [Backend Specialist ▾]           │
│  database   → [Database Specialist ▾]          │
│  testing    → [QA Specialist ▾]                │
│                                                 │
│  Fallback: [General Agent ▾]                   │
│                                                 │
│  [Create Node]                                 │
└─────────────────────────────────────────────────┘
```

**Step 4:** Node created with visual routing indicators

**4. Execution Visualization**

During execution, nodes show real-time status:
```
┌──────────────────────┐
│  Process Milestones  │
│  ┌─┬─┬─┬─┐           │
│  │✓│✓│⚙│⏸│  ⚙       │  ✓ = Complete  ⚙ = Running
│  └─┴─┴─┴─┘           │  ⏸ = Waiting  ✗ = Failed
│  3/6 completed       │
│  Backend: 2 running  │
└──────────────────────┘
```

Click for details:
```
Execution Details:
  Milestone "Auth" (backend)
    → Backend Specialist
    → ✓ Completed (1.2s, 150 tokens, $0.003)

  Milestone "Dashboard" (frontend)
    → Frontend Specialist
    → ✓ Completed (2.1s, 200 tokens, $0.004)

  Milestone "Schema" (database)
    → Database Specialist
    → ⚙ Running... (0.8s elapsed)

  [View Full Output] [View Transcript]
```

**5. Interactive Review Room UI**

When workflow hits review step:
```
┌─────────────────────────────────────────────────────┐
│  Review: Milestones                                 │
│  Strategic Reviewer joined the room                 │
├─────────────────────────────────────────────────────┤
│  [Agent] I've analyzed the 6 milestones created.   │
│          Let me walk you through them:             │
│                                                     │
│          1. Auth System (backend) - Includes...    │
│          2. User Dashboard (frontend) - Contains...│
│          ...                                        │
│                                                     │
│          Do you approve these milestones?          │
├─────────────────────────────────────────────────────┤
│  [You] Change milestone 2 to focus on security    │
│        instead of just UI components               │
├─────────────────────────────────────────────────────┤
│  [Agent] Understood. I'll update milestone 2:      │
│          "Secure Dashboard" - Focus on auth flows, │
│          session management, and XSS protection.   │
│                                                     │
│          Updated. Ready to proceed?                │
├─────────────────────────────────────────────────────┤
│  [You] Approve ✓                                   │
├─────────────────────────────────────────────────────┤
│  [Agent] ✓ Approved. Continuing workflow...        │
└─────────────────────────────────────────────────────┘

[Type message...] [Approve] [Request Changes]
```

Workflow resumes with updated milestones passed to next step.

---

## Critical Files

### To Modify
- `/src/server/executors/dag/mod.rs` - **Major refactor:** Port resolution, envelope wrapping, label routing
- `/src/server/executors/collection_dag/mod.rs` - Update for envelope outputs
- `/src/db/queries/workflows.rs` - Add port/routing queries
- `/src/server/api/workflows.rs` - Extend with port endpoints
- `/src/types/agent.rs` or similar - Add envelope types

### To Create
- `/migrations/XXX_port_based_workflow_system.sql` - All schema changes
- `/src/server/api/workflow_ports.rs` - Port CRUD endpoints (or merge into workflows.rs)
- `/src/server/api/routing_rules.rs` - Routing rule management

### To Remove
- `/migrations/XXX_drop_execution_variables.sql` - Drop variables table
- All `execution_variables` query code
- Variable interpolation logic from DAG executor

### Testing Files to Create/Update
- `/src/server/executors/dag/tests.rs` - Add envelope, routing, port tests
- Integration tests for label routing
- Interactive review room tests

## Schema-Driven Routing Intelligence

### Automatic Routing Field Detection

When user defines an output port with array type, system analyzes the item schema to suggest routing fields:

**Example Output Schema:**
```json
{
  "type": "array",
  "items": {
    "type": "object",
    "properties": {
      "name": {"type": "string"},
      "category": {"type": "string", "enum": ["frontend", "backend", "database", "testing"]},
      "priority": {"type": "string", "enum": ["high", "medium", "low"]},
      "description": {"type": "string"}
    }
  }
}
```

**System detects:**
- `category` field with enum → **Recommended routing field** (limited set of values = good for routing)
- `priority` field with enum → Secondary option (could route high-priority to specialized agent)

**UI suggests:**
```
Detected routing fields:
  ● category (frontend, backend, database, testing) [Recommended]
  ○ priority (high, medium, low)
```

### Routing Field Requirements

For a field to be a valid routing field:
1. Must be a **string type** (or enum)
2. Should be present in **all array items** (required field)
3. Ideally has **limited set of values** (enum preferred)

Non-ideal but supported:
- Open-ended strings (e.g., `task_type: string`) → User must configure all possible values
- Missing field → Falls back to default agent

## Design Decisions (User Confirmed)

1. **Automatic envelope unwrapping:** Edges reference port names directly (`step-a.items`), system automatically reads from `envelope.data.items`

2. **Error handling:** Workflows continue on failures with error envelopes - downstream steps can check status and handle errors

3. **Port definitions:** Manual definition required - ports define the contract between nodes

4. **Wire semantics:** Wires represent logical connectivity, not data paths - users connect nodes, system handles data extraction

5. **For-each parallelization modes:**
   - **Sequential:** Single agent processes entire array one-by-one
   - **Parallel (same agent):** Spawn N identical agents for N items
   - **Parallel (label-based routing):** Route each item to specialist agent based on category/label field

   Key insight: No "output.1" syntax - system handles indexing and routing automatically

6. **Dynamic array sizes:** Label-based routing supports variable-length arrays (4 items or 8 items) with semantic agent assignment
