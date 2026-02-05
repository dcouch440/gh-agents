# Router Modes System - Comprehensive Design Document

## Table of Contents
1. [Current State Analysis](#current-state-analysis)
2. [Problem Statement](#problem-statement)
3. [Proposed Solution](#proposed-solution)
4. [Database Design](#database-design)
5. [Layered Architecture](#layered-architecture)
6. [Runtime Architecture](#runtime-architecture)
7. [Implementation Plan](#implementation-plan)
8. [Edge Cases & Defaults](#edge-cases--defaults)
9. [Integration Points](#integration-points)

---

## Current State Analysis

### What We Have Now

#### 1. **agent_modes** (Per-Agent Behavioral Variants)
```sql
agent_modes (
    id UUID,
    agent_id UUID FK,              -- Belongs to a specific agent
    name TEXT,                      -- e.g., "helpful", "concise"
    system_prompt_suffix TEXT,      -- APPENDED to agent's base prompt
    temperature_override REAL,
    model_override TEXT,
    tool_overrides TEXT[],          -- Array of tool names (strings)
    classifier_hint TEXT            -- For LLM to select this mode
)
```

**Usage**: Agent-specific modes that modify behavior per-turn. Used in chat sessions.

**Limitations**:
- Agent-specific (can't share modes across agents)
- Only appends to system prompt (can't replace it)
- Tool overrides are strings, not IDs (no FK relationship)
- Not optimized for context reduction

#### 2. **tool_routers** (Tool Selection Routers)
```sql
tool_routers (
    id UUID,
    user_id UUID FK,
    name TEXT,
    description TEXT,
    system_prompt TEXT,             -- Router LLM instructions
    model_id TEXT,
    is_active BOOLEAN
)

tool_router_tools (               -- Which tools this router owns
    router_id UUID FK,
    tool_id UUID FK
)
```

**Usage**: Currently used for tool routing decisions (select which tool to call).

**Limitations**:
- No concept of "modes" - just selects individual tools
- No system_prompt or temperature per tool
- Binary decision (tool selected or not)

#### 3. **agents + agent_tools** (Agent Configuration)
```sql
agents (
    id UUID,
    user_id UUID FK,
    name TEXT,
    system_prompt TEXT,
    model_id TEXT,
    temperature REAL,
    max_tokens INT
)

agent_tools (                     -- Which tools agent can use
    agent_id UUID FK,
    tool_id UUID FK
)
```

**Usage**: Agents have a fixed set of tools they can use.

**Problem**: All tools sent to LLM on every request (context bloat).

---

## Problem Statement

### What We're Trying to Solve

1. **Context Bloat**: Agents send ALL their tools (e.g., 50 tools) to the LLM on every request
2. **No Dynamic Personality Selection**: Want to change personality based on user input, not just append suffixes
3. **No Tool Subsetting**: Want to send only 3-5 relevant tools per request, not all 50
4. **Redundant Concepts**: `agent_modes` and the proposed system overlap

### What We Want

**User sends message**:
```
"Help me debug this React component"
```

**System should**:
1. **Classify** the message → "This is a coding task"
2. **Select mode** → "coding" mode
3. **Load mode config**:
   - system_prompt: "You are an expert programmer..."
   - temperature: 0.3
   - max_tokens: 8000
   - **tools**: [bash, read_file, edit_file, search_code] (only 4 tools, not 50)
4. **Call main LLM** with focused context

**Result**: Better responses, lower costs, faster execution.

---

## Proposed Solution

### Design Principle: **Extend `tool_routers`, Deprecate `agent_modes`**

Instead of creating a parallel system, we extend the existing `tool_routers` table to support **modes**.

### Why This Approach?

1. **Consolidation**: One routing system, not two competing systems
2. **Reuse Infrastructure**: RouterStrategy already exists in the engine
3. **Clear Semantics**: "Router selects modes, modes configure behavior"
4. **Backward Compatible**: Existing tool_routers continue to work

---

## Database Design

### Schema Changes

#### Option A: **Extend `tool_routers` with Modes** (RECOMMENDED)

```sql
-- ════════════════════════════════════════════════════════════════════════════
-- EXISTING: tool_routers (keep as-is, add optional parent_id for hierarchy)
-- ════════════════════════════════════════════════════════════════════════════
ALTER TABLE tool_routers
ADD COLUMN parent_router_id UUID REFERENCES tool_routers(id) ON DELETE CASCADE,
ADD COLUMN level INT DEFAULT 1 CHECK (level IN (1, 2, 3));

CREATE INDEX idx_tool_routers_parent ON tool_routers(parent_router_id);

-- ════════════════════════════════════════════════════════════════════════════
-- NEW: tool_router_modes (modes for each router)
-- ════════════════════════════════════════════════════════════════════════════
CREATE TABLE tool_router_modes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    router_id UUID NOT NULL REFERENCES tool_routers(id) ON DELETE CASCADE,

    -- Mode identity
    mode_key TEXT NOT NULL,                  -- e.g., "coding", "research", "chat"
    display_name TEXT NOT NULL,              -- e.g., "Coding Mode"
    description TEXT NOT NULL,               -- For router LLM classification

    -- Behavior configuration
    system_prompt TEXT NOT NULL,             -- System prompt for this mode
    temperature REAL NOT NULL DEFAULT 0.7,
    max_tokens INT NOT NULL DEFAULT 4096,
    append_to_agent_system_prompt BOOLEAN NOT NULL DEFAULT FALSE,  -- Append mode prompt to agent's or replace
    append_to_agent_tools BOOLEAN NOT NULL DEFAULT TRUE,           -- Add mode tools to agent's or replace

    -- Metadata
    display_order INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    UNIQUE (router_id, mode_key),
    CHECK (mode_key ~ '^[a-z][a-z0-9_]*$'),  -- snake_case only
    CHECK (temperature BETWEEN 0.0 AND 2.0),
    CHECK (max_tokens > 0)
);

CREATE INDEX idx_tool_router_modes_router ON tool_router_modes(router_id);
CREATE INDEX idx_tool_router_modes_order ON tool_router_modes(router_id, display_order);

-- ════════════════════════════════════════════════════════════════════════════
-- NEW: tool_router_mode_tools (which tools each mode has access to)
-- ════════════════════════════════════════════════════════════════════════════
CREATE TABLE tool_router_mode_tools (
    mode_id UUID NOT NULL REFERENCES tool_router_modes(id) ON DELETE CASCADE,
    tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
    PRIMARY KEY (mode_id, tool_id)
);

CREATE INDEX idx_tool_router_mode_tools_tool ON tool_router_mode_tools(tool_id);

-- ════════════════════════════════════════════════════════════════════════════
-- MODIFY: agents (add optional router_id)
-- ════════════════════════════════════════════════════════════════════════════
ALTER TABLE agents
ADD COLUMN router_id UUID REFERENCES tool_routers(id) ON DELETE SET NULL;

CREATE INDEX idx_agents_router ON agents(router_id);

-- ════════════════════════════════════════════════════════════════════════════
-- MODIFY: agent_executions (track which mode was selected)
-- ════════════════════════════════════════════════════════════════════════════
ALTER TABLE agent_executions
ADD COLUMN selected_router_mode_id UUID REFERENCES tool_router_modes(id) ON DELETE SET NULL;

CREATE INDEX idx_agent_executions_router_mode ON agent_executions(selected_router_mode_id);

-- ════════════════════════════════════════════════════════════════════════════
-- DEPRECATE: agent_modes (mark for future removal)
-- ════════════════════════════════════════════════════════════════════════════
-- Keep table for now, but add deprecation comment:
COMMENT ON TABLE agent_modes IS 'DEPRECATED: Use tool_router_modes instead. Will be removed in future migration.';
```

### Entity Relationship Diagram

```
┌─────────────────┐
│      USERS      │
└────────┬────────┘
         │
         ├──────────────────────────────────────────────────────────┐
         │                                                          │
┌────────▼────────┐         ┌──────────────┐         ┌─────────────▼───┐
│     AGENTS      │         │ AGENT_TOOLS  │         │     TOOLS       │
│─────────────────│         │ (JOIN TABLE) │         │─────────────────│
│ ID (PK)         │◄────────┤──────────────├────────►│ ID (PK)         │
│ USER_ID (FK)    │         │ AGENT_ID (FK)│         │ USER_ID (FK)    │
│ NAME            │         │ TOOL_ID (FK) │         │ NAME            │
│ SYSTEM_PROMPT   │         └──────────────┘         │ DESCRIPTION     │
│ ROUTER_ID (FK)  │───┐                              │ PARAMETERS(JSON)│
│ ...             │   │                              └────────▲────────┘
└─────────────────┘   │                                       │
                      │                                       │
                      │                                       │
┌─────────────────────▼───────────┐                           │
│      TOOL_ROUTERS               │                           │
│─────────────────────────────────│                           │
│ ID (PK)                         │                           │
│ USER_ID (FK)                    │                           │
│ NAME                            │                           │
│ SYSTEM_PROMPT                   │  ← Router LLM instructions│
│ MODEL_ID                        │    (how to classify)      │
│ PARENT_ROUTER_ID (FK) ◄─────────┼──┐  (for L1→L2→L3)       │
│ LEVEL (1, 2, 3)                 │  │                        │
└────────┬────────────────────────┘  │                        │
         │                           │                        │
         │ 1:N                       └────────────────────────┘
         │
┌────────▼────────────────────┐
│   TOOL_ROUTER_MODES         │
│─────────────────────────────│
│ ID (PK)                     │
│ ROUTER_ID (FK)              │
│ MODE_KEY (UQ per router)    │  ← "coding", "research", etc.
│ DISPLAY_NAME                │
│ DESCRIPTION                 │  ← For router LLM to classify
│ SYSTEM_PROMPT               │  ← Full prompt for this mode
│ TEMPERATURE                 │
│ MAX_TOKENS                  │
│ DISPLAY_ORDER               │
└────────┬────────────────────┘
         │
         │ N:M
         │
┌────────▼────────────────────┐
│ TOOL_ROUTER_MODE_TOOLS      │
│  (JOIN TABLE)               │
│─────────────────────────────│
│ MODE_ID (FK)                │
│ TOOL_ID (FK)                │────────────────────► TOOLS
└─────────────────────────────┘


RUNTIME TRACKING:

┌─────────────────────────────┐
│   AGENT_EXECUTIONS          │
│─────────────────────────────│
│ ID (PK)                     │
│ AGENT_ID (FK)               │
│ SELECTED_ROUTER_MODE_ID(FK) │───► TOOL_ROUTER_MODES
│ INPUT                       │     (tracks which mode was used)
│ OUTPUT                      │
│ ...                         │
└─────────────────────────────┘
```

### Tables to DROP (Consolidation)

```sql
-- ════════════════════════════════════════════════════════════════════════════
-- DROP: agent_modes (replaced by tool_router_modes)
-- ════════════════════════════════════════════════════════════════════════════
-- Migration path:
-- 1. Create tool_router_modes (new system)
-- 2. Migrate existing agent_modes → tool_router_modes (conversion script)
-- 3. Update code to use new system
-- 4. Drop agent_modes table

DROP TABLE IF EXISTS agent_modes_versions CASCADE;  -- Drop history first
DROP TABLE IF EXISTS agent_modes CASCADE;
```

**Why drop `agent_modes`?**
- Redundant with `tool_router_modes`
- Less flexible (suffix-only, agent-specific)
- No tool subsetting capability
- Creates confusion having two "mode" systems

---

## Layered Architecture

### Design Principle: Keep ExecutionEngine Pure, Keep Call Sites In Control

The existing `ExecutionEngine` is **perfectly designed** as a generic LLM execution loop. We should NOT modify it. Each call site (chat, room, DAG) also has context-specific logic that only it understands. Instead of a monolithic orchestrator that tries to create every strategy type, we introduce a **ModeResolver** that resolves routing config and lets each call site apply it.

### Why NOT a Monolithic AgentOrchestrator?

The original design proposed `orchestrator.execute_agent()` as a single entry point. This fails because:

1. **Different strategy types**: Chat needs `ChatStrategy`, rooms need `RoomSpeakerStrategy`, DAGs need `DagStepStrategy`. A single orchestrator would need to know about all of them.
2. **Context-specific composition**: Rooms add room context + agent docs to the system prompt. DAGs add schema enforcement. The orchestrator would need all this context passed in, defeating the purpose.
3. **Tool authority varies**: Rooms have `room.tools_enabled` as a master switch. DAGs always have tools. The orchestrator can't make this decision.
4. **History format varies**: Chat has `Vec<Message>`, rooms have a transcript blob, DAGs have no history. The orchestrator can't normalize these.

### The Correct Architecture: ModeResolver

```
┌──────────────────────────────────────────────────────────────────┐
│  LAYER 3: APPLICATION (Call Sites)                               │
│  ─────────────────────────────────────────────────────────────── │
│  - hub/mod.rs (chat)         → ChatStrategy                     │
│  - room_executor.rs          → RoomSpeakerStrategy               │
│  - hub/dag/mod.rs            → DagStepStrategy                   │
│                                                                  │
│  Each call site:                                                 │
│  1. Calls mode_resolver.resolve(agent, input, context_hint)     │
│  2. Gets back ResolvedModeConfig (prompt, tools, temp)          │
│  3. Applies its own context on top (room context, schema, etc.) │
│  4. Builds its own strategy type                                 │
│  5. Calls engine.execute(strategy, ...)                         │
└──────────────────────────┬───────────────────────────────────────┘
                           │
          ┌────────────────┼────────────────┐
          │                │                │
          ▼                ▼                ▼
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│  ModeResolver │ │  ModeResolver │ │  ModeResolver │
│  .resolve()   │ │  .resolve()   │ │  .resolve()   │
│  (same svc)   │ │  (same svc)   │ │  (same svc)   │
└──────┬───────┘ └──────┬───────┘ └──────┬───────┘
       │                │                │
       └────────────────┼────────────────┘
                        │
                        ▼
┌──────────────────────────────────────────────────────────────────┐
│  LAYER 2: MODE RESOLUTION (NEW - ModeResolver)                   │
│  ─────────────────────────────────────────────────────────────── │
│  Responsibilities:                                               │
│  ✓ Check if agent has router_id                                 │
│  ✓ Load router + modes from DB                                  │
│  ✓ Classify input via Router LLM (with context hint)            │
│  ✓ Load mode tools from DB                                      │
│  ✓ Merge system prompt (append/replace per mode flag)           │
│  ✓ Merge tools (append/replace per mode flag)                   │
│  ✓ Return ResolvedModeConfig                                    │
│                                                                  │
│  This layer ONLY resolves config. It does NOT create strategies  │
│  or call the engine. Each call site remains in control.         │
└──────────────────────────────────────────────────────────────────┘
                        │
                        ▼
┌──────────────────────────────────────────────────────────────────┐
│  LAYER 1: EXECUTION (Existing - ExecutionEngine)                 │
│  ─────────────────────────────────────────────────────────────── │
│  Responsibilities:                                               │
│  ✓ Execute strategy (strategy pattern)                          │
│  ✓ Tool use loop                                                │
│  ✓ Streaming                                                    │
│  ✓ Token tracking                                               │
│  ✓ Cancellation handling                                        │
│                                                                  │
│  This layer is PURE - no agent/routing awareness.               │
│  Just executes whatever strategy it receives.                   │
└──────────────────────────────────────────────────────────────────┘
```

### Why This Architecture?

**ExecutionEngine (Layer 1)** — unchanged:
- ✅ Pure execution loop (generic, reusable)
- ✅ Strategy-agnostic (works with any ExecutionStrategy)
- ✅ No agent awareness (maintains single responsibility)
- ✅ Already exists and works perfectly

**ModeResolver (Layer 2)** — new:
- ✅ Single responsibility: resolve mode config from agent + input
- ✅ Context-agnostic: doesn't know about rooms, DAGs, or chat
- ✅ Returns data, not behavior (no strategy creation)
- ✅ Reusable by any call site with one function call
- ✅ Easy to test (input agent + input text → output config)

**Application Layer (Layer 3)** — each call site stays in control:
- ✅ Calls `mode_resolver.resolve()` to get config
- ✅ Applies its own context on top (room context, schema, etc.)
- ✅ Builds its own strategy type
- ✅ No context leaking between execution contexts

### ModeResolver Interface

```rust
pub struct ModeResolver {
    repo: Arc<dyn ServerRepo>,
    tool_router_repo: Arc<dyn ToolRouterRepo>,
    provider: Arc<dyn LLMProvider>,
}

/// Resolved configuration after mode classification.
/// Contains the merged system prompt, tools, and parameters.
pub struct ResolvedModeConfig {
    /// System prompt with mode applied (agent base + mode, merged per append flag).
    /// Call sites append their own context ON TOP of this.
    pub system_prompt: String,
    /// Resolved tool definitions (agent + mode tools, merged per append flag).
    pub tools: Vec<Tool>,
    /// Tool name allow-list (parallel to tools vec).
    pub tool_names: Vec<String>,
    /// Temperature from mode (or agent default if no mode).
    pub temperature: f32,
    /// Max tokens from mode (or agent default if no mode).
    pub max_tokens: i32,
    /// Which mode was selected (None if no router).
    pub selected_mode_id: Option<Uuid>,
    /// Mode key for logging (None if no router).
    pub selected_mode_key: Option<String>,
}

impl ModeResolver {
    /// Resolve mode config for an agent. Works for ANY execution context.
    ///
    /// - `agent`: The agent row (must already be loaded by caller)
    /// - `user_input`: The user's message (used for classification)
    /// - `context_hint`: Optional extra context for the router LLM.
    ///   Chat passes formatted history, rooms pass transcript,
    ///   DAGs pass step description. This helps the router classify better.
    ///
    /// Returns agent defaults if agent has no router_id.
    pub async fn resolve(
        &self,
        agent: &AgentRow,
        user_input: &str,
        context_hint: Option<&str>,
    ) -> Result<ResolvedModeConfig, RoutingError> {
        // 1. If no router_id → return agent defaults
        // 2. Load router + modes from DB
        // 3. Build classification prompt (user_input + context_hint + mode list)
        // 4. Call Router LLM via ExecutionEngine + RouterStrategy
        // 5. Parse mode key from response (fallback to first mode)
        // 6. Load mode tools from tool_router_mode_tools
        // 7. Merge system prompt:
        //    if mode.append_to_agent_system_prompt → agent.prompt + "\n\n" + mode.prompt
        //    else → mode.prompt (replaces agent)
        // 8. Merge tools:
        //    if mode.append_to_agent_tools → union(agent_tools, mode_tools)
        //    else → mode_tools (replaces agent)
        // 9. Return ResolvedModeConfig
    }
}
```

### Routing with Context Hints

**Key Feature**: Router sees context from any execution environment, not just chat history.

```rust
// Inside ModeResolver::resolve()

let classification_prompt = format!(
    "{context_block}\
     ## Current User Input:\n{user_input}\n\n\
     ## Available Modes:\n{mode_list}\n\n\
     Based on the context and current input, output ONLY the mode key.",
);
```

**What each call site passes as `context_hint`**:

| Call Site | context_hint | Example |
|-----------|-------------|---------|
| **Chat** | Formatted last N messages | `"user: How do I center a div?\nassistant: Use flexbox..."` |
| **Room** | Room transcript | `"## Recent Discussion\n**SecurityLead**: The API needs auth...\n**ArchLead**: I agree..."` |
| **DAG** | Step description / workflow name | `"Workflow: Deploy Pipeline, Step: Code Review"` |

**Why this matters**:
- Router can see conversation flow in chat
- Router can see multi-agent discussion tone in rooms
- Router can see task type in DAG workflows
- Same classification logic, different context sources

### System Prompt Stacking Order

**Critical**: Mode changes personality. Call-site context is structural. They stack in this order:

```
Step 1: agent.system_prompt
           ↓
Step 2: Apply mode (append or replace)     ← WHO the agent IS
           ↓
Step 3: Call site appends its own context   ← WHERE the agent IS
```

**If mode has `append_to_agent_system_prompt: true`**:
```
"You are an expert math tutor."              ← agent base
+ "\n\n"
+ "Be warm, encouraging, patient."           ← mode appends
+ "\n\n"
+ "## Room Context\nYou are MathTutor..."    ← room context (structural)
```

**If mode has `append_to_agent_system_prompt: false` (REPLACE)**:
```
"Be warm, encouraging, patient."             ← mode REPLACES agent base
+ "\n\n"
+ "## Room Context\nYou are MathTutor..."    ← room context preserved
```

The mode replaces personality but room context is NEVER destroyed — it's added by the call site after mode resolution.

| Call Site | What it appends after mode resolution |
|-----------|--------------------------------------|
| **Chat** | Nothing (mode config is final) |
| **Room** | Room context preamble + agent context documents |
| **DAG** | Schema enforcement block (`"Respond with JSON matching..."`) |

### Tool Resolution Order

```
Step 1: ModeResolver merges agent tools + mode tools (per append flag)
           ↓
Step 2: Call site applies its own constraints
```

| Call Site | Constraint | Logic |
|-----------|-----------|-------|
| **Chat** | None | Use resolved tools directly |
| **Room** | `room.tools_enabled` | If false → override to empty, regardless of mode |
| **DAG** | None | Use resolved tools directly |

### Temperature Override

Both `RoomSpeakerConfig` and `DagStepConfig` currently hardcode temperature to `agent.model_temperature`. To support mode temperature:

**Required code change**: Add `temperature: f32` field to both config structs, and update their strategies:

```rust
// RoomSpeakerStrategy and DagStepStrategy — change from:
fn temperature(&self) -> f32 {
    self.config.agent.model_temperature  // ignores mode
}

// To:
fn temperature(&self) -> f32 {
    self.config.temperature  // set by call site from ResolvedModeConfig
}
```

---

## Runtime Architecture

### Execution Flow (via ModeResolver)

```
┌──────────────────────────────────────────────────────────────────────────┐
│ 1. CALL SITE LOADS AGENT + CALLS MODE RESOLVER                           │
│                                                                          │
│    // Each call site loads the agent, then resolves mode config:         │
│    let agent = repo.get_agent(agent_id).await?;                         │
│    let mode = mode_resolver.resolve(                                    │
│        &agent,                                                          │
│        "Help me debug this React component",                            │
│        Some(&context_hint),  // chat history, transcript, or step desc  │
│    ).await?;                                                            │
│                                                                          │
└──────────────────────────┬───────────────────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ 2. MODE RESOLVER - CHECK ROUTER                                          │
│                                                                          │
│    if agent.router_id.is_some():                                        │
│        → HAS ROUTER: Proceed to classification                          │
│    else:                                                                │
│        → NO ROUTER: Return agent defaults as ResolvedModeConfig         │
│                                                                          │
└──────────────────────────┬───────────────────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ 3. MODE RESOLVER - CLASSIFY (if agent.router_id exists)                  │
│                                                                          │
│    router = db.get_tool_router(agent.router_id).await?                  │
│    modes = db.list_router_modes(router.id).await?                       │
│                                                                          │
│    → Build classification prompt with context_hint:                     │
│                                                                          │
│      ## Context (from call site):                                       │
│      user: How do I center a div?                                       │
│      assistant: Use flexbox with justify-content: center...             │
│                                                                          │
│      ## Current User Input:                                             │
│      Help me debug this React component                                 │
│                                                                          │
│      ## Available Modes:                                                │
│      - coding: For programming tasks (bash, edit, search)               │
│      - research: For information gathering (web_search, read)           │
│      - chat: For conversation (no tools)                                │
│                                                                          │
│      Based on context and current input, output ONLY the mode key.     │
│                                                                          │
│    → Call Router LLM via ExecutionEngine + RouterStrategy               │
│    → Parse response: selected_mode_key = "coding"                       │
│    → Load mode: mode = db.get_mode_by_key(router.id, "coding").await?  │
│                                                                          │
└──────────────────────────┬───────────────────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ 4. MODE RESOLVER - LOAD & MERGE CONFIG                                   │
│                                                                          │
│    mode = db.get_router_mode(mode_id)                                   │
│    mode_tool_rows = db.get_mode_tools(mode_id)                          │
│    mode_tools = mode_tool_rows.map(|r| get_tool_definition(&r.name))   │
│                                                                          │
│    // Merge system prompt (append or replace per mode flag)             │
│    system_prompt = if mode.append_to_agent_system_prompt {              │
│        format!("{}\n\n{}", agent.system_prompt, mode.system_prompt)     │
│    } else {                                                             │
│        mode.system_prompt.clone()                                       │
│    };                                                                   │
│                                                                          │
│    // Merge tools (union or replace per mode flag)                      │
│    tools = if mode.append_to_agent_tools {                              │
│        union_by_name(agent_tools, mode_tools)                           │
│    } else {                                                             │
│        mode_tools                                                       │
│    };                                                                   │
│                                                                          │
│    → Return ResolvedModeConfig {                                        │
│        system_prompt,                                                   │
│        tools, tool_names,                                               │
│        temperature: mode.temperature,    // 0.3                         │
│        max_tokens: mode.max_tokens,      // 8000                        │
│        selected_mode_id: Some(mode.id),                                │
│        selected_mode_key: Some("coding"),                               │
│    }                                                                    │
│                                                                          │
└──────────────────────────┬───────────────────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ 5. CALL SITE - APPLY CONTEXT & BUILD STRATEGY                            │
│                                                                          │
│    // Each call site applies its own context on top of mode config:     │
│                                                                          │
│    CHAT:                                                                │
│      strategy = ChatStrategy::new(ChatConfig {                          │
│          system_prompt: mode.system_prompt,  // used directly           │
│          tool_names: mode.tool_names,                                   │
│          temperature: mode.temperature,                                 │
│          model_id: agent.model_id,                                      │
│      })                                                                 │
│                                                                          │
│    ROOM:                                                                │
│      let mut prompt = mode.system_prompt;                               │
│      prompt.push_str(&room_context);    // room context ON TOP         │
│      prompt.push_str(&agent_docs);      // agent docs ON TOP           │
│      let tools = if room.tools_enabled { mode.tools } else { vec![] }; │
│      strategy = RoomSpeakerStrategy::new(RoomSpeakerConfig {           │
│          system_prompt: prompt,                                         │
│          tools, temperature: mode.temperature, ...                     │
│      })                                                                 │
│                                                                          │
│    DAG:                                                                 │
│      let mut prompt = mode.system_prompt;                               │
│      if let Some(schema) = output_schema {                              │
│          prompt.push_str(&schema_enforcement);  // schema ON TOP       │
│      }                                                                  │
│      strategy = DagStepStrategy::new(DagStepConfig {                   │
│          system_prompt: prompt,                                         │
│          tools: mode.tools, temperature: mode.temperature, ...         │
│      })                                                                 │
│                                                                          │
└──────────────────────────┬───────────────────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ 6. EXECUTE MAIN LLM                                                      │
│                                                                          │
│    result = ExecutionEngine::execute(                                   │
│        strategy: &strategy,                                             │
│        input: message.content,                                          │
│        sink: &stream_sink,                                              │
│        recorder: &recorder,                                             │
│        cancel: cancel_token                                             │
│    ).await?                                                             │
│                                                                          │
│    → LLM receives:                                                      │
│       - System prompt: "You are an expert programmer..."                │
│       - Temperature: 0.3                                                │
│       - Tools: [bash, read_file, edit_file, search_code] (4, not 50!)  │
│       - User message: "Help me debug this React component"              │
│                                                                          │
└──────────────────────────┬───────────────────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ 7. SAVE EXECUTION RECORD                                                 │
│                                                                          │
│    db.insert_agent_execution({                                          │
│        agent_id: agent.id,                                              │
│        selected_router_mode_id: mode.selected_mode_id,  ← Track mode   │
│        input: message.content,                                          │
│        output: result.content,                                          │
│        ...                                                              │
│    })                                                                   │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

### Fallback Behavior (No Router)

```
┌──────────────────────────────────────────────────────────────────────────┐
│ AGENT WITHOUT ROUTER (agent.router_id IS NULL)                          │
│                                                                          │
│    → Use agent's default configuration:                                 │
│       - system_prompt: agent.system_prompt                              │
│       - temperature: agent.temperature                                  │
│       - max_tokens: agent.max_tokens                                    │
│       - tools: ALL tools from agent_tools table                         │
│                                                                          │
│    → No classification step (direct execution)                          │
│    → selected_router_mode_id = NULL in agent_executions                │
│                                                                          │
│    ✅ BACKWARD COMPATIBLE: Existing agents work unchanged                │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

### Hierarchical Routing (L1 → L2 → L3)

```
┌──────────────────────────────────────────────────────────────────────────┐
│ HIERARCHICAL ROUTER EXAMPLE                                              │
│                                                                          │
│ User: "I'm really struggling with this math problem 😰"                  │
│                                                                          │
│ ┌────────────────────────────────────────────────────────────────────┐   │
│ │ L1 ROUTER: "support_style"                                         │   │
│ │ Modes: [supportive, direct, neutral]                               │   │
│ │ → Selected: "supportive"                                           │   │
│ └────────────────────────────────────────────────────────────────────┘   │
│                           │                                              │
│                           ▼                                              │
│ ┌────────────────────────────────────────────────────────────────────┐   │
│ │ L2 ROUTER: "supportive_tone" (parent_router_id = L1.id)           │   │
│ │ Modes: [empathetic, encouraging, gentle]                           │   │
│ │ → Selected: "empathetic"                                           │   │
│ └────────────────────────────────────────────────────────────────────┘   │
│                           │                                              │
│                           ▼                                              │
│ ┌────────────────────────────────────────────────────────────────────┐   │
│ │ L3 ROUTER: "formality" (parent_router_id = L2.id)                 │   │
│ │ Modes: [casual, professional, warm]                                │   │
│ │ → Selected: "warm"                                                 │   │
│ └────────────────────────────────────────────────────────────────────┘   │
│                           │                                              │
│                           ▼                                              │
│                   Final Mode Config:                                     │
│                   - system_prompt: "You are a warm, empathetic..."      │
│                   - temperature: 0.8                                    │
│                   - tools: [calculator, wolfram_alpha]                  │
│                                                                          │
│ Implementation:                                                          │
│   for level in 1..=3:                                                   │
│       if router := get_router_at_level(level):                          │
│           mode := classify(router, input)                               │
│           if level < 3 and has_child_router(router):                    │
│               continue  # Go deeper                                     │
│           else:                                                         │
│               return mode  # Use this mode                              │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## Implementation Plan

### Phase 1: Database Migration

**File**: `migrations/064_tool_router_modes.sql`

```sql
-- 1. Add hierarchy support to tool_routers
ALTER TABLE tool_routers
ADD COLUMN parent_router_id UUID REFERENCES tool_routers(id) ON DELETE CASCADE,
ADD COLUMN level INT DEFAULT 1 CHECK (level IN (1, 2, 3));

-- 2. Create tool_router_modes table
CREATE TABLE tool_router_modes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    router_id UUID NOT NULL REFERENCES tool_routers(id) ON DELETE CASCADE,
    mode_key TEXT NOT NULL,
    display_name TEXT NOT NULL,
    description TEXT NOT NULL,
    system_prompt TEXT NOT NULL,
    temperature REAL NOT NULL DEFAULT 0.7,
    max_tokens INT NOT NULL DEFAULT 4096,
    append_to_agent_system_prompt BOOLEAN NOT NULL DEFAULT FALSE,  -- Append mode prompt to agent's or replace
    append_to_agent_tools BOOLEAN NOT NULL DEFAULT TRUE,           -- Add mode tools to agent's or replace
    display_order INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (router_id, mode_key),
    CHECK (mode_key ~ '^[a-z][a-z0-9_]*$'),
    CHECK (temperature BETWEEN 0.0 AND 2.0),
    CHECK (max_tokens > 0)
);

-- 3. Create tool_router_mode_tools junction table
CREATE TABLE tool_router_mode_tools (
    mode_id UUID NOT NULL REFERENCES tool_router_modes(id) ON DELETE CASCADE,
    tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
    PRIMARY KEY (mode_id, tool_id)
);

-- 4. Add router_id to agents
ALTER TABLE agents
ADD COLUMN router_id UUID REFERENCES tool_routers(id) ON DELETE SET NULL;

-- 5. Track selected mode in executions
ALTER TABLE agent_executions
ADD COLUMN selected_router_mode_id UUID REFERENCES tool_router_modes(id) ON DELETE SET NULL;

-- 6. Deprecate agent_modes
COMMENT ON TABLE agent_modes IS 'DEPRECATED: Use tool_router_modes instead. Migrate data and drop in future.';

-- Indexes
CREATE INDEX idx_tool_routers_parent ON tool_routers(parent_router_id);
CREATE INDEX idx_tool_router_modes_router ON tool_router_modes(router_id);
CREATE INDEX idx_tool_router_mode_tools_tool ON tool_router_mode_tools(tool_id);
CREATE INDEX idx_agents_router ON agents(router_id);
CREATE INDEX idx_agent_executions_router_mode ON agent_executions(selected_router_mode_id);
```

### Phase 2: Database Layer

**Files to create/modify**:
- `src/db/mod.rs` - Add `ToolRouterModeRow` struct
- `src/db/traits/mod.rs` - Extend `ToolRouterRepo` trait
- `src/db/pg_repo/mod.rs` - Implement mode CRUD operations
- `src/db/queries/mod.rs` - Add query functions

**Key operations**:
```rust
// Trait additions
trait ToolRouterRepo {
    // Existing operations...

    // NEW: Mode management
    async fn list_router_modes(&self, router_id: Uuid) -> Result<Vec<ToolRouterModeRow>>;
    async fn get_router_mode(&self, id: Uuid) -> Result<Option<ToolRouterModeRow>>;
    async fn get_router_mode_by_key(&self, router_id: Uuid, key: &str) -> Result<Option<ToolRouterModeRow>>;
    async fn create_router_mode(&self, ...) -> Result<ToolRouterModeRow>;
    async fn update_router_mode(&self, ...) -> Result<ToolRouterModeRow>;
    async fn delete_router_mode(&self, id: Uuid) -> Result<()>;

    // Mode tool associations
    async fn get_mode_tools(&self, mode_id: Uuid) -> Result<Vec<ToolRow>>;
    async fn set_mode_tools(&self, mode_id: Uuid, tool_ids: &[Uuid]) -> Result<()>;
}
```

### Phase 3: Tool Registry (Static Definitions)

**File**: `src/tools/registry.rs` (NEW)

```rust
//! Tool registry - maps tool names to hardcoded implementations

use crate::llm::Tool;

/// Get a tool definition by name.
/// Returns None if the tool is not registered.
pub fn get_tool_definition(name: &str) -> Option<Tool> {
    match name {
        "bash" => Some(bash_tool()),
        "read_file" => Some(read_file_tool()),
        "write_file" => Some(write_file_tool()),
        "edit_file" => Some(edit_file_tool()),
        "search_code" => Some(search_code_tool()),
        "web_search" => Some(web_search_tool()),
        "github_create_pr" => Some(github_pr_tool()),
        // ... all other tools
        _ => None,
    }
}

fn bash_tool() -> Tool {
    Tool {
        name: "bash".to_string(),
        description: "Execute bash commands on the system".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The bash command to execute"
                }
            },
            "required": ["command"]
        }),
    }
}

// ... other tool definitions
```

### Phase 4: ModeResolver

**File**: `src/server/hub/mode_resolver/mod.rs` + `tests.rs` (NEW)

```rust
//! Mode resolution service — resolves agent + input into execution config.
//! Data-only: returns ResolvedModeConfig. Does NOT create strategies or call the engine.

use std::sync::Arc;
use uuid::Uuid;
use crate::db::{AgentRow, ToolRouterModeRow};
use crate::db::traits::{ServerRepo, ToolRouterRepo};
use crate::llm::{LLMProvider, Tool};
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::strategies::router::RouterStrategy;
use crate::tools::registry;

pub struct ModeResolver {
    repo: Arc<dyn ServerRepo>,
    tool_router_repo: Arc<dyn ToolRouterRepo>,
    provider: Arc<dyn LLMProvider>,
}

/// Resolved configuration after mode classification.
pub struct ResolvedModeConfig {
    pub system_prompt: String,
    pub tools: Vec<Tool>,
    pub tool_names: Vec<String>,
    pub temperature: f32,
    pub max_tokens: i32,
    pub selected_mode_id: Option<Uuid>,
    pub selected_mode_key: Option<String>,
}

impl ModeResolver {
    /// Resolve mode config for an agent. Works for ANY execution context.
    ///
    /// - `agent`: The agent row (already loaded by caller)
    /// - `user_input`: The user's message (for classification)
    /// - `context_hint`: Optional extra context for the router LLM.
    ///   Chat passes formatted history, rooms pass transcript,
    ///   DAGs pass step description.
    ///
    /// Returns agent defaults if agent has no router_id.
    pub async fn resolve(
        &self,
        agent: &AgentRow,
        user_input: &str,
        context_hint: Option<&str>,
    ) -> Result<ResolvedModeConfig, RoutingError> {
        // 1. If no router_id → return agent defaults
        let router_id = match agent.router_id {
            Some(id) => id,
            None => return self.agent_defaults(agent).await,
        };

        // 2. Load router + modes
        let router = self.tool_router_repo.get_tool_router(router_id).await?
            .ok_or(RoutingError::RouterNotFound)?;
        let modes = self.tool_router_repo.list_router_modes(router_id).await?;
        if modes.is_empty() {
            return Err(RoutingError::NoModesConfigured);
        }

        // 3. Build classification prompt (input + context_hint + mode list)
        let prompt = build_classification_prompt(user_input, context_hint, &modes);

        // 4. Call Router LLM via ExecutionEngine + RouterStrategy
        let strategy = RouterStrategy::new(RouterConfig {
            system_prompt: router.system_prompt.clone(),
            model_id: router.model_id.clone(),
            state: None,
            user_id: None,
        });
        let result = ExecutionEngine::execute(
            &strategy, &prompt, &NoOpSink, &NoOpRecorder, None,
        ).await?;

        // 5. Parse mode key (fallback to first mode)
        let mode_key = parse_mode_key(&result.content)?;
        let mode = self.tool_router_repo
            .get_router_mode_by_key(router_id, &mode_key).await?
            .or_else(|| modes.first().cloned())
            .ok_or(RoutingError::NoModesConfigured)?;

        // 6. Load mode tools
        let mode_tool_rows = self.tool_router_repo.get_mode_tools(mode.id).await?;
        let mode_tools: Vec<Tool> = mode_tool_rows.iter()
            .filter_map(|row| registry::get_tool_definition(&row.name))
            .collect();

        // 7. Merge system prompt (append or replace)
        let system_prompt = if mode.append_to_agent_system_prompt {
            format!("{}\n\n{}", agent.system_prompt, mode.system_prompt)
        } else {
            mode.system_prompt.clone()
        };

        // 8. Merge tools (union or replace)
        let tools = if mode.append_to_agent_tools {
            let agent_tool_rows = self.repo.get_agent_tools(agent.id).await?;
            let agent_tools: Vec<Tool> = agent_tool_rows.iter()
                .filter_map(|row| registry::get_tool_definition(&row.name))
                .collect();
            union_by_name(agent_tools, mode_tools)
        } else {
            mode_tools
        };

        let tool_names = tools.iter().map(|t| t.name.clone()).collect();

        // 9. Return config
        Ok(ResolvedModeConfig {
            system_prompt,
            tools,
            tool_names,
            temperature: mode.temperature,
            max_tokens: mode.max_tokens,
            selected_mode_id: Some(mode.id),
            selected_mode_key: Some(mode.mode_key.clone()),
        })
    }
}

fn build_classification_prompt(
    input: &str,
    context_hint: Option<&str>,
    modes: &[ToolRouterModeRow],
) -> String {
    let context_block = context_hint
        .map(|c| format!("## Context:\n{}\n\n", c))
        .unwrap_or_default();

    let mode_list = modes.iter()
        .map(|m| format!("- {}: {}", m.mode_key, m.description))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "{context_block}\
         ## Current User Input:\n{input}\n\n\
         ## Available Modes:\n{mode_list}\n\n\
         Based on the context and current input, output ONLY the mode key.",
    )
}
```

### Phase 5: Update Call Sites

Each call site adds a `mode_resolver.resolve()` call before building its strategy.

**File**: `src/server/hub/mod.rs` (MODIFY — chat)

```rust
// In run_chat(), BEFORE building ChatStrategy:
let agent = repo.get_agent(agent_id).await?;
let history_text = format_history(&session_history);
let mode = mode_resolver.resolve(&agent, input, Some(&history_text)).await?;

let config = ChatConfig {
    system_prompt: mode.system_prompt,
    tool_names: mode.tool_names,
    temperature: mode.temperature,
    model_id: agent.model_id.clone(),
    ..Default::default()
};
let strategy = ChatStrategy::new(config, state, user_id, session_id, message_id);
engine.execute(&strategy, input, &sink, &recorder, cancel).await?
```

**File**: `src/server/room_executor/mod.rs` (MODIFY — rooms)

```rust
// In execute_room_turn(), BEFORE building RoomSpeakerStrategy:
let mode = mode_resolver.resolve(&agent, user_message, Some(&transcript_block)).await?;

let mut system_prompt = mode.system_prompt;
system_prompt.push_str("\n\n");
system_prompt.push_str(&room_context);
system_prompt.push_str(&agent_docs_block);

let (tools, tool_names) = if room.tools_enabled {
    (mode.tools, mode.tool_names)
} else {
    (vec![], vec![])
};

let config = RoomSpeakerConfig {
    system_prompt,
    tools,
    tool_names,
    temperature: mode.temperature,  // NEW field
    ...
};
```

**File**: `src/server/hub/dag/mod.rs` (MODIFY — DAG steps)

```rust
// In run_step_via_engine(), BEFORE building DagStepStrategy:
let mode = mode_resolver.resolve(&agent, prompt, Some(&step_description)).await?;

let mut system_prompt = mode.system_prompt;
if let Some(schema) = output_schema {
    system_prompt.push_str(&format!("\n\nRespond with JSON matching:\n```json\n{}\n```", schema));
}

let config = DagStepConfig {
    system_prompt,
    tools: mode.tools,
    tool_names: mode.tool_names,
    temperature: mode.temperature,  // NEW field
    ...
};
```

**Required struct changes** (for temperature override support):
- Add `temperature: f32` to `RoomSpeakerConfig`
- Add `temperature: f32` to `DagStepConfig`
- Update `RoomSpeakerStrategy::temperature()` to use `self.config.temperature`
- Update `DagStepStrategy::temperature()` to use `self.config.temperature`

### Phase 6: API Endpoints

**File**: `src/server/api/tool_routers/modes.rs` (NEW)

```rust
// GET /api/tool-routers/:id/modes
pub async fn list_router_modes(...)

// POST /api/tool-routers/:id/modes
pub async fn create_router_mode(...)

// GET /api/tool-routers/:id/modes/:mode_id
pub async fn get_router_mode(...)

// PATCH /api/tool-routers/:id/modes/:mode_id
pub async fn update_router_mode(...)

// DELETE /api/tool-routers/:id/modes/:mode_id
pub async fn delete_router_mode(...)

// PUT /api/tool-routers/:id/modes/:mode_id/tools
pub async fn set_mode_tools(...)  // Set which tools this mode has
```

### Phase 7: Frontend UI

**Files**:
- `frontend/src/types/router.ts` - TypeScript types
- `frontend/src/pages/Routers/RouterBuilderPage.tsx` - Mode configuration UI
- `frontend/src/components/ModeEditor.tsx` - Edit individual modes
- `frontend/src/components/ToolSelector.tsx` - Select tools for each mode

---

## Edge Cases & Defaults

### 1. Agent Without Router (`agent.router_id IS NULL`)

**Behavior**: Use agent's default configuration

```rust
if agent.router_id.is_none() {
    // No routing - use agent defaults
    system_prompt = agent.system_prompt;
    temperature = agent.temperature;
    tools = load_all_agent_tools(agent_id);  // All tools
    selected_mode_id = None;
}
```

**Result**: Backward compatible - existing agents work unchanged.

### 2. Router With No Modes

**Behavior**: Error - router must have at least one mode

```rust
let modes = repo.list_router_modes(router_id).await?;
if modes.is_empty() {
    return Err(RoutingError::NoModesConfigured);
}
```

**Frontend**: Enforce at least 1 mode in UI validation.

### 3. Mode With No Tools

**Behavior**: Allow - mode can be tool-free (pure conversation)

```rust
let tools = repo.get_mode_tools(mode_id).await?;
// tools can be empty - that's valid for chat-only modes
```

**Use case**: "chat" mode with no tools, just conversation.

### 4. Router LLM Returns Invalid Mode Key

**Behavior**: Fallback to first mode (sorted by display_order)

```rust
let mode = repo.get_router_mode_by_key(router_id, &mode_key).await?
    .or_else(|| {
        warn!("Invalid mode key '{}', falling back to default", mode_key);
        modes.first().cloned()
    })
    .ok_or(RoutingError::NoModesConfigured)?;
```

### 5. Tool in DB But Not in Code Registry

**Behavior**: Skip tool with warning

```rust
let tools: Vec<Tool> = tool_rows
    .iter()
    .filter_map(|row| {
        registry::get_tool_definition(&row.name)
            .or_else(|| {
                warn!("Tool '{}' not found in registry, skipping", row.name);
                None
            })
    })
    .collect();
```

### 6. Hierarchical Router (L1 → L2 → L3)

**Behavior**: Chain classifications

```rust
async fn route_hierarchical(
    router_id: Uuid,
    input: &str,
    level: u32,
) -> Result<SelectedMode> {
    // Classify at current level
    let mode = route_at_level(router_id, input).await?;

    // Check if there's a child router
    if let Some(child_router) = get_child_router(router_id).await? {
        // Go deeper
        return route_hierarchical(child_router.id, input, level + 1).await;
    }

    // This is the final level
    Ok(mode)
}
```

---

## Integration Points

### Where Can We Use Routing?

```
┌──────────────────────────────────────────────────────────────┐
│ CURRENT LLM CALL SITES (via ExecutionEngine)                 │
└──────────────────────────────────────────────────────────────┘

1. ✅ CHAT MESSAGES (src/server/chat_consumer/mod.rs)
   - User sends message in chat session
   - Agent responds
   → CAN USE ROUTING: YES (primary use case)

2. ✅ WORKFLOW STEPS (src/server/workflow_executor/mod.rs)
   - DAG step executes with agent
   - Agent processes step input
   → CAN USE ROUTING: YES (step-level mode selection)

3. ✅ ROOM SESSIONS (src/server/room_executor/mod.rs)
   - Multi-agent discussion
   - Each agent takes turn
   → CAN USE ROUTING: YES (per-speaker mode selection)

4. ❌ TOOL ROUTING (src/server/hub/strategies/router/mod.rs)
   - Already IS routing (meta!)
   - Outputs JSON decision, not conversation
   → CAN USE ROUTING: NO (would be circular)

5. ⚠️  AGENT EXECUTIONS (interactive)
   - User sends follow-up to agent execution
   → CAN USE ROUTING: YES, but mode should persist per execution

6. ✅ CHAT STRATEGIES (anywhere ExecutionEngine is called)
   - Any code that calls ExecutionEngine.execute()
   → CAN USE ROUTING: YES (if agent has router_id)
```

### ExecutionEngine Is Routing-Agnostic

**Key insight**: The `ExecutionEngine` doesn't know about routing. It just receives:
- `ExecutionStrategy` (provides system_prompt, tools, temperature)
- Input text
- Streaming sink

**Routing happens BEFORE the engine**:
```rust
// Routing layer (if agent has router)
let selected = routing_service.route(router_id, input).await?;

// Strategy creation (uses routing result or agent defaults)
let strategy = ChatStrategy::new(ChatConfig {
    system_prompt: selected.system_prompt,  // From routing
    tools: selected.tools,                  // From routing
    temperature: selected.temperature,      // From routing
    ...
});

// Engine execution (routing-agnostic)
engine.execute(&strategy, input, ...).await?;
```

### Universal Routing Pattern

```rust
/// Universal routing helper - call before ANY ExecutionEngine.execute()
async fn prepare_execution_config(
    agent: &AgentRow,
    user_input: &str,
    routing_service: &RoutingService,
) -> Result<ExecutionConfig> {
    if let Some(router_id) = agent.router_id {
        // Route to select mode
        let selected = routing_service.route(router_id, user_input).await?;
        Ok(ExecutionConfig {
            system_prompt: selected.system_prompt,
            tools: selected.tools,
            temperature: selected.temperature,
            max_tokens: selected.max_tokens,
            selected_mode_id: Some(selected.mode_id),
        })
    } else {
        // Use agent defaults
        let tools = load_all_agent_tools(agent.id).await?;
        Ok(ExecutionConfig {
            system_prompt: agent.system_prompt.clone(),
            tools,
            temperature: agent.temperature,
            max_tokens: agent.max_tokens,
            selected_mode_id: None,
        })
    }
}
```

---

## Benefits Summary

### 1. Context Optimization
- **Before**: 50 tools sent on every request (20-30KB of context)
- **After**: 3-5 tools sent per request (2-4KB of context)
- **Savings**: 85-90% reduction in tool context

### 2. Better Responses
- **Focused tools** = LLM picks the right tool more often
- **Mode-specific prompts** = Better personality matching
- **Lower temperature for technical tasks** = More accurate code

### 3. Cost Reduction
- Fewer input tokens per request
- Faster responses (less to process)
- Estimated 20-30% cost savings on chat workloads

### 4. Flexibility
- Per-agent routing (some agents route, others don't)
- Per-mode tool sets (coding gets bash+edit, research gets web_search)
- Hierarchical refinement (L1 → L2 → L3 for nuanced selection)

### 5. Observability
- Track which modes are selected (`selected_router_mode_id`)
- Analyze mode effectiveness
- A/B test different mode configurations

---

## Migration Path from `agent_modes`

```sql
-- Migration script (run after new tables created)

-- 1. Create a default router for each agent that has modes
INSERT INTO tool_routers (id, user_id, name, description, system_prompt, model_id)
SELECT
    gen_random_uuid(),
    user_id,
    'Agent ' || name || ' Router',
    'Auto-migrated from agent_modes',
    'Select the appropriate mode based on the user input.',
    'claude-haiku-4-20250414'
FROM agents
WHERE id IN (SELECT DISTINCT agent_id FROM agent_modes);

-- 2. Link agents to their routers
UPDATE agents a
SET router_id = (
    SELECT r.id
    FROM tool_routers r
    WHERE r.name = 'Agent ' || a.name || ' Router'
)
WHERE a.id IN (SELECT DISTINCT agent_id FROM agent_modes);

-- 3. Migrate agent_modes → tool_router_modes
INSERT INTO tool_router_modes (
    id, router_id, mode_key, display_name, description,
    system_prompt, temperature, max_tokens
)
SELECT
    am.id,
    tr.id,
    am.name,
    am.name,
    am.classifier_hint,
    COALESCE(am.system_prompt_suffix, ''),  -- suffix becomes full prompt
    COALESCE(am.temperature_override, 0.7),
    4096
FROM agent_modes am
JOIN agents a ON a.id = am.agent_id
JOIN tool_routers tr ON tr.name = 'Agent ' || a.name || ' Router';

-- 4. Migrate tool associations (if tool_overrides has data)
-- This is tricky since tool_overrides is TEXT[], need to match to tool IDs
-- Might require custom script

-- 5. Drop old table (after verification)
-- DROP TABLE agent_modes_versions CASCADE;
-- DROP TABLE agent_modes CASCADE;
```

---

## Open Questions & Decisions

### Q1: Should router_id be on agents or chat_sessions?

**Option A**: `agents.router_id` (RECOMMENDED)
- Agent always uses the same router
- Simpler model

**Option B**: `chat_sessions.router_id`
- Different sessions can use different routers
- More flexibility but more complexity

**Decision**: Start with Option A, add Option B later if needed.

### Q2: How to handle mode persistence in multi-turn conversations?

**Option A**: Re-route on every message
- Adapts to changing user intent
- More LLM calls (cost)

**Option B**: Route once per session, persist mode
- Consistent personality throughout session
- Less adaptive

**Decision**: Start with Option A (re-route every message). Add session-level mode locking as UI feature later.

### Q3: Should modes have version history?

**Decision**: Yes, add `tool_router_modes_versions` table (follow existing versioning pattern).

### Q4: What's the default mode if routing fails?

**Decision**: First mode (sorted by display_order).

---

## Glossary

- **Router**: An LLM-based classifier that selects modes
- **Mode**: A configuration (system_prompt + tools + temperature)
- **Classification**: The process of selecting a mode from user input
- **Tool Subsetting**: Sending only relevant tools, not all tools
- **Hierarchical Routing**: Chaining routers (L1 → L2 → L3) for refinement
- **Fallback**: What happens when routing isn't configured (use agent defaults)

---

## Architecture Summary

### The Complete Picture

```
APPLICATION LAYER (each call site builds its own strategy)
    │
    │ mode_resolver.resolve(agent, input, context_hint)
    │
    ▼
MODE RESOLUTION LAYER (ModeResolver) - NEW
    │
    ├─ Check agent.router_id
    ├─ Load router + modes from DB
    ├─ Classify via Router LLM (input + context_hint)
    ├─ Merge system prompt (append/replace)
    ├─ Merge tools (append/replace)
    │
    │ Returns ResolvedModeConfig
    │
    ▼
APPLICATION LAYER (call site applies its own context)
    │
    ├─ Chat:  uses config directly → ChatStrategy
    ├─ Room:  appends room context + agent docs → RoomSpeakerStrategy
    ├─ DAG:   appends schema enforcement → DagStepStrategy
    │
    │ engine.execute(strategy, ...)
    │
    ▼
EXECUTION LAYER (ExecutionEngine) - UNCHANGED
    │
    ├─ Tool use loop
    ├─ Streaming
    ├─ Token tracking
    │
    └─ Returns ExecutionResult
```

### Key Points

1. **ExecutionEngine Stays Pure** ✅
   - No agent/routing awareness
   - Just executes strategies
   - Perfectly designed, leave it unchanged

2. **ModeResolver Is Data-Only** ✅
   - Resolves config, does NOT create strategies or call the engine
   - One function: `resolve(agent, input, context_hint) → ResolvedModeConfig`
   - Context-agnostic: doesn't know about rooms, DAGs, or chat
   - Easy to test: input agent + text → output config

3. **Call Sites Stay In Control** ✅
   - Each call site knows its own context (room preamble, schema, history)
   - Each call site builds its own strategy type
   - No context leaking between execution environments
   - Room-specific logic stays in room_executor
   - DAG-specific logic stays in dag/mod.rs

4. **Router Sees Context From Any Environment** ✅
   - Chat passes formatted message history
   - Rooms pass multi-agent transcript
   - DAGs pass step/workflow description
   - Same classification logic, different context sources

5. **Modes Work Everywhere** ✅
   - Chat: mode changes personality + tools
   - Room: mode changes personality + tools, room context preserved on top
   - DAG: mode changes personality + tools, schema enforcement preserved on top
   - Any future execution context: just call `mode_resolver.resolve()`

### Usage Pattern

**Chat** (hub/mod.rs):
```rust
let agent = repo.get_agent(agent_id).await?;
let history_text = format_history(&session_history);
let mode = mode_resolver.resolve(&agent, input, Some(&history_text)).await?;

let config = ChatConfig {
    system_prompt: mode.system_prompt,  // agent + mode already merged
    tool_names: mode.tool_names,
    temperature: mode.temperature,
    model_id: agent.model_id,
    ..Default::default()
};
let strategy = ChatStrategy::new(config, state, user_id, session_id, message_id);
engine.execute(&strategy, input, &sink, &recorder, cancel).await?
```

**Room** (room_executor.rs):
```rust
let mode = mode_resolver.resolve(&agent, user_message, Some(&transcript_block)).await?;

// Mode resolves personality, room executor adds structural context ON TOP
let mut system_prompt = mode.system_prompt;
system_prompt.push_str("\n\n");
system_prompt.push_str(&room_context);       // "## Room Context\nYou are SecurityLead..."
system_prompt.push_str(&agent_docs_block);   // agent knowledge docs

// Room.tools_enabled is a master switch
let (tools, tool_names) = if room.tools_enabled {
    (mode.tools, mode.tool_names)
} else {
    (vec![], vec![])
};

let config = RoomSpeakerConfig {
    system_prompt,
    tools,
    tool_names,
    temperature: mode.temperature,  // NEW field, required for mode override
    ...
};
let strategy = RoomSpeakerStrategy::new(config, state);
engine.execute(&strategy, user_message, &sink, &recorder, cancel).await?
```

**DAG** (dag/mod.rs):
```rust
let mode = mode_resolver.resolve(&agent, prompt, Some(&step_description)).await?;

// Mode resolves personality, DAG adds schema enforcement ON TOP
let mut system_prompt = mode.system_prompt;
if let Some(schema) = output_schema {
    system_prompt.push_str(&format!("\n\nRespond with JSON matching:\n```json\n{}\n```", schema));
}

let config = DagStepConfig {
    system_prompt,
    tools: mode.tools,
    tool_names: mode.tool_names,
    temperature: mode.temperature,  // NEW field, required for mode override
    ...
};
let strategy = DagStepStrategy::new(config, state);
engine.execute(&strategy, prompt, &sink, &recorder, cancel).await?
```

---

## Practical Examples: Union vs Replace (System Prompts & Tools)

### Example 1: Coding Mode (Append System Prompt, Union Tools)

**Scenario**: Agent has base personality and tools. "Coding" mode adds programming-specific instructions and tools.

**Agent Configuration**:
- System Prompt: `"You are a helpful AI assistant."`
- Base Tools: `[read_file, write_file, web_search]`

**Mode Configuration**:
```json
{
  "mode_key": "coding",
  "display_name": "Coding Mode",
  "description": "For programming and development tasks",
  "system_prompt": "Focus on code quality, best practices, and clear explanations. Use bash and git tools when appropriate.",
  "temperature": 0.3,
  "max_tokens": 8000,
  "append_to_agent_system_prompt": true,   // APPEND
  "append_to_agent_tools": true,           // UNION
  "tools": ["bash", "git", "edit_file", "search_code"]
}
```

**Result**:
```
System Prompt:
  "You are a helpful AI assistant.

   Focus on code quality, best practices, and clear explanations. Use bash and git tools when appropriate."
  └── Agent base ──┘  └──────────────── Mode addition ────────────────────┘

Tools:
  [read_file, write_file, web_search, bash, git, edit_file, search_code]
   └────── Agent base tools ────────┘  └────── Mode tools ────────┘
```

**Why Append/Union?** Agent's base personality is preserved while adding coding-specific guidance. General tools (read/write/search) remain available alongside dev tools (bash/git).

---

### Example 2: Minimal Chat Mode (Replace System Prompt, Replace Tools)

**Scenario**: Pure conversation mode with completely different personality and no tools.

**Agent Configuration**:
- System Prompt: `"You are a helpful AI assistant that can help with coding, research, and general tasks."`
- Base Tools: `[read_file, write_file, web_search, bash, ... 46 more tools]`

**Mode Configuration**:
```json
{
  "mode_key": "minimal_chat",
  "display_name": "Minimal Chat",
  "description": "Pure conversation without tools",
  "system_prompt": "You are a friendly conversational AI focused on engaging dialogue and thoughtful responses.",
  "temperature": 0.9,
  "max_tokens": 2048,
  "append_to_agent_system_prompt": false,  // REPLACE
  "append_to_agent_tools": false,          // REPLACE
  "tools": []  // Empty!
}
```

**Result**:
```
System Prompt:
  "You are a friendly conversational AI focused on engaging dialogue and thoughtful responses."
  └── Mode completely replaces agent's system prompt ──┘

Tools:
  []  (no tools available)
  └── Mode replaces with empty list
```

**Why Replace Both?** Completely different use case - want pure conversation with specific personality, no task-oriented language, and zero tool overhead.

---

### Example 3: Research Mode (Append System Prompt, Replace Tools)

**Scenario**: Research tasks need specific instructions and ONLY information gathering tools.

**Agent Configuration**:
- System Prompt: `"You are a helpful AI assistant."`
- Base Tools: `[read_file, write_file, bash, git, edit_file, search_code, web_search, web_fetch, ...]`

**Mode Configuration**:
```json
{
  "mode_key": "research",
  "display_name": "Research Mode",
  "description": "For information gathering and analysis",
  "system_prompt": "Focus on finding accurate information from reliable sources. Cite sources when possible. Avoid speculation.",
  "temperature": 0.5,
  "max_tokens": 4096,
  "append_to_agent_system_prompt": true,   // APPEND
  "append_to_agent_tools": false,          // REPLACE
  "tools": ["web_search", "web_fetch", "read_file"]
}
```

**Result**:
```
System Prompt:
  "You are a helpful AI assistant.

   Focus on finding accurate information from reliable sources. Cite sources when possible. Avoid speculation."
  └── Agent base ──┘  └──────────────── Mode addition ──────────────────┘

Tools:
  [web_search, web_fetch, read_file]
  └── Only research tools (replaced agent's full toolset)
```

**Why Append System + Replace Tools?** Keep agent's base personality but add research-specific guidance. Replace tools because code execution (bash/git/edit) is irrelevant and wastes context.

---

### Example 4: Debug Mode (Append System Prompt, Union Tools)

**Scenario**: Debug mode adds specialized debugging instructions and tools while keeping base capabilities.

**Agent Configuration**:
- System Prompt: `"You are a helpful AI assistant."`
- Base Tools: `[read_file, write_file, edit_file, bash]`

**Mode Configuration**:
```json
{
  "mode_key": "debug",
  "display_name": "Debug Mode",
  "description": "For debugging and troubleshooting code",
  "system_prompt": "Approach problems systematically. Check logs, reproduce errors, isolate root causes. Use debugging tools to inspect runtime behavior.",
  "temperature": 0.2,
  "max_tokens": 8000,
  "append_to_agent_system_prompt": true,   // APPEND
  "append_to_agent_tools": true,           // UNION
  "tools": ["strace", "gdb", "profiler", "log_viewer"]
}
```

**Result**:
```
System Prompt:
  "You are a helpful AI assistant.

   Approach problems systematically. Check logs, reproduce errors, isolate root causes. Use debugging tools to inspect runtime behavior."
  └── Agent base ──┘  └──────────────────── Mode addition ───────────────────────┘

Tools:
  [read_file, write_file, edit_file, bash, strace, gdb, profiler, log_viewer]
   └────── Agent base tools ────────┘  └───── Debug tools ──────┘
```

**Why Append/Union?** Base personality preserved with added debugging methodology. Basic file ops still needed plus specialized debug tools.

---

### Decision Matrix: When to Append vs Replace

#### System Prompts

| Scenario | `append_to_agent_system_prompt` | Reasoning |
|----------|--------------------------------|-----------|
| **Mode adds specific instructions** | `true` (APPEND) | Agent's base personality preserved, mode adds task-specific guidance |
| **Mode tweaks behavior** | `true` (APPEND) | "Be more concise" or "Focus on security" appended to base prompt |
| **Completely different personality** | `false` (REPLACE) | Chat mode vs task mode need totally different personalities |
| **Safety/compliance override** | `false` (REPLACE) | Replace entire prompt with strict safety instructions |
| **Context optimization** | `false` (REPLACE) | Shorter prompt saves tokens when agent's base prompt is verbose |

**Default**: `false` (replace) - modes typically define complete behavior, appending is less common.

#### Tools

| Scenario | `append_to_agent_tools` | Reasoning |
|----------|------------------------|-----------|
| **Specialized mode extending base capabilities** | `true` (UNION) | Mode adds domain-specific tools while keeping general tools available |
| **Context optimization (replace with subset)** | `false` (REPLACE) | Mode knows exact tools needed, replacing agent's default set saves context |
| **Minimal/restricted mode** | `false` (REPLACE) | Explicitly remove tool access (e.g., chat-only mode with empty tools) |
| **Tool experimentation** | `false` (REPLACE) | Testing new tool combinations without agent's tools interfering |
| **Safety/sandboxing** | `false` (REPLACE) | Restrict agent to safe subset (e.g., no bash/git in prod environments) |

**Default**: `true` (union) - safest choice, preserves agent's base capabilities.

#### Common Combinations

| Use Case | System Prompt | Tools | Example |
|----------|--------------|-------|---------|
| **Specialized mode** | APPEND | UNION | Coding mode: keep base personality, add dev guidance + tools |
| **Pure conversation** | REPLACE | REPLACE | Chat mode: different personality, no tools at all |
| **Context-optimized research** | APPEND | REPLACE | Keep personality, add research focus, only search tools |
| **Emergency safety mode** | REPLACE | REPLACE | Override everything with safe behavior + no dangerous tools |

---

## Next Steps

1. ~~Review this document with the team~~ DONE
2. ~~Get approval on schema design~~ DONE
3. ~~Implement Phase 1 (database migration)~~ DONE (`9c32a22`)
4. ~~Build Phase 2 (database layer)~~ DONE (`725009a`)
5. ~~Build Phase 3 (tool registry)~~ DONE (`ed2006d`)
6. ~~Build Phase 4 (ModeResolver)~~ DONE
7. ~~Update call sites (Phase 5)~~ DONE
8. Build API endpoints (Phase 6)
9. Build frontend UI (Phase 7)
10. Migrate existing agent_modes data
11. Drop agent_modes table (see DELETION_PLAN.md)
12. Document and deploy

---

---

## Implementation Phase Updates

**IMPORTANT**: The implementation phases have been updated based on the ModeResolver architecture design:

- **Phase 1**: DONE — Database migration (`9c32a22`)
- **Phase 2**: DONE — Database layer (`725009a`)
- **Phase 3**: DONE — Tool registry (`ed2006d`)
- **Phase 4**: DONE — ModeResolver
- **Phase 5**: DONE — Update Call Sites (chat, rooms, DAG)
- **Phase 6-7**: Unchanged (API endpoints, Frontend UI)

**Key Change**: ModeResolver is a data-only service. Each call site calls `mode_resolver.resolve()` to get config, then applies its own context (room preamble, schema enforcement, etc.) and builds its own strategy type. No monolithic orchestrator.

---

**Document Version**: 1.3
**Last Updated**: 2026-02-04
**Author**: Claude (Sonnet 4.5 / Opus 4.5)
