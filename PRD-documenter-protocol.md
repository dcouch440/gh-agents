# PRD: Documenter Protocol

## Overview

The Documenter Protocol is a new protocol type that turns the workflow canvas into a document generation pipeline. A single protocol node takes a user prompt, receives upstream context through wires, and produces finished documents that appear as first-class nodes on the canvas. Those document nodes can be wired into any downstream agent as context.

The protocol agent handles everything internally. The user sees one protocol node and its output documents — no intermediate agents, no manual wiring of sub-steps. The complexity lives behind the curtain.

## The Problem

Putting individual agent nodes on the screen with system prompts is tedious. The current protocol system (decomp, review, route) requires users to manually assign agents to ports and wire everything together. This doesn't scale for document-heavy workflows where an agent needs to research, synthesize, and produce polished output.

Users need a way to say: "Here are the documents I need. Here's what I want. Go." — and have the system figure out the execution strategy.

## Core Concepts

### The Documenter Protocol Node

A single visible node on the canvas. The user configures:

1. **Document definitions** — each has a name, target length, and description
2. **A prompt** — the user's natural language request

Adding a document definition to the protocol immediately spawns a **blank document node** on the canvas, wired as an output of the protocol.

### Auto-Generated System Prompt

The system builds the protocol agent's system prompt from the document configuration. The user never writes a system prompt — the document definitions ARE the configuration:

```
# System Document Decomp
The user will present you with a request to create 2 documents.
Please carefully review the user's request and write a query
to your fellow agents to get it done.

## Documents
Document 1: API Documentation
Length: 4000 char
Description: Comprehensive REST API reference for the auth service

Document 2: Rust Best Practices
Length: 2000 char
Description: Modern Rust patterns and idioms for our codebase
```

The user's prompt becomes the user message. Upstream context from wires is injected as additional context.

### Hidden Execution Chain

The protocol agent's structured output kicks off a hidden chain of agents inside the same DAG. These agents are real workflow steps with `visible=false` — the DAG executor runs them normally, but the canvas doesn't render them.

The shape of the hidden chain is dynamic. The protocol agent decides what needs to happen based on:
- What upstream context is available (wired inputs)
- What the user is asking for
- What documents need to be produced

### Context-to-Document Pipeline

The documenter is NOT hardcoded to "search the web." The upstream context determines the execution strategy:

```
SCENARIO A: No upstream, pure research
  [Documenter] --> hidden: search agent --> hidden: doc writer --> [Doc]

SCENARIO B: Upstream data, synthesis
  [Perf Agent] ----> [Documenter] --> hidden: aggregator --> hidden: writer --> [Thesis]
  [Load Agent] ---->

SCENARIO C: Code documentation
  [Code Analyzer] -> [Documenter] --> hidden: summarizer --> hidden: writer --> [API Docs]
  [Test Results]  ->
```

The protocol agent looks at what it has and what it needs to produce, then constructs the right pipeline. A performance thesis from upstream data doesn't need web search — it needs aggregation and professional writing. API docs from a codebase need code analysis, not web research.

### Capability-Based Agent Routing

The protocol agent's structured output includes **required capabilities** per document task. The system matches these against agents in the workspace using the existing `tool_capabilities` infrastructure.

```
Protocol agent output:
  Document 1: "API Documentation"
    required_capabilities: [code_analysis]
    strategy: "Analyze the src/server/api/ directory structure..."

  Document 2: "Rust Best Practices"
    required_capabilities: [web_search]
    strategy: "Research Rust 2025 best practices for enterprise..."
```

System matches:
- `code_analysis` capability --> Agent with git_checkout, file_read tools
- `web_search` capability --> Agent with web_search, web_fetch tools

The protocol agent needs to know what capabilities exist in the system so it can request them intelligently. This information is injected into its system prompt alongside the document definitions.

## User Flow

