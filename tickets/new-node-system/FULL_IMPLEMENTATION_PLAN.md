# Dynamic Node System — Full Implementation Plan

## Context

Every node on the canvas starts blank. The user can configure it two ways:
- **Chat path:** Talk to the assistant → assistant determines archetype → configures through conversation
- **Direct pick:** Select an archetype from a UI selector → node switches immediately → chat opens with specialist tools already loaded

Power users pick directly, new users discover through conversation. Same backend, same archetype system, two entry points.

The infrastructure is partially built: step chat sessions exist (find-or-create, messages, WS events), ChatStrategy with StepChatContext routes tools by `execution_mode`, and the documenter archetype is fully wired (tools, dispatch, system prompt, WS events). The goal is to generalize this pattern so all archetypes work through the same system.

Design docs: `tickets/new-node-system/DYNAMIC_TASK_NODE_DESIGN.md`, `NODE_ASSISTANT_PROMPTS.md`, `BOCA_INTEGRATION_PLAN.md`

---

## Phase 1: Generalize the Archetype System

**Goal:** Turn the documenter-specific step chat into a general archetype system. Any `execution_mode` gets its own system prompt block and tool set. Add `set_node_archetype` as the universal transition tool.

### 1a. Base + archetype prompt system

**Create:** `config/protocols/node_assistant/base/system.md`
- The base prompt from `NODE_ASSISTANT_PROMPTS.md` (~350 tokens)
- Identity, archetype routing descriptions, behavioral guidelines
- `{{.System.current_config}}` injection point for graph context
- `{{.System.archetype_block}}` injection point for the active archetype

**Create:** `config/protocols/node_assistant/config.yaml`
- Agent config for the node assistant (model, temperature, max_rounds, context_budget)

**Create:** `config/protocols/node_assistant/documenter/block.md`
- Extract documenter-specific guidance from current `documenter/assistant/system.md`
- This becomes the archetype block injected when mode = "documenter"

**Modify:** `src/config/protocols.rs`
- Add `NODE_ASSISTANT` static config
- Add `NODE_ASSISTANT_BASE` role definition
- Add `NODE_ASSISTANT_DOCUMENTER_BLOCK` for the documenter archetype block content
- Add template vars: `System.archetype_block`, `System.graph_context`

### 1b. Generalize `build_step_system_prompt()`

**Modify:** `src/server/hub/mod.rs` — `build_step_system_prompt()`

Current: hardcoded match on `"documenter"` → loads documenter assistant role. Fallback is generic string.

New:
1. Always load base prompt from `NODE_ASSISTANT_BASE`
2. Build graph context (nodes, edges, selected node state) → inject as `System.graph_context`
3. Build config snapshot per archetype (reuse existing `build_config_snapshot` for documenter, new functions for others)
4. Load archetype block based on `execution_mode` → inject as `System.archetype_block`
5. Resolve template and return

**Create:** `src/server/hub/graph_context.rs`
- `build_graph_context(state, workflow_id, step_id) -> String`
- Loads all steps + edges for the workflow
- Formats: node names, execution modes, connections, selected node's current config
- This is the runtime graph context injected into every step chat prompt

### 1c. Archetype catalog + step config API

**Create:** `src/server/api/archetypes/mod.rs`

`GET /api/archetypes` — static endpoint, returns all available archetypes:
```json
[
  {
    "id": "documenter",
    "name": "Documenter",
    "description": "Research-and-write pipeline that produces structured documents",
    "icon": "file-text",
    "color": "#4A90D9"
  },
  {
    "id": "task_force",
    "name": "Task Force",
    "description": "A team of agents that executes a multi-step mission",
    "icon": "users",
    "color": "#E67E22"
  },
  {
    "id": "belief_capture",
    "name": "Belief Capture",
    "description": "Extracts structured knowledge from upstream results",
    "icon": "lightbulb",
    "color": "#9B59B6"
  },
  {
    "id": "room",
    "name": "Room",
    "description": "Meeting space where agents discuss, debate, or review",
    "icon": "message-circle",
    "color": "#2ECC71"
  }
]
```

