# Visual Dispatch — Vision

## What It Is

Visual Dispatch turns the workflow canvas into a persistent drawing surface and pairs it with a tree-first sidebar for structured navigation, configuration, and execution output. The user sketches what they want — boxes, arrows, annotations — and those drawings stay exactly where the user put them as a static visual guide. The system reads the drawings, builds the workflow structure instantly, then designs the agents asynchronously.

The canvas is the user's whiteboard. The sidebar is the system's structured response.

## Why This Matters

Today, the only way to instruct the manager is through conversation. That works well for simple requests ("add a node that summarizes the output") but breaks down for structural intent. Try describing a four-node DAG with conditional branching in a chat message. The user knows exactly what they want — they can see it in their head — but translating spatial relationships into words is friction.

Visual Dispatch removes that translation step. The user draws what's in their head. The system reads the drawing and builds the structure. The drawings never disappear, never convert, never shake around. They stay right where the user put them — a persistent reference for both the user and the system.

## The Layout

```
┌─────────────────────────────────────┬──────────────────────┐
│                                     │                      │
│          Canvas                     │      Sidebar         │
│                                     │                      │
│  Full drawing surface.              │  Tree Tab (primary)  │
│  Excalidraw overlay.                │  ── Research Team    │
│  User's whiteboard.                 │     ├── Web Search   │
│                                     │     ├── Paper Review │
│  ╭ ─ ─ ─ ─ ─ ╮     ╭ ─ ─ ─ ─ ╮   │     └── Fact Check  │
│  ╎ research    ╎────>╎ write    ╎   │  ── Write Report     │
│  ╎ competitors ╎     ╎ report   ╎   │  ── Output           │
│  ╰ ─ ─ ─ ─ ─ ╯     ╰ ─ ─ ─ ─ ╯   │                      │
│                                     │  Config Panel        │
│  Drawings stay. Always.             │  (click a node)      │
│                                     │  System prompt, tools│
│                                     │  capabilities, etc.  │
│                                     │                      │
│                                     │  Chat (secondary)    │
│                                     │  > Type a message... │
│                                     │                      │
└─────────────────────────────────────┴──────────────────────┘
```

Three surfaces, each with a clear job:

- **Canvas** — full drawing surface. Always visible. The user's whiteboard. Drawings are permanent until the user deletes them. No conversion, no replacement, no animation.
- **Tree tab** (sidebar, primary) — the actual workflow structure rendered as a file tree. Shows every node, its hierarchy, its design status. This is how the user navigates the workflow.
- **Config panel** (sidebar) — click a node in the tree, see its full agent design. System prompt, capabilities, tools, routing rules, agent roster. Edit anything before execution.
- **Chat** (sidebar, secondary) — still there for users who prefer typing. Same dispatch pipeline.

## The Drawing Layer

### Always On

The doodle tools live in the toolbar permanently. Rectangle, arrow, text. The user can draw at any time without entering a special mode. Doodle elements are visually distinct from the sidebar's structured data — they look hand-drawn, sketchy, with rough borders and wobbly lines.

### Drawings Are Permanent

This is the key difference from a "convert sketches to nodes" model. The user draws boxes and arrows on the canvas. Those drawings **stay exactly where they are**. They don't convert to real nodes. They don't fade out. They don't move.

The drawings are the user's visual reference — their blueprint. The system reads them and builds the structured workflow in the sidebar. The user can always look at their canvas and see exactly what they drew. They can keep drawing, annotating, adding notes. The canvas is theirs.

### What Users Can Draw

- **Boxes with text** — describe what a node should do
- **Arrows** — describe the flow between nodes
- **Annotations on existing drawings** — refine, add detail, add context
- **Pictures, diagrams, freeform sketches** — whatever helps them think
- **Comments** — notes to themselves or to the system

The system extracts structure from boxes and arrows. Everything else is ignored by the system but preserved for the user.

## The Dispatch Pipeline

Two entry points, different pipelines converging at the Per-Node Builder. The user can talk to the manager assistant OR draw on the board.

### Entry Point 1 — Chat

The user talks to the manager assistant conversationally. The manager dispatches an instruction to the builder pipeline.

