# Phase 1: System Agent, Step Descriptions, and Documenter Tools

**Scope:** Backend only — add `description` column to `workflow_steps`, create the dedicated documenter assistant agent, and register the tools it needs to manipulate document definitions, prompts, and read the port manifest.

## 1.1 Step Description Column

### What

Every `workflow_step` gets a `description` column that explains what the step is and what it provides. This is seeded with a meaningful default based on the step's `execution_mode` at creation time and is editable by users in the Settings tab.

### Migration

```sql
-- Add description column to workflow_steps
ALTER TABLE workflow_steps
  ADD COLUMN description text NOT NULL DEFAULT '';

-- Backfill existing steps with defaults based on execution_mode
UPDATE workflow_steps SET description = 'User-provided text input, injected directly into the orchestrator.'
  WHERE execution_mode = 'context' AND description = '';
UPDATE workflow_steps SET description = 'Processes input through a configured agent and produces output.'
  WHERE execution_mode = 'single' AND description = '';
UPDATE workflow_steps SET description = 'Iterates over an array input, processing each item through a configured agent.'
  WHERE execution_mode = 'for_each' AND description = '';
UPDATE workflow_steps SET description = 'Document generation orchestrator. Defines and produces documents from incoming context.'
  WHERE execution_mode = 'documenter' AND description = '';
UPDATE workflow_steps SET description = 'Multi-agent discussion room. Agents collaborate to produce a consensus output.'
  WHERE execution_mode = 'room' AND description = '';
UPDATE workflow_steps SET description = 'Document-based dynamic routing with agent collaboration.'
  WHERE execution_mode = 'cavernous' AND description = '';
```

### Default Descriptions

Seeded at step creation time in the backend step creation handler:

| execution_mode | Default description |
|---|---|
| `context` | User-provided text input, injected directly into the orchestrator. |
| `single` | Processes input through a configured agent and produces output. |
| `for_each` | Iterates over an array input, processing each item through a configured agent. |
| `documenter` | Document generation orchestrator. Defines and produces documents from incoming context. |
| `room` | Multi-agent discussion room. Agents collaborate to produce a consensus output. |
| `cavernous` | Document-based dynamic routing with agent collaboration. |

### Model Change

Add `description: String` to `WorkflowStepRow` in `src/db/mod.rs`:

```rust
pub struct WorkflowStepRow {
    // ... existing fields ...
    pub description: String,  // NEW — seeded per execution_mode
}
```

### Step Creation Seeding

In the step creation handler, when `description` is not provided in the request body, auto-populate from a lookup:

```rust
const DEFAULT_DESCRIPTIONS: &[(&str, &str)] = &[
    ("context", "User-provided text input, injected directly into the orchestrator."),
    ("single", "Processes input through a configured agent and produces output."),
    ("for_each", "Iterates over an array input, processing each item through a configured agent."),
    ("documenter", "Document generation orchestrator. Defines and produces documents from incoming context."),
    ("room", "Multi-agent discussion room. Agents collaborate to produce a consensus output."),
    ("cavernous", "Document-based dynamic routing with agent collaboration."),
];
```

### API Exposure

- `GET /api/workflows/{id}/steps` — response includes `description` field
- `PATCH /api/workflows/{id}/steps/{step_id}` — accepts `description` in update body
- Frontend Settings tab exposes the field (Phase 3)

### Files to modify

| File | Change |
|------|--------|
| `migrations/0022_documenter_assistant.sql` | Add `description` column, backfill defaults |
| `src/db/mod.rs` | Add `description: String` to `WorkflowStepRow` |
| `src/server/api/workflows/mod.rs` | Seed default description on step creation, accept in update |

---

## 1.2 Dedicated System Agent

### What

A built-in agent that exists in every Nexor installation, purpose-built for documenter assistance. It does not appear in the user-facing agent list — it's an internal system agent.

### Implementation

**Seed the agent via migration:**

