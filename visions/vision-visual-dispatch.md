# Visual Dispatch — Vision

## What It Is

Visual Dispatch turns the workflow canvas into a freeform drawing surface. The user sketches what they want — boxes, arrows, annotations — directly on the board, right on top of their existing workflow. When they're ready, they hit submit and the drawing becomes a structured changeset that flows through the same dispatch pipeline as a chat message. The manager builder receives it, interprets it, and makes it real.

Drawing and chatting coexist. The manager chat is still there. Some users will always prefer typing. But for spatial, structural, multi-node intent — drawing is faster and more natural than describing topology in words. The user draws three boxes with arrows and writes "research", "validate", "report" inside them. That says more than a paragraph of instructions.

The doodle layer is always on. There's no mode toggle. The drawing tools sit in the toolbar alongside everything else. The user can pick up a pencil and annotate at any time, on any board — empty or full.

## Why This Matters

Today, the only way to instruct the manager is through conversation. That works well for simple requests ("add a node that summarizes the output") but breaks down for structural intent. Try describing a four-node DAG with conditional branching in a chat message. The user knows exactly what they want — they can see it in their head — but translating spatial relationships into words is friction.

Visual Dispatch removes that translation step. The user draws what's in their head. The system reads the drawing.

This also changes the relationship between the user and the board. Right now the board is an output — something the manager builds while the user watches. With Visual Dispatch, the board becomes an input. The user and the agents share the same canvas, both contributing to it in their own way.

## Two Contexts, Same Tools

The doodle tools work the same whether the board is empty or populated. The dispatch pipeline doesn't care — it receives a changeset either way.

### Empty Board — Sketch From Scratch

The user opens a new workflow. The canvas is blank and dark. They grab the rectangle tool and start drawing boxes, connecting them with arrows, typing descriptions inside each one. They're sketching the workflow they want.

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│   ╭ ─ ─ ─ ─ ─ ─ ╮         ╭ ─ ─ ─ ─ ─ ─ ╮               │
│   ╎ research the  ╎ ~~~~>  ╎ analyze and   ╎               │
│   ╎ competitors   ╎        ╎ compare       ╎               │
│   ╰ ─ ─ ─ ─ ─ ─ ╯         ╰ ─ ─ ─ ─ ╮─ ─ ╯               │
│                                       ╎                     │
│                                       ~~~~>                 │
│                             ╭ ─ ─ ─ ─ ─ ─ ╮               │
│                             ╎ write final   ╎               │
│                             ╎ report        ╎               │
│                             ╰ ─ ─ ─ ─ ─ ─ ╯               │
│                                                             │
│  Everything is a doodle. Nothing is real yet.               │
│                                              [Submit]       │
└─────────────────────────────────────────────────────────────┘
```

On submit, the system extracts the topology — three nodes, two edges — and dispatches it to the manager builder. The manager creates real workflow nodes at the positions where the user drew them. Doodles fade out, real nodes fade in. The sketch becomes the workflow.

### Populated Board — Annotate and Change

The user has a running workflow. Three nodes, configured and executing. They want changes. They grab the drawing tools and start annotating directly on the board.

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  ┌──────────────┐         ┌──────────────┐                 │
│  │ Research      │────────│ Summarize    │                 │
│  │ workforce-1   │         │ single-2     │                 │
│  └──────────────┘         └──────┬───────┘                 │
│    ╎ The agents keep ╮           │                          │
│    ╎ hallucinating   ╎           │                          │
│    ╎ sources during  ╎           │       ┌──────────────┐  │
│    ╎ search.         ╎           └───────│ Final Report │  │
│    ╰ ─ ─ ─ ─ ─ ─ ─ ╯                   │ single-3     │  │
│                                          └──────────────┘  │
│          ╭ ─ ─ ─ ─ ─ ─ ─ ─ ╮                  ▲           │
│          ╎ add a fact-check  ╎~~~~~~~~~~~~~~~~~╯           │
│          ╎ validation step   ╎                              │
│          ╰ ─ ─ ─ ─ ─ ─ ─ ─ ╯                              │
│                                                             │
│  REAL nodes + DOODLE annotations on the same canvas.       │
│                                              [Submit]       │
└─────────────────────────────────────────────────────────────┘
```

