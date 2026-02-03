# Legacy Systems Audit

**Last Updated:** 2025-02-03
**Purpose:** Complete inventory of legacy code to be removed/migrated

---

## 🔴 LEGACY: Agent Pool System

**Location:** `src/agents/` (~8,095 lines)

**Components:**
- `pool.rs` - AgentPool for spawning/managing agents
- `dispatcher.rs` - Dispatcher for routing commands to agents
- `agent.rs` - Individual agent execution loops
- `executor.rs` - Agent task executor (react loop)
- `protocol.rs` - AgentCommand, AgentResponse enums
- `channels.rs` - Channel-based communication
- `pipeline.rs` - Old pipeline manager
- `schedule.rs` - Schedule manager for periodic tasks
- `cluster.rs` - Agent clustering
- `router_agent.rs` - Router agent implementation
- `gatekeeper.rs` - Gatekeeper for multi-agent rooms
- `roles.rs` - Role definitions
- `tool_router.rs` - Tool routing system
- `execution_tools.rs` - Tools for agent execution

**Status:**
- ❌ Used by old pipeline system (pipelines table)
- ❌ Used by agent management tools in tools.rs
- ❌ Used by response consumer in orchestrator.rs
- ✅ NOT used by new chat/hub system

**Migration Path:** Replace with ExecutionEngine + strategies

---

## 🔴 LEGACY: Response Consumer

**Location:** `src/server/orchestrator.rs` (lines 23-482, ~460 lines)

**Purpose:** Drains AgentResponse messages from the old agent pool dispatcher and:
- Stores results in `state.task_results`
- Broadcasts task/agent updates to WebSocket
- Handles context requests (auto-reads files)
- Triggers pipeline advancement (lines 220-430)

**Dependencies:**
- `state.dispatcher` - Agent pool dispatcher
- `state.task_results` - Response storage map
- Old agent pool system

**Status:**
- ❌ Only needed for old agent pool
- ❌ Spawned at startup in server/mod.rs:62
- ✅ NOT needed for chat/hub system

**Can Remove:** YES (if agent pool is removed)

---

## 🔴 LEGACY: Schedule Runner

**Location:** `src/server/orchestrator.rs` (lines 484-552, ~70 lines)

**Purpose:** Runs periodic scheduled tasks by:
- Checking `schedule_manager` every 60 seconds
- Creating TaskAssignment for due schedules
- Sending via old agent pool dispatcher

**Dependencies:**
- `state.dispatcher` - Agent pool
- `state.schedule_manager` - Schedule storage
- Old agent pool system

**Status:**
- ❌ Uses old agent pool for execution
- ❌ Spawned at startup in server/mod.rs:64
- ✅ Could be migrated to use ExecutionEngine

**Can Remove:** YES (if no one uses schedules)

---

## 🟡 LEGACY: Agent Management Tools

**Location:** `src/server/tools.rs` (~1,000+ lines of legacy tools)

**Legacy Tools (use agent pool):**
- `list_agents` - List agents in pool
- `create_agent` - Spawn new agent in pool
- `create_agents` - Batch agent creation
- `assign_task` - Assign work to pool agent
- `get_task_result` - Retrieve agent response
- `respond_to_approval` - Approve agent actions
- `list_roles` - List available agent roles

