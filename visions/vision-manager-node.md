# Manager Node — Vision

## What It Is

The manager node is a conversational orchestrator that sits above the workflow DAG. It talks to the user in natural dialogue, architects the workflow topology, and coordinates step configuration — all through plain English dispatch. The user never touches a config screen. They have a conversation, and the workflow materializes around them.

## Core Principle: Talker / Builder Separation

Every agent in the system is either a **talker** (conversational, user-facing) or a **builder** (mutation tools, system knowledge). Never both.

| Layer | Talker | Builder |
|-------|--------|---------|
| **Manager** | Manager Assistant — converses with user, dispatches intent | Manager's Dispatched Agent — creates nodes, wires topology, dispatches to steps |
| **Step** | Step Assistant — discusses its domain, asks clarifying questions | Step's Dispatched Agent — configures prompt, schema, capabilities, roster |

Talkers stay conversational — small context, natural language, no system internals. Builders stay precise — full config access, mutation tools, no conversation skills. Dispatch is the bridge between them.

## How It Works

### Phase 1: Architecture (User + Manager)

The user tells the manager what they need. The manager dispatches to its builder to create the topology.

```
User: "I want to monitor competitor pricing weekly
       and get reports with recommendations"

Manager: "Let's break that down. You'll need data collection,
 analysis, and reporting. Let me set that up."

Manager dispatches → Builder creates:
  [Collector] → [Analyzer] → [Reporter]
  Nodes appear on the board.
```

The manager doesn't know about protocols, execution modes, or port schemas. It describes what's needed in plain English. The builder translates that into system primitives.

### Phase 2: Requirements Gathering (Manager + Step Assistants)

The manager dispatches plain English instructions to each step assistant. Step assistants respond with what they need to know.

```
Manager → Collector: "You'll gather competitor pricing
 data from the web on a weekly basis"

Manager → Analyzer: "You'll receive raw pricing data
 and identify trends, anomalies, and positioning"

Manager → Reporter: "You'll produce executive-ready
 weekly briefings from the analysis"
```

Each step assistant sees its **board context** — where it sits in the DAG, incoming port summaries, outgoing port summaries, connected resources. It responds with informed questions:

```
Analyzer: "I can see I'm getting records with competitor,
 product, price, date, tier from Collector. I also have
 access to the pricing database.

 I need to know:
 - Compare against our current or historical prices?
 - Time window for trend detection?
 - Reporter needs positioning scores — what scale?"
```

### Phase 3: Synthesis (Questions → Manager → User)

Step assistant questions are synthesized and injected into the manager's system prompt:

```
Manager System Prompt (dynamically updated):

Your step assistants need the following:

DATA COLLECTION:
  - Which competitors and URLs?
  - Which products to track?

ANALYSIS:
  - Compare against current or historical pricing?
  - Trend detection time window?
  - Anomaly threshold?

REPORTING:
  - Target audience?
  - Delivery format?

Guide the conversation to resolve these naturally.
```

The manager weaves these into organic dialogue — not a checklist. The user answers naturally. When a milestone is reached (enough information for a step), the manager dispatches the answers back to that step's assistant.

### Phase 4: Step Configuration (Step Assistants → Builders)

When a step assistant has enough information, it dispatches to its own builder:

```
Analyzer Assistant → dispatch:
  "Configure this step to analyze enterprise SaaS pricing.
   Compare against our baseline of $50/seat. Flag changes
   over 10%. Input is raw pricing records from Collector.
   Output positioning scores and anomaly flags to Reporter."

Analyzer's Builder:
  - set_task("Analyze competitor pricing trends...")
  - set_output_schema({anomalies: [...], positioning: [...]})
  - set_capabilities(["data_analysis", "database"])
  - update_notes("Threshold: 10%, baseline: $50/seat...")
```

### Phase 5: Live Summary Refresh

Every time a dispatch completes or a roster changes, affected port summaries regenerate. Only downstream consumers of the changed port get updated — not the whole board.

```
Collector's builder finishes → output schema changed
  → Resummarize Collector's output port
  → Push to Analyzer's assistant context
  → Analyzer sees updated contract immediately
  → Some of Analyzer's pending questions may now be answered
  → Synthesized questions to manager refresh
```

The step assistant always sees the current truth, not what was true when the conversation started.

### Phase 6: Execution

The workflow runs through the normal DAG — topological sort, port resolution, data flow. No special handling needed. The manager node doesn't participate in execution. It configured the system; now the system runs.

### Phase 7: Feedback & Evolution

After a run, step assistants see results and can message each other or the manager:

```
Analyzer → Collector: "12 of 187 records had missing price
 fields. Consider adding validation before output."

Analyzer → Manager: "Run complete. 3 anomalies flagged
 but 2 were data errors. Might need a data quality step
 between Collector and me."

Manager ←→ User: "The team found some data quality issues.
 Want me to add a validation step between collection
 and analysis?"
```

The workflow evolves through conversation, not reconfiguration.

## What the Manager Assistant Sees

```
System Prompt:
  You are the manager for this workflow. Help the user
  define what they need through natural conversation.

  Board state:
    - Collector (configured, ready)
    - Analyzer (needs: anomaly threshold)
    - Reporter (needs: audience, format)

  Open questions from your team:
    - Analyzer: "What % change counts as significant?"
    - Reporter: "Who reads these reports?"

  Guide the conversation to fill these gaps naturally.
  When you have enough for a step, dispatch it.

Tools: dispatch(), think()
Nothing else. No mutation tools. No system knowledge.
```

