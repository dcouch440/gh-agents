# Background Task Execution with Real-Time WebSocket Event Push

## Research for Nexor: Multi-Agent DAG Orchestration Platform

**Date:** 2026-02-17
**Context:** Nexor is a Rust (Axum) + React platform for multi-agent AI orchestration. When the assistant dispatches work, it must run in the background, push progress via WebSocket, notify the assistant on completion, support cancellation, and accept mid-flight instruction updates.

---

## Table of Contents

1. [Background Task Lifecycle Management](#1-background-task-lifecycle-management)
2. [WebSocket Event Design for AI Agent Progress](#2-websocket-event-design-for-ai-agent-progress)
3. [Reactive Assistant Notifications](#3-reactive-assistant-notifications)
4. [Cancellation and Interruption Patterns](#4-cancellation-and-interruption-patterns)
5. [Task Queue Patterns for AI Workloads](#5-task-queue-patterns-for-ai-workloads)
6. [Event Sourcing for Execution State](#6-event-sourcing-for-execution-state)
7. [Recommendations for Nexor](#7-recommendations-for-nexor)

---

## 1. Background Task Lifecycle Management

### 1.1 Industry Patterns

#### Temporal Workflow Engine

Temporal is the dominant durable execution engine for orchestrating long-running background work. Its core contribution is the concept of **durable execution**: the runtime automatically persists every step of a workflow, enabling recovery, replay, and pause at arbitrary points. Netflix reduced transient deployment failures from 4% to essentially zero (0.0001%) by leveraging Temporal's durable execution guarantees.

Key Temporal concepts relevant to Nexor:

- **Activities** are single units of work (analogous to Nexor's DAG steps). They are independently retryable, timeboxed, and idempotent.
- **Workflow state** is automatically checkpointed after each activity completes. On failure, the runtime replays the event history to reconstruct the workflow's state up to the failure point, then resumes from there.
- **Task Queues** decouple workflow scheduling from execution. Workers poll queues for tasks, providing natural backpressure and load distribution.
- **SAGA pattern**: Temporal makes compensating transactions straightforward. Each activity can define a compensation function; on failure, previously completed activities are compensated in reverse order.

**Source:** [Temporal Architecture](https://github.com/temporalio/temporal), [Agentic AI with Temporal](https://intuitionlabs.ai/articles/agentic-ai-temporal-orchestration), [Rise of Temporal](https://medium.com/@milinangalia/the-rise-of-temporal-how-netflix-and-leading-tech-companies-are-revolutionizing-workflow-822fbcc736e6)

#### Restate: Modern Durable Execution

Restate is a cloud-native durable execution runtime written in Rust, with a stream-processing architecture optimized for low latency and high throughput. Its partition processor is a tight event loop running on Tokio, with processors operating independently and accessing exclusively local data structures.

Key concepts:

- **Workflows** are sequences of steps that execute durably. Each workflow definition has a `run` handler that executes exactly once per instance.
- **Virtual Objects** provide an embedded key-value store for persisting state, useful for implementing consistent state machines, session state, or agent context and memory.
- **Async Tasks**: Restate guarantees all tasks run to completion, handles retries and recovery upon failures, and ensures exactly-once execution without additional infrastructure.

**Source:** [Restate Documentation](https://docs.restate.dev/foundations/key-concepts), [Building a Modern Durable Execution Engine](https://www.restate.dev/blog/building-a-modern-durable-execution-engine-from-first-principles), [Rise of Durable Execution Engines](https://www.kai-waehner.de/blog/2025/06/05/the-rise-of-the-durable-execution-engine-temporal-restate-in-an-event-driven-architecture-apache-kafka/)

#### Google A2A Protocol Task Lifecycle

Google's Agent2Agent (A2A) protocol, launched April 2025 with 50+ technology partners, defines a formal task lifecycle state machine for agent-to-agent communication:

```
submitted -> working -> input-required -> completed
                   \-> failed
                   \-> canceled
                   \-> rejected
```

The stream must close when the task reaches a terminal state. Each task has a unique ID and progresses through defined states with Server-Sent Events for real-time progress streaming.

**Source:** [A2A Protocol Specification](https://a2a-protocol.org/latest/specification/), [Google A2A Announcement](https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/)

### 1.2 Task State Machine

Based on industry patterns, a production task lifecycle state machine should include:

```
                 +-----------+
                 |  pending   |  (created, not yet started)
                 +-----+-----+
                       |
                       v
                 +-----------+
            +--->|  running   |<---+
            |    +-----+-----+    |
            |          |          |
            |    +-----+-----+   |
            |    |  paused    |---+  (input-required / human-in-the-loop)
            |    +-----------+
            |          |
     (retry)|    +-----+-----+
            +----| retrying   |
                 +-----------+
                       |
              +--------+--------+
              v                 v
        +-----------+    +-----------+
        | completed  |    |  failed   |
        +-----------+    +-----------+
                              |
                         +-----------+
                         | cancelled  |
                         +-----------+
```

### 1.3 Tokio Patterns for Background Tasks

The idiomatic Rust/Tokio approach combines several primitives:

- **`tokio::spawn`**: Fire-and-forget task spawning. Each spawned task runs concurrently on the Tokio runtime.
- **`tokio_util::task::TaskTracker`**: Tracks spawned tasks for graceful shutdown. Used with `CancellationToken` to signal shutdown and wait for all tasks to drain.
- **`tokio::sync::Semaphore`**: Limits concurrent task execution. `Arc::clone(&sem).acquire_owned().await` before spawning prevents unbounded task creation.
- **Bounded channels (`mpsc`)**: Provide natural backpressure. When the channel is full, the sender blocks, preventing the system from accepting more work than it can handle.

**Source:** [Tokio Task Documentation](https://docs.rs/tokio/latest/tokio/task/), [TaskTracker](https://docs.rs/tokio-util/latest/tokio_util/task/task_tracker/struct.TaskTracker.html), [Tokio Graceful Shutdown](https://tokio.rs/tokio/topics/shutdown)

---

## 2. WebSocket Event Design for AI Agent Progress

### 2.1 AG-UI Protocol (Agent-User Interaction Protocol)

AG-UI is an open, lightweight, event-based protocol standardizing how AI agents connect to frontend applications. Released in 2025, it defines **16 event types** organized into **5 categories**:

1. **Lifecycle Events** -- Track which stage the agent is in (started, in-progress, completed)
2. **Text Message Events** -- `TEXT_MESSAGE_START`, `TEXT_MESSAGE_CONTENT` (token stream), `TEXT_MESSAGE_END` -- create the familiar "typing" effect
3. **Tool Call Events** -- `TOOL_CALL_START`, `TOOL_CALL_ARGS`, `TOOL_CALL_END` -- signal the agent wants to do something
4. **State Events** -- `STATE_SNAPSHOT` (full state at start or for re-sync), `STATE_DELTA` (incremental diffs, like "add 'hello' at index 5")
5. **Control Events** -- `INTERRUPT` (pause for approval on sensitive actions), `CUSTOM` (extension point)

The `STATE_DELTA` pattern is particularly relevant: instead of resending entire state on each change, the agent sends tiny diffs that the frontend applies incrementally.

**Source:** [AG-UI Events Documentation](https://docs.ag-ui.com/concepts/events), [AG-UI GitHub](https://github.com/ag-ui-protocol/ag-ui), [Master the 17 AG-UI Event Types](https://www.copilotkit.ai/blog/master-the-17-ag-ui-event-types-for-building-agents-the-right-way)

### 2.2 OpenAI Realtime API Event Structure

OpenAI's Realtime API uses a flat event structure with a discriminated `type` field:

```json
{
  "type": "response.output_item.added",
  "event_id": "event_abc123",
  "response_id": "resp_xyz",
  "output_index": 0,
  "item": { ... }
}
```

Key event categories:
- **Session**: `session.created`, `session.updated`
- **Response lifecycle**: `response.created`, `response.done` (always emitted regardless of final state)
- **Item streaming**: `response.output_item.added`, `response.output_item.done`
- **Content streaming**: `response.content_part.added`, `response.text.delta`, `response.text.done`
- **Function calls**: `response.function_call_arguments.delta`, `response.function_call_arguments.done`

The `response.done` event is always emitted, even for interrupted/incomplete/cancelled responses, which is a critical reliability pattern.

**Source:** [OpenAI Realtime Server Events](https://platform.openai.com/docs/api-reference/realtime-server-events), [OpenAI Streaming Events](https://platform.openai.com/docs/api-reference/responses-streaming)

### 2.3 Vercel AI SDK Streaming

The Vercel AI SDK (versions 5 and 6, released 2025) uses Server-Sent Events with structured streaming:

- **Tool call inputs stream by default**, providing partial updates as the model generates them
- **Tool execution errors** are scoped to the tool and can be resubmitted to the LLM
- **Multi-step tool loops** (up to 20 steps by default) handle the complete tool execution cycle
- AI SDK 6 introduced the v3 Language Model Specification supporting agents and tool approval flows

**Source:** [AI SDK 6](https://vercel.com/blog/ai-sdk-6), [AI SDK 5](https://vercel.com/blog/ai-sdk-5), [AI SDK Documentation](https://ai-sdk.dev/docs/introduction)

### 2.4 LangSmith Tracing Schema

LangSmith captures execution traces using **nested spans** that record every step from initial input to final response:

- Each span captures: prompts, retrieved context, tool selection logic, tool inputs/outputs, errors, and exceptions
- Spans are hierarchical: agent-level decisions contain sub-spans for LLM generations, tool calls, and data retrievals
- The tracing overhead is virtually zero, making it suitable for production
- Traces support **replay**: any production trace can be replayed locally for debugging

**Source:** [LangSmith Observability](https://www.langchain.com/langsmith/observability), [Advanced LangSmith Tracing](https://sparkco.ai/blog/advanced-langsmith-agent-tracing-techniques-in-2025)

### 2.5 Event Granularity Recommendations

Based on the patterns above, events should exist at three granularity levels:

| Level | Events | Frequency | Use Case |
|-------|--------|-----------|----------|
| **Lifecycle** | `started`, `completed`, `failed`, `cancelled` | Once per execution | Top-level status tracking |
| **Phase/Step** | `step_started`, `step_completed`, `step_failed`, `phase_entered` | Once per step | Progress bars, DAG visualization |
| **Token/Delta** | `token`, `tool_start`, `tool_end`, `state_delta` | High frequency | Live streaming, real-time UX |

The key insight from AG-UI and OpenAI: **always emit terminal events** regardless of how execution ended. The frontend needs to know when to stop showing progress indicators.

### 2.6 Event Schema Best Practices

From Confluent, Wikimedia, and production event systems:

- **Include only required data** in the payload. Avoid redundant or irrelevant information.
- **Structured event IDs** should include domain, type, unique identifier, and version.
- **Sequence IDs** indicate ordering within a stream (Nexor's `BroadcastEnvelope.seq` already does this).
- **Schema versioning** is critical for evolution. Use `oneOf` discriminated patterns rather than polymorphic type fields.
- **Timestamp** every event at the source (Nexor's `WireMessage.ts` already does this).

**Source:** [Confluent Event Design](https://developer.confluent.io/courses/event-design/best-practices/), [WebSocket Architecture Best Practices](https://ably.com/topic/websocket-architecture-best-practices)

---

## 3. Reactive Assistant Notifications

### 3.1 The "Assistant Speaks Unprompted" Pattern

This is one of the most architecturally interesting challenges: when background work completes, the assistant must proactively push a message to the user without being prompted. Several production approaches exist:

#### Microsoft Bot Framework Proactive Messaging

Microsoft's pattern for proactive messages in custom engine agents:

- Push notifications are **asynchronous task updates** delivered via server-initiated HTTP POST requests to a client-provided webhook URL
- The system stores a **conversation reference** when the user first interacts, then uses it to send messages at any time
- Safety guardrails: the bot only sends follow-ups if the user has engaged recently

**Source:** [Microsoft Proactive Messaging](https://learn.microsoft.com/en-us/microsoft-365-copilot/extensibility/custom-engine-agent-asynchronous-flow), [Bot Framework Proactive Messages](https://learn.microsoft.com/en-us/azure/bot-service/bot-builder-howto-proactive-message?view=azure-bot-service-4.0)

#### Meta's Proactive AI (2025)

Meta's implementation trains chatbots to reach out unprompted and follow up on past conversations. Key design constraints:

- Follow-ups only within **14 days** after user initiates conversation
- Requires at least **5 messages** from the user before proactive messaging activates
- Bots remember context from past interactions to make follow-ups relevant

**Source:** [Meta Proactive AI](https://techcrunch.com/2025/07/03/meta-has-found-another-way-to-keep-you-engaged-chatbots-that-message-you-first/)

#### The Completion Notification Pattern

For Nexor's specific case (assistant dispatches work, notifies user on completion), the pattern is:

1. **Background task emits a completion event** on the EventBus with a special payload indicating the task and its results
2. **The chat session subscribes to completion events** for tasks it dispatched
3. **On completion, the system injects an assistant message** into the chat history containing a summary of results
4. **The WebSocket pushes this message** to the frontend as a new chat message, appearing as if the assistant spontaneously spoke

This avoids the complexity of "the assistant thinking" -- it is simply a server-generated message that looks like an assistant response.

### 3.2 Implementation Architecture

```
Assistant dispatches work
  |
  v
TaskRegistry.spawn(task_id, session_id, ...)
  |
  +---> Background task runs on Tokio runtime
  |       |
  |       +---> Progress events via EventBus -> WebSocket -> Frontend
  |       |
  |       +---> Completion event with result summary
  |               |
  |               v
  |         CompletionHandler (listens on EventBus)
  |               |
  |               v
  |         Insert assistant message into session
  |               |
  |               v
  |         Broadcast SessionEvent -> WebSocket -> Frontend
  |
  v
Assistant continues conversation (non-blocking)
```

The critical design decision: **the assistant does not "wait" for completion**. Instead, a separate completion handler service watches for task completion events and generates the notification message. This keeps the assistant's chat loop entirely decoupled from background execution.

### 3.3 Two Strategies for Notification Content

**Strategy A: Template-based (simple).** The completion handler generates a canned message from the task result:

```
"Your workflow 'Research Pipeline' has completed successfully.
3 steps executed, 2 documents generated. View results: [link]"
```

This is deterministic, fast, and requires no LLM call. Suitable for structured workflows with predictable outputs.

**Strategy B: LLM-summarized (rich).** The completion handler makes a lightweight LLM call to summarize the results in conversational tone:

```
"I finished running the research pipeline you asked about.
The key finding is that competitor X launched a new feature last week.
I generated two documents with the full analysis -- want me to walk through them?"
```

This requires a small LLM call but produces more natural, contextual notifications. The LLM call should use a fast, cheap model (Haiku-class) with a tight token budget.

**Recommendation:** Start with Strategy A for reliability, add Strategy B as an opt-in enhancement.

---

## 4. Cancellation and Interruption Patterns

### 4.1 Cooperative Cancellation in Async Rust

Tokio's cancellation model is **cooperative**: tasks are not forcibly terminated. Instead, they check for cancellation signals at safe points and clean up gracefully.

#### CancellationToken (tokio-util)

The primary primitive. Key properties:

- **Cloning** produces an indistinguishable copy -- cancelling one cancels all clones
- **Child tokens** cancel when the parent cancels, but not vice versa
- **`cancelled()`** returns a `Future` that resolves when cancellation is requested
- **Used with `tokio::select!`** to race cancellation against the main work loop

```rust
tokio::select! {
    result = do_work() => { /* handle result */ }
    _ = token.cancelled() => { /* cleanup and exit */ }
}
```

**Source:** [CancellationToken Documentation](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html), [Rust Tokio Cancellation Patterns](https://cybernetist.com/2024/04/19/rust-tokio-task-cancellation-patterns/)

#### Three Parts of Graceful Shutdown

1. **Figure out when to shut down** -- Signal detection (SIGTERM/SIGINT), API-triggered cancellation, or timeout
2. **Tell every part of the program to shut down** -- CancellationToken propagation through the task tree
3. **Wait for other parts to shut down** -- TaskTracker to wait for all spawned tasks to complete cleanup

**Source:** [Tokio Graceful Shutdown Guide](https://tokio.rs/tokio/topics/shutdown)

### 4.2 Cancellation Hierarchy for Nexor

Nexor's current system uses a flat `DashMap<Uuid, CancellationToken>` with parent-child support. The recommended hierarchy for background tasks:

```
shutdown_token (master)
  |
  +-- collection_execution_token
  |     |
  |     +-- workflow_execution_token
  |           |
  |           +-- step_execution_token (per-step for fine-grained cancel)
  |                 |
  |                 +-- llm_call_token (per LLM invocation)
  |
  +-- background_task_token (for assistant-dispatched work)
        |
        +-- sub_task_tokens...
```

### 4.3 Partial Result Preservation

A key advantage of cooperative cancellation: **cleanup handlers run before the task exits**. This enables:

- Flushing partial results to the database before termination
- Emitting a `cancelled` event with a summary of what was completed
- Preserving completed step outputs even when later steps are cancelled
- Recording the cancellation point for potential resumption

Nexor already has the infrastructure for this with `StepOutput` tracking per step. On cancellation, the DAG loop should:

1. Stop dispatching new steps
2. Wait for in-flight steps to reach a safe point (or timeout)
3. Persist all completed step outputs
4. Mark the execution as `cancelled` with metadata about which steps completed

### 4.4 Mid-Flight Instruction Updates

This is an emerging pattern in 2025 AI agent systems. The approach requires a **shared mutable instruction channel**:

```rust
struct TaskHandle {
    cancel_token: CancellationToken,
    instruction_tx: watch::Sender<TaskInstructions>,
    status_rx: watch::Receiver<TaskStatus>,
}
```

The background task holds a `watch::Receiver<TaskInstructions>` and checks for instruction changes at safe points (between steps, between LLM rounds). The `watch` channel is ideal because:

- It always holds the latest value (new receivers immediately see current instructions)
- It is multi-producer, multi-consumer
- Checking for changes is non-blocking (`changed()` returns immediately if no change)

The concrete workflow:

1. User sends "actually, focus on competitor X instead" while a research task runs
2. The assistant parses this as an instruction update, calls `instruction_tx.send(new_instructions)`
3. The background task, between its current step and the next, calls `instruction_rx.has_changed()`
4. If changed, the task reads the new instructions and adjusts its next step's prompt accordingly
5. A `task_instructions_updated` event is broadcast so the frontend can show the change

**Source:** [Azure Agent Design Patterns](https://learn.microsoft.com/en-us/azure/architecture/ai-ml/guide/ai-agent-design-patterns), [Claude Code Web Task Modification](https://beyond.addy.ie/2026-trends/)

---

## 5. Task Queue Patterns for AI Workloads

### 5.1 Direct Spawn vs. Queue-Based Execution

| Approach | Pros | Cons |
|----------|------|------|
| **Direct `tokio::spawn`** | Simple, low latency, no infrastructure | No backpressure, no persistence, no priority |
| **Bounded `mpsc` channel** | Natural backpressure, ordering guarantees | Still in-memory, lost on crash |
| **PostgreSQL queue** | Durable, survives restarts, queryable | Higher latency, polling overhead |
| **External queue (Redis/RabbitMQ)** | Full queue semantics, dead letters | Extra infrastructure, operational complexity |

For AI workloads specifically, the **bounded `mpsc` channel with a semaphore** is the sweet spot for single-server deployments:

```rust
// Concurrency-limited task executor
let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_EXECUTIONS));
let tracker = TaskTracker::new();

loop {
    let task = task_rx.recv().await;
    let permit = semaphore.clone().acquire_owned().await;
    tracker.spawn(async move {
        let _permit = permit; // held until task completes
        execute_task(task).await;
    });
}
```

### 5.2 Concurrency Limits for LLM Workloads

AI workloads have unique concurrency characteristics:

- **LLM API calls are I/O-bound** (waiting for remote API response), not CPU-bound
- **Rate limits** are the real bottleneck (tokens per minute, requests per minute)
- **Cost control** requires limiting concurrent executions (each execution consumes tokens)
- **Memory pressure** comes from holding conversation context, not from compute

Recommended approach: **Two-tier limiting**

1. **Semaphore for total concurrent executions** (e.g., 10 workflows running simultaneously)
2. **Rate limiter per LLM provider** (Nexor already has `RateLimitedProvider` with token bucket + semaphore)

### 5.3 Priority and Ordering

For assistant-dispatched background work, priority matters:

- **User-initiated tasks** (explicit "run this workflow") should have higher priority than system-generated tasks
- **Interactive tasks** (assistant monitoring progress) should preempt batch tasks
- **FIFO within priority bands** for fairness

Implementation: use a `BinaryHeap` or priority channel wrapper around the task queue.

### 5.4 Recommendation for Nexor

Nexor is a single-server deployment. The right pattern is:

1. **Keep `tokio::spawn`** for the actual execution (it is already correct)
2. **Add a `Semaphore`** to limit concurrent workflow executions (prevents resource exhaustion)
3. **Add a `TaskTracker`** to enable graceful shutdown (wait for in-flight tasks before exit)
4. **Skip external queues** unless multi-server deployment becomes a requirement

The existing `RateLimitedProvider` already handles LLM-level concurrency control. The semaphore adds workflow-level control on top.

**Source:** [Tokio Semaphore](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html), [Backpressure in Async Rust](https://medium.com/@trivajay259/async-rust-with-tokio-i-o-streams-backpressure-concurrency-and-ergonomics-74e53df7196d), [Job Queue with Tokio and PostgreSQL](https://cetra3.github.io/blog/implementing-a-jobq/)

---

## 6. Event Sourcing for Execution State

### 6.1 Core Concept

Event sourcing stores the full history of state changes as an append-only log of events, rather than storing only the current state. For AI agent execution, this provides:

- **Complete audit trail**: Every decision, tool call, and state change is recorded
- **State reconstruction**: Any point-in-time state can be reconstructed by replaying events
- **Debugging**: Failures can be precisely diagnosed by examining the event sequence
- **Replay**: Production traces can be replayed locally for debugging (LangSmith's key feature)

**Source:** [Event Sourcing Pattern - Azure](https://learn.microsoft.com/en-us/azure/architecture/patterns/event-sourcing), [Event Sourcing Explained](https://www.baytechconsulting.com/blog/event-sourcing-explained-2025)

### 6.2 Event-Sourced Execution in AI Agents

LangGraph (GA May 2025, used by ~400 companies) demonstrates event sourcing for AI agents:

- **Built-in checkpointing**: Agent state persists after every step and on failure
- **Resume from last checkpoint**: On failure, execution resumes from the last saved state, not from the beginning
- **Replay any production trace**: LangSmith enables local replay of any production trace for debugging
- **Inspect state at every node**: Full state visibility at each point in the execution graph

**Source:** [Advanced LangSmith Tracing](https://sparkco.ai/blog/advanced-langsmith-agent-tracing-techniques-in-2025), [LangChain AI Agents Guide](https://www.digitalapplied.com/blog/langchain-ai-agents-guide-2025)

### 6.3 Graphite: Event-Driven AI Agent Framework

Graphite (2025) is an event-driven AI agent framework that uses event sourcing as a first-class pattern:

- Every agent action produces an event that is persisted
- The agent's state is derived from replaying its event stream
- Events are the single source of truth for what happened during execution

**Source:** [Graphite - Event Driven AI Agent Framework](https://medium.com/binome/introduction-to-graphite-an-event-driven-ai-agent-framework-540478130cd2)

### 6.4 Event Sourcing and WebSocket Integration

The natural architecture:

```
Execution Engine
  |
  v
Event Store (append-only, PostgreSQL table)
  |
  +---> EventBus (broadcast channel)
  |       |
  |       +---> WebSocket connections (real-time push)
  |       |
  |       +---> Completion handlers (proactive notifications)
  |
  +---> Query API (GET /executions/:id/events for full history)
  |
  +---> Replay API (POST /executions/:id/replay for debugging)
```

Events serve dual purpose: they are the durable record of execution AND the real-time notification mechanism. The EventBus is fed from the event store write path, ensuring WebSocket consumers see exactly the same events that are persisted.

### 6.5 Event Store Schema

```sql
CREATE TABLE execution_events (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    execution_id  UUID NOT NULL REFERENCES workflow_executions(id),
    sequence      BIGINT NOT NULL,
    event_type    TEXT NOT NULL,
    data          JSONB NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (execution_id, sequence)
);

CREATE INDEX idx_execution_events_exec_id
    ON execution_events(execution_id, sequence);
```

Benefits:
- Full audit trail queryable with SQL
- State reconstruction by replaying events for an execution_id
- Debugging: `SELECT * FROM execution_events WHERE execution_id = $1 ORDER BY sequence`
- Analytics: aggregate over event types for performance metrics

### 6.6 Practical Considerations

**Write amplification**: Every event is written to both PostgreSQL and the broadcast channel. For high-frequency events (token streaming at ~100 tokens/sec), this can be significant. Mitigation: only persist lifecycle and step-level events; token-level events are ephemeral (broadcast only, not stored).

**Event compaction**: Over time, execution event tables can grow large. Implement a retention policy: keep full event history for recent executions (30 days), compact older executions to summary records.

**Snapshot optimization**: For long-running executions with hundreds of events, replaying the full event stream is slow. Periodically write snapshot records that capture the full state at a point in time, then replay only events after the snapshot.

---

## 7. Recommendations for Nexor

### 7.1 What Nexor Already Has (Strengths)

Reviewing Nexor's codebase reveals a solid foundation:

- **EventBus** with `Arc<BroadcastEnvelope>` pre-serialization -- excellent performance pattern, zero per-connection serialization overhead
- **WireMessage** flat format with `topic`, `event`, `ts`, `run_id`, `data` -- clean, matches industry best practices
- **CancellationToken hierarchy** with parent-child support via `register_child_cancellation` -- correct pattern
- **Run-scoped filtering** (`subscribe_run`) -- enables per-execution event streams without topic pollution
- **Sequence numbers** (`BroadcastEnvelope.seq`) -- enables gap detection and ordering
- **EventsMissed control message** with REST fallback guidance -- proper degradation pattern
- **WorkflowEventKind** with 27+ event variants covering lifecycle, progress, streaming, and mutations -- comprehensive coverage
- **DashMap for response streams and cancellation tokens** -- safe concurrent access without global locks
- **BufferedStream** with replay capability for late-connecting SSE clients -- handles the reconnection problem

### 7.2 Recommended Additions

#### A. Background Task Registry

Add a `TaskRegistry` to `AppState` that manages dispatched background work:

```rust
use tokio::sync::watch;
use tokio_util::task::TaskTracker;

/// Instructions that can be updated mid-flight.
#[derive(Debug, Clone)]
pub struct TaskInstructions {
    pub additional_context: Option<String>,
    pub priority_override: Option<Priority>,
    pub modified_at: DateTime<Utc>,
}

/// Status of a background task, observable by the dispatcher.
#[derive(Debug, Clone)]
pub enum TaskStatus {
    Pending,
    Running { progress_pct: u8, message: String },
    Completed { summary: String },
    Failed { error: String },
    Cancelled { partial_results: Option<String> },
}

/// Handle returned to the dispatcher when spawning a task.
pub struct TaskHandle {
    pub task_id: Uuid,
    pub session_id: Uuid,
    pub description: String,
    pub cancel_token: CancellationToken,
    pub instruction_tx: watch::Sender<TaskInstructions>,
    pub status_rx: watch::Receiver<TaskStatus>,
    pub created_at: DateTime<Utc>,
}

/// Central registry for all background tasks.
pub struct TaskRegistry {
    tasks: DashMap<Uuid, TaskHandle>,
    tracker: TaskTracker,
    semaphore: Arc<Semaphore>,
}

impl TaskRegistry {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            tasks: DashMap::new(),
            tracker: TaskTracker::new(),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    /// Spawn a background task with concurrency control.
    /// Returns a task_id for status queries and cancellation.
    pub async fn spawn<F, Fut>(&self, session_id: Uuid, description: String, f: F) -> Uuid
    where
        F: FnOnce(Uuid, CancellationToken, watch::Receiver<TaskInstructions>) -> Fut,
        Fut: Future<Output = TaskStatus> + Send + 'static,
    {
        let task_id = Uuid::new_v4();
        let cancel_token = CancellationToken::new();
        let (instruction_tx, instruction_rx) = watch::channel(TaskInstructions::default());
        let (status_tx, status_rx) = watch::channel(TaskStatus::Pending);

        let handle = TaskHandle {
            task_id,
            session_id,
            description,
            cancel_token: cancel_token.clone(),
            instruction_tx,
            status_rx,
            created_at: Utc::now(),
        };
        self.tasks.insert(task_id, handle);

        let permit = self.semaphore.clone().acquire_owned().await.unwrap();
        let tasks = self.tasks.clone();

        self.tracker.spawn(async move {
            let _permit = permit;
            let _ = status_tx.send(TaskStatus::Running {
                progress_pct: 0,
                message: "Starting".into(),
            });

            let final_status = f(task_id, cancel_token, instruction_rx).await;
            let _ = status_tx.send(final_status);

            // Cleanup after a delay to allow status queries
            tokio::time::sleep(Duration::from_secs(60)).await;
            tasks.remove(&task_id);
        });

        task_id
    }

    /// Wait for all tasks to complete (used during shutdown).
    pub async fn shutdown(&self) {
        self.tracker.close();
        self.tracker.wait().await;
    }
}
```

This extends the existing `cancellation_tokens: DashMap<Uuid, CancellationToken>` pattern with richer metadata and lifecycle management.

#### B. Completion Notification Service

Add a background service that listens for task completion events and injects assistant messages:

```rust
/// Background service that watches for task completions
/// and generates proactive assistant notifications.
async fn completion_notification_service(state: AppState) {
    let mut rx = state.events().subscribe();

    loop {
        match rx.recv().await {
            Ok(envelope) => {
                if let Some((session_id, summary)) = extract_task_completion(&envelope) {
                    // Insert an assistant-attributed message into the chat session
                    let notification = format_completion_notification(&summary);

                    if let Err(e) = state.repos().chat_messages
                        .insert_assistant_message(session_id, &notification)
                        .await
                    {
                        tracing::error!("Failed to insert completion notification: {}", e);
                        continue;
                    }

                    // Broadcast so the frontend shows the new message
                    state.broadcast_session(SessionEvent {
                        session_id,
                        user_id: None, // broadcast to all session subscribers
                        kind: SessionEventKind::MessageAdded {
                            role: "assistant".into(),
                            content: notification,
                        },
                    });
                }
            }
            Err(RecvError::Lagged(n)) => {
                tracing::warn!("Completion notifier lagged, missed {} events", n);
            }
            Err(RecvError::Closed) => break,
        }
    }
}
```

#### C. New WebSocket Topic: `dispatch`

Add a `Dispatch` topic to the existing `Topic` enum for assistant-dispatched background work:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Topic {
    Workflow,
    Room,
    Session,
    Dispatch,  // background tasks dispatched by the assistant
}
```

Event types for this topic:

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchEventKind {
    /// Task accepted and queued
    TaskCreated {
        task_id: Uuid,
        session_id: Uuid,
        description: String,
    },
    /// Intermediate progress update
    TaskProgress {
        task_id: Uuid,
        progress_pct: u8,
        phase: String,
        message: String,
    },
    /// Task finished successfully
    TaskCompleted {
        task_id: Uuid,
        summary: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        result_url: Option<String>,
        duration_ms: u64,
    },
    /// Task failed
    TaskFailed {
        task_id: Uuid,
        error: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        partial_results: Option<String>,
    },
    /// Task was cancelled
    TaskCancelled {
        task_id: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        partial_results: Option<String>,
    },
    /// Mid-flight instructions were updated
    TaskInstructionsUpdated {
        task_id: Uuid,
        description: String,
    },
}
```

#### D. Concurrency Control Wrapper

Replace the current unbounded `tokio::spawn` in `run_handlers.rs` with registry-managed execution:

```rust
// Current pattern (run_handlers.rs line 146):
//   tokio::spawn(async move { ... });

// Proposed pattern:
let task_id = state.task_registry().spawn(
    session_id,
    format!("Workflow: {}", workflow.name),
    |task_id, cancel_token, instruction_rx| async move {
        // ... existing execution logic with cancel_token checks ...
    },
).await;
```

#### E. Event Store Table (Optional, Phase 6)

Add durable event persistence alongside the existing in-memory EventBus:

```sql
CREATE TABLE execution_events (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    execution_id  UUID NOT NULL,
    sequence      BIGINT NOT NULL,
    event_type    TEXT NOT NULL,
    data          JSONB NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (execution_id, sequence)
);
```

Persist lifecycle and step-level events. Token-level streaming events remain ephemeral (broadcast only) to avoid write amplification.

### 7.3 Event Schema for Background Tasks

The current `WireMessage` format is well-designed. Background task events maintain the same shape:

```json
{
  "topic": "dispatch",
  "event": "task_progress",
  "ts": "2026-02-17T10:30:00Z",
  "run_id": "task-uuid-here",
  "user_id": "user-uuid-here",
  "data": {
    "task_id": "...",
    "session_id": "...",
    "phase": "step_execution",
    "step_name": "Research",
    "progress_pct": 60,
    "message": "Completed 3 of 5 research queries"
  }
}
```

### 7.4 Full Architecture Diagram

```
User Chat Interface
  |
  v
Assistant (chat consumer)
  |
  +-- dispatch({ instruction: "..." }) --> TaskRegistry.spawn()
  |                         |
  |                         +-> Semaphore gate (concurrency limit)
  |                         |     |
  |                         |     +-> TaskTracker.spawn (tracked task)
  |                         |           |
  |                         |           +-> Background Agent (session service layer)
  |                         |           |     - Loads: notes, roster, deliverables, context
  |                         |           |     - Calls: add_agent(), remove_agent(),
  |                         |           |       update_deliverable(), set_execution_order()
  |                         |           |     - Configures the step, then done
  |                         |           |
  |                         |           +-> Progress events -> EventBus -> WS
  |                         |           |
  |                         |           +-> Completion event -> EventBus
  |                         |                                     |
  |                         |                                     v
  |                         |                               CompletionNotifier
  |                         |                                     |
  |                         |                                     v
  |                         |                               Insert assistant msg
  |                         |                                     |
  |                         |                                     v
  |                         |                               SessionEvent -> WS
  |                         |
  |                         +-> CancellationToken (hierarchical)
  |                         +-> watch::channel (instruction updates)
  |                         +-> watch::channel (status observable)
  |
  +-- Continues conversation (non-blocking)
  |
  +-- Can query task status: GET /api/tasks/:id
  |
  +-- Can cancel task: POST /api/tasks/:id/cancel
  |
  +-- Can update instructions: POST /api/tasks/:id/instructions
  |
  +-- Receives completion notification via SessionEvent on WS
```

### 7.5 Migration Path

The changes build incrementally on Nexor's existing infrastructure:

| Phase | Change | Builds On | Independently Valuable |
|-------|--------|-----------|----------------------|
| **1** | `TaskRegistry` added to `AppState` | Existing `DashMap<Uuid, CancellationToken>` | Yes -- structured task management |
| **2** | `Dispatch` topic + event types | Existing `Topic` enum, `WorkflowEventKind` pattern | Yes -- frontend can show task list |
| **3** | Semaphore concurrency control | Existing `tokio::spawn` in `run_handlers.rs` | Yes -- prevents resource exhaustion |
| **4** | `CompletionNotifier` service | Existing `chat_consumer` pattern | Yes -- proactive notifications |
| **5** | `watch` channel for instructions | Phase 1 `TaskHandle` | Yes -- mid-flight updates |
| **6** | `execution_events` table | Existing EventBus | Yes -- audit trail, replay |

Each phase is independently valuable and can be shipped separately. Phases 1-3 are the highest priority. Phase 4 enables the "assistant speaks unprompted" UX. Phase 5 is a power-user feature. Phase 6 is an operational excellence investment.

### 7.6 Key Design Principles

1. **Never block the assistant**: Background work runs on separate Tokio tasks. The assistant's chat loop never waits on execution.
2. **Events are the API**: Progress, completion, and cancellation all flow through the same EventBus/WebSocket infrastructure. No special-case communication channels.
3. **Cooperative cancellation everywhere**: Every async operation checks for cancellation at safe points. No forced termination.
4. **Partial results are always preserved**: Cancellation triggers cleanup that persists whatever was completed.
5. **The assistant does not "wake up"**: Instead, a completion handler service generates an assistant-attributed message. The frontend displays it as if the assistant spoke.
6. **Backpressure at every level**: Semaphore limits concurrent tasks, bounded channels limit event queuing, rate limiters constrain LLM API calls.
7. **Ephemeral vs. durable events**: Token-level streaming is broadcast-only (high frequency, low value for audit). Step-level and lifecycle events are persisted (low frequency, high value for debugging).
8. **Same wire format, new topic**: Background task events use the exact same `WireMessage` shape as workflow events. The frontend's existing WebSocket client works unchanged -- it just subscribes to one more topic.

---

## Sources

### Background Task Lifecycle
- [Temporal Architecture](https://github.com/temporalio/temporal)
- [Agentic AI with Temporal](https://intuitionlabs.ai/articles/agentic-ai-temporal-orchestration)
- [Rise of Temporal at Netflix](https://medium.com/@milinangalia/the-rise-of-temporal-how-netflix-and-leading-tech-companies-are-revolutionizing-workflow-822fbcc736e6)
- [Rise of Durable Execution Engines](https://www.kai-waehner.de/blog/2025/06/05/the-rise-of-the-durable-execution-engine-temporal-restate-in-an-event-driven-architecture-apache-kafka/)
- [Restate Documentation](https://docs.restate.dev/foundations/key-concepts)
- [Building a Modern Durable Execution Engine (Restate)](https://www.restate.dev/blog/building-a-modern-durable-execution-engine-from-first-principles)
- [A2A Protocol Specification](https://a2a-protocol.org/latest/specification/)
- [Google A2A Announcement](https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/)

### Tokio Patterns
- [Tokio Task Documentation](https://docs.rs/tokio/latest/tokio/task/)
- [TaskTracker](https://docs.rs/tokio-util/latest/tokio_util/task/task_tracker/struct.TaskTracker.html)
- [CancellationToken](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html)
- [Tokio Graceful Shutdown](https://tokio.rs/tokio/topics/shutdown)
- [Rust Tokio Task Cancellation Patterns](https://cybernetist.com/2024/04/19/rust-tokio-task-cancellation-patterns/)
- [Tokio Semaphore](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html)
- [Backpressure in Async Rust](https://medium.com/@trivajay259/async-rust-with-tokio-i-o-streams-backpressure-concurrency-and-ergonomics-74e53df7196d)
- [Job Queue with Tokio and PostgreSQL](https://cetra3.github.io/blog/implementing-a-jobq/)

### WebSocket Event Design
- [AG-UI Events Documentation](https://docs.ag-ui.com/concepts/events)
- [AG-UI GitHub](https://github.com/ag-ui-protocol/ag-ui)
- [Master the 17 AG-UI Event Types](https://www.copilotkit.ai/blog/master-the-17-ag-ui-event-types-for-building-agents-the-right-way)
- [OpenAI Realtime Server Events](https://platform.openai.com/docs/api-reference/realtime-server-events)
- [OpenAI Streaming Events](https://platform.openai.com/docs/api-reference/responses-streaming)
- [AI SDK 6 (Vercel)](https://vercel.com/blog/ai-sdk-6)
- [AI SDK 5 (Vercel)](https://vercel.com/blog/ai-sdk-5)
- [AI SDK Documentation](https://ai-sdk.dev/docs/introduction)
- [Liveblocks: Why WebSockets for AI Agents](https://liveblocks.io/blog/why-we-built-our-ai-agents-on-websockets-instead-of-http)
- [WebSocket Architecture Best Practices (Ably)](https://ably.com/topic/websocket-architecture-best-practices)
- [Confluent Event Design Best Practices](https://developer.confluent.io/courses/event-design/best-practices/)

### AI Agent Observability
- [LangSmith Observability](https://www.langchain.com/langsmith/observability)
- [Advanced LangSmith Tracing 2025](https://sparkco.ai/blog/advanced-langsmith-agent-tracing-techniques-in-2025)
- [LangChain AI Agents Guide 2025](https://www.digitalapplied.com/blog/langchain-ai-agents-guide-2025)

### Proactive Notifications
- [Microsoft Proactive Messaging](https://learn.microsoft.com/en-us/microsoft-365-copilot/extensibility/custom-engine-agent-asynchronous-flow)
- [Bot Framework Proactive Messages](https://learn.microsoft.com/en-us/azure/bot-service/bot-builder-howto-proactive-message?view=azure-bot-service-4.0)
- [Meta Proactive AI](https://techcrunch.com/2025/07/03/meta-has-found-another-way-to-keep-you-engaged-chatbots-that-message-you-first/)

### Event Sourcing
- [Event Sourcing Pattern - Azure](https://learn.microsoft.com/en-us/azure/architecture/patterns/event-sourcing)
- [Event Sourcing Explained 2025](https://www.baytechconsulting.com/blog/event-sourcing-explained-2025)
- [Graphite: Event-Driven AI Agent Framework](https://medium.com/binome/introduction-to-graphite-an-event-driven-ai-agent-framework-540478130cd2)
- [Microservices.io Event Sourcing](https://microservices.io/patterns/data/event-sourcing.html)

### Axum and Rust Production Patterns
- [Axum WebSocket 2025](https://medium.com/@mikecode/axum-websocket-468736a5c1c7)
- [Real-time WebSockets with Rust and Axum 2025](https://medium.com/rustaceans/beyond-rest-building-real-time-websockets-with-rust-and-axum-in-2025-91af7c45b5df)
- [Axum Background Task Discussions](https://github.com/tokio-rs/axum/discussions/1998)
- [Azure Agent Design Patterns](https://learn.microsoft.com/en-us/azure/architecture/ai-ml/guide/ai-agent-design-patterns)
