# Board Context: Ambient Node Awareness

## Vision

Users design multi-agent workflows on a canvas by chatting with node-level agents. Each node has its own conversation, but the agents should feel like informed colleagues — aware of what's happening across the board, how the user's decisions connect, and what neighboring nodes are working toward.

The goal is **not metadata injection**. It's making each agent sound like it *knows* what's going on. When the user scrolls from Node A to Node B, the agent at B should feel like a colleague who's been kept in the loop — aware of upstream intent, downstream expectations, and the user's broader design vision.

## How It Works

**Two data layers compose into one Haiku-distilled context block:**

1. **Static layer** (config data, already in the DB) — node names, archetypes, descriptions, connections, archetype config (members, docs, tasks)

2. **Dynamic layer** (conversation-derived) — per-node goal summaries that capture *what the user is trying to accomplish and why*, plus activity signals (message count, last activity)

A **Board Renderer** assembles both layers into a structured document. A **Haiku Distiller** takes that document and produces a targeted, narrative-aware summary from each node's perspective. Results are cached per-node and refreshed lazily.

**Refresh pattern: stale-on-write, refresh-on-read.**
- Any structural change marks the board context stale for all nodes in that workflow
- When a chat message arrives at a stale node, use the fallback for this turn and spawn a background refresh
- By the next turn, Haiku-distilled context is warm and ready

**Cost:** ~$0.001 per Haiku call. A 10-node board refresh costs a penny.

---

## Implementation

### Step 1: Database

**New migration:**

```sql
ALTER TABLE workflow_steps
    ADD COLUMN board_context_cache text NOT NULL DEFAULT '',
    ADD COLUMN board_context_updated_at timestamptz,
    ADD COLUMN goal_summary text NOT NULL DEFAULT '',
    ADD COLUMN goal_summary_updated_at timestamptz;
```

- `board_context_cache` — Haiku's per-node board summary (refreshed on structural changes)
- `goal_summary` — Haiku's distillation of this node's conversational intent (refreshed on conversation shifts)

**Update `WorkflowStepRow`** in `src/db/mod.rs` — add 4 fields.

**New repo methods** in `src/db/traits/mod.rs`:
```rust
async fn update_step_board_context(&self, step_id: Uuid, context: &str) -> Result<()>;
async fn update_step_goal_summary(&self, step_id: Uuid, goal: &str) -> Result<()>;
async fn mark_board_context_stale(&self, workflow_id: Uuid) -> Result<()>;
```

`mark_board_context_stale` nulls out `board_context_updated_at` for all steps in the workflow — one UPDATE statement.

### Step 2: Board Renderer

**New file:** `src/server/hub/board_context/renderer.rs`

Pure data assembly. Two queries: steps + edges. Produces a structured document for Haiku consumption.

```rust
pub async fn render_board(
    repo: &dyn WorkflowRepo,
    workflow_id: Uuid,
) -> Result<String, HubError>
```

**Output format** — includes activity signals and goals alongside static config:

```
Board: Notification Platform Design
Nodes: 4 | Connections: 3

[Node: Research Task Force] (task_force)
  Description: Exploring agent behavior during entertainment
  Goal: Determine how agents behave during idle scenarios
  Activity: 12 messages, last active 5 min ago
  Connections: → Doc Gen, → Quality Review

[Node: Doc Gen] (documenter)
  Description: Technical specification generator
  Goal: Generate API specifications from product requirements
  Activity: 3 messages, last active 20 min ago
  Connections: ← Research Task Force, → Quality Review

[Node: Quality Review] (room)
  Description: Architecture review meeting
  Goal: (not yet established)
  Activity: No conversation yet
  Connections: ← Doc Gen, ← Research Task Force

[Node: Requirements] (context)
  Description: Q2 product requirements
  Goal: (context node — static content)
  Activity: Configured, no active chat
  Connections: → Research Task Force
```

Activity data comes from session message counts and timestamps (join through step → session).

The renderer does NOT fetch archetype-specific config (room members, doc defs, task force agents). That detail belongs to the node's own `current_config` injection, not the board context. The board context operates at the goal/intent level — what each node is *for*, not how it's configured internally.

### Step 3: Goal Distiller

**New file:** `src/server/hub/board_context/distiller.rs`

Two Haiku functions:

```rust
/// Distill a node's conversational intent into a 1-2 sentence goal.
/// Captures WHAT the user wants and WHY — not just the task description.
pub async fn distill_node_goal(
    recent_conversation: &str,
    node_name: &str,
    node_archetype: &str,
    current_goal: &str,
) -> Option<String>
```

