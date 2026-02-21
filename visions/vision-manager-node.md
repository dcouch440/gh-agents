# Manager Node — Vision

## What It Is

The manager node is a conversational orchestrator that sits above the workflow DAG. It talks to the user in natural dialogue, architects the workflow topology, and coordinates node configuration — all through plain English dispatch. The user never touches a config screen. They have a conversation, and the workflow materializes around them.

## Core Principle: Talker / Builder Separation

Every agent in the system is either a **talker** (conversational, user-facing) or a **builder** (mutation tools, system knowledge). Never both.

| Layer | Talker | Builder |
|-------|--------|---------|
| **Manager** | Manager Assistant — converses with user, dispatches intent | Manager's Dispatched Agent — creates nodes, wires topology, writes changesets to node sessions |
| **Node** | Node Assistant — discusses its domain, asks clarifying questions | Node's Dispatched Agent — configures prompt, schema, capabilities, roster |

Talkers stay conversational — small context, natural language, no system internals. Builders stay precise — full config access, mutation tools, no conversation skills. Dispatch is the bridge between them.

This is the same pattern the workforce archetype already uses. The node assistant is the talker, the dispatch agent is the builder. The manager node adds one layer above: a workflow-level talker/builder pair that coordinates the node-level pairs.

## The Dispatch Chain

Every interaction flows through two layers of dispatch. The manager never talks directly to node assistants. The builder does.

```
User ←→ Manager Assistant (talker)
              │
              ├─ dispatch(plain English intent)
              │
              ▼
         Manager's Builder (builder)
              │
              ├─ Has full board detail (configs, ports, schemas)
              ├─ Decides which nodes are affected
              ├─ Writes changesets to affected node sessions
              ├─ Reports back to Manager what was sent
              │
              ▼
         Node Assistants receive changesets as session messages
              │
              ├─ Evaluate against their board context
              ├─ Respond: no changes / dispatching update / question
              ├─ If dispatching: node dispatches to its own builder
              │
              ▼
         Node assistant reads updated context after dispatch
              │
              ├─ Relays status + any new questions back
              │
              ▼
         Questions flow back as "current pending questions"
              → injected into Manager's system prompt
              → Manager forms conversation around filling them
```

### Why Two Layers

The manager sees **summaries** — one-liner descriptions of each node. Enough to converse, not enough to make technical decisions.

```
Manager's board view:
  - Collector: "Scrapes Acme + Widget enterprise pricing weekly"
  - Analyzer: "Compares against $50/seat baseline, flags 10%+ changes"
  - Reporter: "Executive briefing for VP Product, weekly email"
```

The builder sees **everything** — full node configs, port schemas, capabilities, connection topology. It makes the technical judgment calls: which nodes are affected, what changesets to write, how to wire ports.

The manager makes the call to act. The builder makes the call on what to do. Same separation as the workforce dispatch agent having full access to `set_task`, `add_agent`, `set_dependency` while the node assistant just talks.

## Structural Scope Enforcement

Scope is enforced by the tool set, not by prompt instructions. This is the same pattern workforce already uses.

| Agent | Tools | Scope |
|-------|-------|-------|
| Manager Assistant | `dispatch()`, `think()` | Can only dispatch — cannot touch any config |
| Manager's Builder | **Patterns:** `create_pipeline`, `create_parallel`, `insert_node` **Adjustments:** `remove_node`, `update_node`, `wire_edge` **Comms:** `dispatch_to_nodes`, `validate_dag` | Workflow topology only — cannot configure node internals |
| Node Assistant | `dispatch()`, `think()` | Can only dispatch — cannot touch its own config |
| Node's Builder | `set_task`, `set_prompt`, `set_output_schema`, `add_agent`, `set_dependency`, `set_capabilities`, `update_notes` | Its own node only — cannot touch other nodes |

Budget controls (token limits, max rounds, time limits) apply to builders the same way workforce protocol config already defines them via `max_rounds`, `max_tokens`, and `context_budget`.