```sql
-- Add is_system column to agents
ALTER TABLE agents ADD COLUMN is_system boolean NOT NULL DEFAULT false;

-- Seed documenter assistant agent
INSERT INTO agents (id, name, description, model_id, system_prompt, is_system)
VALUES (
  '00000000-0000-0000-0000-000000000001',  -- well-known UUID
  'documenter-assistant',
  'Assists users in defining document targets for documenter workflow steps.',
  'claude-sonnet-4-5-20250929',
  <system_prompt>,
  true
);
```

**System prompt** (stored in DB, editable by admins):

```
You are a document planning assistant for the Nexor workflow engine.

Your job is to help users define the right set of document targets for a documenter step. You understand the documenter's purpose, its incoming context sources, and what kinds of documents would be valuable.

## Your capabilities

You can:
- Create, update, and delete document definitions that appear as nodes on the workflow canvas
- Read and update the documenter's instruction prompt
- Read the port manifest to understand what data flows into this step

## How you work

1. Start by reading the current state: existing document defs, the prompt, and the port manifest
2. Ask clarifying questions if the user's request is ambiguous
3. Create document definitions with clear names, descriptions, and appropriate target lengths
4. Explain your reasoning so the user can adjust

## Understanding incoming context (port manifest)

Upstream nodes connected to this documenter are presented as **context sources** in a port manifest. Each has a name, type, description, and content status:

- **populated** — The source has content right now (e.g., a context node the user has filled in). You can see a preview and word count. Use this content to inform your document definitions.
- **empty** — A context node that exists but hasn't been filled in yet. The user may fill it later, or it may be intentionally blank.
- **pending** — A step that produces output at runtime (e.g., a researcher, a regular processing step). You won't see content now, but you know what it will provide based on its name and description.

When planning documents, reason from the *shape* of incoming context:
- A "Researcher" source with description "User-defined search strategy. Results are researched, compiled, and injected into the orchestrator." tells you research output will be available at runtime — define documents that would leverage that research.
- A "Style Guide" context node that's populated gives you concrete constraints to incorporate.
- A pending source means the document definitions you create should be structured to receive and utilize that content when the workflow runs.

You are defining document *targets* — the actual content generation happens later when the full workflow executes and all context sources are resolved. Your job is to define the right structure, sizing, and descriptions so the documenter protocol can do its job well.

## Guidelines

- Prefer specific, actionable document names (e.g., "API Reference — Authentication Endpoints" over "API Docs")
- Set realistic target_length values: short (500-1000), medium (1500-3000), long (3000-6000)
- Each document should have a single clear purpose — split rather than combine
- Always read the current state before making changes to avoid duplicates
- Size documents relative to the expected incoming context — a researcher producing deep analysis warrants longer documents than a brief style guide
- When updating, preserve the user's manual edits unless they ask you to override
```

**`is_system` column:**

System agents are:
- Excluded from `GET /api/agents` list responses (unless `?include_system=true`)
- Not deletable via API
- Referenced by well-known UUID constants in code

### Files to modify

| File | Change |
|------|--------|
| `migrations/0022_documenter_assistant.sql` | Add `is_system` column to agents, seed agent row |
| `src/db/mod.rs` | Add `is_system: bool` to `AgentRow` |
| `src/server/api/agents/mod.rs` | Filter system agents from list endpoint |
| `src/server/constants.rs` | Add `DOCUMENTER_ASSISTANT_AGENT_ID` UUID constant |

---

## 1.3 Documenter Tools

### What

Six tools scoped to documenter operations. These are registered in the tool registry and only available to the documenter assistant agent (injected via strategy, not globally available).

### Tool Definitions

#### `create_doc_def`

Creates a new document definition on the documenter step.

