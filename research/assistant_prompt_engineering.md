# Assistant Prompt Engineering: Dispatcher/Orchestrator Patterns

Research compiled February 2026. Focused on modern patterns for building a conversational AI assistant that acts as a lightweight dispatcher — understanding user intent, dispatching plain English instructions to a background agent, and maintaining awareness of asynchronous execution.

---

## Table of Contents

1. [Orchestrator/Dispatcher Prompt Patterns](#1-orchestratordispatcher-prompt-patterns)
2. [Capabilities Indexing](#2-capabilities-indexing)
3. [Dispatch Architecture: Service Layers](#3-dispatch-architecture-service-layers)
4. [Context Management for Stateful Assistants](#4-context-management-for-stateful-assistants)
5. [Reactive/Proactive Assistant Patterns](#5-reactiveproactive-assistant-patterns)
6. [Recommendations for Nexor](#6-recommendations-for-nexor)

---

## 1. Orchestrator/Dispatcher Prompt Patterns

### 1.1 The Dispatcher Architecture

Modern multi-agent systems converge on a common pattern: a central orchestrator agent that **never does the work itself** but instead decomposes user intent and routes to specialized workers. This is distinct from monolithic agents that try to do everything.

**Key principle**: The orchestrator is a lightweight coordinator. Its job is to understand, plan, and dispatch. It should be fast, always responsive, and never blocked by long-running work.

#### How leading systems implement this:

| System | Orchestrator Role | Key Pattern |
|--------|------------------|-------------|
| **Claude Code** | Lead agent coordinates teammates, assigns tasks via `TeammateTool` | Orchestrator-worker with shared task board |
| **Anthropic Research** | Lead agent spawns parallel subagents for breadth-first exploration | Each subagent gets: objective, output format, tool guidance, task boundaries |
| **Devin** | Planner decomposes into phases, dispatches to parallel Devin instances | "Architectural Brain" plans before any execution begins |
| **OpenAI Agents SDK** | Central agent uses "agent-as-tool" pattern or handoff pattern | Structured outputs classify task category, then route by category |
| **Google ADK** | Dispatcher agent routes based on sub-agent `description` fields | Description field is "API documentation for the LLM" |
| **CrewAI** | Role-based crew coordination with tasks and SOPs | Emphasizes role assignment and collaboration protocols |
| **LangGraph** | Graph-based state machine with conditional routing | Nodes, edges, conditional branches for traceable flows |

### 1.2 What Makes Orchestrators Good at Decomposition

**From Anthropic's research system**: Early iterations failed because instructions to subagents were too vague. Subagents duplicated work or missed requirements. The fix was explicit instruction formatting:

- Simple fact-finding: 1 agent, 3-10 tool calls
- Direct comparisons: 2-4 subagents, 10-15 calls each
- Complex research: 10+ subagents with clearly divided responsibilities

Each subagent dispatch must include:
1. **Objective** -- what specifically to accomplish
2. **Output format** -- how to structure the result
3. **Tool/source guidance** -- what tools to use and how
4. **Task boundaries** -- what NOT to do (prevents duplication)

**From Devin**: The orchestrator should "clearly outline your preferred approach from the outset." Providing the agent with overall architecture and logic upfront boosts success rates and reduces review time. For complex tasks, structure requests into phases with explicit handoff points.

**From OpenAI Agents SDK**: Use structured outputs to classify intent into categories first, then pick the next agent based on category. This two-step pattern (classify, then dispatch) is more reliable than trying to produce a full dispatch plan in one step.

### 1.3 Orchestrator Prompt Structure

Based on Anthropic's context engineering guidance, orchestrator prompts should be organized into distinct sections:

```
<identity>
You are a dispatcher for the Nexor orchestration platform.
You understand user intent and compose structured dispatch payloads.
You NEVER execute work directly.
</identity>

<capabilities>
[Compact capabilities index -- see Section 2]
</capabilities>

<current_state>
[Condensed status of running/completed tasks -- see Section 4]
</current_state>

<instructions>
[Behavioral rules for decomposition and dispatch]
</instructions>

<tool_guidance>
[How to use the dispatch tool, what payloads look like]
</tool_guidance>

<examples>
[2-3 canonical dispatch examples -- see Section 3]
</examples>
```

**The "right altitude" principle** (Anthropic): Avoid two extremes:
- **Too rigid**: Hardcoding complex, brittle logic that tries to cover every edge case
- **Too vague**: High-level guidance that fails to give concrete signals

Start minimal with the best available model, then iteratively add instructions based on observed failure modes.

### 1.4 Agent-as-Tool vs. Handoff

Two dispatch patterns from OpenAI Agents SDK:

**Agent-as-Tool** (recommended for Nexor's architecture):
- The orchestrator calls agents as if they were tools
- Sub-agents do not take over the conversation
- The orchestrator invokes them for specific subtasks and incorporates results
- Keeps a single thread of control -- the orchestrator manages everything

**Handoff**:
- The orchestrator hands off the full conversation to a specialist agent
- The specialist takes over and responds directly to the user
- Used when the specialist needs direct user interaction

For Nexor, the agent-as-tool pattern maps cleanly: the assistant dispatches to the background execution layer and remains available to the user while work proceeds asynchronously.

---

## 2. Capabilities Indexing

### 2.1 The Problem

The assistant needs to know what it can do without loading full documentation into the context window. Full tool/workflow documentation would consume thousands of tokens and dilute attention.

### 2.2 Patterns for Compact Capability Summaries

#### Tiered Description Pattern (Google ADK)

Google ADK's key insight: the `description` field of sub-agents is "API documentation for the LLM." These descriptions should be:
- Concise (1-3 sentences)
- Action-oriented (what the capability DOES, not how it works)
- Boundary-clear (what it does NOT do)

```
capability: workforce
summary: Create a team of AI agents with assigned deliverables. Agents are designed by a Designer agent, then execute sequentially with tool access.
handles: team composition, document generation, research, multi-step analysis
does_not_handle: real-time chat, single-shot questions, code execution
```

#### Hierarchical Capability Index

Organize capabilities into a two-level hierarchy:

**Level 1 -- Category** (always in context):
```
- TEAM_WORK: Create and manage agent teams for complex deliverables
- SINGLE_AGENT: Run a single agent for focused tasks
- WORKFLOW: Execute multi-step workflows with data flow
- COLLECTION: Run a DAG of entire workflows
- PROTOCOL: Apply reusable workflow templates
```

**Level 2 -- Detail** (loaded on demand when category is selected):
```
TEAM_WORK:
  - workforce: Multi-agent team with deliverables
  - room: Multi-agent conversation/debate
  - sub_workflow: Nested workflow execution
```

This mirrors the document summary index pattern from LlamaIndex: store summaries at the top level, retrieve detail only when the LLM determines relevance.

#### Tool Registry Pattern

Maintain a structured registry that the assistant queries dynamically:

```json
{
  "capabilities": [
    {
      "id": "workforce",
      "name": "Workforce Archetype",
      "summary": "Multi-agent team with designer, agents, and deliverables",
      "triggers": ["team", "agents", "deliverables", "collaborate", "research team"],
      "required_inputs": ["mission_brief"],
      "optional_inputs": ["agent_hints", "deliverable_specs", "constraints"],
      "output_type": "agents[] + deliverables[]"
    }
  ]
}
```

The `triggers` field helps the assistant match user intent to capabilities without needing full documentation.

### 2.3 Dynamic vs. Static Capabilities

**Static**: Baked into the system prompt. Best for core capabilities that rarely change. Keep under 500 tokens total.

**Dynamic**: Loaded via tool call when the assistant needs detail. The assistant calls a `get_capability_detail(id)` tool to retrieve full schema, examples, and constraints for a specific capability.

This follows the just-in-time context loading pattern from Anthropic's guidance: maintain lightweight identifiers and retrieve dynamically at runtime.

### 2.4 MemTool Pattern

From recent research on tool-rich agents: when an agent has many tools, context window pressure grows. The MemTool approach uses agentic removal/addition policies -- removing tool descriptions from context when not needed, adding them back when relevant. This achieves 90%+ tool-removal efficiency without degrading performance.

For Nexor: the assistant's system prompt carries only the capability index (Level 1). When the user's intent matches a category, the assistant loads the relevant tool schemas on demand.

---

## 3. Dispatch Architecture: Service Layers

### 3.1 The Dispatch Model

A critical architectural decision: the assistant dispatches in **plain English**, not structured JSON. Background service layers receive the instruction and do the work asynchronously.

**The Assistant:** Conversational. Sends `dispatch({ instruction: "..." })`. Returns immediately. Continues conversation.

**Background Agent (first service layer):** A session service layer that configures workflow steps. Loads all current state (assistants notes, users notes, agent roster, deliverables, execution order, context) and calls mutation tools: `add_agent()`, `remove_agent()`, `update_deliverable()`, `set_execution_order()`, etc. Does NOT trigger execution — it only configures.

**The Execution Pipeline (separate):** When the user runs the workflow, the existing pipeline handles it: Designer (sub-workflow) → DAG Execution. This is decoupled from the dispatch service layer.

**Future service layers:** The dispatch tool is kept simple (`{ instruction: "..." }`) because more service layers will be added for different kinds of background work.

### 3.2 Schema-Constrained Generation

Modern LLMs support structured outputs natively:

- **OpenAI**: `strict: true` in function definitions guarantees 100% schema compliance
- **Anthropic**: Tool use with JSON schema definitions
- **Constrained decoding**: Logits modified in real time to remove tokens that would violate schema

**Key insight**: Schema-constrained generation via tool calling is strictly more reliable than asking the model to produce JSON in free-form text. Always use tool definitions rather than prompt-based JSON generation.

### 3.3 The Assistant's Dispatch Tool

The assistant's only dispatch tool is deliberately minimal — plain English in, execution ID out. The dispatch tool is kept simple because more service layers will be added in the future, each handling different kinds of background work through the same interface.

```json
{
  "name": "dispatch",
  "description": "Sends a plain English instruction to a background service layer for asynchronous execution. The service layer loads all current state and decides what mutations to make. Returns immediately with an execution ID; progress is delivered via WebSocket events.",
  "parameters": {
    "type": "object",
    "required": ["instruction"],
    "properties": {
      "instruction": {
        "type": "string",
        "description": "Plain English description of what to do."
      }
    }
  }
}
```

The simplicity is intentional. The assistant describes what it wants in natural language. The background service layer (currently: the Background Agent that configures workflow steps) loads all state and decides what structured mutations to make. More service layers will be added in the future — the same simple dispatch interface routes to the appropriate one.

#### Few-Shot Examples in System Prompt

Provide 2-3 canonical examples that demonstrate the dispatch pattern. From Anthropic's guidance: "examples are the 'pictures' worth a thousand words."

**Example 1 -- Simple task:**

```
User: "Write me a technical blog post about Rust's ownership model"

dispatch({
  instruction: "Create a single tech writer agent specializing in systems programming. Add a deliverable for a 2000-3000 word blog post about Rust ownership with practical code examples. User wants practical examples, not just theory. Target audience is intermediate developers."
})
```

**Example 2 -- Multi-agent team:**

```
User: "Research competitor pricing strategies and create a comparison matrix"

dispatch({
  instruction: "Build a research team with a market research analyst and a business analyst. The researcher should gather competitive intelligence data for our top 5 SaaS competitors across all pricing tiers. The analyst should synthesize the research into an executive-level comparison matrix with strategic recommendations. User is a VP of Strategy, prefers concise bullet points over prose. Execute researcher first, then analyst."
})
```

**Example 3 -- Modifying an existing configuration:**

```
User: "Actually, add a fact-checker before the analyst writes the final report"

dispatch({
  instruction: "Add a fact-checker agent that verifies statistical claims and data citations against primary sources. Insert them between the researcher and analyst in the execution order."
})
```

### 3.4 The Background Agent's Mutation Tools

The Background Agent (session service layer) uses individual, focused tools -- not a single complex payload. Each tool has a flat schema that's easy for the LLM to populate:

- `add_agent(name, role)` -- add agent to roster
- `update_agent(name, role)` -- update existing agent
- `remove_agent(name)` -- remove agent from roster
- `add_deliverable(name, description, assigned_agent)` -- add deliverable
- `update_deliverable(name, description?, assigned_agent?)` -- update deliverable
- `remove_deliverable(name)` -- remove deliverable
- `update_assistants_notes(notes)` -- update accumulated context
- `set_execution_order(order)` -- set agent execution sequence

This per-tool approach follows Anthropic's guidance: "One level of nesting is ideal -- tools that perform a single action with at most one level of nested parameters work best." Each tool validates independently, errors are scoped to the specific mutation, and the Background Agent can reason about each change individually.

The Background Agent handles batch operations naturally through its tool-use loop. A single dispatch instruction like "build a 5-person research team" results in multiple tool calls (5x `add_agent`, deliverables, execution order). This is superior to the assistant composing a single complex payload because:
1. The Background Agent has full state context -- it knows what already exists
2. Each mutation validates independently with clear error messages
3. The Background Agent can adapt mid-execution (e.g., if `add_agent` fails, it adjusts)
4. The tool schemas stay flat and easy for the LLM to populate reliably

#### Schema Design Rules for Background Agent Tools

From OpenAI's structured outputs documentation:
- Maximum 100 object properties total, up to 5 levels of nesting
- All fields must be marked `required` (use nullable types for optional values)
- Use `enum` for constrained string values -- prevents hallucinated values

From Anthropic's tool use research:
- **Descriptions are the most important factor** in tool performance. Aim for 3-4 sentences minimum.
- **Tool use examples** improved accuracy from 72% to 90% on complex parameter handling.
- **Start simple, add complexity as needed.**

---

## 4. Context Management for Stateful Assistants

### 4.1 The Core Challenge

The assistant needs awareness of background state (running tasks, agent rosters, execution status) without overwhelming its context window. A naive approach -- injecting all state into every prompt -- wastes tokens and dilutes attention.

### 4.2 Four Context Engineering Strategies

From LangChain/Anthropic's framework:

#### Strategy 1: Write (Persistent External Memory)

The assistant writes notes to an external scratchpad that persists across context resets.

- **Scratchpad files** (e.g., NOTES.md): Agent writes structured notes about decisions, progress, and open questions
- **State store**: Key-value store for tracking dispatched tasks, their status, and results
- **Session memory**: Checkpointed state that survives context compaction

For Nexor: The assistant should maintain a structured state document:
```
## Active Dispatches
- [dispatch-123] workforce "Competitor Analysis" -- RUNNING (2/3 agents complete)
- [dispatch-456] single "Blog Post" -- COMPLETED, awaiting review

## Recent Results
- [dispatch-456] Produced: rust_ownership_blog_post.md (2847 words)
```

#### Strategy 2: Select (Just-in-Time Retrieval)

Rather than pre-loading all state, the assistant retrieves relevant state on demand:

- **Status tool**: `get_dispatch_status(id)` returns current state of a specific dispatch
- **Active dispatches tool**: `list_active_dispatches()` returns summary of running work
- **Result retrieval**: `get_dispatch_result(id)` retrieves output from completed work

This follows the progressive disclosure pattern: the assistant starts with a lightweight summary and drills down only when needed.

#### Strategy 3: Compress (Summarization)

For long-running conversations, compress older context:

- **Hierarchical summarization**: Recent exchanges stay verbatim; older content gets compressed into summary form
- **Auto-compaction**: Claude Code triggers summarization when context reaches 95% capacity, preserving: architectural decisions, unresolved issues, implementation details. Discarding: redundant tool outputs, repeated messages
- **Observation masking** (JetBrains research, NeurIPS 2025): Mask verbose tool outputs while preserving action/reasoning history. Since tool observations dominate token usage, this achieves ~50% cost reduction without degrading performance. Simpler and equally effective as full LLM summarization.

For Nexor: When the conversation grows long, compress dispatch history into summaries:
```
Before: [Full dispatch payload + full execution log + full output]
After: "Dispatched 3-agent workforce for competitor analysis. Completed successfully. Key output: comparison_matrix.md with 5 competitors across 4 pricing tiers."
```

#### Strategy 4: Isolate (Scoped Context per Step)

Different conversation phases need different context:

- **Planning phase**: Capability index + user history + current state
- **Dispatch phase**: Dispatch tool schema + few-shot examples + task details
- **Review phase**: Execution results + user feedback history

In LangGraph, this is implemented by choosing which fields of the state schema to expose to the LLM at each step.

For Nexor: The assistant's system prompt should dynamically include only the relevant context block for the current interaction mode.

### 4.3 Status Condensation Patterns

**Traffic light summary** for background tasks:
```
RUNNING: "Competitor Analysis" (3 agents, 67% complete)
DONE: "Blog Post" (ready for review)
FAILED: "Data Pipeline" (agent_2 error: rate limited)
```

**Delta updates** -- only report what changed since last user interaction:
```
Since your last message:
- "Competitor Analysis" agent_2 (market_researcher) completed
- "Competitor Analysis" agent_3 (analyst) started
```

**Progressive detail** -- summary first, detail on request:
```
Assistant: Your competitor analysis is 67% complete. Want details?
User: Yes
Assistant: [loads full execution log via tool call]
```

### 4.4 Anthropic's Practical Guidance

From Anthropic's context engineering blog:

> "The objective is finding the smallest set of high-signal tokens that maximize the likelihood of some desired outcome."

Three techniques for long-horizon tasks:

1. **Compaction**: Summarize approaching limits. Maximize recall first, then improve precision by eliminating superfluous content. Lightest-touch: clear tool result caches rather than full summarization.

2. **Structured note-taking**: Agent maintains persistent external notes (NOTES.md). Enables tracking across complex, multi-step tasks. Claude's Pokemon agent maintained precise tallies across thousands of game steps using this pattern.

3. **Sub-agent architectures**: Each sub-agent explores extensively but returns condensed summaries (1,000-2,000 tokens). Lead agent synthesizes results. Achieves clear separation of concerns.

---

## 5. Reactive/Proactive Assistant Patterns

### 5.1 Reactive Pattern (Baseline)

The standard request-response cycle: user sends a message, assistant processes and responds. For Nexor, this means:

1. User: "Create a team to research X"
2. Assistant: Composes dispatch, sends to execution layer
3. User: "What's the status?"
4. Assistant: Queries execution layer, reports status

The problem: the user must poll for updates. The assistant is passive between user messages.

### 5.2 Proactive Pattern (Push Updates)

The assistant can push messages to the user without being prompted, typically when:
- A background task completes
- A task fails and needs user intervention
- An intermediate result is available for review
- A long-running task reaches a milestone

#### Architecture for Proactive Push

```
Execution Layer
  |
  v
Event Bus (WebSocket topics)
  |
  v
Assistant State Manager (subscribes to dispatch events)
  |
  v
Client (receives push notifications via WebSocket)
```

**Key insight from AG-UI protocol**: Define standardized event types that the execution layer emits during execution. The assistant subscribes to events for dispatches it created and translates them into user-facing messages.

Event types for Nexor:
- `dispatch.started` -- execution began
- `dispatch.agent.started` -- specific agent began working
- `dispatch.agent.completed` -- specific agent finished
- `dispatch.completed` -- all work finished, results available
- `dispatch.failed` -- execution failed, needs attention
- `dispatch.deliverable.ready` -- a specific deliverable is ready for review

#### The Autonomy Loop Pattern

From proactive agent research: the assistant wraps an event-driven loop that periodically wakes, collects context, and decides whether to notify the user.

```
loop {
  event = await event_bus.next()
  if should_notify(event) {
    message = format_notification(event)
    push_to_user(message)
  }
}
```

The `should_notify` function is critical -- not every event deserves a notification. Criteria:
- **Completion events**: Always notify
- **Failure events**: Always notify
- **Progress events**: Only if > N minutes since last update
- **Milestone events**: Notify at 25%, 50%, 75%, 100%

#### Ambient Agents

A recent pattern (2025-2026): ambient agents are always-on, context-aware systems that monitor enterprise events and take action without prompting. They maintain memory over time and execute or escalate based on policy.

For Nexor, the assistant could function as an ambient agent that:
- Monitors execution events passively
- Surfaces relevant updates to the user at appropriate moments
- Suggests next steps when previous work completes

### 5.3 Bidirectional Communication

From AG-UI and WebSocket research: modern agent-user interfaces use bidirectional event-driven pipelines. The assistant both receives user messages AND pushes events to the client.

Nexor already has WebSocket infrastructure for this (topic subscriptions, event broadcast). The assistant layer would:
1. Subscribe to relevant execution topics when dispatching work
2. Receive events as work progresses
3. Push formatted updates to the user's WebSocket connection

### 5.4 Design Considerations

From CHI 2025 research on proactive programming assistants:
- Increasing suggestion frequency can **negatively** impact user experience
- Users prefer control over notification timing
- Proactive suggestions should be dismissible and non-blocking
- Productivity gains from proactivity can be offset by distraction costs

Recommendation: Allow users to configure notification granularity:
- **Minimal**: Only completion/failure events
- **Normal**: Completion + milestone events
- **Verbose**: All events streamed in real time

---

## 6. Recommendations for Nexor

### 6.1 Assistant System Prompt Architecture

Use a layered context structure:

```
[Layer 1: Identity] ~100 tokens
  - Role: Dispatcher for Nexor orchestration platform
  - Behavioral rules: Never execute directly, always compose and dispatch
  - Personality: Concise, clarifying, action-oriented

[Layer 2: Capabilities Index] ~300 tokens
  - Static, compact summaries of each capability category
  - Trigger words for intent matching
  - "Call get_capability_detail(id) for full schemas"

[Layer 3: Current State] ~200 tokens (dynamic)
  - Active dispatches with traffic-light status
  - Delta since last interaction
  - Injected by the system, not maintained by the assistant

[Layer 4: Instructions] ~400 tokens
  - Decomposition strategy: classify intent first, then compose dispatch
  - When to ask for clarification vs. proceed with defaults
  - How to handle ambiguous requests
  - Error handling guidance

[Layer 5: Tool Descriptions] ~200 tokens
  - dispatch tool (plain English instruction to background service layer)
  - cancel_dispatch tool
  - get_dispatch_status tool
  - list_active_dispatches tool

[Layer 6: Examples] ~500 tokens
  - 2-3 canonical dispatch examples covering:
    1. Simple single-agent task
    2. Multi-agent team with deliverables
    3. Clarification dialog before dispatch
```

Total: ~1,800 tokens for the system prompt. Leaves maximum room for conversation history and tool results.

### 6.2 Dispatch Tool Design

The assistant has one dispatch tool — a plain English instruction:

```
dispatch:
  instruction: string  (required)
    "Build me a 3-person research team for competitor analysis"
```

The assistant does NOT compose structured agent/deliverable definitions. A Background Agent (session service layer) receives the instruction, loads all current state, and decides what mutations to make using individual tools: `add_agent()`, `remove_agent()`, `update_deliverable()`, `set_execution_order()`, etc. The Designer and DAG execution are a separate pipeline triggered when the user runs the workflow. The dispatch tool is kept simple because more service layers will be added in the future.

### 6.3 Intent Classification Strategy

Two-step dispatch process:

1. **Classify**: Determine what the user wants (new dispatch, status check, modify existing, review result)
2. **Compose or Route**: Based on classification, either compose a dispatch payload or retrieve status

This avoids the common failure mode of trying to decompose intent and produce a full payload in one step.

### 6.4 Context Window Management

Implement three mechanisms:

1. **State injection**: System injects current dispatch status into the prompt before each turn (~200 tokens)
2. **Auto-compaction**: When context exceeds 80%, summarize conversation history preserving dispatch decisions and user preferences
3. **Observation masking**: Strip verbose execution logs from context, keeping only action summaries and key outputs

### 6.5 Proactive Notification Pipeline

Leverage Nexor's existing WebSocket infrastructure:

1. When the assistant dispatches work, it subscribes to the execution's event topic
2. The event bus delivers execution events (agent started, completed, failed, deliverable ready)
3. A notification formatter translates events into user-facing messages
4. Messages are pushed to the user's WebSocket connection
5. User notification preferences control granularity

### 6.6 Model Selection

Following Anthropic's research system pattern:
- **Assistant (orchestrator)**: Use the strongest model (Opus/Sonnet) -- it handles intent decomposition, dispatch composition, and user interaction
- **Designer agent**: Use a strong model (Sonnet) -- it engineers system prompts and tool assignments
- **Worker agents**: Use efficient models (Haiku/Sonnet) -- they execute focused tasks with clear instructions

This mirrors the Anthropic finding: "A multi-agent system with Claude Opus 4 as the lead agent and Claude Sonnet 4 subagents outperformed single-agent Claude Opus 4 by 90.2%."

---

## Sources

### Orchestrator/Dispatcher Patterns
- [Anthropic: How we built our multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system)
- [Google Developers: Multi-agent patterns in ADK](https://developers.googleblog.com/developers-guide-to-multi-agent-patterns-in-adk/)
- [OpenAI: Orchestrating multiple agents](https://openai.github.io/openai-agents-python/multi_agent/)
- [OpenAI Cookbook: Orchestrating Agents - Routines and Handoffs](https://cookbook.openai.com/examples/orchestrating_agents)
- [Devin: Coding Agents 101](https://devin.ai/agents101)
- [DataCamp: CrewAI vs LangGraph vs AutoGen](https://www.datacamp.com/tutorial/crewai-vs-langgraph-vs-autogen)
- [Iterathon: Agent Orchestration 2026](https://iterathon.tech/blog/ai-agent-orchestration-frameworks-2026)

### Context Engineering
- [Anthropic: Effective context engineering for AI agents](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)
- [LangChain: Context Engineering for Agents](https://blog.langchain.com/context-engineering-for-agents/)
- [LangChain Docs: Context engineering in agents](https://docs.langchain.com/oss/python/langchain/context-engineering)
- [JetBrains Research: Smarter Context Management for LLM-Powered Agents (NeurIPS 2025)](https://blog.jetbrains.com/research/2025/12/efficient-context-management/)
- [Getmaxim: Context Window Management Strategies](https://www.getmaxim.ai/articles/context-window-management-strategies-for-long-context-ai-agents-and-chatbots/)
- [Mem0: LLM Chat History Summarization Guide](https://mem0.ai/blog/llm-chat-history-summarization-guide-2025)
- [FlowHunt: Context Engineering Definitive 2025 Guide](https://www.flowhunt.io/blog/context-engineering/)

### Structured Outputs
- [OpenAI: Structured model outputs](https://platform.openai.com/docs/guides/structured-outputs)
- [Agenta: Guide to structured outputs and function calling](https://agenta.ai/blog/the-guide-to-structured-outputs-and-function-calling-with-llms)
- [SuperJSON: JSON Schema Structured Output APIs Complete Guide](https://superjson.ai/blog/2025-08-17-json-schema-structured-output-apis-complete-guide/)

### Reactive/Proactive Patterns
- [AG-UI: Core Architecture](https://docs.ag-ui.com/concepts/architecture)
- [CHI 2025: Designing Proactive AI Assistants for Programming](https://dl.acm.org/doi/10.1145/3706598.3714002)
- [ZBrain: Ambient Agents Explained](https://zbrain.ai/ambient-agents/)

### Framework Documentation
- [OpenAI Agents SDK: Handoffs](https://openai.github.io/openai-agents-python/handoffs/)
- [Google ADK: Multi-agent systems](https://google.github.io/adk-docs/agents/multi-agents/)
- [LangGraph: State Management](https://sparkco.ai/blog/mastering-langgraph-state-management-in-2025)
- [LangChain: State of Agent Engineering](https://www.langchain.com/state-of-agent-engineering)
- [Cognition: Devin's 2025 Performance Review](https://cognition.ai/blog/devin-annual-performance-review-2025)
- [Google Developers: Architecting efficient context-aware multi-agent framework](https://developers.googleblog.com/architecting-efficient-context-aware-multi-agent-framework-for-production/)