## Composite Tools: Pattern-Based Topology

Individual tool calls per node and edge burn through round budgets fast — a simple 3-node pipeline takes 12+ rounds with atomic operations. The manager's builder uses **composite tools** that match how humans think about workflow structure. Each pattern is one call.

### Topology Patterns (1 call each)

Every node the manager creates is a **protocol node** — workforce today, room/meeting in the future. The protocol gives each node its internal structure: assistant session, designer pipeline, agent roster. No `mode` field needed; all nodes are workforce by default.

```
create_pipeline(nodes: [
  { name: "Collector" },
  { name: "Analyzer" },
  { name: "Reporter" }
])
→ Creates 3 workforce nodes, auto-wires edges in order
→ 1 round instead of 9

create_parallel(
  source: "Collector",
  parallel: [
    { name: "PriceAnalyzer" },
    { name: "FeatureAnalyzer" },
    { name: "SentimentAnalyzer" }
  ],
  target: "Synthesizer"
)
→ Fan-out from source, fan-in to target
→ 1 round instead of 12+

insert_node(
  between: { from: "Collector", to: "Analyzer" },
  node: { name: "Validator" }
)
→ Splices into existing edge, rewires automatically
→ 1 round instead of 4
```

### Adjustment Tools (for tweaks after patterns)

```
remove_node(name: "Validator", reconnect: true)
→ Removes node, reconnects neighbors

update_node(name: "Analyzer", protocol: "meeting")
→ Change protocol or properties on existing node

wire_edge(from: "Collector", to: "Analyzer")
→ Add a single connection
```

### Communication (always batched)

```
dispatch_to_nodes(
  nodes: ["Collector", "Analyzer", "Reporter"],
  changeset_type: "initial_instruction",
  context: "Competitor pricing monitoring workflow..."
)
→ All changesets in 1 round instead of N
```

### Few-Shot Examples (in builder prompt)

The builder's prompt includes pattern examples so it picks the right composite tool for common intents:

```
<examples>
User intent: "I need research, analysis, and a report"
→ create_pipeline([
    { name: "Researcher" },
    { name: "Analyzer" },
    { name: "Reporter" }
  ])

User intent: "Three researchers from different angles,
              then pull it together"
→ create_parallel(
    source: null,
    parallel: [
      { name: "MarketResearcher" },
      { name: "TechResearcher" },
      { name: "CompetitorResearcher" }
    ],
    target: { name: "Synthesizer" }
  )

User intent: "Add a fact-checker between research and analysis"
→ insert_node(
    between: { from: "Researcher", to: "Analyzer" },
    node: { name: "FactChecker" }
  )

User intent: "Make the meeting node a room for discussion"
→ update_node(name: "ReviewMeeting", protocol: "meeting")
→ dispatch_to_nodes(
    nodes: ["ReviewMeeting"],
    changeset_type: "update",
    context: "Converting to meeting protocol. Set up
              roles and discussion agenda..."
  )
</examples>
```

Patterns handle 80% of cases in 1-2 calls. Adjustment tools handle the remaining 20%. A typical builder session goes from 12-25 rounds down to 2-4.

### Idempotent Disregard

Every tool checks current state before mutating. If the request matches what already exists, the tool returns a disregard response — no error, no duplicate, just acknowledgment:

```
create_pipeline([{name: "Collector"}, {name: "Analyzer"}, {name: "Reporter"}])
→ "Pipeline already exists: Collector → Analyzer → Reporter. No changes made."

wire_edge(from: "Collector", to: "Analyzer")
→ "Edge already exists. No changes made."

set_task("Analyze competitor pricing trends...")
→ "Task already set to this value. No changes made."

dispatch_to_nodes(["Collector"], "initial_instruction", ...)
→ "Collector already received initial instruction. No changes made."
```

