# Dispatch Tool Design: Research & Patterns

Research compiled February 2026. Covers modern patterns for designing batch mutation tools for AI agent orchestration. In Nexor's architecture, these structured tools belong to the **Background Agent** — a session service layer that configures workflow steps in the background. The assistant dispatches plain English instructions via `dispatch({ instruction: "..." })`; the Background Agent loads all current state and uses the structured mutation tools described in this research to configure the step. The Designer and DAG execution are a separate pipeline triggered when the user runs the workflow.

---

## Table of Contents

1. [Batch Mutation Tool Design](#1-batch-mutation-tool-design)
2. [Tool Schema Best Practices for LLM Reliability](#2-tool-schema-best-practices-for-llm-reliability)
3. [Async Dispatch Patterns](#3-async-dispatch-patterns)
4. [Mid-Flight Updates](#4-mid-flight-updates)
5. [Error Handling and Rollback](#5-error-handling-and-rollback)
6. [Existing Framework Patterns](#6-existing-framework-patterns)
7. [Recommendations for Nexor](#7-recommendations-for-nexor)

---

## 1. Batch Mutation Tool Design

### The Core Problem

The dispatch tool must accept a single structured changeset that creates, updates, and removes multiple entity types (agents, deliverables, notes, execution order) in one call. This is fundamentally a **batch mutation** problem -- how do you design a tool schema that an LLM can reliably populate while expressing complex, multi-entity intent?

### Pattern: Changeset Envelope

The dominant pattern across modern agentic systems is a **changeset envelope** -- a single top-level object that wraps all mutations into categorized arrays. This mirrors the CQRS (Command Query Responsibility Segregation) pattern where commands represent business intent rather than low-level data operations.

Key principle from CQRS literature: "Commands should represent specific business tasks instead of low-level data updates." For example, "Design a research team with three analysts" rather than "INSERT agent SET role='analyst'".

**Structure:**

```json
{
  "changes": {
    "agents": {
      "add": [...],
      "update": [...],
      "remove": [...]
    },
    "deliverables": {
      "add": [...],
      "update": [...],
      "remove": [...]
    }
  },
  "notes": "string",
  "execution_order": [...]
}
```

### Pattern: Typed Operation Discriminators

Each mutation entry carries its own operation type, which makes the schema self-describing:

```json
{
  "mutations": [
    { "op": "add", "entity": "agent", "data": { "name": "...", "role": "..." } },
    { "op": "update", "entity": "deliverable", "id": "...", "data": { "description": "..." } },
    { "op": "remove", "entity": "agent", "id": "..." }
  ]
}
```

This flat-discriminator approach is simpler for LLMs but harder to validate structurally (the `data` shape depends on `entity` type). The grouped approach (agents.add, agents.update) provides stronger schema validation at the cost of deeper nesting.

### Pattern: Inter-Agent Message Envelope

Research into multi-agent communication protocols suggests a standard envelope structure for dispatched work:

- **Header**: `msg_id`, `sender_id`, `recipients`, `timestamp`, `version`, `correlation_id`, `ttl`, `priority`
- **Body**: `type` (command/query/event/state), `intent` (goal or task description), `payload` (structured per schema), `context` (current plan, channel, rationale)

For a dispatch tool, the relevant subset is: a unique dispatch ID (for tracking), intent (what the user wants), payload (the changeset), and context (assistant notes accumulated during conversation).

### Recommendation

Use the **grouped changeset** pattern (`agents.add`, `agents.update`, `agents.remove`) rather than the flat discriminator pattern. Reasons:

1. Stronger JSON Schema validation -- each group can have its own item schema
2. LLMs handle fixed-structure objects more reliably than polymorphic arrays
3. Clearer separation of concerns in the backend validator
4. The operation type is implicit in the group name, reducing redundancy

---

## 2. Tool Schema Best Practices for LLM Reliability

### Schema Complexity vs. Accuracy

Research consistently shows an inverse relationship between schema complexity and LLM compliance:

- **Prompt-only JSON**: ~73% validity rate
- **JSON Schema + validation + reprompt**: ~94% validity rate
- **JSON Schema + constrained decoding**: ~97-98% validity rate
- **Deeply nested schemas with many required fields**: GPT-4 shows 11.97% invalid response rate

The takeaway: every level of nesting, every required field, and every enum constraint makes the schema harder to satisfy. But removing constraints makes validation weaker. The art is finding the right balance.

### Anthropic-Specific Findings

Anthropic's tool use documentation provides concrete guidance:

1. **Descriptions are the most important factor** in tool performance. Aim for 3-4 sentences minimum per tool description, more for complex tools. The description tells the model *when* and *why* to use a tool.

2. **Avoid recursive schemas** -- flatten hierarchical structures or limit nesting depth.

3. **Use `additionalProperties: false`** for strict validation to prevent the model from inventing fields.

4. **One level of nesting is ideal** -- tools that perform a single action with at most one level of nested parameters work best.

5. **Tool use examples** improved accuracy from 72% to 90% on complex parameter handling in internal testing. Include 1-5 examples per tool showing minimal, partial, and full specification patterns.

6. **Start simple, add complexity as needed** -- deeply nested schemas with many required fields are harder to satisfy.

### Practical Schema Design Rules

**DO:**

- Use descriptive enum values (`"add"`, `"update"`, `"remove"`) rather than numeric codes
- Make fields optional when the task may not have all information
- Provide concrete examples in the tool description showing valid payloads
- Use `description` on every field, not just the top-level tool
- Keep arrays homogeneous (all items same shape)
- Put semantic meaning in field names (`agent_name` not `name`)

**DO NOT:**

- Use recursive schemas (agent containing agents)
- Require deeply nested objects with many required fields
- Use `oneOf`/`anyOf` for discriminated unions in tool schemas (LLMs struggle with these)
- Rely on format constraints like `"format": "uuid"` -- the LLM will often ignore them
- Create schemas wider than ~15-20 fields at any single level

### Token Budget Considerations

From Anthropic's advanced tool use research:

- Tool examples add ~20-50 tokens for simple cases, ~100-200 for complex nested objects
- System prompt overhead adds 50-200 tokens per schema depending on complexity
- For tools with large schemas, consider Anthropic's **Tool Search Tool** (beta) to defer loading until needed
- **Programmatic Tool Calling** (beta) can reduce token consumption by 37% on complex multi-tool workflows by batching tool calls in generated code rather than round-tripping through the model

### Structured Output Guarantees

As of late 2025, both OpenAI and Anthropic offer structured output modes:

- **OpenAI**: `strict: true` in function definitions guarantees schema compliance via constrained decoding
- **Anthropic**: Structured outputs beta (November 2025) compiles JSON schema into a grammar and restricts token generation during inference

These eliminate the need for retry/reprompt loops for schema compliance, but they do not guarantee *semantic* correctness (the values make sense for the domain).

---

## 3. Async Dispatch Patterns

### The Fundamental Pattern: Fire-and-Track

The dispatch tool returns immediately with a handle (job ID, task ID, execution ID) while work proceeds in the background. The caller receives progress updates via a push mechanism (WebSocket, SSE, webhooks) or polls for status.

This is universally adopted across:
- MCP Tasks (2025 spec revision)
- Google A2A Protocol
- OpenAI Assistants API (runs)
- CrewAI async kickoff
- Devin 2.0 parallel instances

### MCP Tasks: The Reference Implementation

The November 2025 MCP specification introduced **Tasks** as first-class primitives, providing the most detailed specification for async tool dispatch:

**Task Lifecycle State Machine:**

```
submitted -> working -> completed
                    -> failed
                    -> cancelled
         -> input_required -> working (after input provided)
                           -> cancelled
         -> rejected
```

**Key design rules:**

1. **Task IDs are receiver-generated**, UUID-grade and unguessable (they gate access to state/results)
2. **Tasks must outlive the connection** -- persist state in a durable store (DB row, job queue), not in-memory tied to a connection
3. **Terminal states are final** -- once a task reaches `completed`, `failed`, or `cancelled`, it must never move again. This prevents "completed -> working" regressions during retries or network races
4. **Delivery options**: Poll via task ID, or register a callback URL for push delivery
5. **Streaming within tasks**: SSE streams can deliver incremental progress, but the stream must close when the task reaches a terminal state

### Google A2A Protocol

Google's Agent-to-Agent protocol (April 2025, v0.3 July 2025) defines a similar task lifecycle for inter-agent dispatch:

- Task is the fundamental unit of work, identified by a unique ID
- States: `submitted -> working -> input-required -> completed/failed/cancelled/rejected`
- Built on HTTP, SSE, JSON-RPC (and gRPC as of v0.3)
- Push notifications via server-initiated HTTP POST to client webhook URL
- Designed for tasks ranging from quick operations to "deep research that may take hours or days"

### CrewAI Async Patterns

CrewAI provides two async dispatch mechanisms:

1. **`kickoff_async()`** -- thread-pool based, runs synchronous crew execution in a background thread
2. **`akickoff()`** -- native async/await throughout the execution chain (recommended for high concurrency)

Lifecycle hooks: `before_kickoff_callbacks`, `after_kickoff_callbacks`, `step_callback` (after each agent iteration), `task_callback` (after each task completion).

### Devin 2.0 Architecture

Devin 2.0 (April 2025) runs agents in isolated cloud VMs with multi-instance parallelism. One Devin agent can dispatch tasks to other Devin agents. The user interface shows real-time progress via streaming terminal/editor/browser views.

### Practical Implementation for Nexor

The dispatch tool should:

1. **Return immediately** with an `execution_id` (UUID) and initial status `"dispatched"`
2. **Persist the dispatch** in a DB row (`workflow_executions` table) before returning
3. **Push progress via WebSocket** on a dedicated channel (e.g., `workforce:{execution_id}`)
4. **Use terminal state semantics** -- once completed/failed/cancelled, the state is final
5. **Include a summary in the tool result** -- not just the ID, but a human-readable confirmation of what was dispatched ("Dispatched 3 agents with 2 deliverables, execution starting")

---

## 4. Mid-Flight Updates

### The Problem

Once a batch dispatch is running (agents are being designed, prompts are being engineered, DAG is executing), the user may want to:
- Cancel the entire run
- Amend agent instructions
- Add/remove deliverables
- Change execution priority or order
- Provide additional context

### Current State of the Art

Mid-flight updates are the **least mature** area in the agentic systems landscape. Most frameworks handle cancellation but not amendment:

**Cancellation is well-supported:**
- MCP Tasks: `cancelled` is a terminal state, tasks can transition from `working -> cancelled`
- Google A2A: `cancelled` is a defined terminal state
- Most frameworks use cancellation tokens (Go contexts, Rust `CancellationToken`, Python `asyncio.Task.cancel()`)

**Amendment is rare:**
- Google ADK (August 2025): No API endpoint exists to cancel in-progress agent tasks, let alone amend them (open GitHub issue)
- PydanticAI (December 2025): Cancellation of running Agent.run operations surfaces errors (open GitHub issue)
- MCP Tasks: The `input_required` state allows pausing for new input, but this is agent-initiated (the task asks for more input), not user-initiated (the user pushes new instructions)

### Patterns That Do Exist

**1. Message Injection / Shared Scratchpad:**
Rather than modifying running tasks, inject new context into a shared state that the executing agents read. The next time an agent checks its context (between tool calls, between steps), it picks up the new instructions.

**2. Checkpoint + Restart:**
Cancel the current run, snapshot progress at the last checkpoint, and restart with amended parameters. This is the Saga-influenced approach -- accept that in-flight work may be lost, but preserve completed work.

**3. Priority Queue Reordering:**
For execution order changes, if the DAG executor processes steps from a priority queue, you can reorder the queue without stopping running steps. New steps will execute in the amended order.

**4. Two-Phase Dispatch:**
Split dispatch into a "design" phase (which produces the plan) and an "execute" phase. Allow amendments during the design phase before execution begins. This maps naturally to Nexor's Designer -> DAG execution pipeline.

### Recommendation for Nexor

Given Nexor's architecture (Designer phase -> DAG execution), the most practical approach is:

1. **During Designer phase**: Allow full amendment via the dispatch tool (re-dispatch with updated changeset, Designer restarts)
2. **During DAG execution**: Support cancellation only. Amendment requires cancel + re-dispatch with the new changeset, carrying forward completed step outputs
3. **Expose a `cancel_dispatch` tool** alongside the dispatch tool
4. **Use the assistant's notes field** as a running context accumulator that gets forwarded to the Designer, providing a natural mechanism for "additional context" without mid-flight mutation

---

## 5. Error Handling and Rollback

### Saga Pattern for Batch Dispatch

The Saga pattern is the established solution for distributed transactions that span multiple services or entities. In the context of a dispatch tool that creates/updates/removes multiple entities:

**Orchestration-based Saga:** A centralized coordinator (the dispatch handler) manages all mutations and compensating transactions:

1. Validate entire changeset before applying any mutation
2. Apply mutations in order, recording each completed step
3. On failure, run compensating transactions in reverse order
4. Report partial results with clear indication of what succeeded and what failed

**Compensating transactions must be idempotent and retryable** -- if a compensation fails, it can be retried safely.

### Checkpoint-Based Recovery

For long-running dispatch operations:

- **Context snapshots**: Capture state at critical decision points (before API calls, after major processing steps)
- **Store as lightweight JSON** with expiration policies matching workflow duration
- **On failure, resume from last snapshot** rather than starting over
- **Record side effects in reversible formats** (e.g., DB savepoints, soft deletes)

### Practical Error Categories

For a dispatch tool, errors fall into three categories:

**1. Validation Errors (pre-execution):**
The changeset is invalid -- malformed schema, referential integrity violations (deliverable assigned to non-existent agent), impossible configurations. These should fail the entire dispatch and return a clear error to the assistant with actionable guidance.

```json
{
  "status": "validation_failed",
  "errors": [
    {
      "field": "deliverables[1].assigned_agent",
      "error": "Agent 'data_analyst' does not exist in the changeset or current roster",
      "suggestion": "Add an agent named 'data_analyst' or assign to an existing agent"
    }
  ]
}
```

**2. Partial Application Errors (during mutation):**
Some entities were created/updated successfully, others failed. The system should:
- Apply what it can (all-or-nothing is too strict for multi-entity operations)
- Report successes and failures clearly
- Allow the assistant to retry the failed portion

```json
{
  "status": "partial_success",
  "applied": {
    "agents_added": ["researcher", "analyst"],
    "deliverables_added": ["report"]
  },
  "failed": {
    "agents_added": [{
      "name": "reviewer",
      "error": "Duplicate agent name in current roster"
    }]
  }
}
```

**3. Execution Errors (during DAG run):**
The dispatch was accepted and mutations applied, but the Designer or DAG execution fails. These are reported via WebSocket events, not in the tool response (since the tool returns immediately).

### Recommendation: Validate-First, Apply-Atomically

For Nexor's dispatch tool:

1. **Full validation pass** before any mutation -- check all referential integrity, naming uniqueness, schema compliance
2. **Database transaction** wrapping all mutations -- if any individual mutation fails, roll back all of them. This is feasible because all mutations target the same PostgreSQL database.
3. **Clear error response** with per-field error details and suggestions
4. **Execution failures are separate** -- the dispatch succeeded (entities were created), but the background work failed. These come through WebSocket events.

This is simpler than the Saga pattern because Nexor's mutations are all in one database (no distributed transaction problem). Use a single PostgreSQL transaction for atomicity.

---

## 6. Existing Framework Patterns

### OpenAI Agents SDK (March 2025)

**Architecture:** Lightweight framework with three primitives: Agents (with instructions + tools), Handoffs (agent-to-agent delegation), and Guardrails (validation).

**Tool Design:**
- Handoffs are represented as tools to the LLM (e.g., `transfer_to_refund_agent`)
- Function definitions use `strict: true` for guaranteed schema compliance
- Provider-agnostic -- works with Responses API, Chat Completions API, and 100+ other LLMs

**Dispatch Pattern:**
- Handoffs transfer full conversation history to the target agent
- No explicit batch dispatch -- single-agent delegation model
- The orchestrator maintains the agent loop, not the individual agents

**Key Takeaway:** OpenAI's approach is agent-as-tool, where dispatching work to another agent looks exactly like calling a function. This keeps the schema simple but limits batch operations.

### LangGraph

**Architecture:** Stateful graph where nodes are agents/functions and edges define data flow. Explicit, reducer-driven state schemas using TypedDict and Annotated types.

**Tool Design:**
- Tools are nodes in the graph, not standalone function definitions
- State flows through the graph as a shared object
- Conditional edges enable dynamic routing based on tool results
- Checkpointing provides persistent memory and safe parallel task execution

**Dispatch Pattern:**
- ReAct-style: if last message contains tool calls, route to tool execution node; otherwise, end
- State accumulates across the full graph execution
- No explicit "dispatch" -- the graph IS the execution plan

**Key Takeaway:** LangGraph's value is in making the execution DAG explicit and observable. Nexor already has this with its DAG executor. LangGraph's state management pattern (typed reducers) is worth studying for how the dispatch tool accumulates context.

### CrewAI

**Architecture:** Role-based teams where agents have specific responsibilities. Crews compose agents with tasks in sequential or hierarchical process flows.

**Tool Design:**
- Tasks define expected output, assigned agent, and dependencies via `context` (references to other tasks)
- Crews are the dispatch unit -- `crew.kickoff(inputs={...})` starts execution
- Tools are assigned per-agent, not per-task

**Dispatch Pattern:**
- `kickoff()` (sync), `kickoff_async()` (thread-pool), `akickoff()` (native async)
- `kickoff_for_each()` for batch processing across multiple input sets
- Callbacks at crew, task, and step granularity
- Async tasks within a crew: `async_execution=True` on a task allows the crew to continue without waiting

**Key Takeaway:** CrewAI's `kickoff_for_each()` pattern is relevant -- dispatching the same crew template across multiple input sets. The callback hierarchy (step -> task -> crew) maps well to Nexor's event system.

### Google A2A Protocol (April 2025)

**Architecture:** Standard protocol for agent-to-agent communication, built on HTTP/SSE/JSON-RPC (gRPC added in v0.3).

**Tool Design:**
- Agent Card: JSON document declaring capabilities, supported content types, and authentication requirements
- Tasks are the fundamental work unit with unique IDs and lifecycle state machines
- Messages within tasks carry Parts (text, file, structured data)

**Dispatch Pattern:**
- Client sends task request, server returns task with ID and initial status
- Progress via SSE streaming or webhook push notifications
- `input_required` state allows interactive back-and-forth during execution
- 150+ supported organizations as of July 2025

**Key Takeaway:** A2A's task lifecycle state machine (submitted -> working -> completed/failed/cancelled) with push notification support is the most mature and well-specified async dispatch pattern available.

### MCP (Model Context Protocol)

**Architecture:** Standardized protocol for connecting LLMs to external tools, databases, and APIs.

**Tool Design:**
- Tools have unique names, descriptions, and JSON Schema input definitions
- `additionalProperties: false` enforced for strict validation
- Structured output via `structuredContent` field with schema validation
- Task primitive (November 2025) for long-running operations

**Dispatch Pattern:**
- Tasks upgrade MCP from synchronous tool calls to call-now-fetch-later
- Task IDs are receiver-generated, UUID-grade, unguessable
- Five lifecycle states: working, input_required, completed, failed, cancelled
- Tasks must outlive the HTTP/SSE connection -- persist in durable storage
- Terminal states are irrevocable

**Key Takeaway:** MCP Tasks is the reference specification for async tool dispatch. Its design rules (durable persistence, irrevocable terminal states, UUID task IDs) should be adopted directly.

### Anthropic Advanced Tool Use (November 2025)

**Architecture:** Three features addressing tool use bottlenecks: Tool Search Tool (context bloat), Programmatic Tool Calling (latency/tokens), Tool Use Examples (accuracy).

**Relevant Patterns:**
- **Tool Search Tool**: For systems with many tools, mark tools with `defer_loading: true`. Claude finds and loads only needed tools. Relevant if Nexor's assistant has many tools beyond dispatch.
- **Programmatic Tool Calling**: Claude writes code that calls tools programmatically, eliminating round-trips. 37% token reduction on complex workflows. Relevant if the dispatch involves multiple preparatory tool calls.
- **Tool Use Examples**: 1-5 examples per tool showing minimal/partial/full patterns. 72% -> 90% accuracy on complex parameters. Critical for the dispatch tool's complex schema.

---

## 7. Recommendations for Nexor

### 7.1 Background Agent Mutation Tools

> **Tier placement:** These schemas are for the Background Agent — a session service layer that configures workflow steps. The assistant dispatches plain English via `dispatch({ instruction: "..." })`. The Background Agent loads all current state (assistants notes, users notes, agent roster, deliverables, execution order, context) and uses mutation tools to configure the step. It does NOT trigger the Designer or DAG execution — those are a separate pipeline triggered when the user runs the workflow. The grouped changeset pattern documented below represents the internal schema design for these mutation tools.

**Use a grouped changeset with flat entity schemas:**

```json
{
  "name": "dispatch_workforce",
  "description": "Dispatches a workforce of AI agents to execute a mission. Accepts a batch changeset that creates, updates, or removes agents and deliverables in a single atomic operation. The workforce is designed by a specialized Designer agent that engineers system prompts, tool assignments, and inter-agent data routing based on your high-level specifications. Returns immediately with an execution ID; progress is delivered via WebSocket events. Use this tool when the user wants to create or modify a team of agents to accomplish a goal.",
  "input_schema": {
    "type": "object",
    "properties": {
      "agents": {
        "type": "object",
        "description": "Agent mutations. Each agent is a team member with a name and role description.",
        "properties": {
          "add": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "name": { "type": "string", "description": "Unique agent name (snake_case, e.g. 'lead_researcher')" },
                "role": { "type": "string", "description": "What this agent does and what expertise it brings (2-4 sentences)" }
              },
              "required": ["name", "role"]
            }
          },
          "update": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "name": { "type": "string", "description": "Name of existing agent to update" },
                "role": { "type": "string", "description": "Updated role description" }
              },
              "required": ["name"]
            }
          },
          "remove": {
            "type": "array",
            "items": { "type": "string", "description": "Name of agent to remove" }
          }
        }
      },
      "deliverables": {
        "type": "object",
        "description": "Deliverable mutations. Each deliverable is an output artifact assigned to an agent.",
        "properties": {
          "add": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "name": { "type": "string", "description": "Deliverable name (snake_case)" },
                "description": { "type": "string", "description": "What this deliverable contains and its purpose (2-4 sentences)" },
                "assigned_agent": { "type": "string", "description": "Name of the agent responsible for producing this deliverable" }
              },
              "required": ["name", "description", "assigned_agent"]
            }
          },
          "update": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "name": { "type": "string", "description": "Name of existing deliverable to update" },
                "description": { "type": "string" },
                "assigned_agent": { "type": "string" }
              },
              "required": ["name"]
            }
          },
          "remove": {
            "type": "array",
            "items": { "type": "string", "description": "Name of deliverable to remove" }
          }
        }
      },
      "notes": {
        "type": "string",
        "description": "Assistant's accumulated context notes for the Designer. Include user preferences, constraints, domain-specific requirements, and any clarifications gathered during conversation. The Designer uses these to engineer detailed system prompts and tool assignments."
      },
      "execution_order": {
        "type": "array",
        "description": "Ordered list of agent names defining execution sequence. Agents listed at the same position execute in parallel. Use nested arrays for parallel groups: [['researcher'], ['analyst', 'writer']] means researcher runs first, then analyst and writer run in parallel.",
        "items": {
          "oneOf": [
            { "type": "string" },
            { "type": "array", "items": { "type": "string" } }
          ]
        }
      }
    },
    "required": ["agents", "notes"]
  }
}
```

**Key design decisions:**

1. **Grouped by entity type** (agents, deliverables) rather than flat discriminated mutations -- stronger validation, easier for LLMs
2. **One level of nesting** -- the deepest required path is `agents.add[0].role`, which is manageable
3. **`remove` uses plain strings** (names) rather than objects -- simpler schema, easier for the LLM
4. **`execution_order` uses string-or-array-of-strings** to express sequential and parallel groups without deep nesting
5. **`notes` is a simple string** -- don't over-structure the context; the Designer will parse it
6. **`deliverables` and `execution_order` are optional** -- not every dispatch needs them (a simple agent team may have no formal deliverables)

### 7.2 Tool Description and Examples

Provide 2-3 examples in the tool description showing minimal, moderate, and full payloads:

**Minimal (single agent, no deliverables):**
```json
{
  "agents": { "add": [{ "name": "researcher", "role": "Searches for and synthesizes information on the given topic" }] },
  "notes": "User wants a quick summary of recent ML papers on attention mechanisms"
}
```

**Moderate (team with deliverables):**
```json
{
  "agents": {
    "add": [
      { "name": "analyst", "role": "Analyzes market data and identifies trends" },
      { "name": "writer", "role": "Writes clear, executive-level reports from analytical findings" }
    ]
  },
  "deliverables": {
    "add": [
      { "name": "market_report", "description": "Executive summary of market trends with data visualizations", "assigned_agent": "writer" }
    ]
  },
  "notes": "Focus on Q4 2025 SaaS market. User is a VP of Strategy, prefers concise bullet points.",
  "execution_order": ["analyst", "writer"]
}
```

**Update (modifying existing workforce):**
```json
{
  "agents": {
    "update": [{ "name": "analyst", "role": "Analyzes market data with focus on competitive positioning" }],
    "add": [{ "name": "fact_checker", "role": "Verifies all statistical claims and data citations" }]
  },
  "deliverables": {
    "update": [{ "name": "market_report", "assigned_agent": "writer" }]
  },
  "notes": "User wants more rigorous fact-checking. Add verification step before final report.",
  "execution_order": ["analyst", "fact_checker", "writer"]
}
```

### 7.3 Async Dispatch Flow

Adopt the MCP Tasks / A2A lifecycle pattern:

```
Tool called
  -> Validate changeset (sync, in tool handler)
  -> Begin DB transaction
     -> Apply all mutations atomically
     -> Create workflow_execution record (status: "dispatched")
  -> Commit transaction
  -> Spawn background task (Designer -> DAG execution)
  -> Return tool result immediately:
     {
       "status": "dispatched",
       "execution_id": "uuid",
       "summary": "Dispatched 3 agents (analyst, writer, fact_checker) with 1 deliverable (market_report)",
       "channel": "workforce:{execution_id}"
     }

Background:
  -> Designer phase (status: "designing")
     -> WebSocket: DesignerProgress events
  -> DAG execution (status: "executing")
     -> WebSocket: AgentProgress events per agent
  -> Completion (status: "completed" | "failed")
     -> WebSocket: ExecutionCompleted event with outputs
```

**State machine (irrevocable terminal states):**
```
dispatched -> designing -> executing -> completed
                                     -> failed
          -> failed (validation passed but Designer failed to start)
          -> cancelled (user cancelled before completion)
```

### 7.4 Error Handling Strategy

**Layer 1 -- Schema Validation (in tool handler, sync):**
- JSON Schema compliance (handled by the LLM provider's structured output mode)
- Referential integrity: deliverables reference existing or newly-added agents
- Naming uniqueness: no duplicate agent or deliverable names
- Return validation errors with per-field detail and actionable suggestions

**Layer 2 -- Mutation Application (in DB transaction):**
- Wrap all mutations in a single PostgreSQL transaction
- On any failure, roll back everything and return the error
- This is simpler than Saga because everything is in one database

**Layer 3 -- Execution Errors (async, via WebSocket):**
- Designer failure: push `DesignerFailed` event with error details
- Agent failure: push `AgentFailed` event for the specific agent; other agents may continue
- DAG failure: push `ExecutionFailed` event with partial results

**Error response format:**
```json
{
  "status": "validation_failed",
  "errors": [
    {
      "path": "deliverables.add[0].assigned_agent",
      "value": "data_analyst",
      "error": "No agent named 'data_analyst' exists in the current roster or in agents.add",
      "suggestion": "Did you mean 'analyst'? Or add an agent named 'data_analyst' to agents.add"
    }
  ],
  "valid_portions": {
    "agents": { "add": ["analyst", "writer"], "update": [], "remove": [] },
    "deliverables": { "add": [], "update": [], "remove": [] }
  }
}
```

The `valid_portions` field shows the assistant what *would* have worked, enabling it to fix only the broken part and re-dispatch.

### 7.5 Cancellation Tool

Provide a companion `cancel_dispatch` tool:

```json
{
  "name": "cancel_dispatch",
  "description": "Cancels a running workforce dispatch. If the Designer phase is still running, cancels immediately. If agents are already executing, running agents complete their current step but no new steps are started.",
  "input_schema": {
    "type": "object",
    "properties": {
      "execution_id": { "type": "string", "description": "The execution ID returned by dispatch" },
      "reason": { "type": "string", "description": "Why the dispatch is being cancelled (logged for audit)" }
    },
    "required": ["execution_id"]
  }
}
```

### 7.6 Idempotency and Retry Safety

- The dispatch tool should be **idempotent by name** -- if the same agent name is added twice (e.g., due to retry), the second call is a no-op or merges cleanly
- Include a `dispatch_id` (client-generated) in the schema for deduplication: if the same `dispatch_id` arrives twice, return the existing execution rather than creating a duplicate
- This prevents double-dispatch when the LLM retries a tool call due to timeout or ambiguous response

### 7.7 Designer Input Forwarding

The `notes` field bridges the gap between conversational context and structured execution:

- The assistant accumulates context during conversation (user preferences, domain constraints, clarifications)
- Notes are forwarded verbatim to the Designer agent as part of its input context
- The Designer uses notes alongside the structured agent/deliverable definitions to engineer detailed prompts
- This separation means the dispatch tool schema stays simple (notes is a string) while the Designer handles complexity

### 7.8 Future Considerations

**Streaming Changeset Application:**
For very large changesets (10+ agents, 20+ deliverables), consider streaming the mutation application and returning partial progress. This is overkill for current scale but worth designing for.

**Template Dispatch:**
Allow dispatching from saved workforce templates with parameter overrides, reducing the LLM's schema-filling burden for common patterns.

**Composite Dispatch:**
Multiple workforce dispatches chained in a collection DAG, where outputs from one workforce feed into the next. The dispatch tool could accept an optional `depends_on` field referencing a previous execution ID.

---

## Sources

### Batch Mutation and API Design
- [The 2026 Guide to Agentic Workflow Architectures](https://www.stack-ai.com/blog/the-2026-guide-to-agentic-workflow-architectures)
- [Google's Eight Essential Multi-Agent Design Patterns](https://www.infoq.com/news/2026/01/multi-agent-design-patterns/)
- [CQRS Pattern - Azure Architecture Center](https://learn.microsoft.com/en-us/azure/architecture/patterns/cqrs)

### Tool Schema and Structured Output
- [The Guide to Structured Outputs and Function Calling with LLMs](https://agenta.ai/blog/the-guide-to-structured-outputs-and-function-calling-with-llms)
- [How JSON Schema Works for LLM Tools & Structured Outputs](https://blog.promptlayer.com/how-json-schema-works-for-structured-outputs-and-tool-integration/)
- [Structured Output AI Reliability: JSON Schema & Function Calling Guide 2025](https://www.cognitivetoday.com/2025/10/structured-output-ai-reliability/)
- [How to Implement Tool Use - Claude API Docs](https://platform.claude.com/docs/en/agents-and-tools/tool-use/implement-tool-use)
- [Introducing Advanced Tool Use - Anthropic Engineering](https://www.anthropic.com/engineering/advanced-tool-use)
- [Programmatic Tool Calling - Claude API Docs](https://platform.claude.com/docs/en/agents-and-tools/tool-use/programmatic-tool-calling)
- [OpenAI Function Calling](https://platform.openai.com/docs/guides/function-calling)
- [Introducing Structured Outputs in the API - OpenAI](https://openai.com/index/introducing-structured-outputs-in-the-api/)

### Async Dispatch and Task Lifecycle
- [MCP Tools Specification (2025-06-18)](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)
- [Google A2A Protocol Specification](https://a2a-protocol.org/latest/specification/)
- [Announcing the Agent2Agent Protocol (A2A)](https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/)
- [Agent2Agent Protocol v0.3 Upgrade](https://cloud.google.com/blog/products/ai-machine-learning/agent2agent-protocol-is-getting-an-upgrade)
- [CrewAI vs LangGraph vs AutoGen Comparison - DataCamp](https://www.datacamp.com/tutorial/crewai-vs-langgraph-vs-autogen)
- [Top AI Agent Frameworks 2025 - Codecademy](https://www.codecademy.com/article/top-ai-agent-frameworks-in-2025)

### Error Handling and Rollback
- [Saga Design Pattern - Azure Architecture Center](https://learn.microsoft.com/en-us/azure/architecture/patterns/saga)
- [Compensating Transaction Pattern - Azure Architecture Center](https://learn.microsoft.com/en-us/azure/architecture/patterns/compensating-transaction)
- [Saga Pattern in Microservices](https://microservices.io/patterns/data/saga.html)
- [Compensation Transaction Patterns - Orkes](https://orkes.io/blog/compensation-transaction-patterns/)

### Framework Architectures
- [OpenAI New Tools for Building Agents](https://openai.com/index/new-tools-for-building-agents/)
- [Agent Orchestration 2026: LangGraph, CrewAI & AutoGen Guide](https://iterathon.tech/blog/ai-agent-orchestration-frameworks-2026)
- [Devin 2.0 Technical Design Analysis](https://medium.com/@takafumi.endo/agent-native-development-a-deep-dive-into-devin-2-0s-technical-design-3451587d23c0)
- [Devin 2025 Performance Review](https://cognition.ai/blog/devin-annual-performance-review-2025)
- [SWE-agent Architecture Documentation](https://swe-agent.com/latest/background/architecture/)
- [MCP Architecture, Components & Workflow](https://www.kubiya.ai/blog/model-context-protocol-mcp-architecture-components-and-workflow)
- [MCP Apps Extension - November 2025](http://blog.modelcontextprotocol.io/posts/2025-11-21-mcp-apps/)