On submit, the system builds a changeset — only the things the user touched. A comment on workforce-1 about hallucinating sources. A new node sketch for a validation step. A drawn arrow connecting it to single-3. The existing unchanged nodes are not included. The manager builder receives the diff, not the whole board.

## The Doodle Layer

### Always On

The doodle tools live in the toolbar permanently. Rectangle, arrow, text. The user can draw at any time without entering a special mode. Doodle elements are visually distinct from real nodes — they look hand-drawn, sketchy, with rough borders and wobbly lines. Real nodes are clean and solid. You can tell at a glance what's a sketch and what's built.

### Visual Style

Doodle elements use a hand-drawn aesthetic. Rough borders, imperfect lines, a sketch-weight font. This isn't decorative — it's functional. The contrast between sketchy doodles and solid real nodes tells the user exactly what's intent and what's reality.

```
Doodle (intent):              Real (built):

╭ ─ ─ ─ ─ ─ ╮               ┌──────────────┐
╎ research    ╎               │ Research      │
╎ competitors ╎               │ workforce-1   │
╰ ─ ─ ─ ─ ─ ╯               │ ● running     │
                              └──────────────┘
```

### Comments on Existing Nodes

Right-click a real node, select "Add Comment." A comment box appears anchored to the right side of the node. Multiple comments on the same node stack vertically. Each comment is a doodle element — sketchy style, clearly an annotation rather than part of the node.

```
┌──────────────┐ ╭ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ╮
│ Research      │╌╎ Agents hallucinate    ╎
│ workforce-1   │ ╎ sources during search ╎
│               │ ╰ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ╯
└──────────────┘ ╭ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ╮
                 ╎ Also increase the      ╎
                 ╎ search depth to 3      ╎
                 ╰ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ╯
```

Comments are the primary way users annotate existing nodes. They describe problems, request behavior changes, or add context. On submit, each comment is associated with its target node in the changeset.

### Drawn Elements

Beyond comments, the user can draw new boxes and arrows on the board. A new box with text inside is interpreted as a request for a new node. An arrow drawn from a doodle box to a real node (or between two doodle boxes) is interpreted as a requested edge. Arrows don't need to snap or touch — the system infers connections by spatial proximity after the user submits.

## The Changeset

When the user hits submit, the system doesn't send the entire board state. It extracts only what the user drew — the changeset. This is what makes it efficient for the manager. A board with 20 real nodes and 2 doodle annotations produces a changeset with only those 2 annotations.

### Structure

The changeset captures four types of intent:

**Node annotations** — comments attached to existing nodes. The user wants something about that node to change.

**New nodes** — doodle boxes with text that aren't attached to any existing node. The user wants new nodes created.

**Edge additions** — drawn arrows between elements (new-to-existing, new-to-new, or existing-to-existing). The user wants new connections.

**Edge removals** — existing edges the user has marked for deletion (e.g., right-click edge, "Mark for removal," visualized as a strikethrough on the edge).

### Example Changeset

From the annotated board above:

```
Changeset:
  Node Annotations:
    - workforce-1 "Research":
      "The agents keep hallucinating sources during search."

  New Nodes:
    - "add a fact-check validation step"
      Position: (320, 480)

  Edge Additions:
    - [new: fact-check validation] → single-3 "Final Report"

  Edge Removals:
    (none)
```

This is what the manager builder receives. Not "here are 20 nodes and 15 edges." Just the delta.

## The Dispatch Pipeline

### Manager Builder as Single Source of Truth

The changeset always goes to the manager builder (L2). Not to individual node assistants. Not fanned out to parallel L3 sessions. The manager builder is the single brain that sees the full changeset and the full board state. It decides what to do, resolves conflicts between changes, and orchestrates execution in the right order.

This is critical. If annotations blasted directly to individual node assistants in parallel, you'd have five agents independently deciding what to do with no coordination. One might restructure edges while another is creating nodes that depend on those edges. The manager builder prevents that — it sees everything, plans everything, then executes.