The builder sees "already done" and moves on — no wasted rounds. This protects against user repetition (manager dispatches the same thing twice), belief cycles re-triggering processed changesets, and builder retries after partial failures.

The principle: **check before mutate, disregard if unchanged.**

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

### Phase 2: Requirements Gathering (Manager → Builder → Node Assistants)

The manager dispatches to its builder. The builder writes changesets to each node assistant's session.

```
Manager dispatches:
  "Set up each node for competitor pricing monitoring.
   Collector gathers data, Analyzer finds trends,
   Reporter produces briefings."
```

The builder has full board context. It writes targeted changesets to each node session:

**Collector's session receives:**
```
[From Agent: Workflow Manager]
Changeset #c8f1 | type: initial_instruction

You are the data collection node in a competitor pricing
monitoring workflow. Your job is to gather competitor
pricing data from the web on a weekly basis.

Downstream: Analyzer will receive your output to identify
trends and anomalies.

Please review your board context and flag any questions
about what you need to get started.
```

**Analyzer's session receives:**
```
[From Agent: Workflow Manager]
Changeset #c8f2 | type: initial_instruction

You are the analysis node in a competitor pricing
monitoring workflow. You'll receive raw pricing data
from Collector and identify trends, anomalies, and
competitive positioning.

Downstream: Reporter will receive your analysis to produce
executive briefings.

Please review your board context and flag any questions.
```

**Reporter's session receives:**
```
[From Agent: Workflow Manager]
Changeset #c8f3 | type: initial_instruction

You are the reporting node in a competitor pricing
monitoring workflow. You'll receive analysis from Analyzer
and produce executive-ready weekly briefings.

Please review your board context and flag any questions.
```

The builder reports back to the manager what was sent:
```
Builder → Manager:
  "Sent initial instructions to 3 nodes:
   - Collector: data gathering role
   - Analyzer: trend/anomaly analysis role
   - Reporter: executive briefing role
   Waiting on responses."
```

Each node assistant sees its **board context** — where it sits in the DAG, incoming port summaries, outgoing port summaries, connected resources — and responds with informed questions:

```
Collector: "I'll need competitor names and URLs to
 scrape. Which products or tiers should I track?"

Analyzer: "I can see I'm getting records from Collector.
 I need to know: compare against current or historical
 prices? What % change counts as an anomaly?
 Reporter needs positioning scores — what scale?"

Reporter: "Who's the audience? What delivery format?"
```

### Phase 3: Synthesis (Questions → Manager → User)

Each node auto-responds to its changeset (background LLM call). Haiku compresses each response into a 1-2 sentence status + optional pending question. These go directly into the manager's prompt — no deduplication pipeline, the manager LLM handles that naturally in conversation.

```
Manager System Prompt (dynamically updated):

You are the manager for this workflow. Help the user
define what they need through natural conversation.

Your team:
  Collector:
    Status: "Ready for web scraping, weekly cadence"
    Question: "Need competitor names, URLs, and which tiers"

  Analyzer:
    Status: "Configured for pricing analysis"
    Question: "Compare current or historical? What % threshold?"

  Reporter:
    Status: "Waiting on details"
    Question: "Who's the audience? What format?"

Guide the conversation to resolve pending questions
naturally. When you have enough, dispatch the answers.
```

The manager weaves these into organic dialogue — not a checklist:

```
Manager: "Which competitors are top priority for you?"

User: "Acme and Widget Inc, enterprise tier only"

Manager: "Got it. And when your team spots a pricing change,
 what kind of move would you want to hear about — any change,
 or only significant ones?"

User: "Only big moves. 10% or more."

Manager: "Makes sense. One more thing — these weekly reports,
 who's reading them? That'll shape how they're formatted."

User: "VP of Product. She wants to know if we should
 adjust our pricing."
```

### Phase 4: Node Configuration (Manager → Builder → Nodes → Node Builders)

When the manager has enough information, it dispatches. The builder writes update changesets to the relevant node sessions.