Backed by a Rust const/array — no DB. Frontend fetches once on load, caches. Drives both the node selector UI and node visual styling (colors, icons).

**Add to:** `src/server/api/workflows/step_handlers.rs`

`GET /api/workflows/:wid/steps/:sid/config` — unified step config readback:
```typescript
type StepConfig =
  | { archetype: "documenter"; documents: DocDef[] }
  | { archetype: "task_force"; task: string; agents: Agent[] }
  | { archetype: "belief_capture"; focus: string; tags: string[] }
  | { archetype: "room"; purpose: string; members: Member[] }
  | { archetype: null }  // blank node
```

Returns different shape per archetype. Frontend type-discriminates on `archetype` field. For Phase 1, only `documenter` and `null` are populated — others return skeleton responses as archetypes are added in later phases.

### 1d. `set_node_archetype` tool

**Modify:** `src/tools/registry/mod.rs`
- Register `set_node_archetype` tool definition
- Input schema: `{ archetype: string }` with enum of valid values

**Modify:** `src/server/hub/strategies/chat/mod.rs`
- `resolve_step_tools()`: always include `set_node_archetype`, `set_node_name`, `set_node_description` as universal tools alongside archetype-specific ones
- `dispatch_step_tool()`: handle `set_node_archetype` — updates step's `execution_mode` in DB, returns success

**Create:** `src/server/tools/node_assistant/mod.rs`
- `execute_set_archetype(input, repo, ctx) -> Value` — validates archetype, calls `update_step()` with new execution_mode
- `execute_set_name(input, repo, ctx) -> Value`
- `execute_set_description(input, repo, ctx) -> Value`

**Modify:** `src/server/hub/strategies/chat/mod.rs` — `broadcast_documenter_event()`
- Rename to `broadcast_step_event()` — generalize to handle events from any archetype
- Add `ArchetypeChanged { step_id, archetype }` event

**Modify:** `src/server/ws/events.rs`
- Add `WorkflowEventKind::ArchetypeChanged { step_id, archetype: String }`
- Add `WorkflowEventKind::StepNameUpdated { step_id, name: String }`

### 1e. Verify documenter still works

The documenter path must work identically through the new generalized system. The existing `documenter/assistant/system.md` content splits into base + archetype block, but the resolved prompt should be equivalent.

### Files touched (Phase 1)
- **Create:** `config/protocols/node_assistant/base/system.md`, `config/protocols/node_assistant/config.yaml`, `config/protocols/node_assistant/documenter/block.md`
- **Create:** `src/server/hub/graph_context.rs`, `src/server/tools/node_assistant/mod.rs`, `src/server/api/archetypes/mod.rs`
- **Modify:** `src/config/protocols.rs`, `src/server/hub/mod.rs`, `src/server/hub/strategies/chat/mod.rs`, `src/tools/registry/mod.rs`, `src/server/ws/events.rs`, `src/server/tools/mod.rs`, `src/server/api/mod.rs`, `src/server/api/workflows/step_handlers.rs`

### Verification
- `cargo check` + `cargo test` — no regressions
- Existing documenter step chat tests pass
- New tests: `set_node_archetype` tool changes execution_mode, graph context builder formats correctly, base + archetype prompt resolves

---

## Phase 2: Task Force Archetype — Design Time

**Goal:** User can talk to the assistant, select task force archetype, and configure a mission brief with an agent roster. All stored in DB.

### 2a. Migration + DB layer

**Create:** `migrations/0025_task_mission_briefs.sql`

```sql
CREATE TABLE task_mission_briefs (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    step_id uuid NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    task_description text NOT NULL,
    available_capabilities text[] NOT NULL DEFAULT '{}',
    failure_mode text NOT NULL DEFAULT 'fail_fast',
    downstream_context text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE task_agent_roster (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    mission_brief_id uuid NOT NULL REFERENCES task_mission_briefs(id) ON DELETE CASCADE,
    name text NOT NULL,
    role_description text NOT NULL,
    capabilities text[] NOT NULL DEFAULT '{}',
    execution_order integer NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT now()
);
```

