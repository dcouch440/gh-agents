# Migration Plan: Multi-Tier DAG Architecture (Workflow Collections)

## Executive Summary

Add a new tier above workflows: **Workflow Collections** that form a DAG. This enables:
- Multiple workflows working together with dependencies
- Topology sorting at the collection level
- **Configurable execution modes (sequential/parallel) at ALL levels** (collection → workflow → step)
- **Parallel execution respects DAG dependencies** (parallelizes independent branches)
- Variable capture for cross-workflow data flow (full granularity: collection/workflow/step/agent)
- Flexible multi-agent step execution

**Leave alone:** Pipelines system (legacy, not used, don't touch)
**Keep:** Current workflow DAG system (proven, works well)

---

## New Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                 WORKFLOW COLLECTIONS                        │
│  Topology-sorted DAG of workflows                           │
│  Variable scope: $collection → $workflow → $step            │
└───────────────────┬─────────────────────────────────────────┘
                    │
                    │ Kahn's algorithm (like current workflow_steps)
                    │
      ┌─────────────┼─────────────┬──────────────┐
      │             │             │              │
      ▼             ▼             ▼              ▼
  ┌────────┐    ┌────────┐    ┌────────┐    ┌────────┐
  │Workflow│───►│Workflow│───►│Workflow│    │Workflow│
  │   A    │    │   B    │    │   D    │    │   E    │
  │        │    │        │    │        │    │        │
  └────────┘    └───┬────┘    └────────┘    └────────┘
                    │
                    │
                    ▼
                ┌────────┐
                │Workflow│
                │   C    │
                │        │
                └────────┘

Each workflow = existing DAG of steps (no changes) ↓

  ┌──────────────────────────────────────────────────────┐
  │        Workflow A (unchanged)                        │
  │                                                      │
  │  Step 1 ──► Step 2 ──► Step 4                       │
  │             Step 3 ──/                               │
  │                                                      │
  │  Each step: 1+ agents (flexible execution strategy)  │
  └──────────────────────────────────────────────────────┘
```

### Execution Mode Cascade (Configurable All the Way Up)

Like a ticket decomposition tree, execution mode is configurable at every level:

```
Collection (execution_mode: "parallel")
  │
  ├─ Workflow A (mode: "sequential") ──┐
  │  ├─ Step 1 (mode: NULL = inherit)  │
  │  └─ Step 2 (mode: "parallel")      │
  │     ├─ Agent A ──┐                 │
  │     ├─ Agent B ──┼─ Run in parallel│
  │     └─ Agent C ──┘                 │
  │                                    │
  ├─ Workflow B (mode: "parallel") ────┼─ A, B, C run concurrently
  │  ├─ Step 1 → Step 2 → Step 4      │   (respecting dependencies)
  │  └─ Step 3 ────────/               │
  │                                    │
  └─ Workflow C (mode: NULL = inherit)─┘
```

**Parallel mode behavior (respects DAG):**
- If Workflow A → Workflow B (B depends on A), B waits for A to complete
- If Workflow B and C both depend on A, B and C run simultaneously after A completes
- Entry workflows (no dependencies) all start simultaneously
- Same logic applies to steps within workflows, and agents within steps

**Sequential mode behavior (forced serialization):**
- Execute nodes one-at-a-time even if they're independent
- Useful for rate limiting, debugging, or when order matters

```

---

## Database Schema Changes

### New Tables

```sql
-- Collection definition (like workflows table)
CREATE TABLE workflow_collections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    execution_mode TEXT NOT NULL DEFAULT 'parallel', -- "sequential" or "parallel"
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- execution_mode behavior:
-- - "sequential": Execute workflows one-at-a-time in topological order
-- - "parallel": Execute independent workflows concurrently (respects DAG dependencies)

-- Members: which workflows belong to this collection
CREATE TABLE collection_workflows (
    collection_id UUID NOT NULL REFERENCES workflow_collections(id) ON DELETE CASCADE,
    workflow_id UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    display_order INT NOT NULL DEFAULT 0,
    execution_mode TEXT, -- NULL = inherit from collection, "sequential"/"parallel" = override
    PRIMARY KEY (collection_id, workflow_id)
);

-- Execution mode hierarchy (like CSS cascade):
-- 1. collection_workflows.execution_mode (most specific)
-- 2. workflow_collections.execution_mode (fallback)
-- 3. System default: "parallel"

-- DAG edges between workflows (like workflow_step_edges)
CREATE TABLE collection_workflow_edges (
    from_workflow_id UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    to_workflow_id UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    collection_id UUID NOT NULL REFERENCES workflow_collections(id) ON DELETE CASCADE,
    PRIMARY KEY (from_workflow_id, to_workflow_id, collection_id)
);

-- Execution tracking (replaces pipeline_runs)
CREATE TABLE collection_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    collection_id UUID NOT NULL REFERENCES workflow_collections(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status TEXT NOT NULL, -- "running", "completed", "failed", "cancelled"
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    error TEXT
);

-- Workflow-level execution within a collection run
CREATE TABLE workflow_executions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    collection_run_id UUID NOT NULL REFERENCES collection_runs(id) ON DELETE CASCADE,
    workflow_id UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status TEXT NOT NULL, -- "pending", "running", "completed", "failed"
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    outputs JSONB, -- { "variable_name": {...} } for downstream workflows
    error TEXT
);

-- Link existing agent_executions to workflow_executions
-- Add column to agent_executions (compatible with current schema)
ALTER TABLE agent_executions
ADD COLUMN workflow_execution_id UUID REFERENCES workflow_executions(id) ON DELETE CASCADE;

-- Execution variables (for text editor variable capture)
CREATE TABLE execution_variables (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    collection_run_id UUID REFERENCES collection_runs(id) ON DELETE CASCADE,
    workflow_execution_id UUID REFERENCES workflow_executions(id) ON DELETE CASCADE,
    step_execution_id UUID REFERENCES agent_executions(id) ON DELETE CASCADE, -- reuse agent_executions
    variable_name TEXT NOT NULL,
    variable_path TEXT NOT NULL, -- "$workflow_a.step1.analysis"
    value JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_exec_vars_lookup
ON execution_variables(collection_run_id, workflow_execution_id, variable_name);

CREATE INDEX idx_exec_vars_path
ON execution_variables(collection_run_id, variable_path);

-- Add execution_mode to workflows table (for controlling step execution)
ALTER TABLE workflows
ADD COLUMN execution_mode TEXT NOT NULL DEFAULT 'parallel'; -- "sequential" or "parallel"

-- Add execution_mode to workflow_steps table (for controlling multi-agent execution)
ALTER TABLE workflow_steps
ADD COLUMN execution_mode TEXT; -- NULL = inherit from workflow, "sequential"/"parallel" = override

-- Execution mode cascade (configurable all the way up):
-- Collection level: workflow_collections.execution_mode controls workflow execution
-- Workflow level: workflows.execution_mode controls step execution
-- Step level: workflow_steps.execution_mode controls multi-agent execution
```

### Multi-Agent Step Support (Flexible)

```sql
-- Allow multiple agents per step (backwards compatible)
CREATE TABLE workflow_step_agents (
    step_id UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    execution_strategy TEXT NOT NULL, -- "sequential", "parallel", "vote"
    agent_order INT NOT NULL DEFAULT 0, -- for sequential execution
    PRIMARY KEY (step_id, agent_id)
);

-- Migration: populate from existing workflow_steps.agent_id
-- INSERT INTO workflow_step_agents (step_id, agent_id, execution_strategy)
-- SELECT id, agent_id, 'sequential' FROM workflow_steps WHERE agent_id IS NOT NULL;

-- Eventually deprecate workflow_steps.agent_id (nullable for backwards compat)
```

---

## Variable Scope & Resolution

### Hierarchical Variable Namespace

```
$collection.run_id                          -- Collection-level metadata
$collection.started_at
$collection.status

$workflow_a.status                          -- Workflow-level
$workflow_a.started_at
$workflow_a.outputs.analysis                -- Named output from workflow

$workflow_a.step1.output                    -- Step-level
$workflow_a.step1.agent_a.output            -- Agent-level (multi-agent steps)
$workflow_a.step1.agents[0].output          -- Array syntax (parallel agents)
```

### Resolution Algorithm (extend current `resolve_variables()`)

```rust
// Current: src/server/dag_executor/mod.rs:128-192
// Resolves: {variable_name.path.to.field}

// New: multi-tier resolution
fn resolve_variables_hierarchical(
    template: &str,
    collection_vars: &HashMap<String, JsonValue>, // $collection.*
    workflow_vars: &HashMap<String, JsonValue>,   // $workflow_name.*
    step_vars: &HashMap<String, JsonValue>,       // $step_name.*
    current_step_outputs: &HashMap<String, JsonValue>, // {variable_name} (current workflow)
) -> Result<String> {
    // 1. Extract all {variable} patterns
    // 2. Match scope:
    //    - {$collection.X} → look in collection_vars
    //    - {$workflow_name.X} → look in workflow_vars
    //    - {variable_name} → look in current_step_outputs (current behavior)
    // 3. Support dot-path navigation: {$workflow_a.outputs.analysis.findings[0].title}
    // 4. Leave unresolved as-is (for debugging)
}
```

### Variable Storage Flow

```
Collection Run Start:
  ├─ collection_run row created (status: "running")
  └─ execution_variables: { variable_path: "$collection.run_id", value: "uuid..." }

Workflow A Execution:
  ├─ workflow_execution row created
  ├─ Execute workflow DAG (existing logic)
  │  ├─ Step 1 completes → agent_execution.structured_output stored
  │  └─ Step 2 completes → agent_execution.structured_output stored
  ├─ Aggregate outputs → workflow_execution.outputs = { "analysis": {...} }
  └─ execution_variables: {
      variable_path: "$workflow_a.outputs.analysis",
      value: {...},
      workflow_execution_id: workflow_a_id
    }

Workflow B Execution (depends on Workflow A):
  ├─ Load prior workflow outputs from execution_variables
  ├─ Resolve {$workflow_a.outputs.analysis} in prompts
  └─ Execute DAG with resolved variables
```

---

## Topology Sorting & Execution

### Collection-Level DAG Executor

**File:** `src/server/collection_dag_executor.rs` (new)

```rust
pub struct CollectionDagExecutor {
    db_pool: PgPool,
    llm_router: Arc<LlmRouter>,
}

impl CollectionDagExecutor {
    /// Execute a workflow collection (DAG of workflows)
    pub async fn execute_collection(
        &self,
        collection_id: Uuid,
        user_id: Uuid,
    ) -> Result<CollectionRunRow> {
        // 1. Load collection + workflows + edges
        let collection = load_collection(collection_id).await?;
        let workflows = load_collection_workflows(collection_id).await?;
        let edges = load_collection_workflow_edges(collection_id).await?;

        // 2. Create collection_run row
        let run_id = create_collection_run(collection_id, user_id).await?;

        // 3. Execute based on execution_mode
        match collection.execution_mode.as_str() {
            "sequential" => {
                self.execute_collection_sequential(run_id, &workflows, &edges, user_id).await?;
            }
            "parallel" => {
                self.execute_collection_parallel(run_id, &workflows, &edges, user_id).await?;
            }
            _ => return Err(anyhow!("Unknown execution mode: {}", collection.execution_mode)),
        }

        // 4. Mark collection_run as completed
        update_collection_run_status(run_id, "completed").await?;

        Ok(get_collection_run(run_id).await?)
    }

    /// Execute workflows sequentially (one-at-a-time)
    async fn execute_collection_sequential(
        &self,
        run_id: Uuid,
        workflows: &[WorkflowRow],
        edges: &[CollectionWorkflowEdgeRow],
        user_id: Uuid,
    ) -> Result<()> {
        // Topological sort
        let sorted_workflow_ids = topological_sort_workflows(workflows, edges)?;

        let mut completed_workflows: HashMap<Uuid, WorkflowExecutionRow> = HashMap::new();

        for workflow_id in sorted_workflow_ids {
            // Collect variable outputs from completed workflows
            let prior_workflow_outputs = collect_workflow_outputs(&completed_workflows).await?;

            // Execute workflow
            let workflow_exec = execute_workflow_in_collection(
                run_id,
                workflow_id,
                user_id,
                &prior_workflow_outputs,
            ).await?;

            completed_workflows.insert(workflow_id, workflow_exec);
        }

        Ok(())
    }

    /// Execute workflows in parallel (respecting DAG dependencies)
    async fn execute_collection_parallel(
        &self,
        run_id: Uuid,
        workflows: &[WorkflowRow],
        edges: &[CollectionWorkflowEdgeRow],
        user_id: Uuid,
    ) -> Result<()> {
        // Build dependency graph
        let mut in_degree: HashMap<Uuid, usize> = HashMap::new();
        let mut children: HashMap<Uuid, Vec<Uuid>> = HashMap::new();

        for workflow in workflows {
            in_degree.insert(workflow.id, 0);
            children.insert(workflow.id, Vec::new());
        }

        for edge in edges {
            children.get_mut(&edge.from_workflow_id).unwrap().push(edge.to_workflow_id);
            *in_degree.get_mut(&edge.to_workflow_id).unwrap() += 1;
        }

        // Shared state
        let completed = Arc::new(RwLock::new(HashMap::new()));
        let in_degree = Arc::new(RwLock::new(in_degree));

        // Find entry workflows (no dependencies)
        let ready: Vec<Uuid> = {
            let deg = in_degree.read().await;
            deg.iter()
                .filter(|(_, &d)| d == 0)
                .map(|(&id, _)| id)
                .collect()
        };

        // Execute entry workflows in parallel
        let mut handles = vec![];
        for workflow_id in ready {
            let run_id = run_id.clone();
            let user_id = user_id.clone();
            let completed = Arc::clone(&completed);
            let in_degree = Arc::clone(&in_degree);
            let children = children.clone();

            handles.push(tokio::spawn(async move {
                self.execute_workflow_with_cascade(
                    run_id,
                    workflow_id,
                    user_id,
                    completed,
                    in_degree,
                    children,
                ).await
            }));
        }

        // Wait for all workflows to complete
        futures::future::join_all(handles).await;

        Ok(())
    }

    /// Execute a workflow and cascade to children when complete
    async fn execute_workflow_with_cascade(
        &self,
        run_id: Uuid,
        workflow_id: Uuid,
        user_id: Uuid,
        completed: Arc<RwLock<HashMap<Uuid, WorkflowExecutionRow>>>,
        in_degree: Arc<RwLock<HashMap<Uuid, usize>>>,
        children: HashMap<Uuid, Vec<Uuid>>,
    ) -> Result<()> {
        // Collect prior workflow outputs
        let prior_workflow_outputs = {
            let comp = completed.read().await;
            collect_workflow_outputs(&*comp).await?
        };

        // Execute this workflow
        let workflow_exec = execute_workflow_in_collection(
            run_id,
            workflow_id,
            user_id,
            &prior_workflow_outputs,
        ).await?;

        // Mark as completed
        {
            let mut comp = completed.write().await;
            comp.insert(workflow_id, workflow_exec);
        }

        // Decrement in_degree for children and spawn ready children
        let ready_children: Vec<Uuid> = {
            let mut deg = in_degree.write().await;
            let mut ready = vec![];

            for &child_id in &children[&workflow_id] {
                let d = deg.get_mut(&child_id).unwrap();
                *d -= 1;
                if *d == 0 {
                    ready.push(child_id);
                }
            }

            ready
        };

        // Spawn ready children
        let mut handles = vec![];
        for child_id in ready_children {
            let run_id = run_id.clone();
            let user_id = user_id.clone();
            let completed = Arc::clone(&completed);
            let in_degree = Arc::clone(&in_degree);
            let children = children.clone();

            handles.push(tokio::spawn(async move {
                self.execute_workflow_with_cascade(
                    run_id,
                    child_id,
                    user_id,
                    completed,
                    in_degree,
                    children,
                ).await
            }));
        }

        // Wait for all children to complete
        futures::future::join_all(handles).await;

        Ok(())
    }
}

/// Topology sort for workflows (adapted from workflow_steps logic)
fn topological_sort_workflows(
    workflows: &[WorkflowRow],
    edges: &[CollectionWorkflowEdgeRow],
) -> Result<Vec<Uuid>> {
    // Kahn's algorithm (same as current implementation)
    // src/server/dag_executor/mod.rs:64-102

    let mut in_degree: HashMap<Uuid, usize> = HashMap::new();
    let mut adj_list: HashMap<Uuid, Vec<Uuid>> = HashMap::new();

    // Initialize
    for workflow in workflows {
        in_degree.insert(workflow.id, 0);
        adj_list.insert(workflow.id, Vec::new());
    }

    // Build adjacency list
    for edge in edges {
        adj_list.get_mut(&edge.from_workflow_id)
            .unwrap()
            .push(edge.to_workflow_id);
        *in_degree.get_mut(&edge.to_workflow_id).unwrap() += 1;
    }

    // Find entry workflows (in_degree == 0)
    let mut queue: VecDeque<Uuid> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&id, _)| id)
        .collect();

    // Sort entry workflows by display_order
    queue.make_contiguous().sort_by_key(|&id| {
        workflows.iter()
            .find(|w| w.id == id)
            .map(|w| w.display_order)
            .unwrap_or(0)
    });

    let mut sorted = Vec::new();

    while let Some(current) = queue.pop_front() {
        sorted.push(current);

        for &next in &adj_list[&current] {
            let deg = in_degree.get_mut(&next).unwrap();
            *deg -= 1;
            if *deg == 0 {
                queue.push_back(next);
            }
        }
    }

    // Cycle detection
    if sorted.len() != workflows.len() {
        return Err(anyhow!("Cycle detected in workflow collection DAG"));
    }

    Ok(sorted)
}

/// Execute a single workflow within a collection run
async fn execute_workflow_in_collection(
    collection_run_id: Uuid,
    workflow_id: Uuid,
    user_id: Uuid,
    prior_workflow_outputs: &HashMap<String, JsonValue>,
) -> Result<WorkflowExecutionRow> {
    // 1. Create workflow_execution row
    let workflow_exec_id = create_workflow_execution(
        collection_run_id,
        workflow_id,
        user_id,
    ).await?;

    // 2. Execute workflow DAG (reuse existing logic)
    //    File: src/server/dag_executor/mod.rs:390-540
    //    OR: src/server/hub/dag/mod.rs (new engine-based)

    // Pass prior_workflow_outputs to variable resolution
    let outputs = execute_workflow_dag(
        workflow_id,
        user_id,
        prior_workflow_outputs, // NEW: cross-workflow variables
        Some(workflow_exec_id), // Link agent_executions to workflow_execution
    ).await?;

    // 3. Aggregate step outputs into workflow-level outputs
    let workflow_outputs = aggregate_step_outputs(&outputs).await?;

    // 4. Store in workflow_execution.outputs
    update_workflow_execution_outputs(workflow_exec_id, &workflow_outputs).await?;

    // 5. Store in execution_variables for variable resolution
    store_workflow_variables(collection_run_id, workflow_exec_id, &workflow_outputs).await?;

    Ok(get_workflow_execution(workflow_exec_id).await?)
}
```

---

## API Endpoints

**File:** `src/server/api/collections/mod.rs` (new)

```rust
// POST /api/collections
pub async fn create_collection(
    State(app): State<AppState>,
    Json(req): Json<CreateCollectionRequest>,
) -> Result<Json<WorkflowCollectionRow>> {
    // Create collection + members + edges
}

// GET /api/collections/:id
pub async fn get_collection(
    State(app): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<WorkflowCollectionResponse>> {
    // Return collection + workflows + edges (for frontend DAG rendering)
}

// POST /api/collections/:id/run
pub async fn run_collection(
    State(app): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<CollectionRunRow>> {
    // Execute the collection DAG
    let executor = CollectionDagExecutor::new(app.db_pool.clone(), app.llm_router.clone());
    let run = executor.execute_collection(id, user_id).await?;
    Ok(Json(run))
}

// GET /api/collections/runs/:run_id/status
pub async fn get_collection_run_status(
    State(app): State<AppState>,
    Path(run_id): Path<Uuid>,
) -> Result<Json<CollectionRunStatusResponse>> {
    // Return run + workflow_executions + agent_executions (for progress tracking)
}

// GET /api/collections/runs/:run_id/variables
pub async fn get_collection_variables(
    State(app): State<AppState>,
    Path(run_id): Path<Uuid>,
) -> Result<Json<Vec<ExecutionVariableRow>>> {
    // Return all execution_variables for this run (for text editor variable picker)
}
```

---

## Multi-Agent Step Execution (Flexible)

**Strategies:**

1. **Sequential:** Agent A → Agent B → Agent C (pipeline)
2. **Parallel:** All agents run simultaneously, outputs merged
3. **Vote:** All agents run, LLM arbitrates or majority vote

**Implementation:**

**File:** `src/server/dag_executor/mod.rs` (enhance existing step execution)

```rust
// Current: execute_step() calls single agent
// New: execute_step_multi_agent() handles multiple agents

async fn execute_step_multi_agent(
    step: &WorkflowStepRow,
    agents: Vec<AgentRow>, // loaded from workflow_step_agents
    execution_strategy: &str,
    context: &ExecutionContext,
) -> Result<StepOutput> {
    match execution_strategy {
        "sequential" => execute_sequential(step, agents, context).await,
        "parallel" => execute_parallel(step, agents, context).await,
        "vote" => execute_vote(step, agents, context).await,
        _ => Err(anyhow!("Unknown execution strategy: {}", execution_strategy)),
    }
}

async fn execute_sequential(
    step: &WorkflowStepRow,
    agents: Vec<AgentRow>,
    context: &ExecutionContext,
) -> Result<StepOutput> {
    let mut input = context.resolved_prompt.clone();
    let mut outputs = Vec::new();

    for (i, agent) in agents.iter().enumerate() {
        // Execute agent with current input
        let ae = execute_agent(agent, &input, context).await?;
        outputs.push(ae.output.clone());

        // Next agent's input = current agent's output
        input = ae.output.clone().unwrap_or_default();
    }

    // Final output = last agent's output
    Ok(StepOutput {
        variable_name: step.output_variable_name.clone(),
        structured_output: parse_json(&input),
        raw_output: input,
    })
}

async fn execute_parallel(
    step: &WorkflowStepRow,
    agents: Vec<AgentRow>,
    context: &ExecutionContext,
) -> Result<StepOutput> {
    // Spawn all agents concurrently
    let handles: Vec<_> = agents.iter().map(|agent| {
        let agent = agent.clone();
        let prompt = context.resolved_prompt.clone();
        tokio::spawn(async move {
            execute_agent(&agent, &prompt, context).await
        })
    }).collect();

    // Wait for all to complete
    let results: Vec<_> = futures::future::join_all(handles).await;

    // Merge outputs into array
    let outputs: Vec<JsonValue> = results.iter()
        .filter_map(|r| r.as_ref().ok())
        .filter_map(|ae| ae.structured_output.clone())
        .collect();

    Ok(StepOutput {
        variable_name: step.output_variable_name.clone(),
        structured_output: Some(json!(outputs)),
        raw_output: serde_json::to_string_pretty(&outputs)?,
    })
}

async fn execute_vote(
    step: &WorkflowStepRow,
    agents: Vec<AgentRow>,
    context: &ExecutionContext,
) -> Result<StepOutput> {
    // 1. Execute all agents in parallel
    let outputs = execute_parallel(step, agents, context).await?;

    // 2. LLM arbitration
    let arbitration_prompt = format!(
        "Given these {} agent outputs, select the best one or synthesize a consensus:\n\n{}",
        outputs.structured_output.as_ref().unwrap().as_array().unwrap().len(),
        serde_json::to_string_pretty(&outputs.structured_output)?
    );

    let arbitrator = get_arbitrator_agent().await?; // Config: which agent arbitrates
    let final_output = execute_agent(&arbitrator, &arbitration_prompt, context).await?;

    Ok(StepOutput {
        variable_name: step.output_variable_name.clone(),
        structured_output: final_output.structured_output,
        raw_output: final_output.output.unwrap_or_default(),
    })
}
```

---

## Migration Steps

### Phase 1: Database Schema (Non-Breaking)

1. Create new tables (workflow_collections, collection_workflows, collection_workflow_edges, etc.)
2. Add `workflow_execution_id` column to `agent_executions` (nullable, no FK yet)
3. Create `workflow_step_agents` table
4. Migrate existing `workflow_steps.agent_id` → `workflow_step_agents`

**Files:**
- `migrations/` (new SQL files)
- `src/db/mod.rs` (add new row types)
- `src/db/traits/mod.rs` (add new repository traits)

### Phase 2: Collection DAG Executor + Parallel Execution

1. Implement `collection_dag_executor.rs` (parallel workflow execution)
2. Add topology sort for workflows
3. Implement workflow-level variable resolution
4. Link `agent_executions` to `workflow_executions`
5. **Enhance existing workflow executor for parallel step execution**
6. **Add parallel multi-agent execution within steps**

**Files:**
- `src/server/collection_dag_executor.rs` (new - parallel workflow execution)
- `src/server/dag_executor/mod.rs` (enhance for parallel step execution + `workflow_execution_id`)
- `src/server/hub/dag/mod.rs` (if using new engine)

**Parallel execution changes:**
- Collections: Add `execute_collection_parallel()` (respects workflow DAG)
- Workflows: Update `execute_workflow()` to support parallel step execution (respects step DAG)
- Steps: Update `execute_step_multi_agent()` to support parallel agent execution

### Phase 3: API Endpoints

1. Create `/api/collections` CRUD endpoints
2. Create `/api/collections/:id/run` execution endpoint
3. Create `/api/collections/runs/:id/status` status endpoint
4. Create `/api/collections/runs/:id/variables` variable retrieval

**Files:**
- `src/server/api/collections/mod.rs` (new)
- `src/server/api/mod.rs` (register routes)

### Phase 4: Multi-Agent Step Execution (Optional, Future)

1. Implement `execute_step_multi_agent()`
2. Add strategies: sequential, parallel, vote
3. Update frontend to configure multi-agent steps

**Files:**
- `src/server/dag_executor/mod.rs` (enhance step execution)
- `frontend/src/components/WorkflowStepEditor.tsx` (add multi-agent UI)

### Phase 5: Frontend (Collection Builder)

1. Create collection editor (DAG of workflows)
2. Render collection DAG (like workflow step DAG)
3. Variable picker for text editor (future feature)

**Files:**
- `frontend/src/pages/CollectionEditor.tsx` (new)
- `frontend/src/components/CollectionDAG.tsx` (new)
- `frontend/src/components/VariablePicker.tsx` (new)

---

## Critical Files to Modify

### Backend (Rust)

| File | Changes |
|------|---------|
| `migrations/*.sql` | New tables: workflow_collections, collection_workflows, collection_workflow_edges, collection_runs, workflow_executions, execution_variables, workflow_step_agents |
| `src/db/mod.rs` | Add row types: `WorkflowCollectionRow`, `CollectionWorkflowRow`, `CollectionWorkflowEdgeRow`, `CollectionRunRow`, `WorkflowExecutionRow`, `ExecutionVariableRow`, `WorkflowStepAgentRow` |
| `src/db/traits/mod.rs` | Add repository traits for new tables |
| `src/db/repositories/*.rs` | Implement repository functions (CRUD + topo sort queries) |
| `src/server/collection_dag_executor.rs` | **NEW:** Collection DAG executor (topology sort, workflow orchestration) |
| `src/server/dag_executor/mod.rs` | Enhance `execute_workflow()` to accept `workflow_execution_id`, link agent_executions |
| `src/server/dag_executor/mod.rs` | Enhance `resolve_variables()` for hierarchical scopes ($collection, $workflow, $step) |
| `src/server/dag_executor/mod.rs` | Add `execute_step_multi_agent()` for flexible multi-agent strategies |
| `src/server/api/collections/mod.rs` | **NEW:** API endpoints for collections (CRUD, run, status, variables) |
| `src/server/api/mod.rs` | Register `/api/collections` routes |

### Frontend (React)

| File | Changes |
|------|---------|
| `frontend/src/api/api.ts` | Add typed endpoints: `api.collections.*` (list, get, create, update, delete, run, status, variables) |
| `frontend/src/pages/CollectionEditor.tsx` | **NEW:** Collection builder (DAG of workflows, edge creation) |
| `frontend/src/components/CollectionDAG.tsx` | **NEW:** Render collection DAG (reuse workflow DAG rendering logic) |
| `frontend/src/components/VariablePicker.tsx` | **NEW:** Variable picker for text editor (future feature) |
| `frontend/src/types.ts` | Add types: `WorkflowCollection`, `CollectionWorkflow`, `CollectionWorkflowEdge`, `CollectionRun`, `WorkflowExecution`, `ExecutionVariable` |

---

## Verification Plan

### Unit Tests

1. **Topology sort:** Test cycle detection, entry workflow ordering
2. **Variable resolution:** Test hierarchical scopes ($collection, $workflow, $step)
3. **Multi-agent execution:** Test sequential, parallel, vote strategies

**Files:**
- `src/server/collection_dag_executor/tests.rs` (new)
- `src/server/dag_executor/tests.rs` (enhance)

### Integration Tests

1. **Collection execution:** Create collection → run → verify all workflows executed in order
2. **Cross-workflow variables:** Workflow A outputs → Workflow B consumes
3. **Multi-agent steps:** Step with multiple agents → verify outputs merged correctly

**Files:**
- `tests/integration/collection_execution_tests.rs` (new)

### Manual Testing

1. Create a collection with 3 workflows (DAG: A → B, A → C)
2. Workflow A outputs `{ "analysis": {...} }`
3. Workflow B uses `{$workflow_a.outputs.analysis}` in prompt
4. Run collection → verify B receives A's output
5. Check `execution_variables` table for stored variables

---

## Rate Limiting for Parallel Execution

When running workflows/steps/agents in parallel, rate limiting is handled by:

1. **LLM Provider level:** Existing `LlmRouter` has built-in rate limiting per provider
2. **External rate limiter:** Can add semaphore-based concurrency limits at collection/workflow/step level

**Example:**
```rust
// In collection_dag_executor.rs
const MAX_CONCURRENT_WORKFLOWS: usize = 5; // Config value

let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_WORKFLOWS));

// Before spawning workflow execution
let permit = semaphore.acquire().await?;
tokio::spawn(async move {
    let _permit = permit; // Hold until workflow completes
    execute_workflow(...).await
});
```

**Configuration options (future):**
- `max_concurrent_workflows` in `workflow_collections` table
- `max_concurrent_steps` in `workflows` table
- `max_concurrent_agents` in `workflow_steps` table

---

## Future Enhancements

1. **Conditional branching:** Add `condition` field to `collection_workflow_edges` (e.g., run Workflow B only if Workflow A output meets criteria)
2. **Dynamic workflow creation:** Allow workflows to spawn new workflows at runtime
3. **Variable editor UI:** Rich text editor with variable autocomplete ($collection.*, $workflow.*, $step.*)
4. **Workflow templates:** Reusable workflow collections (like blueprints)
5. **Rollback/retry:** Retry failed workflows, rollback collection to previous state
6. **Rate limiting configuration:** Per-collection/workflow/step concurrency limits

---

## Timeline Estimate

**Phase 1 (Database):** 1-2 days
**Phase 2 (Executor):** 2-3 days
**Phase 3 (API):** 1-2 days
**Phase 4 (Multi-agent):** 2-3 days (optional, can defer)
**Phase 5 (Frontend):** 3-4 days

**Total:** 9-14 days (without multi-agent: 7-9 days)

---

## Visual Reference: New Schema

```
users
  │
  ├─ workflow_collections ──┬─ collection_workflows ──► workflows
  │                         │                              │
  │                         └─ collection_workflow_edges   │
  │                                (DAG of workflows)      │
  │                                                         │
  ├─ collection_runs ──┬─ workflow_executions ────────────┘
  │                    │      │
  │                    │      └─ agent_executions (existing)
  │                    │             │
  │                    │             └─ execution_messages (existing)
  │                    │
  │                    └─ execution_variables (variable capture)
  │
  ├─ workflows ──┬─ workflow_steps ──┬─ workflow_step_edges (DAG of steps)
  │              │                    │
  │              │                    └─ workflow_step_agents (multi-agent support)
  │              │                           │
  │              │                           └─ agents
  │              │
  │              └─ (existing step config: prompt_templates, output_schemas, etc.)
  │
  └─ (existing tables: agents, tools, documents, chat_sessions, etc.)
```

---

## Complete System Architecture Summary

```
┌─────────────────────────────────────────────────────────────────┐
│                    WORKFLOW COLLECTIONS                         │
│  - DAG of workflows (topology-sorted)                           │
│  - execution_mode: "sequential" | "parallel" (configurable)     │
│  - Variables: {$collection.run_id}, {$collection.status}        │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         │ If parallel: spawn independent workflows concurrently
                         │ If sequential: one-at-a-time
                         │
         ┌───────────────┼───────────────┬───────────────┐
         │               │               │               │
         ▼               ▼               ▼               ▼
    ┌─────────┐     ┌─────────┐     ┌─────────┐     ┌─────────┐
    │Workflow │────►│Workflow │────►│Workflow │     │Workflow │
    │    A    │     │    B    │     │    D    │     │    E    │
    │ (mode:  │     │ (mode:  │     │ (mode:  │     │ (mode:  │
    │parallel)│     │parallel)│     │parallel)│     │inherit) │
    └────┬────┘     └────┬────┘     └────┬────┘     └────┬────┘
         │               │               │               │
         │ Each workflow = DAG of steps  │               │
         │ Variables: {$workflow_a.outputs.analysis}     │
         │                                               │
         ▼                                               │
    ┌──────────────────────────────────────────┐        │
    │  Workflow A                              │        │
    │  - DAG of steps (topology-sorted)        │        │
    │  - execution_mode controls step execution│        │
    │                                          │        │
    │  ┌──────┐      ┌──────┐      ┌──────┐  │        │
    │  │Step 1│─────►│Step 2│─────►│Step 4│  │        │
    │  │(mode:│      │(mode:│      │(mode:│  │        │
    │  │para) │      │para) │      │inher)│  │        │
    │  └──────┘      └───┬──┘      └──────┘  │        │
    │                    │                    │        │
    │                    ▼                    │        │
    │                ┌──────┐                 │        │
    │                │Step 3│                 │        │
    │                │(mode:│                 │        │
    │                │seq)  │                 │        │
    │                └───┬──┘                 │        │
    │                    │                    │        │
    │                    │ Each step = 1+ agents       │
    │                    │ Variables: {$workflow_a.step1.output}
    │                    │                    │        │
    │                    ▼                    │        │
    │            ┌─────────────────┐          │        │
    │            │  Multi-Agent    │          │        │
    │            │  Step Execution │          │        │
    │            │─────────────────│          │        │
    │            │ ■ Agent A       │          │        │
    │            │ ■ Agent B       │          │        │
    │            │ ■ Agent C       │          │        │
    │            │                 │          │        │
    │            │ Strategies:     │          │        │
    │            │ - Sequential    │          │        │
    │            │ - Parallel      │          │        │
    │            │ - Vote          │          │        │
    │            └─────────────────┘          │        │
    └──────────────────────────────────────────┘        │
                                                        │
                Variables: {$workflow_a.step2.agent_a.output}
```

## End of Plan

This plan provides:
- ✅ Multi-tier DAG (collections → workflows → steps → agents)
- ✅ Topology sorting at collection and workflow levels
- ✅ **Configurable execution modes at ALL levels (collection/workflow/step)**
- ✅ **Parallel execution that respects DAG dependencies**
- ✅ **Full variable granularity (collection/workflow/step/agent)**
- ✅ Flexible multi-agent execution strategies (sequential/parallel/vote)
- ✅ Variable capture for cross-workflow data flow
- ✅ Backwards compatible with existing workflows
- ✅ Pipelines left untouched (legacy, not used)
- ✅ Rate limiting support for parallel execution