```
Manager dispatches:
  "Competitors are Acme and Widget Inc, enterprise tier only.
   Anomaly threshold is 10%+. Reports go to VP Product for
   pricing adjustment decisions."
```

The builder sends targeted updates:

**Collector's session:**
```
[From Agent: Workflow Manager]
Changeset #c8f4 | type: update

Competitors confirmed:
  - Acme Corp
  - Widget Inc
Enterprise tier only. Weekly cadence.
```

**Analyzer's session:**
```
[From Agent: Workflow Manager]
Changeset #c8f5 | type: update

Answers to your questions:
  - Compare against our current pricing (baseline: $50/seat)
  - Anomaly threshold: 10%+ price change
  - Enterprise tier only
```

**Reporter's session:**
```
[From Agent: Workflow Manager]
Changeset #c8f6 | type: update

Audience: VP of Product
Purpose: Inform pricing adjustment decisions
Format: Executive briefing, weekly
```

Each node assistant reads the changeset, verifies it has enough to configure itself, and dispatches to its own builder:

```
Analyzer Assistant → dispatch:
  "Configure this node to analyze enterprise SaaS pricing.
   Compare against our baseline of $50/seat. Flag changes
   over 10%. Input is raw pricing records from Collector.
   Output positioning scores and anomaly flags to Reporter."

Analyzer's Builder:
  - set_task("Analyze competitor pricing trends...")
  - set_output_schema({anomalies: [...], positioning: [...]})
  - set_capabilities(["data_analysis", "database"])
  - update_notes("Threshold: 10%, baseline: $50/seat...")
```

After each builder completes, the node assistant reads its updated context and follows up — relaying status and any new questions as **current pending questions** back to the manager:

```
Analyzer: "Configured. No further questions."
  → Analyzer's pending questions clear from manager's prompt
  → Manager sees: "Analyzer: configured, ready"

Collector: "Configured for Acme and Widget enterprise tier.
 But I need the actual scraping URLs."
  → New pending question appears in manager's prompt
```

### Phase 5: Mid-Conversation Changes

The user changes their mind. The flow is the same — manager dispatches, builder evaluates blast radius with full detail, writes changesets, node assistants verify.

```
User: "Actually, I want to track 5 competitors now
       and include free tier pricing too."

Manager dispatches:
  "User wants 5 competitors (was 2) and include
   free tier pricing alongside enterprise."
```

The builder evaluates with full knowledge — it sees every config, port, and schema. It makes the judgment call on which nodes are affected:

**Collector's session:**
```
[From Agent: Workflow Manager]
Changeset #c8f7 | type: update

Expanding competitor tracking:
  - 5 competitors total (previously 2: Acme, Widget Inc)
  - 3 new competitors TBD — names coming
  - Include free tier pricing alongside enterprise
```

**Analyzer's session:**
```
[From Agent: Workflow Manager]
Changeset #c8f8 | type: upstream_change

Collector is being updated:
  - 5 competitors instead of 2
  - Free tier pricing data alongside enterprise
  - Output will include a new "tier" field per record

Your current config compares enterprise pricing against
a $50/seat baseline and flags 10%+ changes. With free
tier data incoming, your analysis approach may need
to change.

Can you still do your job, or do you need to adjust?
```

**Reporter's session:**
```
[From Agent: Workflow Manager]
Changeset #c8f9 | type: upstream_change

Upstream changes in progress:
  - Competitor count increasing from 2 to 5
  - Free tier data being added to analysis

Your current format produces executive briefings.
Flagging in case report structure needs to change.
```

Node assistants verify against the changeset. This is evaluation, not automatic action — sometimes the answer is "I'm fine":

```
Collector: "I can handle more competitors. Need the
 3 new names and URLs."

Analyzer: "Free tier is fundamentally different from
 enterprise. Should I analyze them separately? Does
 10% threshold apply to free tier?"

Reporter: "No changes needed. My format handles
 variable competitor counts."
```