```
User: "Hey I want to add something to my refiner but I cant decide what?"
Manager: "Based on your pipeline, a verification step after the refiner
          would catch errors before output. Want me to add an online
          verifier that cross-checks the refined content?"
User: "Yeah that sounds good, do it."
Manager: dispatch("Add an online verifier for the refiner")
  → Builder Agent: creates the node, wires edges, fills in description → passdown
  → Per-Node Builder: configures the workforce team (roster, dependencies, plan)
  → Designer: writes per-agent prompts to .system/design/
```

### Entry Point 2 — Board Submit

The user draws on the canvas and hits submit. Phase 0 handles the structural diff agentlessly, then the Board Dispatcher kicks off per-node configuration.

```
User: draws complete schema on canvas → submits
POST → Phase 0: create nodes, delete removed ones, rewire edges (agentless DB writes)
  → Board Dispatcher: reads changeset, dispatches to each node
  → Per-Node Builder: configures the workforce team (roster, dependencies, plan)
  → Designer: writes per-agent prompts to .system/design/
```

### Phase 0 — Structural (Board Submit Only, Instant, Agentless)

Only runs on board submit. Phase 0 has two layers: the serializer pipeline and the structural executor.

**Serializer pipeline:** A 4-pass classifier reads raw Excalidraw elements and extracts structured data:

1. **Nodes** — rectangles with bound text. Captures `raw_text` (full box content the user wrote), `bounds` (canvas position/size).
2. **Edges** — arrows with both endpoints bound to nodes. Captures `source_node_id`, `target_node_id`.
3. **Annotations** — free-floating text assigned to the nearest node within 100px by spatial proximity. Text beyond the threshold becomes a `GlobalNote` (board-level context).
4. **Sketches** — freeform drawings inside node bounds, rasterized to ASCII art via Bresenham line algorithm.

The classifier produces a `CanvasSnapshot`, which is diffed against the previous snapshot (persisted per workflow). The diff is filtered through whitespace normalization, oscillation detection, pan detection, reorder detection, and token scoring (Myers + Sørensen-Dice hybrid). The output is a `FilteredChangeset` with three tiers:

- **Agentless** — deletes, rewires, moves (pure DB writes, no AI)
- **Noise** — filtered out (whitespace-only, oscillation, pan, reorder)
- **Meaningful** — new nodes, updated nodes, new edges (scored by significance, topologically sorted)

**Structural executor:** Takes the `FilteredChangeset` and acts on it:

- Execute agentless tier as DB writes (delete nodes/edges, rewire edges, update positions)
- Create new nodes in the DB with the user's content, annotations, and sketches preserved
- Update existing nodes when text or annotations change (name, prompt, board context)
- The frontend updates from the POST response — no WebSocket events for Phase 0

Nodes appear in the tree tab as soon as the user hits submit. This is agentless. Pure mechanical diff → DB writes. The board skeleton is built, with the user's original content and annotations intact.

The chat entry point skips Phase 0 — the manager dispatches directly to the Builder Agent.

### The Dispatch Agents

Three dispatch agents plus the Designer. The chat path uses Builder → Per-Node Builder → Designer. The board path uses Board Dispatcher → Per-Node Builder → Designer. The Per-Node Builder and Designer are shared by both paths.

**Builder Agent** — Chat path only. Creates the workflow structure (nodes, edges, hierarchy) AND fills in the box descriptions — the text that represents each node's job on the canvas. Uses topology tools (`create_pipeline`, `insert_node`, `wire_edge`, etc.) and content tools in a single pass. Produces a passdown with the finalized schema. **The board is locked** while this agent works.

**Board Dispatcher** — Board path only. Reads the changeset from Phase 0 — which nodes were created, which were updated, what the user wrote. Dispatches configuration instructions to the Per-Node Builder for each affected node. No topology tools — the topology is already built by Phase 0. A simple, focused agent whose only tools are `dispatch_to_builders` and `think`.

**Per-Node Builder** — Both paths. Receives a dispatch instruction for a single node. Reads the **full box content** — raw text, annotations, sketches. Configures the workforce: agent rosters, dependencies, capabilities, and a plan. Writes configs to the system store (`.system/design/`). Already built.

**Designer** — Both paths. Runs async after the Per-Node Builder completes. Retrieves relevant orchestration examples from the example library (@visions/vision-example-library.md) via embedding similarity, then writes per-agent prompts (system prompt, assignment, expected_output, tools) to the system store as a ReAct agent. Reads back prior configs to verify format chain coherence. Prompts appear in the config panel for user review before execution. See @visions/vision-system-store.md.

After design, each node is ready for workforce execution — the executor reads prompts from the store.

