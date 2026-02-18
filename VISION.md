# Smarter Assistant

A layered system for non-blocking AI orchestration.

## The Problem

The assistant currently adds agents one by one, calling tools individually. This floods the UI with tool-use displays, often cutting off mid-stream and leaving the chat empty. The user can't continue talking while changes are being made.

## The Solution: Dispatch Service Layers

The assistant dispatches plain English instructions to background service layers. Each service layer handles a specific kind of background work. The dispatch tool is kept simple because more service layers will be added in the future.

```
Chat Session
  │
  ├── Assistant (conversational, always responsive)
  │     │
  │     ├── dispatch({ instruction: "..." })
  │     │     └── Background Agent (session service layer)
  │     │           - Loads current state (notes, roster, deliverables, context)
  │     │           - Calls mutation tools to configure the step
  │     │           - Done. Step is now configured.
  │     │
  │     ├── cancel_dispatch(execution_id)
  │     │
  │     └── Future dispatch layers (different background tasks)
  │
  └── Workflow Execution (separate trigger, existing pipeline)
        └── Step (configured by the service layer above)
              └── Designer (sub-workflow to the step)
                    └── DAG Execution (agents run)
```

These are **two separate pipelines**. The dispatch service layer configures. The workflow pipeline executes. They share state through the database but are otherwise decoupled.

### The Assistant

The assistant is conversational and always responsive. It gathers user intent through dialogue and dispatches plain English instructions to background service layers. It never calls structured mutation tools directly.

**Tools:**
- `dispatch({ instruction: "..." })` — sends a plain English task to a background service layer
- `cancel_dispatch(execution_id)` — cancels a running background task

**Context:**
- Capabilities index (compact summaries of what's possible)
- Active dispatch statuses (traffic-light summaries)
- Recent results

The assistant can dispatch NEW information DURING an update, dynamically changing direction.

### The Background Agent (First Service Layer)

A session service layer that runs asynchronously. Its ONE task: configure the workflow step.

**What it configures:**
- Step title, header, metadata
- Agent roster (add, update, remove agents)
- Deliverables (add, update, remove)
- Assistant's notes (accumulated context)
- Execution order

**How it works:**
1. Receives a plain English instruction from the assistant
2. Loads all current state from the database
3. Decides what mutations to make
4. Calls structured tools: `add_agent()`, `remove_agent()`, `update_deliverable()`, `set_execution_order()`, etc.
5. Done. The step is configured.

It does NOT trigger the Designer or DAG execution. Those happen separately when the user runs the workflow.

### The Existing Execution Pipeline (Separate)

When the user runs the workflow, the existing pipeline takes over:
1. **Designer** (sub-workflow to the step) — engineers system prompts, tool assignments, inter-agent routing
2. **DAG Execution** — agents execute in React-style tool loops

This pipeline already exists. The dispatch service layer just configures what gets executed.

## Key Properties

1. **Non-blocking**: The assistant dispatches and continues the conversation immediately. A tool display shows background activity. Users can cancel at any moment.
2. **Reactive**: When background work completes, the assistant is notified and can push a message to the frontend without the user asking.
3. **Capabilities-aware**: The assistant knows "what's possible" via a compact capabilities index (~300 tokens). Detailed information loads on demand.
4. **Mid-flight updates**: The assistant can dispatch additional instructions during execution.
5. **Extensible**: The dispatch tool is simple (`{ instruction: "..." }`) because more service layers will be added. Each handles a different kind of background work through the same interface.

## Assistant Config

```
Has run to review: true
Last run date: ...
Agent Roster: [...]
Context: <></>
User Messages: [...]
Dispatched Task Statuses: [...]
```
