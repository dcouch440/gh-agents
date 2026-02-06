# Plan: Unified Workflow System - Ports, Capabilities, Cavernous Routing & Collaborative Agents

## Executive Summary

**Goal:** Transform nexor into a complete agentic orchestration platform with three execution tiers: static agents, label-based routing, and cavernous dynamic routing - all built on port-based data flow with collaborative agent rooms.

**Scope:** Complete system redesign - backend data model, execution engine, tool capability system, enhanced rooms, document-based routing configs, and master system configuration. UI vision documented for future.

**Strategy:** Build all systems together from the start. No refactoring - unified architecture supporting:
1. **Port-based workflows** - Explicit input/output connections (replaces variables)
2. **Tool capability registry** - Semantic tool selection (extends existing mode system)
3. **Cavernous routing** - Document-based configuration with agent collaboration
4. **Enhanced rooms** - Structured agent-to-agent output passing
5. **Master system config** - Admin-controlled truth defining capabilities, tools, constraints

**Key Innovation:** Document-based routing configuration - agents search document titles to find routing configs, enabling rich instructions without bloating LLM context. Agents can collaborate to select optimal routing.

**Application Status:** Has not run in production. No backwards compatibility concerns. Clean slate implementation.

**Access Model:** System admin defines routing configs, capabilities, and master rules. Power users build workflows using pre-configured elements.

---

## System Overview

### High-Level Architecture

```
User Request / Workflow Trigger
    ↓
Master System Config (capabilities, tools, agents, routing strategies, constraints)
    ↓
Workflow Definition (nodes + edges + ports)
    ↓
Mode Resolution (agent + router → tools + capabilities + system prompt)
    ↓
DAG Executor (topological execution)
    ↓
Step Execution (port-based inputs)
    ├─ TIER 1: Single Agent Execution (static, predictable)
    ├─ TIER 2: For-Each with Label Routing (array → specialist agents)
    └─ TIER 3: Cavernous Routing (document-based dynamic config)
        ├─ Agent searches routing config documents
        ├─ Agents collaborate to select config (optional room)
        ├─ Apply document config as execution plan
        ├─ Route to specialist agents
        └─ Aggregate results
    ↓
Output Envelope (status, data, metadata, error)
    ↓
Enhanced Rooms (structured agent-to-agent collaboration)
    ├─ Gatekeeper selects speakers
    ├─ Agents receive structured outputs from previous speakers
    ├─ Agents produce structured outputs for next speakers
    └─ Room state accumulates outputs
    ↓
Next Step (reads from upstream ports)
```

### Three Execution Tiers

**TIER 1: Static Agent Execution**
- User explicitly selects agent for step
- Predefined tools, predefined system prompt (or mode-resolved)
- Fastest, most predictable
- Use case: Known requirements, specific expertise needed

**TIER 2: Label-Based Routing**
- Array items routed to specialist agents by category field
- Static routing rules configured in workflow (label → agent mapping)
- Dynamic array size (4-8 items) handled automatically
- Use case: Heterogeneous items with known categories (frontend/backend/database/testing)

**TIER 3: Cavernous Routing** (NEW - Document-Based Dynamic Configuration)
- Step marked with `execution_mode: "cavernous"`
- Agent analyzes task → searches routing config documents
- **Document title = routing config key** (e.g., "routing:research_and_documentation", "routing:full_stack_implementation")
- **Document content = execution plan** (tools, subtasks, agent assignments, prompts)
- Optional: Agents collaborate in room to select best routing config
- Apply document config → spawn subtasks → aggregate results
- Use case: Complex tasks requiring adaptive decomposition, multi-step workflows

### Document-Based Configuration System

**Core Concept:** Leverage existing `documents` system as configuration store.

**How it works:**
1. System admin creates routing config documents with semantic titles
2. Agent executing cavernous step uses document search tool to find configs
3. Agent retrieves top 5 matching documents (with descriptions)
4. Agent selects appropriate config (or collaborates with other agents to decide)
5. Document content applied as execution plan
6. Subtasks spawned according to plan

**Benefits:**
- Rich instructions in documents (large context) without bloating LLM context (small loop iterations)
- Versionable, searchable configuration
- Agents use existing document search tools
- Can update configs without code changes
- Supports agent collaboration for complex routing decisions

### Core Concepts

**1. Master System Configuration** - Admin-controlled truth
- **Capability taxonomy**: Official list of capabilities (file_ops, code_generation, web_search)
- **System agents and tools**: Immutable core agents/tools available to all users
- **Routing strategies**: Pre-defined routing config documents (admin creates, users select)
- **Execution constraints**: Safety limits (max subtasks, nesting depth, timeouts, cost caps, dangerous operations toggle)
- Stored in: `system_config` table + config documents with `system:` prefix
- Configurable: Supports "unsafe" operations for power users (admin enables per-tenant)

**2. Ports** - Explicit input/output definitions on steps
- Steps declare what they produce (output ports)
- Steps declare what they need (input ports)
- Edges connect output port → input port
- Automatic envelope unwrapping (wire connects `step-a.items`, system reads `envelope.data.items`)

**3. Envelopes** - Consistent wrapper for all outputs
```json
{
  "status": "success" | "error" | "partial",
  "data": <actual output>,
  "metadata": {execution_id, timing, cost, agent_id, routing_label, selected_routing_config},
  "error": <error details if failed>
}
```

**4. Tool Capabilities** - Semantic tool taxonomy
- Tools declare capabilities they provide (file_read, code_execution, web_fetch)
- Modes specify required capabilities
- System auto-selects tools matching capabilities
- Extends existing `tool_routers` + `tool_router_modes` system

**5. Label Routing** - Semantic agent assignment for arrays
- Array items declare category/type field
- Routing rules map category → specialist agent
- Dynamic size support (4 items or 8 items)
- Fallback agent for unmatched categories

**6. Document-Based Routing Configs** - Searchable execution plans
- Routing configs stored as documents with title convention: `routing:<name>`
- Document content: JSON execution plan (subtasks, tools, agents, prompts)
- Agents search documents to find appropriate routing
- Rich instructions without bloating LLM context

**7. Enhanced Rooms** - Structured agent collaboration
- Agents produce structured outputs (not just text)
- Next speakers receive previous agents' structured data
- Gatekeeper uses output schemas to select informed speakers
- Room state accumulates outputs for querying

---

## Master System Configuration

### Purpose

The master config is the **single source of truth** for system-wide capabilities, tools, agents, and constraints. It ensures:
- Consistency across all workflows
- Safety guardrails (configurable per-tenant)
- Centralized capability taxonomy
- Admin-controlled routing strategies

### Configuration Storage

```sql
CREATE TABLE system_config (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    config_type TEXT NOT NULL,  -- "capability", "constraint", "routing_strategy", "system_agent"
    config_key TEXT NOT NULL UNIQUE,  -- "max_subtasks", "unsafe_operations_enabled", "capability:code_execution"
    config_value JSONB NOT NULL,
    description TEXT,
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_system_config_type ON system_config(config_type);
```

### Configuration Categories

**1. Capability Taxonomy** (`config_type: "capability"`)
- Defines official capabilities (file_ops, code_generation, web_search, shell_execution)
- Each capability has:
  - `capability_key`: Unique identifier
  - `display_name`: Human-readable name
  - `category`: Group (filesystem, web, computation, version_control)
  - `safety_level`: "safe", "caution", "unsafe"
  - `description`: What it enables

Example:
```json
{
  "config_key": "capability:shell_execution",
  "config_value": {
    "display_name": "Shell Command Execution",
    "category": "system",
    "safety_level": "unsafe",
    "description": "Execute arbitrary shell commands",
    "requires_approval": true
  }
}
```

**2. Execution Constraints** (`config_type: "constraint"`)
- Safety limits applied to all executions
- Configurable per-tenant (admin override)

Default constraints:
```json
{
  "max_subtasks_per_cavernous_step": 10,
  "max_cavernous_nesting_depth": 3,
  "max_execution_time_minutes": 60,
  "max_cost_per_execution_usd": 10.0,
  "max_tokens_per_step": 100000,
  "unsafe_operations_enabled": false,  -- Admin can enable
  "dangerous_tools_require_approval": true
}
```

**3. System Agents** (`config_type: "system_agent"`)
- Core agents available to all users (immutable)
- Examples: "General Assistant", "Code Analyzer", "Security Reviewer"

**4. Routing Strategies** (`config_type: "routing_strategy"`)
- References to routing config documents
- Metadata about what each strategy is for

Example:
```json
{
  "config_key": "routing_strategy:full_stack_implementation",
  "config_value": {
    "document_id": "uuid-of-routing-doc",
    "description": "Multi-agent full-stack development with frontend/backend/database specialists",
    "capabilities_required": ["file_write", "code_generation", "git_ops"],
    "complexity_level": "high"
  }
}
```

### Routing Config Documents

**Document Naming Convention:** `routing:<strategy_name>`

**Example Document: "routing:research_and_documentation"**

Title: `routing:research_and_documentation`