**Modify:** `src/db/mod.rs` — `TaskMissionBriefRow`, `TaskAgentRosterRow`, `NewTaskMissionBrief`, `NewTaskAgentRoster`
**Modify:** `src/db/traits/mod.rs` — `TaskForceRepo` trait (CRUD for briefs + roster)
**Modify:** `src/db/pg_repo/mod.rs` — PgRepo impl

### 2b. Task force tools

**Create:** `src/server/tools/task_force/mod.rs`
- `execute_set_task(input, repo, ctx)` — create-or-update mission brief
- `execute_add_agent(input, repo, ctx)` — add agent to roster
- `execute_update_agent(input, repo, ctx)` — update agent name/role/capabilities
- `execute_remove_agent(input, repo, ctx)` — remove agent from roster
- `execute_set_capabilities(input, repo, ctx)` — set available capabilities on brief
- `execute_set_failure_mode(input, repo, ctx)` — set failure_mode on brief

**Modify:** `src/tools/registry/mod.rs` — register all task force tool definitions

### 2c. Task force archetype block + wiring

**Create:** `config/protocols/node_assistant/task_force/block.md`
- From `NODE_ASSISTANT_PROMPTS.md` task force section

**Modify:** `src/config/protocols.rs` — `NODE_ASSISTANT_TASK_FORCE_BLOCK` role content

**Modify:** `src/server/hub/strategies/chat/mod.rs`
- `resolve_step_tools("task_force")` → returns task force tools + universal tools
- `dispatch_step_tool()` → add `"task_force"` branch routing to task force handlers

**Create:** `src/server/tools/task_force/snapshot.rs`
- `build_task_force_snapshot(repo, ctx)` — format current mission brief + roster for prompt injection

**Modify:** `src/server/hub/mod.rs` — `build_step_system_prompt()`
- Add `"task_force"` branch that loads base + task force block + snapshot

### 2d. WS events for task force mutations

**Modify:** `src/server/ws/events.rs`
- `TaskUpdated { step_id }`, `AgentRosterChanged { step_id, agent_count }`

**Modify:** `src/server/hub/strategies/chat/mod.rs` — `broadcast_step_event()`
- Handle task force tool events

### Files touched (Phase 2)
- **Create:** `migrations/0025_task_mission_briefs.sql`, `src/server/tools/task_force/mod.rs`, `src/server/tools/task_force/snapshot.rs`, `config/protocols/node_assistant/task_force/block.md`
- **Modify:** `src/db/mod.rs`, `src/db/traits/mod.rs`, `src/db/pg_repo/mod.rs`, `src/config/protocols.rs`, `src/tools/registry/mod.rs`, `src/server/hub/strategies/chat/mod.rs`, `src/server/hub/mod.rs`, `src/server/ws/events.rs`

### Verification
- Create step → chat → set archetype to task_force → set_task → add_agent × 3 → verify DB rows
- Config snapshot reflects current roster in system prompt
- Switch from task_force back to documenter works (archetype swap)

---

## Phase 3: Belief Capture Archetype — Design Time

**Goal:** User can configure a belief capture node through the assistant — extraction focus, tag vocabulary, contradiction handling.

### 3a. Migration + DB layer

**Create:** `migrations/0026_beliefs.sql`