The builder reports back to the manager what it sent. Questions flow into the manager's pending questions. The conversation continues naturally until everything is resolved.

### Phase 6: Belief-Based Change Propagation

When a node's builder reconfigures and the output port changes, the change is recorded as a **belief** — not cascaded in real time.

#### Why Not Real-Time Cascading

Broadcasting changes immediately creates problems:
- Cascading updates — one change triggers downstream changes that trigger more changes
- User interruption — the user might be focused on a different node
- Ordering issues — responses arrive out of sequence
- Research shows proactive agent actions are more disruptive than helpful

#### The Belief Model

```
Collector's builder finishes → output port "raw_data" schema changed
  → Belief recorded: "Collector now outputs historical pricing data.
     New fields: price_history[]. Previous: current prices only."
  → Affected nodes: [Analyzer]
  → Belief state: unacknowledged
```

The belief is not dispatched. It sits on the board. When any conversation touches an affected area — user opens the session, builder writes a changeset, or pre-run verification runs — the belief is already in context.


```
User opens Analyzer's session later:
  → Analyzer's context loads with current beliefs
  → Pending belief visible in board context
  → Analyzer assistant sees it naturally:
    "I notice Collector now includes historical pricing
     that wasn't there when I was configured. Want me
     to update my approach to include trend analysis?"
```

This follows the same pattern as session summarization — context from one conversation feeds into the next conversation, not into the current one.

#### Belief Lifecycle

```
Belief: "Collector now outputs historical pricing"
  created: 2:15pm (when Collector was reconfigured)
  acknowledged_by:
    - Analyzer: null       ← hasn't been revisited
    - Reporter: 3:30pm     ← user confirmed no changes needed
  state: partially_acknowledged
```

When acknowledged, the belief doesn't propagate further — unless the acknowledging node's output port also changes, creating a new belief downstream.

#### Propagation Control

Propagation only continues when an output port **actually changes** — not when internals change.

```
Analyzer reconfigures in response to upstream change:
  - Prompt: changed (added historical trend logic)
  - Capabilities: changed (added time_series_analysis)
  - Output schema: same (still {anomalies, positioning})

  Port diff: no change → no downstream belief created
  Cascade stops here.
```

### Phase 7: Pre-Run Verification

Before execution, the system verifies that all nodes are current. This is a build gate — the workflow has to pass the test before it can run.

```
User clicks "Run"
        │
        ▼
  Pre-run verification
        │
        ├─ For each node:
        │   ├─ Are all required input ports connected?
        │   ├─ Do upstream output schemas match expectations?
        │   ├─ Are there unacknowledged belief changes?
        │   ├─ Are all required fields configured?
        │   └─ Pass / Fail / Warning
        │
        ├─ Question gate (fresh, not cached):
        │   ├─ Run extraction on ALL nodes — no cache,
        │   │   this is a verification gate
        │   ├─ Any node has unresolved questions?
        │   │    YES → Block run
        │   │    "Collector still needs scraping URLs"
        │   │    "Analyzer needs clarification on tier comparison"
        │   │    NO  → Pass
        │   │
        │   Why fresh? A node's questions could have been
        │   answered seconds ago via direct conversation.
        │   The cache might not reflect that. The gate is
        │   the one place you pay for a full sweep.
        │
        ├─ All pass → Execute
        │
        ├─ Failures → Block run, show what's broken
        │   "Analyzer expects current-only pricing but
        │    Collector now outputs historical. Reconfigure
        │    Analyzer before running."
        │
        └─ Warnings → User decides
            "Reporter was configured before Analyzer added
             trend data. Run anyway?"
```

If nothing changed since last configuration and no questions are open, every node is green. The check is instant — nothing to verify. The question gate and belief check only fire for nodes with activity since last verification.

### Phase 8: Execution

The workflow runs through the normal DAG — topological sort, port resolution, data flow. No special handling needed. The manager node doesn't participate in execution. It configured the system; now the system runs.