Content:
```json
{
  "strategy_name": "research_and_documentation",
  "description": "Research a topic, synthesize findings, write documentation",
  "subtasks": [
    {
      "task_name": "research",
      "agent_role": "researcher",
      "agent_id": "system-researcher-agent",
      "tools": ["web_search", "web_fetch", "document_create"],
      "prompt_template": "Research {topic}. Find 5-10 authoritative sources.",
      "output_schema": {"type": "object", "properties": {"sources": {"type": "array"}, "key_findings": {"type": "array"}}}
    },
    {
      "task_name": "synthesize",
      "agent_role": "synthesizer",
      "agent_id": "system-synthesizer-agent",
      "depends_on": ["research"],
      "input_mapping": {"sources": "research.sources", "findings": "research.key_findings"},
      "tools": ["think"],
      "prompt_template": "Synthesize research findings into coherent outline.",
      "output_schema": {"type": "object", "properties": {"outline": {"type": "object"}}}
    },
    {
      "task_name": "write_documentation",
      "agent_role": "technical_writer",
      "agent_id": "system-writer-agent",
      "depends_on": ["synthesize"],
      "input_mapping": {"outline": "synthesize.outline"},
      "tools": ["document_create", "document_update"],
      "prompt_template": "Write comprehensive documentation based on outline.",
      "output_schema": {"type": "object", "properties": {"document_id": {"type": "string"}}}
    }
  ],
  "aggregation_mode": "final_output",  -- "final_output", "all_outputs", "merge"
  "max_parallel": 1,  -- Sequential execution
  "timeout_minutes": 30
}
```

### Agent Collaboration for Config Selection

**Scenario:** Cavernous step needs to select routing config

**Option 1: Single agent selects**
```
Agent: Search documents for "routing:*"
       → Returns: routing:research_and_documentation, routing:full_stack_implementation, routing:code_review, ...
       → Agent analyzes task → selects best match
       → Apply config
```

**Option 2: Multi-agent collaboration (room)**
```
Step marked: cavernous + collaborative_selection: true
    ↓
Create temporary routing selection room
    ↓
Members: Task Analyzer, Domain Expert, Routing Specialist
    ↓
Turn 1: Task Analyzer → analyzes input, produces structured breakdown
Turn 2: Domain Expert → identifies domain requirements
Turn 3: Routing Specialist → searches routing docs, proposes configs
    ↓
Room aggregates → selects routing config
    ↓
Apply selected config to cavernous execution
```

### Configuration Management API

```
Admin-only endpoints:

POST   /api/admin/system-config           -- Create config entry
GET    /api/admin/system-config           -- List all configs
GET    /api/admin/system-config/:key      -- Get specific config
PUT    /api/admin/system-config/:key      -- Update config
DELETE /api/admin/system-config/:key      -- Delete config

POST   /api/admin/routing-configs         -- Create routing config document
GET    /api/admin/routing-configs         -- List all routing configs
GET    /api/admin/routing-configs/:id     -- Get routing config
PUT    /api/admin/routing-configs/:id     -- Update routing config
DELETE /api/admin/routing-configs/:id     -- Delete routing config
```

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

## Complete Database Schema

### Overview

Four major schema extensions:
1. **Port-Based Workflows** - step_inputs, step_outputs, enhanced edges, routing rules
2. **Tool Capability Registry** - tool_capabilities, capability assignments, mode requirements
3. **Cavernous Routing** - workflow_steps extensions, routing analysis storage
4. **Enhanced Rooms** - room_execution_outputs, room member ports, structured state

### Migration 067: Port-Based Workflow System

```sql
-- ============================================================================
-- Port-Based Workflows - Replaces Variable System
-- ============================================================================

-- Output ports (what each step produces)
CREATE TABLE step_outputs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_step_id UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    port_name TEXT NOT NULL,
    port_type TEXT NOT NULL,  -- "string", "array", "object", "number", "boolean"
    json_path TEXT NOT NULL,  -- Path in envelope.data
    description TEXT,
    json_schema JSONB,  -- Optional validation schema
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(workflow_step_id, port_name)
);

CREATE INDEX idx_step_outputs_step ON step_outputs(workflow_step_id);

-- Input ports (what each step expects)
CREATE TABLE step_inputs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_step_id UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    port_name TEXT NOT NULL,
    port_type TEXT NOT NULL,
    required BOOLEAN NOT NULL DEFAULT false,
    default_value JSONB,
    description TEXT,
    json_schema JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(workflow_step_id, port_name)
);

CREATE INDEX idx_step_inputs_step ON step_inputs(workflow_step_id);

-- Routing rules for label-based agent assignment
CREATE TABLE step_routing_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_step_id UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    label_value TEXT NOT NULL,  -- "frontend", "backend", "database", "testing"
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    display_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(workflow_step_id, label_value)
);

CREATE INDEX idx_step_routing_rules_step ON step_routing_rules(workflow_step_id);
CREATE INDEX idx_step_routing_rules_agent ON step_routing_rules(agent_id);

-- Extend workflow_steps with ports and routing
ALTER TABLE workflow_steps
    ADD COLUMN position_x FLOAT,
    ADD COLUMN position_y FLOAT,
    ADD COLUMN width FLOAT DEFAULT 200,
    ADD COLUMN height FLOAT DEFAULT 100,
    ADD COLUMN routing_mode TEXT,  -- NULL, "label", "cavernous"
    ADD COLUMN routing_field TEXT,  -- For label routing
    ADD COLUMN cavernous_config_document_id UUID REFERENCES documents(id);  -- NEW: Document-based routing

CREATE INDEX idx_workflow_steps_routing ON workflow_steps(routing_mode)
    WHERE routing_mode IS NOT NULL;

-- Enhance workflow_step_edges with port connections
ALTER TABLE workflow_step_edges
    ADD COLUMN id UUID DEFAULT gen_random_uuid(),
    ADD COLUMN from_output_port TEXT,
    ADD COLUMN to_input_port TEXT,
    ADD COLUMN transform_jsonpath TEXT,
    ADD COLUMN condition_type TEXT,
    ADD COLUMN condition_value JSONB,
    ADD COLUMN edge_label TEXT;

ALTER TABLE workflow_step_edges DROP CONSTRAINT workflow_step_edges_pkey;
ALTER TABLE workflow_step_edges ADD CONSTRAINT workflow_step_edges_pkey PRIMARY KEY (id);
ALTER TABLE workflow_step_edges
    ADD CONSTRAINT workflow_step_edges_from_to_unique
    UNIQUE(from_step_id, to_step_id);

CREATE INDEX idx_workflow_step_edges_ports ON workflow_step_edges(from_output_port, to_input_port);

-- Drop execution_variables (migration from variable to port system)
DROP TABLE IF EXISTS execution_variables;

COMMENT ON TABLE workflow_steps IS
    'Workflow DAG nodes - output_variable_name deprecated, use step_outputs table';

COMMENT ON COLUMN agent_executions.structured_output IS
    'Standard envelope: {status, data, metadata, error}. For for_each, data is array of iteration envelopes.';
```

### Migration 068: Tool Capability Registry

```sql
-- ============================================================================
-- Tool Capability System - Extends Existing Router/Mode System
-- ============================================================================

-- Capability taxonomy (predefined)
CREATE TABLE tool_capabilities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    capability_key TEXT NOT NULL UNIQUE,  -- "file_read", "code_execution"
    display_name TEXT NOT NULL,
    category TEXT NOT NULL,  -- "filesystem", "web", "computation", etc.
    safety_level TEXT NOT NULL DEFAULT 'safe',  -- "safe", "caution", "unsafe"
    description TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (capability_key ~ '^[a-z][a-z0-9_]*$')
);

CREATE INDEX idx_tool_capabilities_category ON tool_capabilities(category);

-- Which capabilities each tool provides
CREATE TABLE tool_capability_assignments (
    tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
    capability_id UUID NOT NULL REFERENCES tool_capabilities(id) ON DELETE CASCADE,
    PRIMARY KEY (tool_id, capability_id)
);

CREATE INDEX idx_tool_capability_assignments_capability ON tool_capability_assignments(capability_id);

-- Mode capability requirements (extends tool_router_modes)
CREATE TABLE mode_required_capabilities (
    mode_id UUID NOT NULL REFERENCES tool_router_modes(id) ON DELETE CASCADE,
    capability_id UUID NOT NULL REFERENCES tool_capabilities(id) ON DELETE CASCADE,
    is_required BOOLEAN NOT NULL DEFAULT true,
    PRIMARY KEY (mode_id, capability_id)
);

CREATE INDEX idx_mode_required_capabilities_mode ON mode_required_capabilities(mode_id);

-- Seed common capabilities
INSERT INTO tool_capabilities (capability_key, display_name, category, safety_level, description) VALUES
    ('file_read', 'File Reading', 'filesystem', 'safe', 'Read file contents from disk'),
    ('file_write', 'File Writing', 'filesystem', 'caution', 'Create or modify files'),
    ('file_search', 'File Search', 'filesystem', 'safe', 'Search for files by pattern'),
    ('content_search', 'Content Search', 'filesystem', 'safe', 'Search file contents'),
    ('git_read', 'Git Read Operations', 'version_control', 'safe', 'View git history, diffs, status'),
    ('git_write', 'Git Write Operations', 'version_control', 'caution', 'Commit, branch, merge'),
    ('shell_execution', 'Shell Execution', 'system', 'unsafe', 'Execute shell commands'),
    ('web_fetch', 'Web Fetching', 'web', 'safe', 'Fetch content from URLs'),
    ('web_search', 'Web Search', 'web', 'safe', 'Search the internet'),
    ('code_analysis', 'Code Analysis', 'development', 'safe', 'Analyze code structure'),
    ('test_execution', 'Test Execution', 'development', 'caution', 'Run test suites'),
    ('database_query', 'Database Query', 'data', 'caution', 'Query databases'),
    ('api_call', 'API Calls', 'integration', 'caution', 'Make HTTP API requests'),
    ('document_create', 'Document Creation', 'knowledge', 'safe', 'Create documents'),
    ('document_search', 'Document Search', 'knowledge', 'safe', 'Search documents')
ON CONFLICT (capability_key) DO NOTHING;
```