```sql
CREATE TABLE belief_extraction_plans (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    step_id uuid NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    extraction_focus text NOT NULL DEFAULT '',
    tag_vocabulary text[] NOT NULL DEFAULT '{}',
    contradiction_handling text NOT NULL DEFAULT 'flag',
    confidence_threshold text NOT NULL DEFAULT 'low',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE beliefs (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_id uuid NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    workflow_execution_id uuid NOT NULL,
    workflow_step_id uuid NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    content text NOT NULL,
    reasoning text NOT NULL,
    belief_type text NOT NULL DEFAULT 'fact',
    confidence text NOT NULL DEFAULT 'medium',
    semantic_tags text[] NOT NULL DEFAULT '{}',
    source_step_name text NOT NULL,
    source_document_title text,
    extraction_model text NOT NULL,
    extraction_tokens_in integer NOT NULL DEFAULT 0,
    extraction_tokens_out integer NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_beliefs_workflow_execution ON beliefs(workflow_execution_id);
CREATE INDEX idx_beliefs_step ON beliefs(workflow_step_id);
CREATE INDEX idx_beliefs_tags ON beliefs USING GIN(semantic_tags);
```

**Modify:** `src/db/mod.rs` — `BeliefExtractionPlanRow`, `BeliefRow`, `NewBelief`
**Modify:** `src/db/traits/mod.rs` — `BeliefRepo` trait
**Modify:** `src/db/pg_repo/mod.rs` — PgRepo impl

### 3b. Belief capture tools + archetype block

**Create:** `src/server/tools/belief_capture/mod.rs`
- `execute_set_extraction_focus`, `execute_set_tag_vocabulary`, `execute_set_contradiction_handling`, `execute_set_confidence_threshold`

**Create:** `config/protocols/node_assistant/belief_capture/block.md`

**Wire into:** registry, resolve_step_tools, dispatch_step_tool, build_step_system_prompt (same pattern as Phase 2)

### Verification
- Configure belief capture node via assistant chat
- Extraction plan stored in DB with correct values

---

## Phase 4: Room Archetype — Design Time Tools

**Goal:** User can configure room meetings through the assistant. Room execution already exists — this phase adds the design-time configuration tools.

### 4a. Room configuration tools

**Create:** `src/server/tools/room_config/mod.rs`
- `execute_set_meeting_purpose`, `execute_add_member`, `execute_update_member`, `execute_remove_member`, `execute_set_max_turns`, `execute_set_interaction_mode`
- These write to existing `rooms` + `room_members` tables (or create them)

**Create:** `config/protocols/node_assistant/room/block.md`

**Wire into:** registry, resolve_step_tools, dispatch_step_tool, build_step_system_prompt

### Verification
- Configure room via assistant → verify room + members created in DB
- Room execution still works with assistant-configured rooms

---

## Phase 5: Task Force — Runtime Execution

**Goal:** When a workflow runs and hits a task_force step, it executes the mission brief — sequentially running each agent in the roster with the full plan context.

### 5a. DAG executor branch

**Create:** `src/server/hub/dag/task_force/mod.rs`

```
execute_task_force_step(state, step, ctx, edges, ...) -> Result
  1. Load mission brief + roster for step_id
  2. Build full plan context (mission + roster + upstream outputs)
  3. For each agent in roster (ordered by execution_order):
     a. Build system prompt: "You are {name}. Full plan: {plan}. Your role: {role}. Previous outputs: {previous}."
     b. Resolve capabilities to actual tools
     c. Execute via ExecutionEngine
     d. Capture output for next agent
  4. Combine all outputs into StepExecutionEnvelope
  5. Return
```

**Modify:** `src/server/hub/dag/mod.rs`
- Add `"task_force"` branch in the execution mode routing (alongside documenter, room, etc.)

### 5b. Agent prompts for task force execution

**Create:** `config/protocols/task_force/agent/system.md`
- Template for task force agent system prompts at runtime
- Variables: `{{.Agent.name}}`, `{{.Agent.role}}`, `{{.System.full_plan}}`, `{{.System.previous_outputs}}`

### Verification
- Create workflow: context → task_force step with 3 agents → verify sequential execution
- Each agent sees previous agent's output
- Output envelope contains combined results

---

## Phase 6: Belief Capture — Runtime Execution

**Goal:** When a workflow runs and hits a belief_capture step, it reads upstream artifacts, runs gatekeeper LLM calls, and stores beliefs.