```
1. User drops a Documenter protocol node on the canvas

2. User adds document definitions:
   +-------------------------------+
   | Documenter Protocol           |
   |-------------------------------|
   | Doc 1: API Docs (4000 chars)  |
   | Doc 2: Best Practices (2000)  |
   |-------------------------------|
   | Prompt: "I am building a Rust |
   | app, research best practices  |
   | and document our API..."      |
   +-------------------------------+
          |              |
          v              v
   [Doc: API Docs]  [Doc: Best Practices]
      (blank)           (blank)

3. User wires upstream context (optional):
   [Code Analyzer] --> [Documenter Protocol]

4. User wires document outputs to downstream agents:
   [Doc: API Docs] --> [Implementation Agent]

5. Execution:
   - System builds system prompt from doc config
   - Protocol agent receives prompt + upstream context
   - Protocol agent outputs structured strategy per document
   - Hidden agents execute (matched by capability)
   - Documents populated, canvas updates in real-time
   - Downstream agents can now consume the documents
```

## Execution Detail

### Phase 1: Strategy Generation (Visible)

The protocol agent (the visible node) runs with:
- **System prompt**: auto-generated from document definitions + available capabilities
- **User message**: the user's prompt from the node
- **Context**: upstream data from wired inputs
- **Output schema**: structured response with per-document strategy

The agent responds with a plan — what needs to happen for each document, what capabilities are needed, and the search/analysis strategy.

### Phase 2: Research/Analysis (Hidden)

For each document, the system:
1. Reads the required capabilities from the strategy
2. Matches an agent via `tool_capabilities`
3. Creates a hidden (`visible=false`) workflow step
4. Executes the agent with the strategy as its prompt + upstream context as references

This phase handles the diverse workloads — web search, code analysis, container inspection, data aggregation — whatever the protocol agent determined was needed.

### Phase 3: Document Creation (Hidden)

For each document, a hidden document-writer agent:
1. Receives the research/analysis output from Phase 2
2. Receives the document spec (name, target length, description)
3. Produces polished document content
4. Saves to the `documents` table
5. Broadcasts update via WebSocket — canvas document node goes from blank to populated

## Existing Infrastructure We Build On

### documents table (exists)

```sql
CREATE TABLE documents (
    id          uuid PRIMARY KEY,
    user_id     uuid NOT NULL,
    session_id  uuid,
    title       text NOT NULL,
    content     text DEFAULT '' NOT NULL,
    summary     text DEFAULT '',
    doc_type    text DEFAULT 'architecture',
    ref_tag     text DEFAULT '',
    tags        text[] DEFAULT '{}',
    created_at  timestamptz DEFAULT now(),
    updated_at  timestamptz DEFAULT now()
);
```

Full CRUD API already exists at `/api/documents`.

### step_documents join table (exists)

```sql
CREATE TABLE step_documents (
    step_id     uuid NOT NULL,
    document_id uuid NOT NULL
);
```

Links steps to documents. Used for the protocol step --> document output relationship.

### tool_capabilities table (exists, unused)

```sql
CREATE TABLE tool_capabilities (
    id              uuid PRIMARY KEY,
    capability_key  text NOT NULL,      -- e.g. 'web_search', 'code_analysis'
    display_name    text NOT NULL,
    category        text NOT NULL,
    safety_level    text DEFAULT 'safe',
    description     text NOT NULL,
    created_at      timestamptz DEFAULT now()
);
```

Plus `tool_capability_assignments` (tool --> capability) and `mode_required_capabilities` (mode --> capability). This is the foundation for capability-based agent routing. Currently unwired — needs to be activated.

### protocol system (exists)

Protocol engine with expanders (decomp, review, route, transform). The documenter becomes a new expander type following the same `ProtocolConfig --> ProtocolExpansion` pattern.

## Proposed Schema Changes

### Add to `documents`

```sql
ALTER TABLE documents
    ADD COLUMN workflow_id uuid REFERENCES workflows(id),
    ADD COLUMN target_length integer,
    ADD COLUMN is_static boolean DEFAULT false,
    ADD COLUMN source_protocol_step_id uuid REFERENCES workflow_steps(id);
```

- `workflow_id`: ties doc to a workflow for canvas rendering
- `target_length`: from the user's document definition
- `is_static`: persists through runs (true) vs regenerated each run (false)
- `source_protocol_step_id`: which protocol step owns this document

### Add to `workflow_steps`

```sql
ALTER TABLE workflow_steps
    ADD COLUMN visible boolean DEFAULT true;
```

Hidden steps in the DAG. Executed normally, not rendered on canvas.

### New: `protocol_document_defs`