### Why This Agent Split

Anthropic's multi-agent research (2025) found that each agent in a chain should have a *meaningfully different job requiring different context*. If two agents read the same context and do similar reasoning, they should be one agent. Each handoff adds 100-500ms latency and risks information loss — the "game of telephone" problem where each summarization step loses nuance.

The Builder Agent combines topology and content creation into one pass because both tasks require the same context: the user's instruction, the current board state, and knowledge of what nodes and edges need to exist. Splitting them into two agents would mean two LLM calls reading the same board state, with a passdown between them that adds latency and loses context.

The Board Dispatcher is separate from the Builder because it does a fundamentally different job — the topology already exists, it just needs to fan out dispatch instructions. And the Per-Node Builder is separate because it operates at a different scope (single node vs. whole workflow) and needs different context (full box content, upstream outputs, beliefs).

The Designer is separate from the Per-Node Builder because it has a meaningfully different job — the builder decides WHO is on the team, the designer decides HOW they work. The builder reads full box content, beliefs, and upstream topology. The designer reads the builder's plan, the roster, and retrieved orchestration examples. Different context, different output, clear handoff: the builder's plan IS the designer's input.

The industry consensus (Google ADK, OpenAI Agents SDK, LangGraph) is: start with the fewest agents that have meaningfully distinct jobs, and only split when you can prove the split improves output quality. Mechanical work that needs no reasoning (Phase 0) should never be an agent. Coordination overhead between agents is real cost — minimize handoffs.

### The Beliefs Layer

The dispatch agents don't read walls of box text to understand board-level context. They read **beliefs** — structured, tagged, confidence-rated knowledge extracted automatically from every conversation on the board.

Beliefs accumulate from two sources:
- **User chats** — when a user talks to a node's assistant, Haiku extracts beliefs in the background
- **Builder conversations** — when agents work on nodes, their conversations also trigger belief extraction

Each belief is tagged by type (fact, goal, requirement, risk, assumption), confidence level (low, medium, high), and semantic tags. Contradictions between nodes are flagged as cross-source tensions.

The Builder Agent sees beliefs from all connected nodes via `get_beliefs_for_connected_steps()`. This gives it a compressed, structured understanding of what each node is about — without reading the full box content. The Per-Node Builder gets both beliefs AND full box content, because it needs the detail to configure the workforce.

### Cost Model

| Phase | What | Model | Speed |
|-------|------|-------|-------|
| 0 — Structural (board only) | Diff → DB writes | No LLM | Instant |
| Builder Agent (chat only) | Create topology + fill content | Smart, one pass | Seconds |
| Board Dispatcher (board only) | Read changeset, dispatch to per-node | Smart, one pass | Seconds |
| Per-Node Builder (both) | Read full box content, configure workforce | Smart, per node | Seconds |
| Example Retrieval (both) | Embed task, query example library | No LLM (embedding only) | Instant |
| Designer (both) | Write per-agent prompts to store | Smart, ReAct, per node | Seconds |
| Execution | Run each node's workforce | Per-node model selection | Background |

## The Guided Engineering Pipeline

This is not "draw and execute." It's "draw, design, review, refine, then execute."

### The Workflow

1. **Draw** — user sketches their workflow on the canvas
2. **Submit** — system builds the structure instantly, designs agents async
3. **Review** — user clicks nodes in the tree, reads the agent designs in the config panel
4. **Refine** — user edits system prompts, adjusts capabilities, tweaks routing
5. **Execute** — when the user is satisfied, they run the workflow

The execution is the last step, not the automatic next step. The system does the heavy lifting of designing agents, but the user has full visibility and control before anything runs. The config panel is the inspection and editing surface for each node's design.

### Design Status in the Tree

The tree shows more than just structure. It shows design status:

```
── Research Pipeline
   ├── Web Search          ● designed
   ├── Paper Review        ● designed (edited)
   ├── Fact Check          ◐ designing...
   └── Write Report        ○ pending
```

The user can see at a glance which nodes have been designed, which ones they've manually edited, which are still being designed, and which are waiting. No guessing, no checking individual panels.

## The Sidebar

### Tree Tab (Primary)

The tree is the primary navigation surface. It shows the full workflow structure as a file tree — nodes, their hierarchy, their relationships. This is the `AsciiTree` renderer, the same component already built in `frontend/src/utils/AsciiTree.ts`.