**Still Valid Tools (don't use agent pool):**
- ✅ `create_doc` - Document creation
- ✅ `update_doc` - Document updates
- ✅ `search_docs` - Document search
- ✅ `submit_prd` - PRD submission
- ✅ `submit_ticket` - Ticket submission

**Status:**
- ❌ Agent management tools reference `state.pool` and `state.dispatcher`
- ✅ Document tools are fine
- ⚠️ May break existing agents configured to use these tools

**Can Remove:** YES (but check agent configurations first)

---

## 🟡 LEGACY: Old DAG Executor

**Location:** `src/server/dag_executor.rs` (~46,332 bytes)

**Status:** PARTIAL LEGACY
- ✅ Utility functions STILL USED by hub/dag.rs:
  - `topological_sort()`
  - `resolve_variables()`
  - `compose_prompt()`
  - `find_entry_steps()`
  - etc.
- ❌ Main executor LEGACY:
  - `execute_workflow()` function (line 628)
  - Has own react loop (replaced by ExecutionEngine)

**Migration Path:**
- Keep utility functions
- Remove `execute_workflow()` and execution logic
- Everything uses `hub/dag.rs::execute_workflow_via_engine()` instead

---

## 🟡 LEGACY: Room Executor

**Location:** `src/server/room_executor.rs` (~22,282 bytes)

**Purpose:** Multi-agent room conversations

**Status:** PARTIAL LEGACY
- ✅ Still called from API (api.rs:4943)
- ❌ Has own execution loop (not using ExecutionEngine)
- ⚠️ Could be migrated to use RoomSpeakerStrategy + ExecutionEngine

**Notes:** hub/strategies/room_speaker.rs exists but might not be fully integrated

---

## 🟡 LEGACY: Router Service

**Location:** `src/server/router_service.rs` (~14,238 bytes)

**Purpose:** Tool call routing to cluster agents

**Status:** UNCLEAR
- May be replaced by RouterStrategy in hub
- Needs investigation

---

## 🔴 LEGACY: Old Pipeline System

**Database Table:** `pipelines` + `pipeline_stages` + `pipeline_runs`

**vs NEW:** `workflows` + `workflow_steps`

**How They Differ:**
- **OLD Pipelines:** Use agent pool, depend on response consumer for advancement
- **NEW Workflows:** Use ExecutionEngine, self-contained execution

**Status:**
- ✅ **CONFIRMED LEGACY** - User migrated to workflows
- ❌ `pipelines` table has 0 rows (empty)
- ❌ Response consumer advances old pipelines (orchestrator.rs:223)
- ✅ Workflows use hub (hub/dag.rs)
- ✅ API endpoints still exist but reference empty tables

**Can Remove:** YES - User confirmed workflows are the current system

**What to Delete:**
1. Pipeline advancement logic in response consumer (orchestrator.rs:223-430)
2. `PipelineManager` from AppState
3. Pipeline-related API endpoints (or keep for backward compatibility)
4. Eventually: Drop `pipelines`, `pipeline_stages`, `pipeline_runs` tables from DB

---

## 🔴 LEGACY: AppState Fields

**Location:** `src/server/state.rs`

**Legacy Fields:**
```rust
pub pool: Option<Arc<Mutex<AgentPool>>>,              // Line 117
pub dispatcher: Option<Arc<Mutex<Dispatcher>>>,       // Line 119
pub task_results: Arc<RwLock<HashMap<Uuid, AgentResponse>>>, // Line 121
pub cluster_manager: Arc<RwLock<ClusterManager>>,     // May be legacy
pub pipeline_manager: Arc<RwLock<PipelineManager>>,   // May be legacy (old pipelines)
pub schedule_manager: Arc<RwLock<ScheduleManager>>,   // Line 129 (legacy)
```

**Still Valid:**
```rust
pub llm_provider: // Will add this
pub repo: Arc<dyn ServerRepo>,
pub response_streams: // For SSE streaming
pub feed_tx, task_tx, agent_tx: // WebSocket broadcasts
```

---

## 🟢 MODERN: Hub System (Keep!)

**Location:** `src/server/hub/`

**Components:**
- ✅ `engine.rs` - Unified ExecutionEngine (the core!)
- ✅ `strategy.rs` - ExecutionStrategy trait
- ✅ `strategies/chat.rs` - ChatStrategy for conversations
- ✅ `strategies/dag_step.rs` - DagStepStrategy for workflows
- ✅ `strategies/router.rs` - RouterStrategy for classification
- ✅ `strategies/room_speaker.rs` - RoomSpeakerStrategy for multi-agent
- ✅ `streaming.rs` - StreamSink for SSE/WebSocket
- ✅ `recorder.rs` - ExecutionRecorder for logging
- ✅ `mod.rs` - `run_chat()` function (entry point)
- ✅ `dag.rs` - `execute_workflow_via_engine()` for workflows
- ✅ `pipeline_advance.rs` - Pipeline stage advancement logic

**This is the NEW architecture - DO NOT REMOVE!**

---

## Summary: What Can Be Deleted?

### Phase 1: Safe Deletions (~500 lines)
1. ✅ Response consumer (orchestrator.rs:23-482)
2. ✅ `task_results` from AppState
3. ✅ Schedule runner (orchestrator.rs:484-552)
4. ✅ `schedule_manager` from AppState

### Phase 2: Agent Pool Removal (~9,000+ lines)
1. ✅ Entire `src/agents/` directory
2. ✅ Agent management tools in tools.rs
3. ✅ `pool` and `dispatcher` from AppState
4. ✅ Agent pool initialization in state.rs

### Phase 3: Executor Migration (~100KB)
1. ✅ Old `execute_workflow()` from dag_executor.rs (keep utilities)
2. ⚠️ Migrate room_executor.rs to use RoomSpeakerStrategy
3. ⚠️ Check router_service.rs vs RouterStrategy

### Phase 4: Database Cleanup
1. ✅ Drop `pipelines`, `pipeline_stages`, `pipeline_runs` tables (confirmed unused)
2. ✅ Drop `schedules` table if unused
3. ✅ Remove `PipelineManager` from AppState

---

## Total Legacy Code Estimate

- Agent pool system: ~8,095 lines
- Response consumer: ~460 lines
- Schedule runner: ~70 lines
- Agent tools: ~1,000 lines
- Old executors: ~100KB
- **TOTAL: ~10,000+ lines of legacy code**

---

## Migration Checklist

Before removing agent pool:
- [ ] Check database: `SELECT COUNT(*) FROM pipelines;`
- [ ] Check database: `SELECT COUNT(*) FROM schedules;`
- [ ] Grep codebase: `git grep "state.pool\|state.dispatcher"`
- [ ] Check agent configurations: Do any agents use agent management tools?
- [ ] Test workflows: Ensure DAG execution works via hub
- [ ] Test rooms: Ensure multi-agent conversations work

---

## Recommended Removal Order

1. **Week 1:** Remove response consumer + schedule runner (safe, ~530 lines)
2. **Week 2:** Remove agent management tools (check configurations first)
3. **Week 3:** Remove agent pool system (~8,000 lines)
4. **Week 4:** Migrate executors to hub strategies
5. **Week 5:** Database cleanup + final validation

---

**Next Steps:** Start with Phase 1 (response consumer) - it's 100% safe and removes the most obvious legacy code.
