# Workflow Builder — Vision

## What It Is

The Workflow Builder is a conversational agent that designs and modifies workflow topology — nodes, edges, and node content — by editing the user's board as a virtual filesystem. The user talks to the manager, the manager dispatches to the Workflow Builder, and the builder proposes changes that appear as unsaved edits on the user's canvas. The user reviews, adjusts, and submits. Phase 0 remains the only gateway to the database.

The builder never touches the DB. It's a canvas editor — an AI pair-programmer for workflow design.

## Why This Matters

Today, there are two ways to build a workflow:

1. **Draw on the canvas** — the user sketches boxes and arrows, hits submit, Phase 0 diffs and builds. Fast, visual, but the user does all the structural thinking.
2. **Chat with the manager** — the user describes what they want conversationally, the manager dispatches to the Builder Agent which creates topology directly in the DB. Flexible, but the user has no review step before changes land.

Neither path gives the user what they actually want: **a conversation that proposes visual changes they can review before committing**. The Workflow Builder closes this gap. The user describes intent, the AI proposes structure, the user sees it on their canvas, edits it, and submits when ready.

This also enables the full guided engineering pipeline at the workflow level — the same design → review → refine → execute cycle that already works for agent prompts via the Designer, but applied one tier up to the workflow itself.

## The Agent Chain

```
User talks to manager
  → Manager understands intent
  → Workflow Builder edits the virtual board
  → Modified board pushed to frontend
  → User sees unsaved changes on canvas
  → User reviews, edits, deletes, rearranges
  → User hits submit
  → Phase 0 (unchanged) → Per-Node Builder → Designer (unchanged)
```

Three agents in the dispatch chain, each with a meaningfully different job:

| Agent | Job | Context | Output |
|-------|-----|---------|--------|
| **Manager** | Understand user intent | Conversation history, beliefs | Dispatch instruction |
| **Workflow Builder** | Design topology + node content | Classified board snapshot, dispatch instruction | Modified board files |
| **Per-Node Builder** | Configure workforce per node | Full box content, upstream topology, beliefs | Agent rosters, plans |
| **Designer** | Write per-agent prompts | Builder's plan, roster, example library | System prompts in `.system/design/` |

The Manager → Workflow Builder handoff is new. Everything downstream (Per-Node Builder → Designer) is unchanged.

### Why Not Fewer Agents?

The Manager and Workflow Builder could theoretically be one agent. But they have fundamentally different interfaces:

- The **Manager** is conversational — it talks to the user, asks clarifying questions, maintains rapport. It's a long-running session agent.
- The **Workflow Builder** is operational — it reads a board snapshot, makes structural edits, and returns. It's a single-shot task agent with file tools.

Merging them would mean the conversational agent also has topology editing tools, which creates confusion about when to talk vs when to edit. The handoff is clean: the manager decides *what* to do, the builder does *it*.

## The Virtual Board Environment

When the manager dispatches to the Workflow Builder, the current canvas state is projected into a file tree:

```
.workflow/
├── nodes/
│   ├── research-competitors.md
│   ├── analyze-findings.md
│   └── write-report.md
└── graph.json
```

### Node Files

Each node is a markdown file with frontmatter:

```markdown
---
name: Research Competitors
execution_mode: workforce
---

Search for competitor pricing across Q3 and Q4,
compare year-over-year trends, flag anomalies above 10%.
```

The frontmatter carries structural metadata. The body is the node's description — the content the user wrote in the box (or that a previous builder pass generated). This is what the Per-Node Builder reads to understand what the node should do.

### Graph File

The wiring between nodes:

```json
{
  "edges": [
    { "from": "research-competitors", "to": "analyze-findings" },
    { "from": "analyze-findings", "to": "write-report" }
  ]
}
```

### What's NOT in the Virtual Environment

- **Annotations** — the user's freeform notes on the canvas. These are the user's visual reference, not the builder's to edit. If the manager needs annotation context, it includes relevant details in the dispatch instruction as text.
- **Sketches** — freeform drawings, images, pen strokes. Preserved through the serialization round-trip untouched.
- **Positions** — the builder doesn't know or care that "Research" is at (400, 200). The frontend handles layout. New nodes get auto-positioned based on their edges.
- **Global notes** — board-level text not attached to any node. Read-only context for the manager, not files the builder manages.