### Migration 069: Cavernous Routing

```sql
-- ============================================================================
-- Cavernous Routing - Document-Based Dynamic Execution
-- ============================================================================

-- Extend agent_executions for routing analysis storage
ALTER TABLE agent_executions
    ADD COLUMN routing_analysis JSONB,  -- Document search + selection reasoning
    ADD COLUMN selected_routing_document_id UUID REFERENCES documents(id);  -- Which config was used

CREATE INDEX idx_agent_executions_routing_doc ON agent_executions(selected_routing_document_id)
    WHERE selected_routing_document_id IS NOT NULL;

CREATE INDEX idx_agent_executions_routing_analysis ON agent_executions
    USING gin(routing_analysis) WHERE routing_analysis IS NOT NULL;

COMMENT ON COLUMN agent_executions.routing_analysis IS
    'For cavernous routing: Document search results + selection reasoning.
     Format: {search_query, documents_found: [{id, title, score}], selected_document_id, reasoning}';

COMMENT ON COLUMN workflow_steps.routing_mode IS
    'Execution routing strategy:
     - NULL: Use step agent_id directly
     - "label": Route array items by label_value field to specialist agents
     - "cavernous": Document-based dynamic routing with agent collaboration';

-- Execution mode values
COMMENT ON COLUMN workflow_steps.execution_mode IS
    'Execution strategy:
     - "single": Execute once with step agent (TIER 1)
     - "for_each": Iterate over array (sequential/parallel, with optional label routing) (TIER 2)
     - "cavernous": Document-based routing with dynamic task decomposition (TIER 3)
     - "room": Multi-agent room discussion';
```

### Migration 070: Enhanced Rooms

```sql
-- ============================================================================
-- Enhanced Rooms - Structured Agent Collaboration
-- ============================================================================

-- Room execution outputs (structured data passed between speakers)
CREATE TABLE room_execution_outputs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_session_id UUID NOT NULL REFERENCES room_sessions(id) ON DELETE CASCADE,
    agent_execution_id UUID NOT NULL REFERENCES agent_executions(id) ON DELETE CASCADE,
    agent_id UUID NOT NULL REFERENCES agents(id),
    speaker_order INTEGER NOT NULL,
    turn_number INTEGER NOT NULL,
    output_name TEXT NOT NULL,  -- "analysis", "implementation_plan", "code_review"
    structured_output JSONB NOT NULL,
    raw_output TEXT NOT NULL,
    schema_id UUID REFERENCES output_schemas(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(room_session_id, turn_number, output_name)
);

CREATE INDEX idx_room_outputs_session ON room_execution_outputs(room_session_id, turn_number);
CREATE INDEX idx_room_outputs_agent ON room_execution_outputs(agent_id);

-- Extend room_members with port configuration
ALTER TABLE room_members
    ADD COLUMN input_schema_id UUID REFERENCES output_schemas(id),
    ADD COLUMN output_schema_id UUID REFERENCES output_schemas(id),
    ADD COLUMN output_name TEXT;  -- What this agent's output should be called

CREATE INDEX idx_room_members_input_schema ON room_members(input_schema_id)
    WHERE input_schema_id IS NOT NULL;
CREATE INDEX idx_room_members_output_schema ON room_members(output_schema_id)
    WHERE output_schema_id IS NOT NULL;

-- Extend room_sessions with structured state
ALTER TABLE room_sessions
    ADD COLUMN structured_outputs JSONB,  -- Aggregated outputs: {output_name: {data}}
    ADD COLUMN final_decision JSONB;  -- Final synthesized room output

CREATE INDEX idx_room_sessions_outputs ON room_sessions
    USING gin(structured_outputs) WHERE structured_outputs IS NOT NULL;

-- Extend rooms with output configuration
ALTER TABLE rooms
    ADD COLUMN default_output_schema_id UUID REFERENCES output_schemas(id),
    ADD COLUMN aggregation_mode TEXT DEFAULT 'final_speaker';  -- "final_speaker", "consensus", "all_outputs"

CREATE INDEX idx_rooms_output_schema ON rooms(default_output_schema_id)
    WHERE default_output_schema_id IS NOT NULL;

COMMENT ON TABLE room_execution_outputs IS
    'Structured outputs from room members for agent-to-agent data passing';

COMMENT ON COLUMN rooms.aggregation_mode IS
    'How to aggregate room outputs:
     - "final_speaker": Use last speaker output
     - "consensus": Synthesize consensus from all speakers
     - "all_outputs": Return array of all speaker outputs';
```

---

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

## Cavernous Routing: Document-Based Dynamic Execution

### Architecture

**Core Concept:** Instead of hardcoding task decomposition logic, store routing configurations as searchable documents. Agents search documents to find appropriate execution plans, enabling rich instructions without bloating LLM context.

**Execution Flow:**

```
Step marked execution_mode: "cavernous"
    ↓
Phase 1: Document Search
    ├─ Agent analyzes incoming task
    ├─ Builds search query from task description
    ├─ Searches documents with title prefix "routing:"
    ├─ Retrieves top 5 matching routing config documents
    └─ Each document includes: title, description, config JSON
    ↓
Phase 2: Config Selection (two modes)

    Mode A: Single Agent Selection
        ├─ Agent reviews 5 routing config options
        ├─ Compares task requirements vs. config capabilities
        ├─ Selects best matching config
        └─ Returns: selected_document_id + reasoning

    Mode B: Collaborative Selection (if step.collaborative_routing: true)
        ├─ Create temporary routing selection room
        ├─ Members: Task Analyzer, Domain Expert, Routing Specialist
        ├─ Turn 1: Task Analyzer → structured task breakdown
        ├─ Turn 2: Domain Expert → domain requirements
        ├─ Turn 3: Routing Specialist → searches docs, proposes config
        ├─ Room aggregates → consensus on config
        └─ Returns: selected_document_id + room consensus
    ↓
Phase 3: Config Application
    ├─ Fetch selected routing config document
    ├─ Parse JSON config (subtasks, agents, tools, prompts)
    ├─ Validate against master system config constraints
    ├─ Build execution plan: task DAG from subtasks
    └─ Store routing_analysis in agent_executions
    ↓
Phase 4: Subtask Execution
    ├─ For each subtask in config:
    │   ├─ Resolve agent (from config.subtasks[i].agent_id)
    │   ├─ Build input from dependencies (input_mapping)
    │   ├─ Create child agent_execution (parent_id = cavernous execution)
    │   ├─ Execute via ExecutionEngine + DagStepStrategy
    │   └─ Collect output
    ├─ Handle dependencies (subtask B waits for subtask A)
    └─ Spawn in parallel where dependencies allow
    ↓
Phase 5: Aggregation
    ├─ Collect all subtask outputs
    ├─ Aggregate according to config.aggregation_mode
    ├─ Build final envelope (status, data, metadata, errors)
    └─ Store in agent_executions.structured_output
```

### Routing Config Document Format

**Document Title:** `routing:<strategy_name>`