### Phase 9: Feedback & Evolution

> **Future consideration:** Simulation mode, dry-run capabilities, and manager-initiated test runs are deferred. The execution and testing model will be designed once the core conversation and configuration patterns are proven.

After a run, node assistants see results and can message each other or the manager:

```
Analyzer → Collector: "12 of 187 records had missing price
 fields. Consider adding validation before output."

Analyzer → Manager: "Run complete. 3 anomalies flagged
 but 2 were data errors. Might need a data quality node
 between Collector and me."

Manager ←→ User: "The team found some data quality issues.
 Want me to add a validation node between collection
 and analysis?"
```

The workflow evolves through conversation, not reconfiguration.

## Changesets: The Universal Message Format

All communication from the manager's builder to node sessions uses **changesets** — a structured message format that appears in the node's session as an agent message.

### Message Format in Session

Changesets appear in node sessions as messages from the Workflow Manager:

```
[From Agent: Workflow Manager]
Changeset #c8f5 | type: update

Answers to your questions:
  - Compare against our current pricing (baseline: $50/seat)
  - Anomaly threshold: 10%+ price change
  - Enterprise tier only
```

The node assistant processes this the same way it processes any other message — from the user, from another node, from a scheduled event. One input path, one processing model. The user can open any node's session and see every changeset received, every response sent, every dispatch triggered.

### Changeset Structure

```
Changeset {
  id: uuid
  source: "manager" | "node:{node_name}" | "system"
  type: "initial_instruction" | "update" | "upstream_change"
        | "feedback" | "peer_message"

  context: {
    what_changed: "Collector output now includes historical pricing"
    why: "User requested historical data tracking"
    diff: {
      port: "raw_data"
      previous: "Current pricing only"
      current: "Current + historical, adds price_history[]"
    }
  }

  affected_nodes: ["analyzer"]

  expects_response: true
  response_options: ["no_changes_needed", "dispatching_update", "question"]
}
```

### Changeset Types

| Type | When | Example |
|------|------|---------|
| `initial_instruction` | Builder first assigns the node | "You are the analysis node..." |
| `update` | Builder sends answers from user | "Threshold is 10%, baseline $50/seat" |
| `upstream_change` | A connected port's config changed | "Collector now includes free tier data" |
| `feedback` | Post-run results | "Last run: 3 anomalies, 2 were false positives" |
| `peer_message` | Another node sends a message | "Collector says: added data validation" |

### Response Contract

Node assistants respond to changesets with one of:

```
no_changes_needed  → Belief acknowledged. No propagation.
dispatching_update → Node dispatches to its own builder.
                     After completion, system checks if output
                     port changed. If yes, new belief created.
                     If not, propagation stops.
question           → Flows to synthesis → manager's system prompt
                     → manager → user → answer dispatched back.
```

After responding, the node assistant reads its updated context and relays current status and any new pending questions back to the manager.

## Question Framework

Questions are the feedback loop between nodes and the manager. The design is simple: compress each node's response, show it to the manager, let the LLM handle the rest.

### How It Works

When a changeset lands on a node, the node assistant auto-responds (background LLM call). Haiku compresses that response into 1-2 sentences: a **status** and an optional **pending question**.

```
Collector responds: "I'll need competitor names and URLs to
  scrape. Which products or tiers should I track? I can
  handle weekly cadence, my scraping tools support..."

Haiku compresses:
  Status:   "Ready for web scraping, weekly cadence"
  Question: "Need competitor names, URLs, and which tiers"
```

One cheap Haiku call per node response. No batch pass. No structured dedup.

### What the Manager Sees

The compressed status + question for every node goes directly into the manager's prompt:

```
Your team:
  Collector:
    Status: "Ready for web scraping, weekly cadence"
    Question: "Need competitor names, URLs, and which tiers"

  Analyzer:
    Status: "Configured for pricing analysis"
    Question: "Compare current or historical? What % threshold?"

  Reporter:
    Status: "Waiting on details"
    Question: "Who's the audience? What format?"
```