The builder controls **topology and node content**. Everything else passes through unchanged.

### Tools

The builder uses the same store tool pattern as the Designer:

| Tool | Purpose |
|------|---------|
| `store_read_file` | Read a node file or graph.json |
| `store_write_file` | Create or modify a node file, update graph.json |
| `store_delete_file` | Remove a node (also removes its edges from graph.json) |
| `store_list_files` | See what nodes exist |

Same interaction pattern the Designer already uses for `.system/design/`. The builder reads the current state, creates new node files, modifies existing ones, updates `graph.json` to wire edges.

### Ephemeral Workspace

The Designer's `.system/design/` files are **persistent** — they're the source of truth for agent prompts across runs. The builder's `.workflow/` files are **ephemeral** — they exist only for the duration of the builder's work.

```
Canvas snapshot
  → serialize to .workflow/ files (input)
  → builder reads and writes files
  → serialize .workflow/ files back to board elements (output)
  → push to frontend as unsaved changes
  → files discarded
```

The DB (via Phase 0 on submit) is the source of truth for topology. The files are the builder's working medium, not storage.

## Canvas Snapshot as Input

When the user sends a chat message, the frontend attaches the current canvas state:

```typescript
const elements = boardElementStore.getElements()
const serialized = serializeToExcalidraw(elements)

api.post(API.SESSION_CHAT(sessionId), {
  content: message,
  canvas_snapshot: serialized,
})
```

The backend runs the snapshot through the board serializer's `classify` pass — the same pipeline Phase 0 uses — to extract semantic content:

- **Nodes** — names, descriptions, execution modes
- **Edges** — source → target connections
- **Annotations** — attached to nearest nodes (read-only context for the manager)

The classified output is projected into `.workflow/` files for the builder. Spatial data (positions, bounds) is stripped — the builder sees structure, not coordinates.

`boardElementStore` is an external store (not tied to React lifecycle), so `getElements()` works synchronously from anywhere — no hooks required. The serializer already produces the exact Excalidraw format the backend expects.

## Modified Board as Output

When the builder finishes editing `.workflow/` files, the system:

1. Reads all node files and `graph.json` from the ephemeral workspace
2. Diffs against the input snapshot to identify what changed
3. Generates modified board elements (new boxes, updated text, new arrows)
4. Pushes the modified elements to the frontend via WebSocket

The frontend receives the changes and updates `boardElementStore` — the same store the canvas reads from. New nodes appear on the canvas. Modified text updates in place. Deleted nodes disappear. All as **unsaved changes**.

The user sees the builder's proposals on their canvas immediately. They can:

- **Accept as-is** — hit submit
- **Edit** — rename nodes, rewrite descriptions, move things around
- **Delete** — remove proposals they don't like
- **Add more** — draw additional nodes or annotations
- **Ignore** — keep chatting, ask for revisions

Submit processes everything — builder proposals + user edits — through Phase 0 as a single diff. Phase 0 doesn't know or care that some nodes were created by the builder and some were drawn by the user.

## The Full Lifecycle

### First Conversation — Building from Scratch

```
User: "I need a pipeline that scrapes competitor pricing,
       analyzes trends, and generates an executive report."

Manager: dispatches to Workflow Builder with instruction +
         empty board snapshot (no existing nodes)

Builder:
  → writes nodes/scrape-pricing.md (workforce, web scraping team)
  → writes nodes/analyze-trends.md (workforce, data analysis team)
  → writes nodes/generate-report.md (workforce, writing team)
  → writes graph.json (linear pipeline: scrape → analyze → report)

Frontend: 3 new boxes appear on canvas with arrows
User: drags them into position, tweaks the report node description
User: hits submit

Phase 0: creates 3 nodes + 2 edges in DB
Per-Node Builder: configures workforce for each node
Designer: writes per-agent prompts to .system/design/
User: reviews agent designs in config panel
User: runs workflow
```

### Refinement — Modifying Existing Structure