**Document Content (JSON):**
```json
{
  "strategy_name": "full_stack_implementation",
  "version": "1.0",
  "description": "Implement full-stack feature with frontend, backend, database specialists",
  "capabilities_required": ["file_write", "code_generation", "git_ops"],
  "complexity_level": "high",

  "subtasks": [
    {
      "id": "database_schema",
      "task_name": "Design Database Schema",
      "agent_id": "system-database-architect",
      "tools": ["file_write", "database_query"],
      "prompt_template": "Design database schema for {feature_description}. Output SQL migration.",
      "depends_on": [],
      "input_mapping": {},
      "output_schema": {
        "type": "object",
        "properties": {
          "migration_sql": {"type": "string"},
          "tables": {"type": "array"}
        }
      }
    },
    {
      "id": "backend_api",
      "task_name": "Implement Backend API",
      "agent_id": "system-backend-specialist",
      "tools": ["file_write", "file_read", "test_execution"],
      "prompt_template": "Implement backend API using database schema: {database_schema.migration_sql}",
      "depends_on": ["database_schema"],
      "input_mapping": {
        "database_schema": "database_schema"  -- Maps to previous subtask output
      },
      "output_schema": {
        "type": "object",
        "properties": {
          "endpoints": {"type": "array"},
          "files_created": {"type": "array"}
        }
      }
    },
    {
      "id": "frontend_ui",
      "task_name": "Build Frontend UI",
      "agent_id": "system-frontend-specialist",
      "tools": ["file_write", "file_read"],
      "prompt_template": "Build React UI consuming API endpoints: {backend_api.endpoints}",
      "depends_on": ["backend_api"],
      "input_mapping": {
        "api_endpoints": "backend_api.endpoints"
      },
      "output_schema": {
        "type": "object",
        "properties": {
          "components": {"type": "array"},
          "files_created": {"type": "array"}
        }
      }
    }
  ],

  "aggregation_mode": "all_outputs",  -- "final_output", "all_outputs", "merge"
  "max_parallel": 2,  -- Max concurrent subtasks
  "timeout_minutes": 60,
  "cost_limit_usd": 5.0
}
```

### CavernousStepStrategy Implementation

**File:** `/src/server/hub/strategies/cavernous/mod.rs`

```rust
pub struct CavernousStepConfig {
    pub step: WorkflowStepRow,
    pub user_prompt: String,
    pub execution_context: Option<ExecutionContext>,
    pub collaborative_routing: bool,  // Use room for config selection
}

pub struct CavernousStepStrategy {
    config: CavernousStepConfig,
    state: AppState,
    phase: Arc<RwLock<CavernousPhase>>,
}

enum CavernousPhase {
    SearchingConfigs,
    SelectingConfig { options: Vec<DocumentRow> },
    ApplyingConfig { selected_doc: DocumentRow },
    ExecutingSubtasks { plan: RoutingConfigPlan, results: Vec<SubtaskResult> },
    Complete,
}

struct RoutingConfigPlan {
    strategy_name: String,
    subtasks: Vec<Subtask>,
    dependencies: HashMap<String, Vec<String>>,  // subtask_id → [depends_on_ids]
    aggregation_mode: String,
}

impl ExecutionStrategy for CavernousStepStrategy {
    async fn build_messages(&self, input: &str) -> Result<Vec<LlmMessage>> {
        let phase = self.phase.read().await;

        match *phase {
            CavernousPhase::SearchingConfigs => {
                // Build search query generation prompt
                Ok(vec![
                    LlmMessage::system("You are a routing analyst. Analyze the task and generate a search query for routing config documents."),
                    LlmMessage::user(&format!("Task: {}\n\nGenerate a search query to find appropriate routing configuration.", input))
                ])
            }
            CavernousPhase::SelectingConfig { ref options } => {
                // Build config selection prompt
                let options_text = options.iter()
                    .map(|doc| format!("- {}: {}", doc.title, doc.description.as_ref().unwrap_or(&"".to_string())))
                    .collect::<Vec<_>>()
                    .join("\n");

                Ok(vec![
                    LlmMessage::system("You are a routing selector. Choose the best routing configuration for the task."),
                    LlmMessage::user(&format!("Task: {}\n\nAvailable routing configs:\n{}\n\nSelect the best config and explain why.", input, options_text))
                ])
            }
            _ => Ok(vec![])
        }
    }

    async fn on_complete(&self, response: &str, usage: &TokenUsage) -> Result<()> {
        let mut phase = self.phase.write().await;

        match *phase {
            CavernousPhase::SearchingConfigs => {
                // Parse search query from response
                let search_query = extract_search_query(response)?;

                // Search documents with title prefix "routing:"
                let documents = self.state.repo()
                    .search_documents_by_title_prefix("routing:", Some(&search_query))
                    .await?
                    .into_iter()
                    .take(5)
                    .collect();

                // Transition to selection phase
                *phase = CavernousPhase::SelectingConfig { options: documents };
            }
            CavernousPhase::SelectingConfig { ref options } => {
                // Parse selected document ID from response
                let selection = parse_config_selection(response, options)?;

                // Fetch full document
                let doc = self.state.repo().get_document(selection.document_id).await?;

                // Store routing analysis
                let analysis = json!({
                    "search_results": options,
                    "selected_document_id": doc.id,
                    "selection_reasoning": selection.reasoning
                });

                self.state.repo().update_agent_execution_routing_analysis(
                    self.config.execution_id,
                    &analysis,
                    Some(doc.id)
                ).await?;

                // Transition to application phase
                *phase = CavernousPhase::ApplyingConfig { selected_doc: doc };

                // Parse and execute routing plan
                self.execute_routing_plan(&doc).await?;
            }
            _ => {}
        }

        Ok(())
    }
}

impl CavernousStepStrategy {
    async fn execute_routing_plan(&self, doc: &DocumentRow) -> Result<()> {
        // Parse routing config from document content
        let config: RoutingConfigPlan = serde_json::from_str(&doc.content)?;

        // Validate against system constraints
        validate_routing_config(&config, &self.state).await?;

        // Build dependency graph
        let task_graph = build_task_dag(&config.subtasks)?;

        // Execute subtasks in topological order (parallel where possible)
        let results = execute_subtasks_dag(task_graph, &self.state, &self.config).await?;

        // Aggregate results
        let final_output = aggregate_subtask_outputs(results, &config.aggregation_mode)?;

        // Store final output
        self.state.repo().update_agent_execution_output(
            self.config.execution_id,
            &final_output
        ).await?;

        Ok(())
    }
}
```

### Collaborative Routing Selection (Room-Based)

When `step.collaborative_routing == true`:

```rust
async fn collaborative_config_selection(
    task_description: &str,
    routing_options: Vec<DocumentRow>,
    state: &AppState,
) -> Result<Uuid> {
    // Create temporary room for routing selection
    let room = create_routing_selection_room(state).await?;

    // Add specialized agents:
    // - Task Analyzer: breaks down task requirements
    // - Domain Expert: identifies domain-specific needs
    // - Routing Specialist: compares options, proposes config

    let session = start_room_session(&room, state).await?;

    // Turn 1: Task Analyzer
    let analysis = execute_room_speaker(
        room.id,
        "task-analyzer-agent",
        &format!("Analyze this task: {}", task_description),
        state
    ).await?;

    // Turn 2: Domain Expert (receives task analysis)
    let domain_reqs = execute_room_speaker(
        room.id,
        "domain-expert-agent",
        &format!("Based on analysis: {}, what domain requirements are needed?", analysis),
        state
    ).await?;

    // Turn 3: Routing Specialist (receives both)
    let routing_selection = execute_room_speaker(
        room.id,
        "routing-specialist-agent",
        &format!("Task requirements: {}. Available configs: {:?}. Select best config.", domain_reqs, routing_options),
        state
    ).await?;

    // Parse selected config from room consensus
    let selected_doc_id = parse_selected_config_from_room(&routing_selection)?;

    Ok(selected_doc_id)
}
```

---

## Enhanced Rooms: Implementation Details

### Room State Service

**File:** `/src/server/executors/room/state.rs` (NEW)

```rust
pub struct RoomState {
    session_id: Uuid,
    outputs: HashMap<String, RoomExecutionOutput>,  // Latest by output_name
    all_outputs: Vec<RoomExecutionOutput>,
}

impl RoomState {
    pub fn get_output(&self, name: &str) -> Option<&RoomExecutionOutput> {
        self.outputs.get(name)
    }

    pub fn get_outputs_by_schema(&self, schema_id: Uuid) -> Vec<&RoomExecutionOutput> {
        self.all_outputs.iter()
            .filter(|o| o.schema_id == Some(schema_id))
            .collect()
    }

    pub fn add_output(&mut self, output: RoomExecutionOutput) {
        self.outputs.insert(output.output_name.clone(), output.clone());
        self.all_outputs.push(output);
    }
}
```

### Prompt Building with Structured Inputs

**Modify** `/src/server/executors/room/mod.rs::build_speaker_prompt`:

```rust
async fn build_speaker_prompt_with_structured_inputs(
    user_message: &str,
    transcript_block: &str,
    member: &RoomMemberRow,
    room_state: &RoomState,
) -> Result<String> {
    let mut prompt = String::new();

    // 1. Transcript (conversation context)
    if !transcript_block.is_empty() {
        prompt.push_str(transcript_block);
        prompt.push_str("\n---\n\n");
    }

    // 2. Structured inputs (if member has input_schema_id)
    if let Some(input_schema_id) = member.input_schema_id {
        let matching_outputs = room_state.get_outputs_by_schema(input_schema_id);

        if !matching_outputs.is_empty() {
            prompt.push_str("## Structured Inputs from Previous Speakers\n\n");
            for output in matching_outputs {
                prompt.push_str(&format!(
                    "**{}**:\n```json\n{}\n```\n\n",
                    output.output_name,
                    serde_json::to_string_pretty(&output.structured_output)?
                ));
            }
        }
    }

    // 3. User message
    prompt.push_str(&format!("**User message**: {}\n", user_message));

    Ok(prompt)
}
```