```json
{
  "name": "create_doc_def",
  "description": "Create a new document definition on the documenter step. The document will appear as a node on the workflow canvas.",
  "input_schema": {
    "type": "object",
    "properties": {
      "name": {
        "type": "string",
        "description": "Document name (e.g., 'API Reference', 'Migration Guide')"
      },
      "description": {
        "type": "string",
        "description": "What this document should contain and its purpose"
      },
      "target_length": {
        "type": "integer",
        "description": "Target word count. Short: 500-1000, Medium: 1500-3000, Long: 3000-6000"
      }
    },
    "required": ["name"]
  }
}
```

**Execution:** Calls existing `workflows.createDocumentDef(workflow_id, step_id, body)` internally. The `workflow_id` and `step_id` come from `DocumenterToolContext`, not tool input. Returns the created `ProtocolDocumentDefRow` as JSON. Broadcasts WS event (Phase 5).

#### `update_doc_def`

Updates an existing document definition.

```json
{
  "name": "update_doc_def",
  "description": "Update an existing document definition. Use read_context first to see current definitions.",
  "input_schema": {
    "type": "object",
    "properties": {
      "doc_def_id": {
        "type": "string",
        "description": "ID of the document definition to update"
      },
      "name": { "type": "string" },
      "description": { "type": "string" },
      "target_length": { "type": "integer" }
    },
    "required": ["doc_def_id"]
  }
}
```

**Execution:** Calls existing `workflows.updateDocumentDef(...)`. Returns updated def.

#### `delete_doc_def`

Removes a document definition (and its canvas node).

```json
{
  "name": "delete_doc_def",
  "description": "Delete a document definition. The corresponding node will be removed from the canvas.",
  "input_schema": {
    "type": "object",
    "properties": {
      "doc_def_id": {
        "type": "string",
        "description": "ID of the document definition to delete"
      }
    },
    "required": ["doc_def_id"]
  }
}
```

**Execution:** Calls existing `workflows.deleteDocumentDef(...)`. Returns `{ "deleted": true }`.

#### `read_context`

Reads the current state of the documenter step: existing doc defs, prompt, and the port manifest of incoming context sources.

```json
{
  "name": "read_context",
  "description": "Read the current state of the documenter step including existing document definitions, the instruction prompt, and the port manifest of incoming context sources. Upstream nodes are presented as labeled context sources with descriptions and content status. Call this before making changes to understand the current state.",
  "input_schema": {
    "type": "object",
    "properties": {},
    "required": []
  }
}
```

**Execution:** Fetches and assembles:
- Current document defs for this step (from `protocol_document_defs` table)
- Current `prompt_template` from the step row
- **Port manifest** — incoming context sources built from upstream steps

**Returns structured JSON:**

```json
{
  "step_name": "API Documenter",
  "prompt_template": "Generate comprehensive API documentation...",
  "document_definitions": [
    { "id": "uuid", "name": "API Reference", "description": "...", "target_length": 3000 }
  ],
  "incoming_context": [
    {
      "source_name": "Researcher",
      "source_type": "single",
      "description": "User-defined search strategy. Results are researched, compiled, and injected into the orchestrator.",
      "content_status": "pending",
      "content_preview": null,
      "content_length_words": null
    },
    {
      "source_name": "API Spec",
      "source_type": "context",
      "description": "User-provided text input, injected directly into the orchestrator.",
      "content_status": "populated",
      "content_preview": "openapi: 3.0.0\ninfo:\n  title: Auth Service...",
      "content_length_words": 2400
    },
    {
      "source_name": "Style Guide",
      "source_type": "context",
      "description": "User-provided text input, injected directly into the orchestrator.",
      "content_status": "empty",
      "content_preview": null,
      "content_length_words": null
    }
  ]
}
```

**Port manifest construction:**

For each upstream step (steps with edges pointing to this documenter):

1. Read the step's `name`, `execution_mode`, and `description` from `workflow_steps`
2. Classify `content_status`:
   - If `execution_mode == "context"` AND `prompt_template` is non-empty → `"populated"` (include preview of first 500 chars, word count)
   - If `execution_mode == "context"` AND `prompt_template` is empty → `"empty"`
   - All other execution modes → `"pending"` (runtime-producing steps)