```
  ╭──────────────╮         ╭──────────────╮
  │ User draws   │         │ User types   │
  │ on canvas    │         │ in chat      │
  ╰──────┬───────╯         ╰──────┬───────╯
         │                        │
         ▼                        ▼
  ┌──────────────┐         ┌──────────────┐
  │ Extract      │         │ Chat message │
  │ changeset    │         │ (plain text) │
  └──────┬───────┘         └──────┬───────┘
         │                        │
         └───────────┬────────────┘
                     ▼
              ┌──────────────┐
              │ Manager      │
              │ Builder (L2) │
              │              │
              │ Single source│
              │ of truth.    │
              │ Sees full    │
              │ changeset +  │
              │ full board.  │
              └──────┬───────┘
                     │
          ┌──────────┼──────────┐
          ▼          ▼          ▼
     create_node  add_edge  dispatch_to_builders
                              │
                    ┌─────────┼─────────┐
                    ▼         ▼         ▼
                  L4 node   L4 node   L4 node
                  builder   builder   builder
```

Drawing and chatting are two input methods into the same pipeline. The manager builder already knows how to create nodes, wire edges, and dispatch to node builders. The changeset just gives it structured input instead of freeform text. The topology is explicit, not buried in a paragraph.

### Thinking Layer / Action Layer

A large changeset — say 5 node annotations, 3 new nodes, 4 edges — doesn't need one expensive agent doing everything sequentially. The changeset itself is the thinking. By the time the system has extracted "these are the nodes, these are the edges, these are the comments," the hard reasoning is done. What remains is execution.

This splits naturally into two layers:

**Thinking layer** — one pass over the full changeset. The manager builder reads everything, understands the full board context, resolves conflicts, and produces a plan of atomic work items. This is where intelligence lives. One smart call.

**Action layer** — parallel execution of atomic work items. Each item is a single tool call: create a node, wire an edge, dispatch an instruction to an L4 builder. These don't require reasoning. They're mechanical. Cheap model, or even direct service calls with no LLM.

```
Changeset (13 items)
      │
      ▼
┌─────────────────────┐
│ Thinking Layer       │  ← One pass. Manager builder.
│                      │     Reads full changeset + board state.
│ Splits into phases:  │     Resolves conflicts.
│   Phase 1: creates   │     Produces atomic work items.
│   Phase 2: edges     │
│   Phase 3: behaviors │
└──────────┬──────────┘
           │
  ┌────────┼────────┐
  ▼        ▼        ▼
┌──────┐┌──────┐┌──────┐    ← Phase 1: create nodes (parallel, cheap)
│Create││Create││Create│
│node A││node B││node C│
└──┬───┘└──┬───┘└──┬───┘
   └────┬──┘───────┘
        ▼                         (barrier — wait for node IDs)
  ┌─────┼─────┐
  ▼     ▼     ▼
┌────┐┌────┐┌────┐              ← Phase 2: wire edges (parallel, cheap)
│Edge││Edge││Edge│
│ 1  ││ 2  ││ 3  │
└────┘└────┘└────┘
  ┌─────┼─────┐
  ▼     ▼     ▼
┌────┐┌────┐┌────┐              ← Phase 3: behavior changes (parallel)
│ L4 ││ L4 ││ L4 │                 Dispatch to node builders.
│bld ││bld ││bld │                 This is where reasoning lives —
└────┘└────┘└────┘                 but scoped to one node each.
```

The cost tiers:

| Tier | What | Model | Why |
|------|------|-------|-----|
| Thinking | Parse changeset, plan execution | Smart (but structured input makes it easier) | Needs full board context, conflict resolution |
| Action — structural | Create nodes, wire edges | Cheapest, or no LLM at all | Mechanical tool calls, no reasoning |
| Action — behavioral | L4 node builder dispatches | Smart, but scoped to one node | "Fix hallucinating sources" requires real interpretation |

The thinking layer might not even need an expensive model. The changeset already did the hard work of identifying what the user wants. The manager builder is reading structured input, not parsing prose. And the structural actions (creates, edges) might not need an LLM at all — they're database writes with explicit parameters.