### System Prompt with Output Schema Enforcement

```rust
async fn build_speaker_system_prompt(
    agent: &AgentRow,
    member: &RoomMemberRow,
) -> Result<String> {
    let mut sys_prompt = agent.system_prompt.clone();

    // If member has output_schema_id, enforce structure
    if let Some(output_schema_id) = member.output_schema_id {
        let schema = state.repo().get_output_schema(output_schema_id).await?;
        sys_prompt.push_str(&format!(
            "\n\n## Output Format Requirement\n\n\
            You MUST respond with valid JSON matching this schema:\n\
            ```json\n{}\n```\n\n\
            Wrap your JSON response in a ```json code fence.",
            serde_json::to_string_pretty(&schema.schema)?
        ));
    }

    Ok(sys_prompt)
}
```

### Gatekeeper Enhancement

**Extend gatekeeper input** with available structured outputs:

```rust
struct GatekeeperInput {
    user_message: String,
    mentions: Vec<String>,
    transcript_tail: String,
    roster: Vec<RosterEntry>,
    available_outputs: Vec<AvailableOutput>,  // NEW
    max_speakers: i32,
}

struct AvailableOutput {
    output_name: String,
    agent_name: String,
    schema_summary: String,
}

// In gatekeeper system prompt:
const GATEKEEPER_PROMPT_EXTENSION: &str = r#"
## Available Structured Outputs

Previous speakers have produced:
{available_outputs_list}

When selecting speakers, consider:
1. Which agents can best consume the available structured outputs
2. Match agent input_schema_id to available output schemas
3. Create data flow: analysis → planning → implementation
4. Reference specific outputs in followup_context
"#;
```

---

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

### Implementation Roadmap

**Total Duration:** 25-35 days (5-7 weeks)

**Approach:** Build all systems in parallel where possible. Each phase produces working, testable functionality.

---

### Phase 1: Foundation (3-4 days)

**Database Schema**

1. **Migration 067: Port-Based Workflows**
   - Create `step_outputs`, `step_inputs`, `step_routing_rules`
   - Extend `workflow_steps` (positioning, routing_mode, cavernous_config_document_id)
   - Enhance `workflow_step_edges` (ports, transforms)
   - Drop `execution_variables`

2. **Migration 068: Tool Capabilities**
   - Create `tool_capabilities` table
   - Create `tool_capability_assignments`, `mode_required_capabilities`
   - Seed common capabilities (15+ entries)

3. **Migration 069: Cavernous Routing**
   - Extend `agent_executions` (routing_analysis, selected_routing_document_id)
   - Add execution_mode comments

4. **Migration 070: Enhanced Rooms**
   - Create `room_execution_outputs`
   - Extend `room_members` (input/output schema ports)
   - Extend `room_sessions`, `rooms` (structured outputs, aggregation)

5. **Migration 071: Master System Config**
   - Create `system_config` table
   - Seed default configurations (constraints, capabilities)

**Verification:**
```bash
docker exec -it gh-agents-postgres-1 psql -U nexor -d nexor
\d step_outputs
\d tool_capabilities
\d room_execution_outputs
\d system_config
SELECT COUNT(*) FROM tool_capabilities;  -- Should have 15+ rows
```

**Critical Files:**
- `/migrations/067_port_based_workflows.sql`
- `/migrations/068_tool_capabilities.sql`
- `/migrations/069_cavernous_routing.sql`
- `/migrations/070_enhanced_rooms.sql`
- `/migrations/071_master_system_config.sql`

---

### Phase 2: Type Definitions (2 days)

**Rust Types**

1. **Port Types** (`src/types/workflow.rs`):
   ```rust
   pub struct StepInputRow { id, workflow_step_id, port_name, port_type, required, default_value, json_schema }
   pub struct StepOutputRow { id, workflow_step_id, port_name, port_type, json_path, json_schema }
   pub struct StepRoutingRuleRow { id, workflow_step_id, label_value, agent_id, display_order }
   pub struct EdgeWithPorts { id, from_step_id, to_step_id, from_output_port, to_input_port, transform_jsonpath }
   ```

2. **Envelope Types** (`src/types/execution.rs`):
   ```rust
   pub struct StepExecutionEnvelope {
       pub status: ExecutionStatus,  // Success, Error, Partial
       pub data: Option<JsonValue>,
       pub metadata: ExecutionMetadata,
       pub error: Option<ExecutionError>,
   }

   pub struct ExecutionMetadata {
       pub execution_id: Uuid,
       pub execution_time_ms: u64,
       pub tokens_in, tokens_out, cost_usd,
       pub agent_id: Option<Uuid>,
       pub routing_label: Option<String>,
       pub selected_routing_document_id: Option<Uuid>,
   }

   pub struct ForEachAggregateEnvelope {
       pub status: ExecutionStatus,
       pub data: Vec<StepExecutionEnvelope>,
       pub metadata: ForEachMetadata,
       pub errors: Vec<IterationError>,
   }
   ```

3. **Capability Types** (`src/types/tool.rs`):
   ```rust
   pub struct ToolCapabilityRow { id, capability_key, display_name, category, safety_level, description }
   pub struct ToolCapabilityAssignment { tool_id, capability_id }
   pub struct ModeRequiredCapability { mode_id, capability_id, is_required }
   ```

4. **Cavernous Routing Types** (`src/types/routing.rs` - NEW):
   ```rust
   pub struct RoutingConfigDocument {
       pub strategy_name: String,
       pub description: String,
       pub capabilities_required: Vec<String>,
       pub subtasks: Vec<Subtask>,
       pub aggregation_mode: String,
       pub max_parallel: usize,
   }

   pub struct Subtask {
       pub id: String,
       pub task_name: String,
       pub agent_id: Uuid,
       pub tools: Vec<String>,
       pub prompt_template: String,
       pub depends_on: Vec<String>,
       pub input_mapping: HashMap<String, String>,
       pub output_schema: Option<JsonValue>,
   }

   pub struct RoutingAnalysis {
       pub search_query: String,
       pub documents_found: Vec<DocumentSummary>,
       pub selected_document_id: Uuid,
       pub selection_reasoning: String,
   }
   ```

5. **Room Types** (`src/types/room.rs`):
   ```rust
   pub struct RoomExecutionOutput {
       pub id: Uuid,
       pub room_session_id: Uuid,
       pub agent_execution_id: Uuid,
       pub output_name: String,
       pub structured_output: JsonValue,
       pub schema_id: Option<Uuid>,
   }

   pub struct RoomState {
       outputs: HashMap<String, RoomExecutionOutput>,
       all_outputs: Vec<RoomExecutionOutput>,
   }
   ```

**Verification:**
- `cargo check` passes
- All new types compile
- No unused imports

**Critical Files:**
- `/src/types/workflow.rs`
- `/src/types/execution.rs`
- `/src/types/routing.rs` (NEW)
- `/src/types/tool.rs`
- `/src/types/room.rs`

---

### Phase 3: Database Queries (3-4 days)

**Repository Extensions**

1. **Workflow Ports** (`src/db/queries/workflows.rs`):
   ```rust
   async fn query_step_inputs(workflow_id: Uuid) -> Result<Vec<StepInputRow>>
   async fn query_step_outputs(workflow_id: Uuid) -> Result<Vec<StepOutputRow>>
   async fn query_step_routing_rules(step_id: Uuid) -> Result<Vec<StepRoutingRuleRow>>
   async fn create_step_input(...) -> Result<StepInputRow>
   async fn create_step_output(...) -> Result<StepOutputRow>
   async fn create_routing_rule(...) -> Result<StepRoutingRuleRow>
   ```

2. **Tool Capabilities** (`src/db/queries/tools.rs`):
   ```rust
   async fn query_tool_capabilities() -> Result<Vec<ToolCapabilityRow>>
   async fn query_capabilities_by_tool(tool_id: Uuid) -> Result<Vec<ToolCapabilityRow>>
   async fn query_tools_by_capability(capability_key: &str) -> Result<Vec<ToolRow>>
   async fn query_mode_capabilities(mode_id: Uuid) -> Result<Vec<ToolCapabilityRow>>
   async fn assign_capability_to_tool(tool_id, capability_id) -> Result<()>
   ```

3. **Routing Configs** (`src/db/queries/documents.rs`):
   ```rust
   async fn search_routing_configs(query: &str) -> Result<Vec<DocumentRow>>
   async fn get_routing_config_by_name(name: &str) -> Result<Option<DocumentRow>>
   async fn create_routing_config(name, content, user_id) -> Result<DocumentRow>
   ```

4. **Room Outputs** (`src/db/queries/rooms.rs`):
   ```rust
   async fn save_room_execution_output(output: &RoomExecutionOutput) -> Result<()>
   async fn query_room_outputs(session_id, turn_number) -> Result<Vec<RoomExecutionOutput>>
   async fn query_room_outputs_by_schema(session_id, schema_id) -> Result<Vec<RoomExecutionOutput>>
   ```

5. **System Config** (`src/db/queries/system_config.rs` - NEW):
   ```rust
   async fn get_system_config(config_key: &str) -> Result<Option<SystemConfigRow>>
   async fn set_system_config(config_type, key, value) -> Result<()>
   async fn get_execution_constraints() -> Result<ExecutionConstraints>
   ```

**Tests:**
- Unit tests for each query
- Integration tests with test database
- Verify foreign key constraints

**Critical Files:**
- `/src/db/queries/workflows.rs`
- `/src/db/queries/tools.rs`
- `/src/db/queries/documents.rs`
- `/src/db/queries/rooms.rs`
- `/src/db/queries/system_config.rs` (NEW)

---

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

---

### Phase 4: Mode Resolver Enhancement (2-3 days)

**Capability-Based Tool Selection**

**Extend** `/src/server/hub/mode_resolver/mod.rs`:

```rust
pub struct ResolvedModeConfig {
    // Existing fields...
    pub capabilities: Vec<String>,  // NEW
}