Clicking a node in the tree opens its config panel. The tree is always visible while the config panel shows the selected node's details below it.

### Config Panel

Click a node in the tree. The config panel shows everything about that node's design:

- **Name and type** — workforce, single, sub_workflow, etc.
- **System prompt** — the full prompt the agent will use
- **Capabilities** — what tools the agent has access to
- **Routing rules** — how outputs flow to downstream nodes
- **Agent roster** (workforce nodes) — the team hierarchy, each agent's role
- **Constraints** — token limits, model selection, temperature

Everything is editable. The user can rewrite system prompts, add or remove capabilities, restructure the agent roster. The design agent gives them a starting point. They refine it.

### Chat (Secondary)

The chat is still there. Some users will always prefer typing. The chat feeds into the same dispatch pipeline — the manager builder receives the message and acts on it. But the tree and config panel are the primary interface. Chat is for conversational refinement, not primary navigation.

## Execution Output Stream

When the user runs the workflow, the output renders as a vertical stream of full-width document blocks in the sidebar. Each block is a step's output — plain English or code, rendered cleanly.

A tree gutter runs along the left edge showing the pipeline flow. When steps run in parallel, the gutter branches. When they merge, it rejoins.

```
│ ┌─────────────────────────────────────────┐
│ │ Research Results                        │
│ │                                         │
│ │ Found 12 competitor pricing entries     │
│ │ across 4 major providers. The Q4 data  │
│ │ shows a 15% average price increase...  │
│ └─────────────────────────────────────────┘
│
├─┬─┌─────────────────────────────────────────┐
│ │ │ 1A · Web Search                         │
│ │ │                                         │
│ │ │ Scraped 4 competitor sites. Found       │
│ │ │ current pricing for all tiers...        │
│ │ └─────────────────────────────────────────┘
│ │
│ ├─┌─────────────────────────────────────────┐
│ │ │ 1B · Paper Review                       │
│ │ │                                         │
│ │ │ Analyzed quarterly earnings report.     │
│ │ │ Key finding: margin compression in      │
│ │ │ enterprise tier...                      │
│ │ └─────────────────────────────────────────┘
│ │
│ └─┌─────────────────────────────────────────┐
│   │ 1C · Fact Check                         │
│   │                                         │
│   │ Verified 11 of 12 claims. One source    │
│   │ (CompetitorD Q3 report) could not be    │
│   │ confirmed — flagged for review.         │
│   └─────────────────────────────────────────┘
│
│ ┌─────────────────────────────────────────┐
│ │ Final Report                            │
│ │                                         │
│ │ ## Competitive Pricing Analysis — Q4    │
│ │                                         │
│ │ ```sql                                  │
│ │ SELECT provider, tier, price, delta     │
│ │ FROM pricing_entries                    │
│ │ WHERE quarter = 'Q4'                    │
│ │ ORDER BY delta DESC                     │
│ │ ```                                     │
│ │                                         │
│ │ Three of four competitors increased     │
│ │ enterprise pricing by 12-18%...         │
│ └─────────────────────────────────────────┘
```

Every document is full width. The tree gutter on the left shows the flow at a glance — you can scan the left edge and see the shape of the pipeline without reading any content. Sequential steps flow straight down. Parallel steps branch with `├──` connectors. Merge points rejoin the main pipe.

Code blocks render inside the documents. Plain English renders as readable prose. The whole execution reads top to bottom like a story.

## The Semantic Diff

The system maintains a snapshot of the board state at each submit. When the user submits again, it diffs the current Excalidraw state against the last snapshot. Every element has a stable ID, so the diff is exact. Three categories fall out:

### Diff (Programmatic, No AI)

Structural changes that can be handled as pure DB writes. No reasoning needed.

```
Diff: handled programmatically
  Edge A → connected to Edge C
  Node "Validation" → deleted
  Node "Research" → repositioned to (400, 200)
```

These are immediate. Edge rewired? Update the DB. Node deleted? Remove the row. Node moved? Update position. No agent touches these.

### Updating (AI Interprets the Delta)

The user changed the text inside an existing node. The system has both the before and after — the Board Dispatcher includes this diff in the dispatch instruction so the Per-Node Builder sees exactly what shifted and why it matters.

```
Updating: User changed node description from:
  "Query the database for new website visits"
  →
  "Query the database for new website visits and aggregate the
   company information by first selecting the companies then
   placing their IDs in your final response."
```