## What a Step Assistant Sees

```
System Prompt:
  You are the assistant for the "Analyzer" step.

  Board Context:
    Workflow: "Competitor Pricing Monitor"
    Position: Step 2 of 3
    Upstream: Collector → you
    Downstream: you → Reporter

  Incoming Ports:
    - "raw_data" (from Collector): "Weekly scraped pricing
       for enterprise SaaS tiers. JSON array of
       {competitor, product, price, date, tier}.
       ~50-200 records per run."

  Outgoing Ports:
    - "analysis" (to Reporter): "Structured analysis with
       trend data, anomaly flags, and positioning scores."

  Connected Resources:
    - "pricing_db": "PostgreSQL — 2 tables: price_history
       (50k rows), our_pricing (12 rows, product/tier/price)"

  Capabilities: [data_analysis, database]

Messages:
  Manager: "You'll analyze pricing data, compare against
   our $50/seat baseline, flag 10%+ changes."
  You: "Got it. Time window for trends — rolling 4 weeks
   or since tracking started?"

Tools: dispatch(), think()
Nothing else.
```

## What the Builders See

**Manager's Builder:**
```
System Prompt:
  Full protocol catalog, execution modes, port types.
  Current board topology.

Tools: create_node, remove_node, wire_ports,
       set_step_mode, dispatch_to_step, preview_topology,
       validate_dag

Instruction: (plain English from manager)
```

**Step's Builder:**
```
System Prompt:
  Full step configuration, current schema, capabilities.

Tools: set_task, set_prompt, set_output_schema,
       add_agent, set_dependency, set_capabilities,
       update_notes

Instruction: (plain English from step assistant)
```

## Agent-to-Agent Communication

Every step is a session. Every session is addressable. Messages between agents use the same format as user messages.

```
Sources of messages to any step assistant:
  - User (opens the session in the UI)
  - Manager (dispatching instructions or user context)
  - Other step assistants (cross-step issues or requests)
  - Scheduled events ("nightly: review your last run")
```

All enter the same way. All processed by the same assistant. All visible in the same conversation history. The user can open any session and see the full audit trail.

## Visualization

The manager node is not a regular DAG node with edges. It sits **above** the DAG as a different visual element.

```
         ┌──────────────┐
         │   Manager    │
         └──────┬───────┘
                │
     conversation channels
     (ambient, not edges)
      ┌─────────┼──────────┐
      ╎         ╎          ╎
 ┌────▼───┐ ┌───▼────┐ ┌───▼───┐
 │Collect │→│Analyze │→│Report │
 └────────┘ └────────┘ └───────┘

 ─── solid: data flow (ports, schemas)
 ╎╎╎ ambient: conversation channels (sessions)
```

Two visual layers:
- **Data edges** (solid): Port-wired DAG connections. The execution path.
- **Conversation channels** (ambient): The manager's presence. No explicit lines — maybe a glow or highlight showing which steps have active threads.

The manager might not even need visible connections. Its presence on the board implies it can talk to anything. Active conversations could show as subtle indicators on the step nodes themselves.

## Build Order: One Step at a Time

The manager walks the user through steps sequentially, following the natural flow of the DAG. By the time a downstream step is being configured, all upstream steps already have port summaries defined.

```
1. Manager creates Collector → user configures it
2. Manager creates Analyzer → sees Collector's output summary
3. Manager creates Reporter → sees Analyzer's output summary
```

No step needs runtime data to be configured. Port summaries are the interface contract. Runtime data matters later — for feedback, iteration, and the self-improvement loop after the first run.

## What Already Exists

- **Dispatch model**: Chat assistant → dispatch → background builder agent. Working pattern for workforce today.
- **Step chat sessions**: Every step can have a conversational session. Messages stored, history preserved.
- **Workforce tools**: `set_task`, `add_agent`, `set_dependency`, `update_notes` — mutation tools for step builders.
- **Pipeline service**: Child workflow CRUD, topological sort, cycle detection.
- **Port system**: Typed ports with json_path extraction, conditional edges, variable interpolation.
- **Protocol engine**: `ProtocolCompiler` trait, `ProtocolExpansion` output, `apply_protocol` materializer.
- **Templates**: `WorkflowSnapshot` captures and restores full workflow configurations.
- **WebSocket events**: Live broadcast of dispatch progress, step updates, configuration changes.

## What Needs To Be Built

1. **Workflow-level mutation tools**: `create_node`, `wire_ports`, `set_step_mode`, `dispatch_to_step`, `preview_topology`, `validate_dag` — thin wrappers around existing services for the manager's builder.

2. **Manager strategy**: A new `ExecutionStrategy` (or `DispatchStrategy` variant) with workflow-scoped tools instead of step-scoped tools. System prompt describes available step types, port semantics, and design principles.

3. **Port summary generation**: Summarizer that produces human-readable descriptions of port contents from structured config (schemas, capabilities, agent roster). Regenerates on dispatch completion or roster change.

4. **Question synthesis**: Aggregates open questions from step assistants, distills them, and injects into the manager's system prompt. Updates dynamically as questions are answered.

5. **Cross-session messaging**: Ability for one step's session to send a message to another step's session. Same format as user messages. Routed through the existing chat message infrastructure.

6. **Manager visualization**: Board-level UI for the manager node — ambient connections, active conversation indicators, board state overview.