impl ModeResolver {
    async fn resolve_capabilities(&self, mode_id: Uuid) -> Result<Vec<String>> {
        let caps = self.tool_router_repo
            .get_mode_capabilities(mode_id)
            .await?;
        Ok(caps.into_iter().map(|c| c.capability_key).collect())
    }

    async fn resolve_tools_from_capabilities(&self, caps: &[String]) -> Result<Vec<Tool>> {
        let mut tools = Vec::new();
        for cap_key in caps {
            let tool_rows = self.tool_router_repo
                .get_tools_by_capability(cap_key)
                .await?;
            // Get tool definitions from registry
            for row in tool_rows {
                if let Some(tool_def) = registry::get_tool_definition(&row.name) {
                    tools.push(tool_def);
                }
            }
        }
        Ok(tools)
    }
}
```

**Tests:**
- Mode with capabilities → resolves correct tools
- Multiple capabilities → union of tools
- Capability not found → graceful fallback

**Critical Files:**
- `/src/server/hub/mode_resolver/mod.rs`

---

### Phase 5: Port-Based DAG Executor (4-5 days)

**Port Resolution + Envelope Wrapping**

**Modify** `/src/server/hub/dag/mod.rs`:

1. **Port Input Resolution:**
   ```rust
   async fn build_step_inputs(
       step_id: Uuid,
       workflow_execution_id: Uuid,
       edges: &[EdgeWithPorts],
       pool: &PgPool,
   ) -> Result<HashMap<String, JsonValue>> {
       // For each incoming edge:
       //   1. Get source execution's envelope
       //   2. Extract data from envelope.data.<from_output_port>
       //   3. Apply optional transform_jsonpath
       //   4. Map to to_input_port
       // Fill defaults for missing optional inputs
   }
   ```

2. **Envelope Wrapping:**
   ```rust
   fn wrap_in_envelope(
       execution_id: Uuid,
       agent_id: Uuid,
       output: Option<JsonValue>,
       error: Option<anyhow::Error>,
       timing: ExecutionTiming,
       tokens: TokenUsage,
   ) -> StepExecutionEnvelope {
       StepExecutionEnvelope {
           status: if error.is_some() { ExecutionStatus::Error } else { ExecutionStatus::Success },
           data: output,
           metadata: ExecutionMetadata { execution_id, execution_time_ms, tokens_in, tokens_out, cost_usd, agent_id, ... },
           error: error.map(|e| ExecutionError { message, error_type, retryable }),
       }
   }
   ```

3. **For-Each Label Routing:**
   ```rust
   if step.routing_mode == Some("label") {
       let routing_field = step.routing_field.as_ref()?;
       let routing_rules = query_step_routing_rules(step.id).await?;
       let agent_map: HashMap<String, Uuid> = routing_rules.iter()
           .map(|r| (r.label_value.clone(), r.agent_id))
           .collect();

       for (idx, elem) in array.iter().enumerate() {
           let label = elem.get(routing_field).and_then(|v| v.as_str())?;
           let agent_id = agent_map.get(label).copied().unwrap_or(step.agent_id);
           // Execute with routed agent...
       }
   }
   ```

**Remove Variable System:**
- Delete all `execution_variables` code
- Remove `{variable}` interpolation
- Update prompt building to use port inputs

**Tests:**
- Port resolution from upstream steps
- Envelope structure validation
- For-each label routing to different agents
- Error preservation in envelopes

**Critical Files:**
- `/src/server/hub/dag/mod.rs`
- `/src/server/executors/dag/mod.rs` (if still used)

---

### Phase 6: Cavernous Routing (5-6 days)

**Document-Based Dynamic Execution**

**Create** `/src/server/hub/strategies/cavernous/mod.rs`:

1. **Phase 1: Document Search**
   - Build search query from task
   - Search documents with prefix `routing:`
   - Return top 5 matches

2. **Phase 2: Config Selection**
   - Single agent mode: agent selects from options
   - Collaborative mode: create room, agents discuss, select config

3. **Phase 3: Config Application**
   - Parse routing config JSON from document
   - Validate against system constraints
   - Build subtask DAG

4. **Phase 4: Subtask Execution**
   - Spawn child executions for each subtask
   - Handle dependencies (topological order)
   - Parallel execution where possible

5. **Phase 5: Aggregation**
   - Collect outputs
   - Aggregate per config.aggregation_mode
   - Build final envelope

**Tests:**
- Document search returns routing configs
- Config selection (single agent)
- Config selection (collaborative room)
- Subtask execution with dependencies
- Aggregation modes (all_outputs, final_output, merge)
- Constraint validation (max_subtasks, cost_limit)

**Critical Files:**
- `/src/server/hub/strategies/cavernous/mod.rs` (NEW)
- `/src/server/hub/strategies/cavernous/config.rs` (NEW)
- `/src/server/hub/strategies/cavernous/executor.rs` (NEW)

---

### Phase 7: Enhanced Rooms (4-5 days)

**Structured Agent Collaboration**

**Modify** `/src/server/executors/room/mod.rs`:

1. **Room State Service:**
   ```rust
   // New module: src/server/executors/room/state.rs
   impl RoomState {
       fn add_output(&mut self, output: RoomExecutionOutput)
       fn get_output(&self, name: &str) -> Option<&RoomExecutionOutput>
       fn get_outputs_by_schema(&self, schema_id: Uuid) -> Vec<&RoomExecutionOutput>
   }
   ```

2. **Prompt Building with Structured Inputs:**
   - Load prior structured outputs
   - Format as JSON in prompt
   - Include schema enforcement in system prompt

3. **Output Parsing + Storage:**
   - Parse structured output from speaker response
   - Save to `room_execution_outputs`
   - Update room state

4. **Gatekeeper Enhancement:**
   - Extend input with available_outputs
   - Update prompt to suggest schema-aware speaker selection
   - Include output references in followup_context

**Tests:**
- Structured output passing between speakers
- Input schema matching
- Room state accumulation
- Gatekeeper schema-aware selection

**Critical Files:**
- `/src/server/executors/room/mod.rs`
- `/src/server/executors/room/state.rs` (NEW)
- `/src/agents/gatekeeper.rs`

---

### Phase 8: API Endpoints (3-4 days)

**REST APIs for All Systems**

1. **Port Management:**
   ```
   POST   /api/steps/{id}/inputs
   POST   /api/steps/{id}/outputs
   GET    /api/workflows/{id}/ports
   ```

2. **Routing Rules:**
   ```
   POST   /api/steps/{id}/routing-rules
   GET    /api/steps/{id}/routing-rules
   PUT    /api/routing-rules/{id}
   DELETE /api/routing-rules/{id}
   ```

3. **Routing Configs (Admin):**
   ```
   POST   /api/admin/routing-configs
   GET    /api/admin/routing-configs
   GET    /api/admin/routing-configs/:id
   PUT    /api/admin/routing-configs/:id
   ```

4. **System Config (Admin):**
   ```
   GET    /api/admin/system-config
   POST   /api/admin/system-config
   PUT    /api/admin/system-config/:key
   GET    /api/admin/system-config/constraints
   ```

5. **Room Outputs:**
   ```
   GET    /api/room-sessions/{id}/outputs
   GET    /api/room-sessions/{id}/state
   ```

**Tests:**
- All endpoints with auth
- Validation (port names unique, schemas valid)
- Admin-only enforcement

**Critical Files:**
- `/src/server/api/workflow_ports.rs` (NEW)
- `/src/server/api/routing_configs.rs` (NEW)
- `/src/server/api/system_config.rs` (NEW)
- `/src/server/api/rooms.rs` (extend)

---

### Phase 9: Integration Testing (3-4 days)

**End-to-End Workflows**

1. **Test Case 1: Simple Port-Based Pipeline**
   - Step 1: Analyzer (output: analysis)
   - Step 2: Implementer (input: analysis from Step 1)
   - Verify: Port connection, envelope unwrapping

2. **Test Case 2: Label Routing**
   - Step 1: Decomposer (output: array with category field)
   - Step 2: For-each label routing (4-6 items to different specialist agents)
   - Verify: Correct agent selection, parallel execution

3. **Test Case 3: Cavernous Routing**
   - Step with execution_mode: "cavernous"
   - Document search → config selection → subtask execution
   - Verify: Config applied, subtasks spawned, aggregation

4. **Test Case 4: Enhanced Room**
   - Room with 3 agents (Analyzer, Planner, Implementer)
   - Each has output_schema_id
   - Verify: Structured outputs passed, gatekeeper uses schemas

5. **Test Case 5: Full Stack**
   - PRD → Cavernous decomposition → Label routing → Room review → Final implementation
   - All features together

**Verification:**
- All tests pass
- No panics
- Error messages actionable
- Cost tracking accurate

---

### Phase 10: Documentation & Polish (2-3 days)

1. **README Update:**
   - New architecture diagram
   - Three execution tiers explanation
   - Getting started with cavernous routing

2. **API Documentation:**
   - OpenAPI spec for all endpoints
   - Examples for port configuration

3. **Admin Guide:**
   - Creating routing config documents
   - Managing system config
   - Setting up capability taxonomy

4. **Code Cleanup:**
   - Remove old variable system remnants
   - `cargo fmt` + `cargo clippy` all files
   - Fix all warnings

5. **Frontend Stubs:**
   - Document UI requirements for visual builder
   - API integration points

---

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

---

## Progressive Mode Evolution: Feedback-Driven Layering

### Concept

**"Defining greater and greater bots for every loop of a task"** - As workflow executes, modes evolve based on step outputs. Later steps receive increasingly sophisticated modes informed by prior success.

### Architecture

**Not just cavernous routing** - This applies to ALL execution modes:
- Static agent execution → analyzes output → selects enhanced mode for next step
- Label routing → successful iterations inform next layer's mode selection
- Cavernous routing → subtask results feedback into next cavernous decision

### Mode Layer Evolution Flow

```
Workflow Start
    ↓