The Board Dispatcher dispatches to the Per-Node Builder with the before/after context. The Per-Node Builder understands: the user wants aggregation logic, company selection, and structured ID output. It rewrites the agent's system prompt and capabilities to match. The before/after gives it precise context — not "here's a node, design it from scratch" but "here's what changed, update the design accordingly."

### New (AI Classifies + Designs)

A new box on the board. The system has never seen this element before. Two things happen:

**1. Protocol selection (cheap bot, instant):**

A lightweight classifier reads the node description and selects the right protocol. No expensive reasoning — just pattern matching against the protocol catalog.

```
New: User created a new node (cheap bot):
  Select the correct protocol to fulfill the job:
  Protocols: {available protocols from registry}
  Input: "File a report"
  Output: workforce
```

The selector looks at "File a report" and determines the right protocol from the registry. The protocol is assigned automatically. The user never thinks about execution modes.

**2. Board Dispatcher dispatches to Per-Node Builder (async):**

The Board Dispatcher reads the changeset — the new node, its protocol, and its position in the topology. It dispatches a configuration instruction to the Per-Node Builder for this node.

The Per-Node Builder picks up the instruction with the full board context — incoming edges, upstream outputs, the user's description, extracted beliefs. It configures the workforce: agent rosters, dependencies, capabilities, and a plan.

```
New: User created a new node:
  "File a report"
  Protocol: workforce (from selector)
  → Board Dispatcher dispatches to Per-Node Builder with board context
  → Per-Node Builder reads full box content, upstream topology, beliefs
  → Configures workforce: agent roster, dependencies, capabilities, plan
  → Designer runs async: writes per-agent prompts to .system/design/
  → Prompts appear in config panel for user review
```

The Per-Node Builder doesn't configure in isolation. It sees what's upstream, understands what data flows into this node, and designs the agents to handle that specific context. "File a report" with research data upstream produces a different configuration than "File a report" with raw database output upstream.

**3. Designer phase (async, part of board submit):**

The Designer runs async after the Per-Node Builder completes — it is part of the board submit pipeline. It retrieves relevant orchestration examples, then writes per-agent prompts (system prompt, assignment, expected_output, tools) to the system store as a ReAct agent. Prompts appear in the config panel for user review before execution. See @visions/vision-system-store.md.

### Full Example

A user has a three-node workflow. They edit one node's description, add a new node, and rewire an edge. On submit:

```
Semantic Diff:

  Agentless (Phase 0, instant):
    Edge: "Research" → "Summarize" reconnected to "Research" → "Validate"

  Updating (Board Dispatcher → Per-Node Builder):
    Node "Research":
      "Search for competitor pricing"
      →
      "Search for competitor pricing across Q3 and Q4, compare
       year-over-year trends, flag anomalies above 10%."

  New (Board Dispatcher → Per-Node Builder → Designer):
    Node: "Generate executive summary"
    Protocol selector: workforce
    → Board Dispatcher dispatches to Per-Node Builder with upstream context
    → Per-Node Builder configures workforce: 3-agent team (analyst, writer, reviewer)
    → Designer writes per-agent prompts to .system/design/
    → Prompts appear in config panel for review
```

Phase 0 handles the edge rewire instantly as a DB write. The Board Dispatcher reads the changeset (the updated description and new node) and dispatches to the Per-Node Builder for each affected node. The Designer runs async to write per-agent prompts. The user sees the edge change immediately, watches the designs populate in the config panel, and runs when ready.

## The Protocol Selector

Context nodes go away. The user never picks an execution mode. Instead, a lightweight selector classifies each new node into the right protocol based on its description and board position.

The selector is a cheap, fast classifier — not a reasoning agent. It reads the node description, looks at the available protocols, and picks the best match. This runs inline during Phase 1, before the design agent even starts.

```
┌─────────────────────────────────────┐
│ Protocol Selector (cheap bot)       │
│                                     │
│ Protocols: {from registry}          │
│                                     │
│ Input:  "File a report"             │
│ Output: workforce                   │
│                                     │
│ Input:  "Research competitors and   │
│          summarize findings"        │
│ Output: workforce                   │
│                                     │
│ Input:  "Run the full QA pipeline"  │
│ Output: sub_workflow                │
└─────────────────────────────────────┘
```