### 6a. Content normalization

**Create:** `src/server/hub/dag/belief_capture/mod.rs`

```
execute_belief_capture_step(state, step, ctx, edges, ...) -> Result
  1. Load extraction plan for step_id
  2. For each upstream step:
     a. Normalize content by upstream type:
        - documenter → load produced documents (title + content)
        - task_force → load combined agent outputs
        - room → load transcript
        - context → load prompt_template content
        - single → load envelope data
     b. Run gatekeeper LLM call per source with extraction focus + tag vocabulary
     c. Parse beliefs, store in DB
  3. Broadcast BeliefsExtracted event
  4. Wrap belief summary in envelope for downstream
```

### 6b. Gatekeeper module

**Create:** `src/server/hub/beliefs/mod.rs`
- `extract_beliefs(state, content, source_name, extraction_plan, workflow_ids) -> Vec<BeliefRow>`
- Gatekeeper prompt adapted from BOCA v2

### Verification
- Workflow: context → documenter → belief_capture → verify beliefs in DB
- Beliefs have correct tags, sources, confidence levels

---

## Phase 7: Belief Injection into Rooms

**Goal:** When a room step has upstream belief capture nodes, inject beliefs into each room agent's system prompt.

**Modify:** `src/server/executors/room/mod.rs`
- In system prompt construction, check for upstream belief capture steps
- Load beliefs for the current execution, format, and append to agent system prompts

**Reuse:** `format_beliefs_for_mask()` from BOCA plan — shared formatting utility

### Verification
- Workflow: task_force → belief_capture → room → verify room agents see beliefs in their prompts

---

## Phase 8: Mask Agent (Conversational Interface)

**Goal:** After a workflow executes, users can chat with a mask agent that answers from beliefs.

**Create:** `src/server/api/workflows/mask_handlers.rs`
- POST `/api/workflows/:wid/executions/:eid/mask/chat` — creates/finds mask session, loads beliefs, routes to ChatStrategy

**Uses existing:** ChatStrategy, chat_sessions, chat_messages infrastructure

### Verification
- Execute workflow → open mask chat → ask questions → mask answers from beliefs with citations

---

## Phase 9: Frontend — Blank Nodes + Step Chat

**Goal:** Blank node on canvas, chat panel opens, node visuals update as assistant configures.

- Blank node component (no type badge, just a chat bubble icon)
- Step chat panel integration (reuse ChatPanel components)
- Real-time node updates from WS events (archetype change → node appearance changes, agent roster → shows count, doc defs → shows document list)
- Archetype-specific node skins (documenter, task_force, belief_capture, room each have distinct appearance)

---

## Phase 10: Resource Nodes (Future)

- GitHub, Database, S3 resource node types
- Hand-configured by user (no assistant)
- Edge from resource → task force means capabilities are provisioned
- Container orchestration for code-based task forces

---

## Phase 11: Runtime Planner (Future)

- Planner LLM call before task force execution
- Sees mission brief + live environment (repo structure, etc.)
- Creates detailed step assignments and parallelization strategy
- Plan stored in `task_execution_plans` table for observability

---

## Implementation Start: Phase 1

Phase 1 is the foundation — everything else builds on the generalized archetype system. Estimated scope: ~8 files modified/created, focused on prompt architecture and tool routing generalization. The documenter must still pass all existing tests through the new system.

### Key existing code to reuse
- `RoleDefinition.resolve()` in `src/config/protocols.rs` — template resolution with variable injection
- `build_config_snapshot()` in `src/server/tools/documenter/mod.rs` — pattern for archetype-specific context
- `resolve_step_tools()` / `dispatch_step_tool()` in `src/server/hub/strategies/chat/mod.rs` — already archetype-switched, just needs more branches
- `broadcast_documenter_event()` — generalize to `broadcast_step_event()`
- `DocumenterToolContext` pattern — reuse for universal `StepToolContext`