### Changeset as Manager Prompt

The changeset is serialized into the manager builder's dispatch instruction as structured text:

```
## Visual Dispatch — Change Request

### Current workflow
- workforce-1 "Research" → single-2 "Summarize" → single-3 "Final Report"

### Node annotations
**workforce-1 (Research):**
> The agents keep hallucinating sources during search.

### New nodes requested
1. "add a fact-check validation step"
   - Suggested position: (320, 480)
   - User drew edge: [this node] → single-3 "Final Report"

### Edge changes
- Add: [new validation node] → single-3
```

### What Lands in the Node Chat Sessions

The L3 node assistant chat sessions don't make decisions during Visual Dispatch — the manager builder does. But the chat sessions are updated as a record of what happened. When the manager builder dispatches a behavior change to a node's L4 builder, the result appears in that node's chat history.

```
┌─────────────────────────────────┐
│ Research — Chat                  │
│                                  │
│ ┄┄┄ earlier conversation ┄┄┄    │
│                                  │
│ 📌 Manager dispatched:           │
│ "Add source verification to      │
│  search prompt — user reported   │
│  hallucinated sources."          │
│                                  │
│ ● Builder completed.             │
│                                  │
│ > Type a message...              │
└─────────────────────────────────┘
```

The user can open any node's chat panel and see the full history — changes from Visual Dispatch, changes from direct chat, everything in one timeline. The chat box is both an input method (type to the assistant) and an audit trail (see what happened via drawing).

## The Submit Flow

### Step by Step

1. **User draws** — annotates existing nodes, sketches new boxes, draws arrows. No constraints, no snapping. Pure freeform.

2. **User hits Submit** — the board locks. No more drawing until this changeset is processed.

3. **System extracts the changeset** — associates comments with nodes by anchor relationships, infers arrow connections by spatial proximity, identifies new node requests from unattached doodle boxes.

4. **Changeset dispatches to manager builder** — same L2 dispatch pipeline as chat. The manager builder receives the structured changeset and begins working.

5. **Board shows processing state** — doodles dim and pulse. The user watches.

6. **Manager builder creates/modifies nodes** — using existing tools (create_node, remove_node, dispatch_to_builders). New nodes get placed at the positions from the user's drawing. Node behavior changes dispatch to the appropriate L4 builders.

7. **Transition** — as each real node is created or updated, it appears on the board. Fast fade-swap: doodle fades out (~300ms), real node fades in at the same position. Quick, no ceremony.

8. **Partial success handling** — if some changes succeed and others fail, the successful ones stay real. Failed doodles restore to their original state so the user can edit and resubmit. The user sees a mixed state: some nodes materialized, some sketches came back.

9. **Board unlocks** — the user can draw again.

## Spatial Inference

### Arrow-to-Node Connection

Arrows don't need to touch the boxes they connect. The system uses spatial proximity to infer connections after the user submits.

For each drawn arrow, the system finds the nearest rectangle to the arrow's start point and the nearest rectangle to the arrow's end point. If both are within a reasonable threshold (accounting for zoom level), the arrow registers as a connection between those two elements. If an arrow points at nothing, it's ignored.

This works for all combinations: doodle-to-doodle, doodle-to-real, real-to-real.

### Position Normalization

Users draw sloppy. Boxes overlap, sizes vary wildly. Real nodes have standard dimensions. The system normalizes positions before creating real nodes:

1. Take the centroid (center point) of each doodle box as the intended position.
2. Apply standard node dimensions (real nodes are all the same size).
3. Run overlap resolution — push nodes apart while preserving the user's relative layout. What was above stays above. What was left stays left.

The user's spatial intent is preserved. The exact pixel positions are cleaned up.

## User Stories

**"I want to sketch a workflow from nothing."**
Open a new workflow. Draw three boxes. Write what each one does. Connect them with arrows. Hit submit. Watch the real nodes appear where the sketches were. Done. No conversation needed.

**"I want to restructure my workflow."**
The board has five nodes. Draw a new box between nodes 2 and 3. Draw arrows connecting it. Add a comment to node 4: "remove this, it's redundant." Hit submit. The manager adds the new node, rewires the edges, and removes node 4.