The selector's decision populates into the node immediately. If the user disagrees, they can change it in the config panel before execution. But the default is usually right — "research competitors" is obviously a workforce, "run the validation suite" is obviously a sub-workflow. The selector picks from whatever protocols are registered, so as the protocol catalog grows, the selector's options grow with it.

This eliminates the concept of "context" as a node type. If a node needs upstream context, the protocol handles it. The selector and design agent wire context flow through the protocol's built-in mechanisms — ports, variable interpolation, prompt composition. The user just describes what the node should do. The system figures out how.

## Two Entry Points, Different Pipelines

Drawing and chatting are two ways to build a workflow, but they use different dispatch pipelines that converge at the Per-Node Builder.

```
  ╭──────────────╮                    ╭──────────────╮
  │ User draws   │                    │ User types   │
  │ on canvas    │                    │ in chat      │
  ╰──────┬───────╯                    ╰──────┬───────╯
         │                                   │
         ▼                                   ▼
  ┌──────────────┐                    ┌──────────────┐
  │  Phase 0     │                    │   Manager    │
  │  Structural  │                    │  Assistant   │
  │  (agentless) │                    │  dispatch()  │
  │  - create    │                    └──────┬───────┘
  │    nodes     │                           │
  │  - delete    │                           ▼
  │    removed   │                    ┌──────────────┐
  │  - rewire    │                    │  Builder     │── create topology
  │    edges     │                    │  Agent       │   + fill content
  └──────┬───────┘                    └──────┬───────┘
         │                                   │ passdown
         ▼                                   │
  ┌──────────────┐                           │
  │  Board       │                           │
  │  Dispatcher  │                           │ // todo Builder agent needs to
  │  (changeset  │                           │ // Board dispatcher too.
  │   → per-node │                           │
  │   dispatch)  │                           │
  └──────┬───────┘                           │
         │                                   │
         └──────────────┬────────────────────┘
                        ▼
                 ┌──────────────┐
                 │  Per-Node    │──── reads full box content
                 │  Builder     │     configures workforce
                 └──────┬───────┘
                        │
                        ▼
                 ┌──────────────┐
                 │  Example     │──── embed task, query pgvector
                 │  Retrieval   │     inject 2-3 relevant examples
                 └──────┬───────┘
                        │
                        ▼
                 ┌──────────────┐
                 │  Designer    │──── ReAct agent, writes per-agent
                 │  (async)     │     prompts to .system/design/
                 └──────┬───────┘
                        │
                        ▼
                 ┌──────────────┐
                 │  User Review │──── config panel shows prompts
                 │  (optional)  │     user edits before execution
                 └──────┬───────┘
                        │
                        ▼
                 ┌──────────────┐
                 │  Workforce   │──── executor reads prompts
                 │  Execution   │     from .system/design/
                 └──────────────┘
```

**Board path** (left): Phase 0 builds the topology agentlessly — nodes created, edges wired, positions set. The Board Dispatcher reads the changeset and dispatches configuration instructions to the Per-Node Builder for each new or updated node. No topology tools, no content generation — the user already did both by drawing.

**Chat path** (right): The Manager Assistant dispatches to the Builder Agent, which creates the topology (nodes, edges) and fills in the box descriptions in a single pass. The Per-Node Builder then configures the workforce for each node.

Both paths converge at the Per-Node Builder. After the builder finishes, relevant orchestration examples are retrieved from the example library and injected into the Designer's prompt. The Designer writes per-agent prompts to the system store as a ReAct agent, reading back prior configs to verify coherence. Prompts appear in the config panel for the user to review and edit before triggering execution.

## Hierarchy Trees Inside Nodes

Workforce nodes have agent teams. The tree tab shows these as nested hierarchies:

```
── Research Pipeline
   ├── Research Team               [workforce]
   │   ├── Lead Researcher
   │   ├── Web Searcher
   │   ├── Paper Analyzer
   │   └── Fact Checker
   ├── Write Report                [single]
   └── Output                      [single]
```

The user can draw nested boxes on the canvas to describe team structure — a large rectangle containing smaller rectangles with arrows between them. The system reads the containment and builds the agent roster. Or the user can configure the roster directly in the config panel. Both paths produce the same result.

## User Stories

**"I want to sketch a workflow from nothing."**
Open a new workflow. Draw boxes. Write what each one does. Connect them with arrows. Hit submit. The tree populates instantly with the structure. Designs fill in over the next few seconds. Click each node to review. Run when ready.