Step 1: Execute with Base Mode (e.g., "general_coding")
    ├─ Agent executes task
    ├─ Produces output with metadata (complexity, quality, issues found)
    └─ Store in envelope.metadata.output_analysis
    ↓
Mode Feedback Analysis (after Step 1)
    ├─ Analyze Step 1's output envelope
    ├─ Evaluate: code_quality, test_coverage, security_issues, complexity
    ├─ Determine next mode layer based on results
    └─ Store mode_evolution_path in workflow execution
    ↓
Step 2: Execute with Evolved Mode
    ├─ Mode selected based on Step 1 analysis
    ├─ Example: If Step 1 had security issues → Step 2 uses "security_hardened" mode
    ├─ If Step 1 was high complexity → Step 2 uses "advanced_architect" mode
    └─ Produces output, continues evolution
    ↓
Continues through pipeline...
    ├─ Each step analyzes previous output
    ├─ Selects more specialized/capable mode
    ├─ "Greater and greater bots" = progressive enhancement
    └─ "Dynamics down the loop" = adaptive based on actual execution results
```

### Database Schema Extension

```sql
-- Mode evolution tracking
CREATE TABLE workflow_execution_mode_evolution (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_execution_id UUID NOT NULL REFERENCES workflow_executions(id),
    step_id UUID NOT NULL REFERENCES workflow_steps(id),
    step_order INTEGER NOT NULL,

    -- Mode used for this step
    mode_id UUID REFERENCES tool_router_modes(id),
    mode_key TEXT NOT NULL,

    -- Feedback from previous step that informed this mode choice
    feedback_source_step_id UUID REFERENCES workflow_steps(id),
    feedback_analysis JSONB,  -- What was analyzed from previous output

    -- Evolution reasoning
    evolution_reasoning TEXT,  -- Why this mode was selected
    evolution_level INTEGER,   -- How many layers deep (1 = base, 2+ = evolved)

    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_mode_evolution_execution ON workflow_execution_mode_evolution(workflow_execution_id);

-- Extend workflow_steps with evolution config
ALTER TABLE workflow_steps
    ADD COLUMN enable_mode_evolution BOOLEAN DEFAULT false,
    ADD COLUMN evolution_strategy TEXT,  -- "quality_based", "complexity_based", "error_based", "custom"
    ADD COLUMN evolution_rules JSONB;     -- Rules for mode selection based on feedback

COMMENT ON COLUMN workflow_steps.enable_mode_evolution IS
    'If true, mode for this step is selected based on previous step outputs, not static config';

COMMENT ON COLUMN workflow_steps.evolution_rules IS
    'Mode evolution rules: {
       "if": {"output.code_quality": "<0.7"},
       "then": {"mode": "code_reviewer"},
       "if": {"output.security_issues": ">0"},
       "then": {"mode": "security_specialist"}
     }';
```

### Evolution Strategies

**1. Quality-Based Evolution:**
```json
{
  "strategy": "quality_based",
  "rules": [
    {
      "condition": {"envelope.metadata.code_quality_score": {"lt": 0.7}},
      "action": {"select_mode": "expert_code_reviewer", "reason": "Low quality detected"}
    },
    {
      "condition": {"envelope.metadata.test_coverage": {"lt": 0.8}},
      "action": {"select_mode": "test_specialist", "reason": "Insufficient test coverage"}
    }
  ]
}
```

**2. Complexity-Based Evolution:**
```json
{
  "strategy": "complexity_based",
  "rules": [
    {
      "condition": {"envelope.metadata.complexity_level": {"eq": "high"}},
      "action": {"select_mode": "advanced_architect", "reason": "High complexity requires expert"}
    },
    {
      "condition": {"envelope.metadata.dependencies_count": {"gt": 10}},
      "action": {"select_mode": "dependency_specialist", "reason": "Many dependencies"}
    }
  ]
}
```

**3. Error-Based Evolution:**
```json
{
  "strategy": "error_based",
  "rules": [
    {
      "condition": {"envelope.status": {"eq": "partial"}},
      "action": {"select_mode": "error_recovery_specialist", "reason": "Partial success needs remediation"}
    },
    {
      "condition": {"envelope.metadata.security_issues_found": {"gt": 0}},
      "action": {"select_mode": "security_hardened", "reason": "Security issues detected"}
    }
  ]
}
```

### Implementation

**File:** `/src/server/hub/mode_evolution/mod.rs` (NEW)

```rust
pub struct ModeEvolutionEngine {
    state: AppState,
}

impl ModeEvolutionEngine {
    /// Analyze previous step output and select evolved mode for next step
    pub async fn evolve_mode(
        &self,
        current_step: &WorkflowStepRow,
        previous_step_execution: &AgentExecutionRow,
        workflow_execution_id: Uuid,
    ) -> Result<ResolvedModeConfig> {
        // 1. Parse previous step's output envelope
        let prev_envelope: StepExecutionEnvelope =
            serde_json::from_value(previous_step_execution.structured_output.clone())?;

        // 2. Extract feedback metadata
        let feedback = FeedbackAnalysis {
            code_quality: prev_envelope.metadata.get("code_quality_score"),
            complexity: prev_envelope.metadata.get("complexity_level"),
            issues_found: prev_envelope.metadata.get("issues"),
            test_coverage: prev_envelope.metadata.get("test_coverage"),
        };

        // 3. Apply evolution rules
        let evolution_rules: EvolutionRules = serde_json::from_value(
            current_step.evolution_rules.clone().unwrap_or(json!({}))
        )?;

        let selected_mode = self.apply_evolution_rules(&evolution_rules, &feedback)?;

        // 4. Resolve mode config
        let mode_config = self.mode_resolver.resolve_by_mode_key(&selected_mode.mode_key).await?;

        // 5. Track evolution
        self.track_mode_evolution(
            workflow_execution_id,
            current_step.id,
            selected_mode,
            &feedback
        ).await?;

        Ok(mode_config)
    }

    fn apply_evolution_rules(
        &self,
        rules: &EvolutionRules,
        feedback: &FeedbackAnalysis,
    ) -> Result<ModeSelection> {
        for rule in &rules.rules {
            if self.evaluate_condition(&rule.condition, feedback)? {
                return Ok(ModeSelection {
                    mode_key: rule.action.select_mode.clone(),
                    reasoning: rule.action.reason.clone(),
                });
            }
        }

        // Default: use current mode or base mode
        Ok(ModeSelection {
            mode_key: "general".to_string(),
            reasoning: "No evolution conditions matched".to_string(),
        })
    }
}
```

### Integration with DAG Executor

**Modify** `/src/server/hub/dag/mod.rs::execute_workflow_via_engine`:

```rust
for step in topological_order {
    // Check if this step has mode evolution enabled
    let mode_config = if step.enable_mode_evolution && previous_step_execution.is_some() {
        // Evolve mode based on previous step's output
        let evolution_engine = ModeEvolutionEngine::new(state.clone());
        evolution_engine.evolve_mode(&step, &previous_step_execution.unwrap(), workflow_execution_id).await?
    } else {
        // Standard mode resolution
        mode_resolver.resolve(&agent, &user_input, None).await?
    };

    // Execute step with evolved mode
    let result = execute_step_with_mode(step, mode_config, ...).await?;

    previous_step_execution = Some(result);
}
```

### Example: Progressive Enhancement Pipeline

```
Workflow: "Implement Authentication System"