**"I want to fix how a node behaves."**
Right-click a node. Add comment: "The output is too verbose, limit to 3 bullet points." Hit submit. The manager dispatches to that node's builder with the instruction. The node gets reconfigured.

**"I want to do something complex that's hard to draw."**
Open the chat. Type it out. The manager handles it the same way. Chat and drawing coexist. Use whichever is more natural for the task at hand.

**"I submitted but some changes failed."**
Three doodles submitted. Two became real nodes. One came back as a sketch — the manager couldn't interpret it. Edit the sketch, add more detail, resubmit just that one.

**"I changed my mind mid-drawing."**
Select a doodle element. Delete it. It's gone. Doodles are ephemeral until submitted. No undo needed — just delete and redraw.

## Hierarchy Trees Inside Nodes

Doodle boxes can contain other doodle boxes. A large rectangle with smaller rectangles and arrows inside it describes a team hierarchy — and that maps directly to workforce nodes with agent rosters.

```
╭ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ╮
╎ Research Team                            ╎
╎                                          ╎
╎    ╭ ─ ─ ─ ─ ─ ─ ╮                     ╎
╎    ╎ Lead          ╎                     ╎
╎    ╎ Researcher    ╎                     ╎
╎    ╰ ─ ─ ─ ┬ ─ ─ ╯                     ╎
╎         ╭───┼────╮                       ╎
╎         ▼   ▼    ▼                       ╎      ╭ ─ ─ ─ ─ ─ ╮
╎   ╭ ─ ─ ╮╭ ─ ─ ╮╭ ─ ─ ─ ╮             ╎      ╎ Write      ╎
╎   ╎ Web ╎╎Paper╎╎ Fact  ╎             ╎~~~~>╎ final     ╎
╎   ╎Srch ╎╎Anlyz╎╎ Check ╎             ╎      ╎ report    ╎
╎   ╰ ─ ─ ╯╰ ─ ─ ╯╰ ─ ─ ─ ╯             ╎      ╰ ─ ─ ─ ─ ─ ╯
╎                                          ╎
╰ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ╯
```

The system classifies rectangles by spatial containment. Is rectangle B fully inside rectangle A's bounds? Then B is an inner element — an agent in A's team. Arrows between inner elements describe the team's reporting structure. Arrows between outer elements describe the workflow DAG.

The extraction produces a nested changeset:

```
New Nodes:
  - "Research Team"
    Position: (50, 50)
    Hierarchy:
      Agents:
        - "Lead Researcher" (lead)
        - "Web Searcher"
        - "Paper Analyzer"
        - "Fact Checker"
      Reporting:
        - Lead Researcher → Web Searcher
        - Lead Researcher → Paper Analyzer
        - Lead Researcher → Fact Checker

  - "Write final report"
    Position: (500, 200)
    Hierarchy: none

Edges:
  - "Research Team" → "Write final report"
```

The manager builder reads "Research Team" with a hierarchy and creates a workforce node with that agent roster. The inner tree is the team structure. Lead Researcher is the designer, the three children are the agents. "Write final report" with no hierarchy becomes a single node. The user just drew their entire agent team on a canvas and the system understood it.

No forms. No config screens. No "add agent" buttons. Draw the tree, submit, watch it build.

## Rich Text Inside Nodes

### TerminalBlock in Doodle Nodes

The existing `TerminalBlock` component — the custom markdown renderer built from scratch with a full AST parser, JetBrains Mono, headings, lists, tables, code blocks, blockquotes — goes inside the doodle nodes. The user types markdown, the node renders it beautifully.

This means doodle nodes aren't limited to plain text. The user can write structured content:

```
WHAT THE USER TYPES:              WHAT THEY SEE IN THE NODE:

# Research Team                   Research Team
                                  ─────────────
- search the web for              • search the web for
  competitor pricing                competitor pricing
- analyze **quarterly reports**   • analyze quarterly reports
- fact-check all claims           • fact-check all claims

> Focus on Q4 2025 data only.     ┃ Focus on Q4 2025 data only.
```