```sql
CREATE TABLE protocol_document_defs (
    id                  uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    protocol_step_id    uuid NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    document_id         uuid REFERENCES documents(id),
    name                text NOT NULL,
    description         text NOT NULL,
    target_length       integer NOT NULL,
    display_order       integer DEFAULT 0,
    created_at          timestamptz DEFAULT now()
);
```

Stores the user's document definitions on the protocol step. Links to the actual document once created. This is the configuration that drives the auto-generated system prompt.

## The Bigger Pattern: Protocols as Agents

The documenter protocol is the first protocol that **thinks**. Today's protocols (decomp, review, route) are static templates — pure functions that map port config to steps and edges. The documenter introduces an LLM call into the expander, making it the first protocol that looks at context and decides its own execution strategy.

This pattern, once proven, unlocks intelligent versions of every protocol type:
- **Smart decomp**: instead of the user pre-defining ports, the protocol agent looks at the task + available agent capabilities and generates the fan-out itself
- **Smart review**: adapts its evaluation criteria based on what it's actually reviewing
- **Smart route**: understands content semantics, not just field matching

### Design-Time Generation, Not Runtime

A critical constraint: **the canvas must be fully formed before execution starts.** The DAG loads all steps and edges upfront, topologically sorts them, and iterates. It does not support runtime topology changes.

This means intelligent protocols generate at **Apply time**, not at runtime. The Apply endpoint (`POST /api/protocols/:id/apply/:step_id`) already does exactly this — it runs an expander and creates steps + edges on the canvas. Today that expander is a pure function. Making it async and LLM-powered is the only change needed. Everything downstream of the expander (step creation, edge wiring, snapshot storage) stays identical.

For protocols like smart decomp where the generated steps are **visible** (user needs to wire them), this becomes a "Generate" action: the user writes a prompt, hits Generate, the protocol agent creates the steps, and they appear on the canvas for the user to review and wire before execution. Same Apply infrastructure, different visibility.

## What We Are NOT Building (Yet)

- **Auto mode**: Agent decides what documents to create. For now, documents are always user-configured. This is a future mode toggle (manual vs auto) where the agent's first call determines the document topology.
- **Document versioning**: Tracking changes across runs. Future work.
- **Static document editing**: In-canvas markdown editor for persistent docs. Future work.
- **Agent-level capability tags**: Higher-level semantic tags (researcher, writer) derived from or layered on top of tool capabilities. For now, we route on tool capabilities directly.

## Ticket Summary

### Foundation
1. **Schema migrations**: `documents` additions (workflow_id, target_length, is_static, source_protocol_step_id), `workflow_steps.visible`, `protocol_document_defs` table
2. **Wire up tool_capabilities**: Activate the existing but unused capability system. Build the matching function: given required capabilities, find the best agent
3. **Hidden step support in DAG**: Canvas mapper filters `visible=false` steps. DAG executor unchanged — hidden steps run normally

### Protocol Core
4. **Documenter expander**: New protocol type `"documenter"`. Expansion generates the hidden chain (strategy step, per-doc research steps, per-doc writer steps) with `visible=false`. System prompt auto-generated from `protocol_document_defs`
5. **Protocol document definitions API**: CRUD for document defs on a protocol step. Creating a def auto-creates a blank document in the `documents` table and a `DocumentNode` on the canvas
6. **Capability injection**: Protocol agent's system prompt includes available capabilities from the registry so it can request them intelligently in its structured output

### Frontend
7. **Document canvas node**: New React Flow node type rendering document name, description, content preview. Output port for wiring to downstream steps. Blank vs populated states
8. **Documenter protocol UI**: Document definition manager in the protocol node (add/remove/edit docs with name, length, description). Prompt text area. No system prompt field — it's auto-generated
9. **Real-time document updates**: WebSocket subscription for document content changes. Canvas node transitions from blank to populated during execution

### Execution
10. **Hidden chain execution**: Protocol agent output (structured strategy) triggers creation and execution of hidden steps in the DAG. Capability matching routes each doc task to the right agent
11. **Document save hook**: When a hidden doc-writer step completes, save output to `documents.content`. Broadcast via WebSocket
12. **Document port resolution**: Extend `resolve_port_inputs()` to pull document content when an edge source is a document node