Step 1: Initial Design
    Mode: "general_architect"
    Output: {design: {...}, complexity: "high", security_critical: true}
    ↓
    Mode Evolution Analysis:
        - Complexity: high → next mode should be expert-level
        - Security critical: true → need security specialist
    ↓
Step 2: Security Review (evolved mode)
    Mode: "security_specialist" (selected based on Step 1 output)
    Output: {issues: [{type: "auth_bypass", severity: "high"}], security_score: 0.6}
    ↓
    Mode Evolution Analysis:
        - Security score low → need hardened implementation mode
        - High severity issue → need extra validation
    ↓
Step 3: Secure Implementation (evolved mode)
    Mode: "security_hardened_implementer" (selected based on Step 2 output)
    Output: {code: "...", security_tests: [...], security_score: 0.95}
    ↓
    Mode Evolution Analysis:
        - Security score high → validation mode can be lighter
        - Tests comprehensive → need integration validator
    ↓
Step 4: Integration Validation (evolved mode)
    Mode: "integration_validator"
    Output: {validated: true, all_tests_pass: true}
```

**Key Point:** Each step gets a "greater bot" (more specialized mode) based on previous step's actual output, not predefined workflow logic.

### Benefits

1. **Adaptive Expertise**: Workflow automatically escalates to expert modes when needed
2. **Cost Optimization**: Only use expensive/sophisticated modes when justified by feedback
3. **Quality Improvement**: "Dynamics down the loop" ensures progressive refinement
4. **Error Recovery**: Automatically switch to specialized modes when issues detected
5. **Context-Aware**: Mode selection informed by actual execution results, not assumptions

---

## Critical Files Reference

### Database Migrations (Create)

| File | Purpose | Priority |
|------|---------|----------|
| `/migrations/067_port_based_workflows.sql` | step_outputs, step_inputs, step_routing_rules, edges enhancement, drop execution_variables | **CRITICAL** |
| `/migrations/068_tool_capabilities.sql` | tool_capabilities, assignments, mode requirements, seed data | **HIGH** |
| `/migrations/069_cavernous_routing.sql` | agent_executions extensions (routing_analysis, selected_routing_document_id) | **HIGH** |
| `/migrations/070_enhanced_rooms.sql` | room_execution_outputs, room member ports, session structured state | **MEDIUM** |
| `/migrations/071_master_system_config.sql` | system_config table, default constraints | **HIGH** |
| `/migrations/072_mode_evolution.sql` | workflow_execution_mode_evolution, workflow_steps evolution config | **MEDIUM** |

### Core Rust Types (Create/Extend)

| File | Purpose | Priority |
|------|---------|----------|
| `/src/types/execution.rs` | StepExecutionEnvelope, ForEachAggregateEnvelope, ExecutionMetadata | **CRITICAL** |
| `/src/types/workflow.rs` | StepInputRow, StepOutputRow, StepRoutingRuleRow, EdgeWithPorts | **CRITICAL** |
| `/src/types/routing.rs` | RoutingConfigDocument, Subtask, RoutingAnalysis | **HIGH** |
| `/src/types/tool.rs` | ToolCapabilityRow, capability assignments | **MEDIUM** |
| `/src/types/room.rs` | RoomExecutionOutput, RoomState | **MEDIUM** |

### Database Queries (Extend)

| File | Functions to Add | Priority |
|------|------------------|----------|
| `/src/db/queries/workflows.rs` | query_step_inputs, query_step_outputs, query_step_routing_rules, create_step_input/output | **CRITICAL** |
| `/src/db/queries/tools.rs` | query_tool_capabilities, query_tools_by_capability, assign_capability_to_tool | **HIGH** |
| `/src/db/queries/documents.rs` | search_routing_configs, get_routing_config_by_name | **HIGH** |
| `/src/db/queries/rooms.rs` | save_room_execution_output, query_room_outputs, query_outputs_by_schema | **MEDIUM** |
| `/src/db/queries/system_config.rs` | get_system_config, set_system_config, get_execution_constraints | **HIGH** |

### Execution Engine (Modify)

| File | Changes | Priority |
|------|---------|----------|
| `/src/server/hub/dag/mod.rs` | Port resolution, envelope wrapping, for-each label routing, mode evolution integration | **CRITICAL** |
| `/src/server/hub/mode_resolver/mod.rs` | Capability-based tool selection, ResolvedModeConfig.capabilities field | **HIGH** |
| `/src/server/executors/room/mod.rs` | Structured output passing, room state service, gatekeeper enhancement | **HIGH** |
| `/src/server/executors/dag/mod.rs` | Remove variable system completely, use ports | **CRITICAL** |
| `/src/agents/gatekeeper.rs` | Extend input with available_outputs, schema-aware prompt | **MEDIUM** |

### New Execution Strategies (Create)

| File | Purpose | Priority |
|------|---------|----------|
| `/src/server/hub/strategies/cavernous/mod.rs` | CavernousStepStrategy implementation (document search, config selection, subtask execution) | **HIGH** |
| `/src/server/hub/strategies/cavernous/config.rs` | RoutingConfigDocument parsing, validation | **HIGH** |
| `/src/server/hub/strategies/cavernous/executor.rs` | Subtask DAG execution, aggregation | **HIGH** |
| `/src/server/executors/room/state.rs` | RoomState service (output accumulation, queries) | **MEDIUM** |
| `/src/server/hub/mode_evolution/mod.rs` | ModeEvolutionEngine (feedback analysis, mode selection) | **MEDIUM** |

### API Endpoints (Create)

| File | Endpoints | Priority |
|------|-----------|----------|
| `/src/server/api/workflow_ports.rs` | POST/GET/PUT/DELETE for step inputs/outputs, routing rules | **HIGH** |
| `/src/server/api/routing_configs.rs` | Admin CRUD for routing config documents | **HIGH** |
| `/src/server/api/system_config.rs` | Admin CRUD for system config, get constraints | **MEDIUM** |
| `/src/server/api/rooms.rs` | GET room outputs, GET room state (extend existing) | **LOW** |

### Code to Remove

| Location | What to Remove | Why |
|----------|----------------|-----|
| `/src/server/executors/dag/mod.rs` | All `execution_variables` code, variable interpolation `{variable}` | Replaced by ports |
| `/src/db/queries/executions.rs` | `save_execution_variable`, `get_execution_variables` | Table dropped |
| Database | `execution_variables` table | Port system replaces it |

### Frontend Stubs (Document for Future)

| Concept | UI Requirements | Phase |
|---------|-----------------|-------|
| Visual workflow builder | React Flow canvas, port connections, node configuration | Future |
| Routing config editor | Document-based config creation, JSON schema editor | Future |
| System config dashboard | Admin panel for capabilities, constraints, agents | Future |
| Room execution viewer | Real-time structured output display, state visualization | Future |
| Mode evolution tracker | Visualization of mode changes through workflow | Future |

---

## Success Criteria

### Phase Completion Checklist

**Foundation:**
- [x] All 6 migrations run successfully
- [x] No foreign key constraint errors
- [x] Seed data inserted (15+ capabilities)
- [x] `execution_variables` table dropped

**Core Functionality:**
- [x] Ports resolve correctly (input from upstream output)
- [x] Envelopes wrap all step outputs
- [x] For-each label routing works (routes by category to different agents)
- [x] Tool capabilities query/filter correctly
- [x] Mode resolver includes capability-based tools

**Advanced Features:**
- [x] Cavernous routing: document search → config selection → subtask execution
- [x] Enhanced rooms: structured outputs pass between agents
- [x] Gatekeeper uses output schemas for speaker selection
- [x] Mode evolution: feedback from step N informs mode for step N+1

**Quality:**
- [x] `cargo test` all passing
- [x] `cargo clippy` clean (no warnings)
- [x] All API endpoints tested (Postman/curl)
- [x] Integration tests cover all three execution tiers
- [x] Error messages are actionable

**Documentation:**
- [x] README updated with new architecture
- [x] OpenAPI spec for all endpoints
- [x] Admin guide for routing config creation
- [x] Example workflows for each tier

---

## Total Timeline Summary

**Foundation:** 3-4 days
**Core Types + Queries:** 5-6 days
**Mode Resolver + DAG Executor:** 6-8 days
**Cavernous Routing:** 5-6 days
**Enhanced Rooms:** 4-5 days
**API + Integration + Docs:** 6-9 days
**Mode Evolution (bonus):** 3-4 days (optional, can be later phase)

**Total (without mode evolution):** 29-38 days (~6-8 weeks)
**Total (with mode evolution):** 32-42 days (~6.5-8.5 weeks)

**Recommendation:** Build in order, test each phase before proceeding. Mode evolution can be added after core system is stable.

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