**"I want to restructure my workflow."**
Draw new boxes on the canvas. Draw arrows connecting them to the existing structure. Submit. New nodes appear in the tree instantly. The drawing stays on the canvas as your reference.

**"I want to fix how a node behaves."**
Click the node in the tree. The config panel opens. Edit the system prompt directly. No submit needed — you're editing the live configuration. Or draw an annotation on the canvas near the node and submit for the design agent to interpret.

**"I want to review everything before running."**
Walk through the tree. Click each node. Read the system prompt, check the capabilities, verify the routing. Edit anything that doesn't look right. The guided engineering pipeline means nothing runs until you say so.

**"I want to see what my workflow produced."**
Switch to the execution output tab. Scroll through full-width document blocks. The tree gutter on the left shows the pipeline flow — sequential steps flow straight down, parallel steps branch and merge. Read the whole execution like a story.

## Technical Foundation

### Drawing Engine

The doodle layer uses the Excalidraw React component (`@excalidraw/excalidraw`, MIT licensed). It provides rectangle, arrow, and text tools with a hand-drawn aesthetic out of the box. The component runs as an overlay on the existing page, providing a full drawing surface.

Excalidraw elements persist locally as the user's visual reference. They are never converted to workflow nodes. The system reads them on submit to extract structure, but the drawings themselves stay.

### Tree Rendering

The sidebar tree uses the `AsciiTree` class (`frontend/src/utils/AsciiTree.ts`) — a generic tree builder that converts flat data into hierarchical ASCII text with box-drawing characters. Already built, tested (23 unit tests), and integrated into the workforce agents tab as a proof of concept.

### Execution Output Stream

The execution output is a vertical scroll of full-width document blocks. A tree gutter component renders along the left edge using box-drawing characters to show pipeline flow. The gutter branches for parallel steps and merges when they rejoin. Each document block contains the step's output — plain text rendered as prose, code rendered in fenced blocks.

### Existing Canvas

The existing React Flow canvas (`@xyflow/react`) with `CanvasNode` components stays as-is. It becomes the "advanced" or "spatial" view for users who want the full DAG editor. The new tree-first sidebar is the primary experience. Both views read from the same stores, same API, same types. Two views into the same data.

## What This Doesn't Change

- Node configuration, execution, the DAG engine — all untouched.
- The workforce pipeline infrastructure is reused, not rebuilt.
- The beliefs extraction system continues as-is — it feeds the pipeline agents with compressed context.
- The existing canvas view continues to work as an advanced/spatial view.
- All stores, API endpoints, and types are shared between views.
- The designer phase generates per-agent system prompts at **design time** (board submit), not execution time. Prompts persist in the system store for user review and re-execution. See @visions/vision-system-store.md.

## What This Builds On

| Capability | Already built | Visual Dispatch adds |
|------------|--------------|---------------------|
| Workflow structure | Stores, API, DB (50+ entity types) | Tree-first sidebar navigation |
| Workforce pipeline | Pipeline service, sequential agent execution | Reused by Per-Node Builder |
| Dispatch tool | Dispatch mechanism with passdowns | 3 dispatch agents: Builder, Board Dispatcher, Per-Node Builder |
| Beliefs extraction | Chat-phase Haiku extraction, neighbor awareness | Primary context layer for pipeline agents |
| Board serializer | `board_serializer` module (classify, diff, filter, score) | Feeds changeset into the structural executor |
| Board submit API | `POST /workflows/:id/board/submit` with snapshot persistence + Phase 0 executor | Entry point for canvas → pipeline |
| Rich text rendering | `TerminalBlock` (custom markdown AST parser) | Execution output stream with tree gutter |
| Tree rendering | `AsciiTree` class (box-drawing hierarchies) | Primary navigation in sidebar |
| Workforce agent teams | Agent rosters, pipeline service | Visual hierarchy in tree + config panel editing |
| Canvas with nodes and edges | React Flow (`@xyflow/react`) | Stays as advanced view; Excalidraw overlay for drawing |
| Agent configuration | Config screens, chat-based dispatch | Config panel in sidebar with full edit capability |
| System store | — | `.system/` filesystem for agent configs and artifacts (@visions/vision-system-store.md) |
| Example library | — | Curated orchestration examples retrieved by similarity at design time (@visions/vision-example-library.md) |

Nothing gets thrown away. Nothing gets rewritten. Visual Dispatch is a new front door — a guided engineering pipeline that builds on every system that already exists.