The manager LLM naturally deduplicates in conversation. It reads "Collector needs competitor names" and "Analyzer needs company names" and asks the user one question that covers both. No binder pipeline needed — that's what LLMs are good at.

### What the Builder Sees

The builder's `<board_state>` reuses the same compressed status combined with the board overview role (what the node does). No full notes, no port schemas, no json_path — just a one-liner per node:

```
<node name="Collector" protocol="workforce" status="configured"
      agents="WebScraper, DataCleaner">
  Scrapes competitor pricing data weekly — Configured for Acme + Widget, enterprise tier
</node>
<node name="Analyzer" protocol="workforce" status="configured" receives="Collector"
      agents="TrendAnalyzer, AnomalyDetector">
  Identifies pricing trends and anomalies — Configured, 10% threshold, $50/seat baseline
</node>
```

The `role` comes from the board overview summary (already exists). The `compressed_status` comes from the question framework compression. Concatenated at prompt build time — no extra LLM call.

### Questions Are Generational

Questions are **derived** from each node's latest conversational state. Every time a node's conversation advances — user message, changeset, builder completion — the compression re-runs on the new state.

No invalidation logic. No staleness checks. No expiration timers. The node's current state IS the question list.

| Scenario | What happens |
|----------|-------------|
| User talks to node directly, answers questions | Node's state advances → re-compress → question gone → manager sees updated status on next prompt build |
| New changeset supersedes old context | Node gets new message → state advances → old status/question replaced |
| User pivots the entire workflow | New changesets to all nodes → all states advance → everything reflects new direction |

### Dispatch State Tracking

The builder tracks progress through a changeset round:

```
dispatch_pending    → changesets sent, waiting on responses
  ├─ 1 of 3 responded → Haiku compresses immediately
  ├─ 2 of 3 responded → Haiku compresses immediately
  ├─ 3 of 3 responded → all_responded
  │
  ▼
questions_ready     → manager prompt updated, ready for conversation
```

The manager sees "dispatch active" during the window, so it doesn't try to send more instructions while nodes are still responding.

## Prompt Hierarchy

Full system prompts for all four layers are defined in [`manager-node-prompts.md`](manager-node-prompts.md).

| Layer | Agent | Key Sections | Context Size |
|-------|-------|-------------|-------------|
| **1** | Manager Assistant | `<team>` status/questions, `<dispatch_status>`, `<protocols>` | Small — summaries only |
| **2** | Manager's Builder | `<board_state>` full detail, `<protocols>` detailed, `<examples>` | Large — full board |
| **3** | Node Assistant | `<board_context>`, `<your_notes>`, `<pending_beliefs>`, `<execution_pipeline>` | Own neighborhood |
| **4** | Node's Builder | `<node_config>` full detail, `<examples>` | Own node only |

Scope is enforced structurally — talkers (Layers 1, 3) get only `dispatch()` and `think()`. Builders (Layers 2, 4) get mutation tools scoped to their level. The manager's builder cannot configure node internals; the node's builder cannot touch other nodes.

## Agent-to-Agent Communication

Every node is a session. Every session is addressable. Messages between agents use the same format as user messages.