Headings describe the node's purpose. Lists describe subtasks or agents. Blockquotes add context or constraints. Bold emphasizes what matters. All rendered by a component that already exists and already handles theming, memoization, and performance.

### Edit / Preview Toggle

The text editing experience uses a two-state toggle rather than a hybrid WYSIWYG approach. This avoids the hardest UX problem in canvas editing — cursor positioning inside rendered content.

Click into a doodle node: you see raw markdown in a plain text input. Type freely. Click away (or press Escape): the raw text is replaced by the TerminalBlock-rendered output. Click back in to edit again.

```
EDITING (clicked into node):          VIEWING (clicked away):

╭ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ╮        ╭ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ╮
╎ # Research Team             ╎        ╎ Research Team               ╎
╎                             ╎        ╎ ─────────────               ╎
╎ - Web Searcher              ╎   →    ╎  • Web Searcher             ╎
╎ - Paper Analyzer            ╎        ╎  • Paper Analyzer           ╎
╎ - **Fact Checker**          ╎        ╎  • Fact Checker              ╎
╎                             ╎        ╎                              ╎
╎ > Require citations.|       ╎        ╎  ┃ Require citations.       ╎
╰ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ╯        ╰ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ╯
     raw markdown                        TerminalBlock rendered
```

This also applies to side-anchored comments on real nodes. Click to type markdown, click away to see it rendered. The comments look polished in their sketchy containers.

## Technical Foundation

### Drawing Engine

The doodle layer uses the Excalidraw React component (`@excalidraw/excalidraw`, MIT licensed). It provides rectangle, arrow, and text tools with a hand-drawn aesthetic out of the box. The component runs as an overlay on the existing React Flow canvas, sharing the same viewport (pan and zoom stay synchronized).

Excalidraw elements are never persisted to the database. They live in local component state until submit, at which point they're extracted into a changeset and cleared. Doodles are ephemeral.

### Text Rendering

Inside doodle nodes and comment boxes, the existing `TerminalBlock` component handles markdown rendering. It brings the full parser (headings, lists, tables, code blocks, blockquotes, inline formatting), JetBrains Mono typography, and the terminal theme — all already built, tested, and memoized. No new rendering code needed.

### Real Node Rendering

Real workflow nodes continue to render through React Flow with the existing `CanvasNode` component. No changes to how real nodes look or behave. The doodle layer is purely additive.

### Rough.js (Future Enhancement)

For a fully unified aesthetic, real nodes could optionally render their borders using rough.js to match the hand-drawn feel. This is a visual polish pass, not a launch requirement. The contrast between sketchy doodles and clean real nodes is actually useful — it communicates state.

## What This Doesn't Change

- The manager assistant chat works exactly as it does today.
- The dispatch pipeline (L1 → L2 → L3 → L4) is unchanged.
- The manager builder remains the single source of truth for all workflow mutations.
- Node configuration, execution, the DAG engine — all untouched.
- Node chat sessions continue to work — they gain an audit trail of Visual Dispatch changes alongside direct conversation history.
- The doodle layer is a new input method, not a new execution path.

## What This Builds On

Every piece of this feature already exists in some form:

| Capability | Already built | Visual Dispatch adds |
|------------|--------------|---------------------|
| Canvas with nodes and edges | React Flow (`@xyflow/react`) | Doodle overlay via Excalidraw |
| Manager builder dispatch | L2 dispatch pipeline | Structured changeset as input |
| Node builder dispatch | L4 dispatch pipeline | Manager routes drawing annotations to builders |
| Rich text rendering | `TerminalBlock` (custom markdown AST parser) | Same component inside doodle nodes |
| Workforce agent teams | Agent rosters, pipeline service | Drawn hierarchy trees map to rosters |
| Node chat sessions | L3 assistant chat | Audit trail for drawing-initiated changes |
| Board state | `board_state::build()` queries DB | Changeset is the visual diff on top of board state |

Nothing gets thrown away. Nothing gets rewritten. Visual Dispatch is a visual front door to the system that already exists.

The insight is simple: the user already thinks visually about their workflow. The board is already spatial. Visual Dispatch lets them express intent in the same medium they're already looking at.