**Goal distiller system prompt:**
```
Distill this node's purpose from the user's conversation into 1-2 sentences.
Capture what the user is trying to accomplish AND their reasoning or emphasis.
Write as if briefing a colleague: "Focused on X because Y" not "This node does X."

<examples>
<example>
<conversation>User asked to set up a research team to investigate how agents behave during idle time, specifically entertainment. Added a Researcher agent focused on behavioral patterns and an Analyst for data synthesis. User emphasized wanting "real behavioral data, not speculation."</conversation>
<goal>Investigating agent entertainment behavior with emphasis on real behavioral data over speculation. Team is research-heavy with dedicated analysis capacity.</goal>
</example>
<example>
<conversation>User set up a security-focused review room. Added Security Lead first, then Tech Lead and Architect. Set interaction mode to moderated. User said "security is the lens everything goes through."</conversation>
<goal>Security-first architecture review where compliance is the primary lens. User prioritized the security perspective above other concerns.</goal>
</example>
</examples>

If the conversation is too early to determine intent, return: "Still being defined by the user"
Return only the goal statement.
```

```rust
/// Distill the full board render into targeted context for a specific node.
pub async fn distill_board_for_node(
    board_render: &str,
    node_name: &str,
    node_archetype: &str,
) -> Option<String>
```

**Board distiller system prompt:**
```
Summarize this workflow board from the perspective of the specified node.
Write 3-5 sentences that help this node's assistant understand:
- What neighboring nodes are working on and why the user set them up
- How this node fits into the user's broader design
- Any user emphasis or priorities visible from connected nodes

Write in second person as a brief from a colleague. Be specific about
what matters to THIS node given its role and connections. Sound informed,
not clinical.

<examples>
<example>
<node>Doc Gen (documenter)</node>
<context>Your upstream Research Task Force is actively being configured — the user has invested significant effort there, defining a research team focused on agent entertainment behavior with an emphasis on real data over speculation. Your downstream Quality Review is set up as a security-first review room where the Security Lead has primary authority. Your specifications will be evaluated through a security compliance lens, so technical accuracy and threat surface coverage matter.</context>
</example>
</examples>

Return only the context summary.
```

### Step 4: Refresh Orchestrator

**New file:** `src/server/hub/board_context/refresh.rs`

```rust
/// Refresh board context for all nodes in a workflow.
/// Renders the board once, distills per-node concurrently.
pub async fn refresh_board_context(
    state: &AppState,
    workflow_id: Uuid,
) -> Result<(), HubError>
```

Flow:
1. `render_board()` — one call, two queries
2. For each node, `distill_board_for_node()` — concurrent Haiku calls via `JoinSet`
3. Store results via `repo.update_step_board_context()` per node

```rust
/// Refresh a single node's goal summary from its conversation.
pub async fn refresh_node_goal(
    state: &AppState,
    step_id: Uuid,
    session_id: Uuid,
) -> Result<(), HubError>
```

Flow:
1. Load last 10 session messages
2. `distill_node_goal()` — single Haiku call
3. Store via `repo.update_step_goal_summary()`
4. If goal changed, `repo.mark_board_context_stale()` so neighbors pick up the change

### Step 5: Stale-on-Write Triggers

**Where structural changes mark the board stale:**

In `ChatStrategy::broadcast_step_event()` — after broadcasting, call `mark_board_context_stale(workflow_id)` for these events:
- `ArchetypeChanged`
- `StepConfigUpdated`
- `StepNameUpdated`
- `DocDefCreated`, `DocDefUpdated`, `DocDefDeleted`

In workflow API handlers (edge CRUD endpoints) — after creating/deleting an edge, call `mark_board_context_stale(workflow_id)`.

In `ChatStrategy::on_complete()` — after config-changing tool calls, spawn `refresh_node_goal()`. If the goal changed, `mark_board_context_stale` is called inside `refresh_node_goal`.

This is a single `UPDATE workflow_steps SET board_context_updated_at = NULL WHERE workflow_id = $1` — cheap and atomic.

### Step 6: Refresh-on-Read Integration

**File:** `src/server/hub/mod.rs` — `build_step_system_prompt()`

```rust
let step = repo.get_step(step_id).await?;

let board_context = if step.board_context_updated_at.is_some() {
    // Cache is warm — use it
    step.board_context_cache.clone()
} else if step.board_context_cache.is_empty() {
    // Never been rendered — use structural fallback
    let fallback = graph_context::build_graph_context(repo, workflow_id, step_id).await?;
    // Spawn background refresh for next turn
    spawn_board_refresh(state.clone(), workflow_id);
    fallback
} else {
    // Has a stale cache — use it but refresh in background
    spawn_board_refresh(state.clone(), workflow_id);
    step.board_context_cache.clone()
};

vars_map.insert(vars::system::BOARD_CONTEXT.to_string(), board_context);
```