```
Sources of messages to any node assistant:
  - User (opens the session in the UI)
  - Manager's builder (changesets via dispatch_to_nodes)
  - Other node assistants (cross-node issues or requests)
  - System (belief notifications at context load time)
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
- **Conversation channels** (ambient): The manager's presence. No explicit lines — maybe a glow or highlight showing which nodes have active threads.

Nodes show indicators for:
- Active conversation threads
- Unacknowledged beliefs (pending changes)
- Pre-run verification status (pass/fail/warning)
- Configuration status (unconfigured / has questions / ready)

The manager might not even need visible connections. Its presence on the board implies it can talk to anything.

## Build Order: One Node at a Time

The manager walks the user through nodes sequentially, following the natural flow of the DAG. By the time a downstream node is being configured, all upstream nodes already have port summaries defined.

```
1. Manager creates Collector → user configures it
2. Manager creates Analyzer → sees Collector's output summary
3. Manager creates Reporter → sees Analyzer's output summary
```

No node needs runtime data to be configured. Port summaries are the interface contract. Runtime data matters later — for feedback, iteration, and the self-improvement loop after the first run.

## What Already Exists

- **Dispatch model**: Chat assistant → dispatch → background builder agent. Working pattern for workforce today.
- **Structural scope enforcement**: Workforce builders get only node-scoped tools. Same pattern extends to manager.
- **Budget controls**: Protocol config defines `max_rounds`, `max_tokens`, `context_budget` per builder.
- **Chat sessions**: Every node can have a conversational session (`chat_sessions` table). Messages stored, history preserved.
- **Workforce tools**: `set_task`, `add_agent`, `set_dependency`, `update_notes` — mutation tools for node builders.
- **Pipeline service**: Child workflow CRUD, topological sort, cycle detection.
- **Port system**: Typed ports with json_path extraction, conditional edges, variable interpolation.
- **Protocol engine**: `ProtocolCompiler` trait, `ProtocolExpansion` output, `apply_protocol` materializer.
- **Templates**: `WorkflowSnapshot` captures and restores full workflow configurations.
- **WebSocket events**: Live broadcast of dispatch progress, node updates, configuration changes.
- **Belief extraction**: Existing system for summarizing conversations into small truths that persist across sessions.
- **Capability safety levels**: `safe`, `caution`, `unsafe` classifications in `config/capabilities.yaml`.

## What Needs To Be Built

1. **Composite topology tools**: Pattern tools (`create_pipeline`, `create_parallel`, `insert_node`) that handle 80% of topology operations in 1 call. Adjustment tools (`remove_node`, `update_node`, `wire_edge`) for tweaks. Batched communication (`dispatch_to_nodes`). Few-shot examples in builder prompt for common workflow patterns. Thin wrappers around existing services.

2. **Manager strategy**: A new `DispatchStrategy` variant with workflow-scoped tools. System prompt includes full board detail for the builder. Budget controlled by protocol config.

3. **Board overview roles**: One-liner descriptions of what each node does, derived from board overview summary (`get_board_overview_summary()` already exists). Combined with compressed status at prompt build time to produce the builder's `<board_state>` — no full notes, no port schemas, just role + state per node.

4. **Question framework**: Haiku-powered compression of each node assistant's response into 1-2 sentence status + optional pending question. Generational — re-derived from latest node state on every manager prompt build. Feeds both the manager's `<team>` block (status + question) and the builder's `<board_state>` (compressed status). No deduplication pipeline; the manager LLM handles that naturally in conversation.

5. **Changeset system**: Structured message format for builder-to-node communication. Messages appear in node sessions as `[From Agent: Workflow Manager]`. Includes types (`initial_instruction`, `update`, `upstream_change`, `feedback`, `peer_message`), response contracts, and state tracking.

6. **Belief change tracking**: Records output port changes as beliefs with per-node acknowledgment state. Beliefs surface at context load time (next conversation), not in real time. Integrates with existing belief extraction system.

7. **Pre-run verification**: Build gate that checks all nodes for unacknowledged beliefs, port mismatches, missing configuration, and **unresolved questions** (fresh extraction, not cached) before workflow execution. Instant pass if nothing changed and no questions are open. Blocks on failures, warns on stale configuration.

8. **Cross-session messaging**: Ability for the manager's builder to write messages to node sessions, and for node assistants to message each other. Same format as user messages. Routed through existing chat message infrastructure.

9. **Manager visualization**: Board-level UI for the manager node — ambient connections, active conversation indicators, belief status per node, pre-run verification status, configuration completeness.