```
User: "Add a validation step between analysis and the report.
       I want it to fact-check the claims before they go into
       the executive summary."

Manager: dispatches to Workflow Builder with instruction +
         current board snapshot (3 existing nodes + 2 edges)

Builder:
  → reads existing nodes and graph.json
  → writes nodes/validate-claims.md (workforce, fact-checking team)
  → updates graph.json:
      scrape → analyze → validate → report (rewired)

Frontend: new "Validate Claims" box appears between Analyze and Report
          arrow from Analyze now points to Validate
          new arrow from Validate to Report
User: reviews, submits

Phase 0: diffs — 1 new node, 1 deleted edge, 2 new edges
Per-Node Builder: configures workforce for Validate Claims only
Designer: writes agent prompts for the new node only
```

### Complex Restructuring

```
User: "Actually, I want scraping and analysis to happen in
       parallel — two separate teams working simultaneously,
       then merge results into the report."

Builder:
  → reads existing graph.json (linear: scrape → analyze → validate → report)
  → updates graph.json:
      scrape ──→ validate → report
      analyze ─↗
      (parallel fork, merge at validate)
  → updates nodes/validate-claims.md description to reflect
    it now receives input from both scrape and analyze

Frontend: topology restructures — two parallel branches merge at Validate
User: reviews the new DAG shape, submits
```

## Programs That Build Programs

The Workflow Builder + Pin system enables a powerful pattern: **workflows that create reusable programs**.

### The Compile → Pin → Execute Pattern

Consider a user who says: "I need a tool that vectorizes my customer data for RAG search."

The Workflow Builder creates:

```
[Classify & Plan]  →  [Write Code]  →  [Run & Verify]
```

**First run (compilation):**
- Agent team 1 analyzes the data schema, designs a chunking strategy
- Agent team 2 writes the Python code (embedding pipeline, vector store integration)
- Agent team 3 runs the code in a container (`run_command`), verifies results

**Pin the first two nodes:**

```
[Classify & Plan]  →  [Write Code]  →  [Run & Verify]
     📌 pinned          📌 pinned         ← only this runs
```

**Subsequent runs (execution):**
- Pinned nodes replay their output (the code) — zero tokens charged
- Only the executor agent fires — reads the code from the store, runs it against new data
- Dead-path elimination skips any nodes that only feed pinned outputs

The workflow went from a 3-step AI pipeline to a single-step program executor. Expose it via API with the data source as input, and you've built a microservice — an AI-designed, AI-coded microservice that a human reviewed before pinning.

### Real-World Examples

**Compliance auditor.** Agents read regulations, write SQL validation scripts. Pin them. The executor runs quarterly against live data. Regulation changes? Unpin, agents rewrite the checks, re-pin.

**Invoice processor.** Agents analyze sample invoices, write extraction code (PDF parsing, field mapping). Pin them. Every new invoice goes through the extractor. New vendor format? Unpin the classifier, it adapts, re-pin.

**Security scanner.** Agents analyze your codebase patterns, write custom static analysis rules tuned to your stack. Pin them. Runs against every PR.

**Data migration.** Agents analyze source and target schemas, write transformation scripts, test on sample data. Pin everything except the runner. Reusable migration tool across environments.

The pattern is always: **the AI is the engineer, not the operator**. It designs and builds the tool, the human reviews and pins, then the tool runs without AI. The expensive creative work happens once. Execution is cheap and repeatable.

### Container Execution

The infrastructure for running generated code already exists. Agents in container mode get:

| Tool | Purpose |
|------|---------|
| `read_file` / `write_file` | File I/O in the container |
| `run_command` | Execute arbitrary shell commands |
| `run_tests` | Run test suites |
| `git_*` | Version control operations |

An executor agent can: read code from the store → write to container filesystem → `pip install -e .` → run the program → check exit code → report results. All inside a sandboxed Docker container.

## Entry Point Unification

Both canvas and chat now start from the same data — the current board state. The difference is who processes it:

```
┌──────────────────────┐          ┌──────────────────────┐
│  Canvas Entry Point  │          │   Chat Entry Point   │
│                      │          │                      │
│  User draws on board │          │  User sends message  │
│  User hits submit    │          │  + canvas snapshot    │
└──────────┬───────────┘          └──────────┬───────────┘
           │                                 │
           ▼                                 ▼
    ┌──────────────┐                  ┌──────────────┐
    │   Phase 0    │                  │   Manager    │
    │  (agentless) │                  │  (conversa-  │
    │  Diff → DB   │                  │   tional)    │
    └──────┬───────┘                  └──────┬───────┘
           │                                 │
           │                          ┌──────▼───────┐
           │                          │  Workflow    │
           │                          │  Builder     │
           │                          │  (file edit) │
           │                          └──────┬───────┘
           │                                 │
           │                          Modified board
           │                          pushed to frontend
           │                          as unsaved changes
           │                                 │
           │                          User reviews + submits
           │                                 │
           │                          ┌──────▼───────┐
           │                          │   Phase 0    │
           │                          │  (agentless) │
           │                          │  Diff → DB   │
           │                          └──────┬───────┘
           │                                 │
           └──────────────┬──────────────────┘
                          ▼
                   ┌──────────────┐
                   │  Per-Node    │
                   │  Builder     │
                   └──────┬───────┘
                          ▼
                   ┌──────────────┐
                   │  Designer    │
                   └──────┬───────┘
                          ▼
                   ┌──────────────┐
                   │  User Review │
                   │  + Execute   │
                   └──────────────┘
```

**Canvas path** (left): direct to Phase 0. User did the structural thinking.

**Chat path** (right): Manager → Workflow Builder → user reviews → Phase 0. AI did the structural thinking, user approves.

Both paths converge at Phase 0 → Per-Node Builder → Designer. The downstream pipeline doesn't know or care how the topology was created.

## What Already Exists

| Capability | Status | Used by |
|------------|--------|---------|
| Board serializer (classify, diff, filter) | Built | Phase 0, will feed builder's virtual environment |
| `boardElementStore.getElements()` | Built | Board submit, will attach to chat messages |
| `serializeToExcalidraw()` | Built | Board submit, will serialize builder output |
| Store tools (`store_read_file`, `store_write_file`) | Built | Designer, will be reused by builder |
| Phase 0 structural executor | Built | Board submit, unchanged |
| Per-Node Builder + Designer pipeline | Built | Both paths, unchanged |
| Container execution (`run_command`, `run_tests`) | Built | Executor agents |
| Pin system (replay, dead-path elimination) | Built | DAG executor |
| WebSocket event push to frontend | Built | All real-time updates |
| Manager assistant with dispatch | Built | Chat path |

## What's New

| Component | Purpose |
|-----------|---------|
| Workflow Builder agent | Reads/writes `.workflow/` files to propose topology changes |
| Canvas snapshot attachment | Frontend sends current board state with chat messages |
| Snapshot → file projection | Classified board snapshot projected into `.workflow/` node files + `graph.json` |
| File → board serialization | Builder's modified files serialized back to board elements |
| Board push via WebSocket | Modified elements pushed to frontend as unsaved changes |

## Implementation Order

### Part 1: Canvas Snapshot in Chat

- Attach `boardElementStore.getElements()` serialized output to chat POST requests
- Backend chat handler accepts optional `canvas_snapshot` field
- Thread snapshot through to manager context

### Part 2: Snapshot → File Projection

- Reuse board serializer `classify` pass to extract nodes, edges, annotations
- Project classified data into `.workflow/` node files + `graph.json`
- Strip spatial data (positions, bounds) — builder sees structure only

### Part 3: Workflow Builder Agent

- New agent type with store tools scoped to `.workflow/`
- Reads node files and graph.json, makes structural edits
- Returns when editing is complete

### Part 4: File → Board Serialization

- Read modified `.workflow/` files
- Diff against input snapshot
- Generate new/modified board elements
- Auto-layout new nodes based on edge relationships

### Part 5: Board Push

- Push modified elements to frontend via WebSocket
- Frontend updates `boardElementStore` with unsaved changes
- User sees proposals on canvas, reviews, submits

### Part 6: Manager Dispatch Integration

- Manager learns when to dispatch to Workflow Builder vs respond conversationally
- Dispatch instruction includes user intent + relevant annotation context
- Manager can iterate — user asks for changes, builder edits again, new proposals appear

## What This Doesn't Change

- Phase 0 — unchanged, still the only path from canvas to DB
- Per-Node Builder — unchanged, still configures workforce per node
- Designer — unchanged, still writes agent prompts to `.system/design/`
- Board serializer — unchanged, still diffs Excalidraw snapshots
- Pin system — unchanged, still replays frozen step outputs
- Container execution — unchanged, still runs code in Docker
- Store tools — reused, same pattern at a different scope