Three states:
1. **Warm cache** (`updated_at` is set) → use directly
2. **Never rendered** (cache empty, `updated_at` is null) → structural fallback + background refresh
3. **Stale cache** (cache has content, `updated_at` is null) → use stale content + background refresh

The user gets a response immediately. The stale cache is usually "good enough" — things haven't changed dramatically. By the next turn, the fresh context is ready.

### Step 7: System Prompt Template

**Update** `config/protocols/node_assistant/base/system.md`:

Replace `<graph_context>{{.System.graph_context}}</graph_context>` with:

```xml
<board_context>
{{.System.board_context}}
</board_context>
```

**Update** `src/config/protocols.rs`:
- Add `pub const BOARD_CONTEXT: &str = "System.board_context";` to `vars::system`

**Update** `src/server/hub/mod.rs`:
- Replace `vars::system::GRAPH_CONTEXT` usage with `vars::system::BOARD_CONTEXT`

Keep `graph_context/` module as fallback — used when cache is empty (cold start).

### Step 8: Constants

**Add to** `src/constants.rs`:

```rust
pub const MAX_TOKENS_BOARD_CONTEXT: u32 = 512;
pub const MAX_TOKENS_GOAL_SUMMARY: u32 = 128;
pub const GOAL_REFRESH_MIN_TURNS: u32 = 3;
pub const BOARD_RENDER_MAX_DESCRIPTION_CHARS: usize = 200;
```

### Step 9: Module Organization

```
src/server/hub/
├── board_context/
│   ├── mod.rs              # pub use, spawn_board_refresh()
│   ├── renderer.rs         # render_board() — pure data, two queries
│   ├── distiller.rs        # Haiku calls with few-shot examples
│   ├── refresh.rs          # refresh_board_context(), refresh_node_goal()
│   └── tests.rs
├── graph_context/          # Kept as fallback for cold starts
│   ├── mod.rs
│   └── tests.rs
```

### Step 10: Tests

**`board_context/tests.rs`:**
- `render_board_includes_all_nodes` — steps, connections, goals in output
- `render_board_includes_activity_signals` — message count, last active
- `render_board_empty_workflow` — minimal output
- `stale_cache_triggers_background_refresh` — verify spawn on stale read
- `warm_cache_returns_directly` — no refresh spawned
- `cold_start_falls_back_to_graph_context` — empty cache uses structural fallback

**Update existing tests:**
- `WorkflowStepRow` construction sites — add 4 new fields with defaults
- Template var validation — add `BOARD_CONTEXT` to known set

---

## Files Modified

| File | Change |
|------|--------|
| `migrations/NNNN_board_context.sql` | **New** — ALTER TABLE (4 columns) |
| `src/db/mod.rs` | Add 4 fields to `WorkflowStepRow` |
| `src/db/traits/mod.rs` | Add 3 repo methods + mock |
| `src/db/pg_repo/mod.rs` | Implement 3 new methods |
| `src/server/hub/board_context/mod.rs` | **New** — module root, spawn helper |
| `src/server/hub/board_context/renderer.rs` | **New** — `render_board()` |
| `src/server/hub/board_context/distiller.rs` | **New** — Haiku distillers with few-shot examples |
| `src/server/hub/board_context/refresh.rs` | **New** — refresh orchestrator |
| `src/server/hub/board_context/tests.rs` | **New** — unit tests |
| `src/server/hub/mod.rs` | Wire board_context into `build_step_system_prompt()`, add `pub mod` |
| `src/server/hub/strategies/chat/mod.rs` | Mark stale in `broadcast_step_event()`, goal refresh in `on_complete()` |
| `src/config/protocols.rs` | Add `BOARD_CONTEXT` var, update test |
| `src/constants.rs` | Add board context constants |
| `config/protocols/node_assistant/base/system.md` | Replace `<graph_context>` with `<board_context>` |

---

## Verification

1. `cargo check` — all changes type-check
2. `cargo test hub::board_context` — new tests pass
3. `cargo test hub::graph_context` — existing tests still pass
4. `cargo test protocols::tests` — template var validation
5. `cargo clippy` — no new warnings
6. Run migration against dev DB
7. Manual: create 3 connected nodes, chat with node A for 5+ turns, then open node B — verify B's system prompt contains Haiku-distilled context that sounds informed about A's purpose and the user's intent
8. Verify cold start: new node with no cache falls back to structural graph_context
9. Verify stale refresh: make a config change on node A, send a message on node B — first response uses stale cache, second response uses fresh Haiku context