3. `description` comes directly from the step's `description` column — NOT from `prompt_template`

**Why this model matters:**

At design time, the agent works from the *shape* of incoming context — not the content itself. The `description` column tells the agent what kind of data each source provides, even when no content exists yet. The `content_status` field makes the design-time vs runtime distinction explicit so the agent doesn't hallucinate content or ask the user to fill in things that are meant to be runtime-populated.

This is the "live state read" — ensures the agent sees manual edits the user made in the Documents tab AND understands the full picture of what context will be available at runtime.

#### `update_prompt`

Updates the documenter step's instruction prompt.

```json
{
  "name": "update_prompt",
  "description": "Update the documenter step's instruction prompt template. This controls how the documenter generates documents.",
  "input_schema": {
    "type": "object",
    "properties": {
      "prompt_template": {
        "type": "string",
        "description": "The new instruction prompt for the documenter step"
      }
    },
    "required": ["prompt_template"]
  }
}
```

**Execution:** Updates the step's `prompt_template` via existing `workflows.updateStep(...)`. Returns `{ "updated": true, "prompt_template": "..." }`. Broadcasts WS event so the Prompt tab updates if open.

#### `think`

Reasoning scratchpad (reuse existing `think` tool).

Already registered in the tool registry. Include it in the documenter assistant's tool set — it helps the agent reason through document structure before acting.

### Tool Execution Context

Tools need `workflow_id` and `step_id` to operate. These are NOT passed in tool inputs (the agent shouldn't need to know IDs). Instead, they're injected via the strategy context:

```rust
pub struct DocumenterToolContext {
    pub workflow_id: Uuid,
    pub step_id: Uuid,
    pub user_id: UserId,
    pub state: AppState,
}
```

When the documenter assistant session is created (Phase 2), the `workflow_id` and `step_id` are stored in `chat_sessions.draft_config` as JSON. The strategy reads them at execution time and passes them to tool handlers.

### Files to create/modify

| File | Change |
|------|--------|
| `src/tools/registry/mod.rs` | Register 5 new tool definitions |
| `src/server/tools/documenter/mod.rs` | **New** — tool execution handlers |
| `src/server/tools/documenter/tests.rs` | **New** — unit tests for each tool |
| `src/server/tools/mod.rs` | Add `pub mod documenter;` route to new handlers |
| `src/server/hub/strategies/chat/mod.rs` | Detect documenter session, inject tool context |

### Tests

- Unit test each tool handler with mock DB state
- Test `read_context` returns correct assembled structure with port manifest
- Test `read_context` classifies content_status correctly (populated vs empty vs pending)
- Test `read_context` reads `description` from the step's `description` column, not `prompt_template`
- Test `create_doc_def` calls correct repo method and returns expected shape
- Test tool definitions have valid JSON schemas
- Test that system agent is excluded from public agent list
- Test that default descriptions are seeded correctly for each `execution_mode`

## Acceptance Criteria

- [ ] `description` column exists on `workflow_steps`, backfilled for existing rows
- [ ] Default descriptions seeded at step creation time based on `execution_mode`
- [ ] `description` included in step API responses and accepted in update requests
- [ ] `is_system` column exists on `agents` table, system agents filtered from list endpoint
- [ ] Documenter assistant agent seeded in DB with well-known UUID
- [ ] All 5 new tools registered with valid JSON schemas (+ existing `think`)
- [ ] Tool execution handlers call existing CRUD methods (no new DB queries for doc defs)
- [ ] `DocumenterToolContext` provides workflow_id/step_id without tool input
- [ ] `read_context` assembles port manifest with description from `workflow_steps.description` column
- [ ] `read_context` classifies content_status: populated (context with content), empty (context without), pending (non-context)
- [ ] All tool handlers have unit tests
