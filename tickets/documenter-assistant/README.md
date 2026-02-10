# Documenter Assistant — Generative Agent for Document Definition

## Vision

The Documenter node currently requires users to manually define document targets (name, description, target length) through a form UI. Users often don't know what documents they need until they think about it — and most would ask an LLM anyway. We cut the middleman: a built-in agent that understands the documenter's task, can see upstream context, and creates document definitions directly on the canvas through conversation.

The user opens an **Assistant tab** on the documenter node, chats with a dedicated agent, and watches document nodes materialize on the canvas in real time.

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Agent | Dedicated system agent | Purpose-built for documenter assistance, consistent behavior regardless of step config |
| Session model | One persistent session per step, with clear option | Resume where you left off, wipe to start fresh |
| Manual + AI sync | Agent reads live state before each tool call | Stays aware of manual edits in Documents tab |
| Streaming | SSE via existing `/sessions/{id}/chat/{msg_id}/stream` | Reuses proven infrastructure |
| Tools | Scoped to documenter actions only | `create_doc_def`, `update_doc_def`, `delete_doc_def`, `read_context`, `update_prompt`, `think` |
| Context model | Upstream nodes as labeled context sources | Agent reasons from shape, not content — handles design-time vs runtime gap |
| Step descriptions | `description` column on `workflow_steps`, seeded per `execution_mode` | Every node knows what it is from creation. Port manifest reads this column. Editable in Settings tab. |

## Phases

| Phase | Ticket | Summary |
|-------|--------|---------|
| 1 | [phase-1-system-agent-and-tools.md](./phase-1-system-agent-and-tools.md) | Backend: `description` column on `workflow_steps`, dedicated agent, documenter tools, tool execution |
| 2 | [phase-2-session-management.md](./phase-2-session-management.md) | Backend: session lifecycle tied to documenter steps, context injection using port manifest |
| 3 | [phase-3-frontend-assistant-tab.md](./phase-3-frontend-assistant-tab.md) | Frontend: Assistant tab on DocumenterNode, ChatPanel integration, description in Settings tab |
| 4 | [phase-4-streaming-and-tool-indicators.md](./phase-4-streaming-and-tool-indicators.md) | Frontend: SSE streaming, tool start/end indicators in chat |
| 5 | [phase-5-reactive-canvas.md](./phase-5-reactive-canvas.md) | Full loop: WS events on doc-def mutation, canvas reactivity |

## Context Source Model

A key design concept: upstream nodes are presented to the agent as **labeled context sources** via a port manifest. Each source has a `description` (from the `workflow_steps.description` column) and a `content_status`:

| Status | Meaning | Example |
|--------|---------|---------|
| `populated` | Content exists now (user typed it in, or a previous run produced it) | Context node with an OpenAPI spec pasted in |
| `empty` | Context node exists but has no content yet | User created a "Style Guide" node but hasn't filled it |
| `pending` | Runtime-producing step — content won't exist until workflow executes | A Researcher node, a processing step |

Every `workflow_step` has a `description` column, seeded with a meaningful default based on its `execution_mode` at creation time:

| execution_mode | Default description |
|---|---|
| `context` | User-provided text input, injected directly into the orchestrator. |
| `single` | Processes input through a configured agent and produces output. |
| `for_each` | Iterates over an array input, processing each item through a configured agent. |
| `documenter` | Document generation orchestrator. Defines and produces documents from incoming context. |
| `room` | Multi-agent discussion room. Agents collaborate to produce a consensus output. |

Users can customize per-instance in the Settings tab (e.g., "This researcher specifically analyzes authentication patterns in the Rust API layer") but the defaults are always meaningful out of the box.

The agent reasons from the **shape** of incoming context: a step named "Codebase Analyzer" with description "User-defined search strategy. Results are researched, compiled, and injected into the orchestrator." tells it enough to define documents like "Error Handling Reference" and "API Pattern Guide" with appropriate sizing — even though the research hasn't happened yet. At runtime, the full context arrives and the documenter protocol generates actual content.

## End-to-End Flow

```
User clicks Assistant tab on DocumenterNode
  -> find-or-create chat_session for this step
  -> load history from GET /sessions/{id}/chat/history
  -> system prompt auto-injected with port manifest

User types: "Set up docs for this REST API service"
  -> POST /sessions/{id}/chat -> message_id
  -> connect SSE /sessions/{id}/chat/{msg_id}/stream

Agent reads context (port manifest):
  -> sees Researcher (pending) — description: "User-defined search strategy..."
  -> sees API Spec (populated) — preview of OpenAPI doc, ~2400 words
  -> reasons about document structure from this shape

Agent streams response, decides to call tools:
  -> StreamChunk::ToolStart("create_doc_def")
  -> backend creates def via existing document-def CRUD
  -> broadcasts WS event on Workflow topic: { event: "doc_def_changed", step_id }
  -> StreamChunk::ToolEnd("create_doc_def")
  -> canvas receives WS event -> refetches doc defs -> DocumentNode appears
  -> agent continues streaming: "I've created 3 documents for you..."

User sees documents materialize on canvas while reading the agent's response.
User can say "remove the changelog, add a migration guide instead"
  -> agent calls delete_doc_def + create_doc_def
  -> canvas updates in real time

Later, workflow runs:
  -> Researcher executes, produces real analysis
  -> Documenter protocol receives full context + document defs
  -> Generates actual document content with everything available
```

## Architecture Diagram

```
                    DocumenterNode (Frontend)
                    +---------------------------+
                    | Prompt | Docs | In | Set | AI |  <-- 5 tabs
                    |                           |
                    | [Assistant Tab]           |
                    | +- ChatPanel ---------+  |
                    | | user: set up docs   |  |
                    | | agent: I'll create  |  |
                    | |   [tool: creating]  |  |
                    | | agent: Done! 3 docs |  |
                    | +--------------------+  |
                    +---------------------------+
                              |
            POST /sessions/{id}/chat
            GET  /sessions/{id}/chat/{msg}/stream
                              |
                    +---------v---------+
                    |   Chat Consumer   |
                    |   (existing)      |
                    +---------+---------+
                              |
                    +---------v---------+
                    | ExecutionEngine   |
                    | + ChatStrategy    |
                    |   system_prompt:  |
                    |     base prompt   |
                    |     + port        |
                    |       manifest    |
                    |   tools:          |
                    |     doc-def CRUD  |
                    |     prompt R/W    |
                    |     context read  |
                    +---------+---------+
                              |
                    +---------v---------+
                    | Tool Execution    |
                    | create_doc_def -->+---> DB write
                    |                   |---> WS broadcast
                    +-------------------+
                              |
                    +---------v---------+
                    |  WorkflowCanvas   |
                    |  receives WS evt  |
                    |  refetches defs   |
                    |  DocumentNodes    |
                    |  appear on canvas |
                    +-------------------+
```
