# Canvas Placement Engine — Vision & Feature Spec

## Problem Statement

The manager node's builder creates workflow nodes in rapid bursts (`create_pipeline`, `create_parallel`, `insert_node`). These nodes arrive at the frontend without positions. The canvas needs a placement engine that:

1. Computes grid-snapped positions for AI-created nodes in real time
2. Never moves existing user-placed nodes
3. Handles known DAG topologies (pipelines, fan-out/fan-in, splice-insert)
4. Scales to dozens of nodes in mixed topologies
5. Supports future node types (database, repo) placed manually by users

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Flow direction | Left to right | Pipelines read naturally L→R. Parallels fan out vertically. |
| Position computation | Frontend computes | Server creates steps with sentinel positions. Frontend detects unplaced nodes and runs placement. Layout logic stays in UI layer. |
| Existing node policy | Never move | Find open space around fixed nodes. User-placed positions are sacred. |
| Insert behavior | Shift immediate neighbors | `insert_node` places at midpoint, shifts the downstream neighbor (and only that neighbor) right. |
| Manager node | Regular canvas node, singleton, auto-created | One per workflow. Permanent history. Same node system as workforce. |
| Animation | Light fade-in | Subtle opacity transition. No heavy animation — lots of text on screen. |
| User-placed nodes | Context menu (existing pattern) | Database, repo, and other future types added to the context menu. |
| Algorithm approach | Custom placement engine | No free library guarantees "never move existing nodes." Known topologies don't need general-purpose graph layout. See algorithm research below. |

## Algorithm Research Summary

### Why Not Off-the-Shelf

| Library | Fatal Flaw |
|---------|------------|
| **Dagre** | Cannot fix node positions. Adding nodes shifts everything. |
| **d3-dag** | Cannot fix node positions. Full re-layout only. |
| **d3-force** | Poor directional flow for DAGs. Jittery convergence. Non-deterministic. |
| **ELK (elkjs)** | 1MB+ bundle. Interactive mode *reduces* but doesn't *guarantee* zero movement. Async-only. |
| **yFiles** | $15k+/year commercial license. |

### Recommended: 4 Focused Placement Strategies

The topologies are known and constrained. Four specialized strategies compose to handle all cases:

```
PlacementEngine
  ├── PipelinePlacer          — left-to-right chain placement
  ├── FanOutPlacer            — vertical distribution, optional convergence
  ├── SplicePlacer            — insert between, shift downstream neighbor
  └── FreeSpaceFinder         — spiral grid scan for open space (fallback)
```

Each is O(n) per node, deterministic, synchronous, grid-snapped on first call.

For rapid-fire bursts: batch all nodes → topological sort → place in topo order (each sees prior placements as "existing") → single React state update.

### Key Concept: Pinned vs Auto-Placed

- **User-placed nodes** → `pinned: true`. Never moved by any algorithm.
- **AI-placed nodes** → `pinned: false`. Can be shifted by splice operations.
- **User drags an AI-placed node** → becomes `pinned: true`.

Only the `SplicePlacer` ever moves existing nodes, and only auto-placed ones.

---

## Existing Infrastructure

The canvas already has foundational layout utilities that the placement engine builds on:

| Module | What It Does | Reuse |
|--------|-------------|-------|
| `layout/collisionDetection.ts` | `detectOverlaps()`, `resolveOverlaps()` with cascading push | Collision checking for all placers |
| `layout/snapAlignment.ts` | `buildAlignmentGuides()`, `computeSnap()`, `computeMagneticSnap()` | Guide-based snapping during user drag |
| `layout/gridNotch.ts` | `notchToGrid()` — quantize to grid multiple | Grid-snap all computed positions |
| `layout/autoLayout.ts` | `computeTowerPositions()` — tier-based agent stacking | Pattern reference for new placers |
| `layout/autoLayoutConfig.ts` | `TOWER_LAYOUT` spacing constants | Extend with pipeline/fan-out constants |
| `layout/types.ts` | `LayoutNode`, `LayoutEdge`, `Overlap`, `SnapResult` | Core types for all placement logic |
| `constants.ts` | `CANVAS.GRID_SIZE` (24px), `CANVAS.COLLISION_GAP` (24px) | Grid and gap values |
| `CanvasNode/registry.ts` | `VARIANT_CONFIGS` — default width/height per variant | Node dimensions for placement |
| `CanvasContextMenu.tsx` | `workflowStore.createStep()` with position | Entry point for user-created nodes |

---

## User Stories

### US-1: Pipeline Auto-Placement

**As** the manager's builder,
**when** I call `create_pipeline([{name: "Collector"}, {name: "Analyzer"}, {name: "Reporter"}])`,
**I want** the three nodes to appear on the canvas in a left-to-right chain with even spacing,
**so that** the user sees a clean `Collector → Analyzer → Reporter` layout without manual positioning.

#### Acceptance Criteria

- Nodes are placed left-to-right: each node's X = previous node's right edge + `H_GAP`
- All nodes vertically aligned (same Y coordinate)
- All positions snapped to 24px grid
- No overlap with any existing node on the canvas
- If the preferred position collides, the entire pipeline shifts to the nearest open space (preserving internal spacing)
- Edges between pipeline nodes render correctly after placement

#### Placement Algorithm

```
pipeline_place(nodes: Node[], anchor: Point):
  1. Start at anchor (or rightmost existing node + H_GAP if no anchor)
  2. For each node in order:
     a. candidateX = previousNode.right + H_GAP (or anchor.x for first)
     b. candidateY = anchor.y
     c. Snap (candidateX, candidateY) to 24px grid
     d. If overlaps existing → scan downward by (nodeHeight + V_GAP) increments
     e. If downward exhausted → scan upward
     f. Record position, treat as "existing" for next node
  3. Return all positions
```

#### Sequence

```
Server (builder tool)         Frontend (WebSocket)              PlacementEngine
─────────────────────         ────────────────────              ───────────────
create_pipeline(A,B,C)
  → DB: 3 steps created
  → DB: 2 edges created
  → WS: step_created × 3  ──→  Receives 3 steps
  → WS: edge_created × 2  ──→  Receives 2 edges

                                Detects unplaced nodes ────────→ batch([A,B,C], edges)
                                                                  │
                                                                  ├─ Topo sort: A→B→C
                                                                  ├─ Classify: pipeline
                                                                  ├─ PipelinePlacer.place()
                                                                  │   ├─ A at (anchor)
                                                                  │   ├─ B at (A.right + H_GAP, A.y)
                                                                  │   └─ C at (B.right + H_GAP, B.y)
                                                                  ├─ Collision check all
                                                                  └─ Return positions

                                Apply positions ←─────────────── {A: pos, B: pos, C: pos}
                                  │
                                  ├─ setNodes() (single update)
                                  ├─ PATCH position_x/y to server
                                  └─ Light fade-in animation
```

---

### US-2: Fan-Out Auto-Placement

**As** the manager's builder,
**when** I call `create_parallel(source: "Collector", parallel: [{name: "PriceAnalyzer"}, {name: "FeatureAnalyzer"}, {name: "SentimentAnalyzer"}], target: "Synthesizer")`,
**I want** the parallel nodes stacked vertically to the right of the source, with the target node to their right,
**so that** the user sees a clean fan-out/fan-in pattern.

#### Acceptance Criteria

- Source node already exists and is not moved
- N parallel nodes placed in a vertical stack, one column to the right of source
- Stack centered on source's vertical midpoint
- Target node placed one column to the right of the parallel stack, vertically centered
- All positions grid-snapped
- No overlap with existing nodes
- If the fan column collides, the entire group shifts as a unit to find open space

#### Placement Algorithm

```
fan_out_place(source: Rect, parallel: Node[], target: Node | null):
  1. fanColumnX = source.right + H_GAP
  2. totalHeight = N * nodeHeight + (N-1) * V_GAP
  3. startY = source.centerY - totalHeight / 2
  4. For each parallel node i:
     a. position = (fanColumnX, startY + i * (nodeHeight + V_GAP))
     b. Snap to grid
  5. Collision check entire group as a unit
     a. If any node collides → shift entire group down (preserve relative positions)
     b. If still colliding → shift group further down in V_GAP increments
  6. If target exists:
     a. targetX = fanColumnX + maxParallelWidth + H_GAP
     b. targetY = source.centerY (vertically aligned with source)
     c. Collision check target independently
  7. Return all positions
```

#### Visual Result

```
                    ┌────────────────┐
                    │ PriceAnalyzer  │
                    └───────┬────────┘
                            │
┌───────────┐       ┌───────┴────────┐       ┌─────────────┐
│ Collector ├───────┤FeatureAnalyzer ├───────┤ Synthesizer │
└───────────┘       └───────┬────────┘       └─────────────┘
                            │
                    ┌───────┴────────┐
                    │SentimentAnalyz.│
                    └────────────────┘
```

---

### US-3: Insert-Between (Splice)

**As** the manager's builder,
**when** I call `insert_node(between: {from: "Collector", to: "Analyzer"}, node: {name: "Validator"})`,
**I want** the new node placed between the two existing nodes, with the downstream node shifted right if needed,
**so that** the pipeline stays readable with the new node spliced in.

#### Acceptance Criteria

- New node placed at the horizontal midpoint between `from` and `to`
- Vertically aligned with the edge path
- If there's enough space (from.right + H_GAP + nodeWidth + H_GAP < to.left), place in-gap without moving anyone
- If not enough space, shift `to` rightward by `(nodeWidth + 2 * H_GAP)` — but only if `to` is auto-placed (`pinned: false`)
- If `to` is user-pinned, place the new node above or below the edge line instead
- Only the immediate downstream neighbor shifts — not the entire downstream chain
- Grid-snap all positions

#### Placement Algorithm

```
splice_place(from: Rect, to: Rect, newNode: NodeDims, toIsPinned: boolean):
  1. availableGap = to.left - from.right
  2. requiredGap = H_GAP + newNode.width + H_GAP
  3. If availableGap >= requiredGap:
     a. newX = from.right + H_GAP
     b. newY = midpoint(from.centerY, to.centerY)
     c. Snap to grid → done, no shifting needed
  4. Else if !toIsPinned:
     a. shiftAmount = requiredGap - availableGap
     b. to.x += shiftAmount (snap to grid)
     c. newX = from.right + H_GAP
     d. newY = midpoint(from.centerY, to.centerY)
     e. Collision check shifted `to` against other nodes
  5. Else (to is pinned, can't shift):
     a. newX = midpoint(from.right, to.left) - newNode.width / 2
     b. newY = from.centerY - newNode.height - V_GAP (try above)
     c. If overlaps → try below: newY = from.centerY + from.height / 2 + V_GAP
     d. Snap to grid
  6. Return newPosition + optional toShift
```

#### Sequence

```
Before:   [Collector] ────────────────────→ [Analyzer]

After:    [Collector] → [Validator] → [Analyzer]
                                        (shifted right if needed)
```

---

### US-4: Open-Space Placement (Fallback)

**As** a user or the manager's builder,
**when** a node is created without a known topological relationship to existing nodes (disconnected, or edges aren't yet defined),
**I want** it placed in the nearest open space on the canvas,
**so that** it doesn't overlap anything and is easy to find.

#### Acceptance Criteria

- Node placed at the nearest open position to a preferred anchor
- Preferred anchor: to the right of the rightmost node + H_GAP, or canvas center if empty
- Search expands outward from the anchor in a spiral pattern
- All positions grid-snapped
- For user context-menu creation: preferred anchor is the click position (existing behavior preserved)

#### Placement Algorithm

```
find_open_space(nodeWidth, nodeHeight, preferredX, preferredY, existingRects, gridSize):
  1. Pad each existing rect by COLLISION_GAP (24px) on all sides
  2. Candidate = (snap(preferredX), snap(preferredY))
  3. If candidate doesn't overlap any padded rect → return candidate
  4. Spiral search:
     for radius = gridSize to MAX_SEARCH_RADIUS step gridSize:
       for each (dx, dy) in spiral_ring(radius, gridSize):
         candidate = (snap(preferredX + dx), snap(preferredY + dy))
         if no overlap with any padded rect → return candidate
  5. Absolute fallback: (rightmostNode.right + H_GAP * 4, preferredY)
```

---

### US-5: Manager Node Auto-Creation

**As** the system,
**when** a new workflow is created,
**I want** a manager node to be automatically added to the canvas,
**so that** the user has a conversational entry point from the start.

#### Acceptance Criteria

- Exactly one manager node per workflow (singleton constraint)
- Auto-created when the workflow is first opened (or created)
- Positioned at a fixed initial location (e.g., far left, vertically centered)
- Cannot be deleted by the user
- Has permanent chat history (session persists across page loads)
- Uses the same canvas node system as workforce (TabbedLayout, chat tab)
- Context menu does not show "Manager" as a creatable type (since it's auto-created)

#### Manager Node Variant

```typescript
// Addition to CanvasNode/registry.ts
manager: {
  label: 'Manager',
  color: '#f97316',           // orange — distinct from all other types
  icon: AdminPanelSettingsOutlined,
  layout: 'tabbed',
  canvasNodeKind: CanvasNodeKind.MANAGER,
  defaultWidth: 560,
  defaultHeight: 500,
  constraints: { minWidth: 360, minHeight: 300, maxWidth: 1800, maxHeight: 1600 },
}
```

#### Positioning

The manager node sits at the far left of the canvas. When the builder creates pipeline/parallel nodes, they appear to the right of the manager.

```
 ┌───────────┐
 │  Manager  │      [Collector] → [Analyzer] → [Reporter]
 │  (chat)   │
 └───────────┘
```

The manager is not connected by data edges. Its presence is ambient — it can dispatch to any node.

---

### US-6: Rapid-Fire Burst Handling

**As** the placement engine,
**when** the manager's builder creates 3-10 nodes in a burst (multiple WS events arriving within milliseconds),
**I want** to batch them into a single placement pass,
**so that** the canvas re-renders once with all nodes in correct positions.

#### Acceptance Criteria

- WebSocket events arriving within a `BURST_WINDOW` (e.g., 100ms) are batched
- Batch is topologically sorted by internal edges before placement
- Each node's placement sees prior nodes in the batch as "existing"
- Single `setNodes()` call after the entire batch is placed
- Single PATCH to server with all position updates
- Light fade-in animation applied to all nodes in the batch simultaneously

#### Batching Sequence

```
t=0ms    WS: step_created(A)  ──→ Buffer
t=5ms    WS: step_created(B)  ──→ Buffer
t=10ms   WS: step_created(C)  ──→ Buffer
t=15ms   WS: edge_created(A→B) ──→ Buffer
t=20ms   WS: edge_created(B→C) ──→ Buffer
t=100ms  BURST_WINDOW expires  ──→ Flush buffer
                                     │
                                     ├─ Topo sort: A, B, C
                                     ├─ Classify topology: pipeline
                                     ├─ PipelinePlacer.place(A, B, C)
                                     ├─ Collision check against existing
                                     ├─ setNodes() — single render
                                     └─ PATCH positions to server
```

---

### US-7: Pinned Node Transition

**As** a user,
**when** I drag an AI-placed node to a new position,
**I want** that node to become "pinned" (immovable by the placement engine),
**so that** future AI operations respect my manual arrangement.

#### Acceptance Criteria

- All nodes created by the manager's builder start as `pinned: false`
- All nodes created by the user (context menu) start as `pinned: true`
- Dragging an auto-placed node sets `pinned: true`
- The manager node is always `pinned: false` (auto-positioned at far left, but can be dragged to pin)
- The `pinned` flag is persisted (either in step metadata or a canvas-local store)
- `SplicePlacer` checks `pinned` before shifting a downstream node

---

### US-8: Light Entrance Animation

**As** a user,
**when** new nodes appear on the canvas (from AI placement),
**I want** a subtle fade-in so I notice them without distraction,
**so that** the canvas feels alive without overwhelming the text-heavy UI.

#### Acceptance Criteria

- New nodes fade in from `opacity: 0` to `opacity: 1` over 200ms
- No scale or slide animation (too heavy with lots of text)
- Animation is CSS-only (no JS animation frame loops)
- Batch-created nodes all animate simultaneously (not staggered)
- Animation does not block interaction — nodes are interactive immediately

#### Implementation

```css
.canvas-node-enter {
  animation: nodeEnter 200ms ease-out;
}

@keyframes nodeEnter {
  from { opacity: 0; }
  to   { opacity: 1; }
}
```

---

## Architecture

### Placement Engine Module Structure

```
frontend/src/components/canvas/layout/
├── placement/
│   ├── mod.ts                  # PlacementEngine — orchestrator
│   ├── types.ts                # PlacementIntent, PlacementResult, NodePlacement
│   ├── pipelinePlacer.ts       # Left-to-right chain placement
│   ├── fanOutPlacer.ts         # Vertical fan-out/fan-in
│   ├── splicePlacer.ts         # Insert-between with neighbor shift
│   ├── freeSpaceFinder.ts      # Spiral grid scan fallback
│   ├── topologyClassifier.ts   # Edges → PlacementIntent classification
│   ├── occupancyIndex.ts       # Padded rect collection for collision queries
│   ├── constants.ts            # H_GAP, V_GAP, BURST_WINDOW, MAX_SEARCH_RADIUS
│   └── tests/
│       ├── pipelinePlacer.test.ts
│       ├── fanOutPlacer.test.ts
│       ├── splicePlacer.test.ts
│       ├── freeSpaceFinder.test.ts
│       ├── topologyClassifier.test.ts
│       └── occupancyIndex.test.ts
├── collisionDetection.ts       # (existing)
├── snapAlignment.ts            # (existing)
├── gridNotch.ts                # (existing)
├── autoLayout.ts               # (existing — tower layout)
├── autoLayoutConfig.ts         # (existing — extend with placement constants)
└── types.ts                    # (existing — extend with placement types)
```

### Core Types

```typescript
/** How a node was placed — determines whether placement engine can move it. */
type PlacementSource = 'user' | 'auto'

/** Classification of a batch of unplaced nodes by their edge topology. */
type PlacementIntent =
  | { type: 'pipeline'; nodes: UnplacedNode[]; edges: EdgeInfo[] }
  | { type: 'fan_out'; source: string; parallel: UnplacedNode[]; target: UnplacedNode | null; edges: EdgeInfo[] }
  | { type: 'splice'; from: string; to: string; newNode: UnplacedNode }
  | { type: 'disconnected'; nodes: UnplacedNode[] }

/** An unplaced node needing a position. */
type UnplacedNode = {
  id: string
  variant: NodeVariant
  width: number
  height: number
}

/** The result of placing a batch of nodes. */
type PlacementResult = {
  positions: ReadonlyMap<string, Point>
  shifts: ReadonlyMap<string, Point>     // existing nodes that were shifted (splice only)
}

/** Padded rectangle in the occupancy index. */
type OccupiedRect = {
  nodeId: string
  rect: Rect               // original rect
  paddedRect: Rect          // rect + COLLISION_GAP padding
  pinned: boolean
}
```

### Spacing Constants

```typescript
// placement/constants.ts

/** Horizontal gap between pipeline nodes (3 grid cells = 72px). */
const H_GAP = 72

/** Vertical gap between fan-out nodes (2 grid cells = 48px). */
const V_GAP = 48

/** Time window (ms) to batch rapid-fire WS events before placement. */
const BURST_WINDOW = 100

/** Maximum search radius (px) for spiral open-space scan. */
const MAX_SEARCH_RADIUS = 2400

/** Padding around existing nodes for collision queries. */
const OCCUPANCY_PAD = 24
```

### Data Flow

```
                    ┌─────────────────────────────────────────────┐
                    │              WebSocket Events                │
                    │  step_created, edge_created (rapid-fire)    │
                    └──────────────────┬──────────────────────────┘
                                       │
                                       ▼
                    ┌──────────────────────────────────────────────┐
                    │            Burst Buffer (100ms)              │
                    │  Collects unplaced nodes + new edges         │
                    └──────────────────┬──────────────────────────┘
                                       │ flush
                                       ▼
                    ┌──────────────────────────────────────────────┐
                    │         TopologyClassifier                   │
                    │  Edges → PlacementIntent                     │
                    │  (pipeline | fan_out | splice | disconnected)│
                    └──────────────────┬──────────────────────────┘
                                       │
                                       ▼
              ┌────────────────────────────────────────────────────────┐
              │                   PlacementEngine                      │
              │                                                        │
              │  1. Build OccupancyIndex from all existing nodes       │
              │  2. Route to strategy:                                  │
              │     ├─ pipeline   → PipelinePlacer                     │
              │     ├─ fan_out    → FanOutPlacer                       │
              │     ├─ splice     → SplicePlacer                       │
              │     └─ disconnected → FreeSpaceFinder                  │
              │  3. Each strategy returns PlacementResult               │
              │  4. Merge results                                       │
              └────────────────────────┬──────────────────────────────┘
                                       │
                                       ▼
              ┌────────────────────────────────────────────────────────┐
              │                    Apply Positions                      │
              │                                                        │
              │  1. setNodes() — single React state update             │
              │  2. PATCH position_x/y to server (batched)             │
              │  3. Apply .canvas-node-enter CSS class (fade-in)       │
              │  4. Mark all placed nodes as pinned: false              │
              └────────────────────────────────────────────────────────┘
```

---

## Topology Classifier Logic

The classifier examines a batch of unplaced nodes + their edges to determine which placement strategy to use.

```
classify(unplacedNodes: UnplacedNode[], newEdges: EdgeInfo[], existingNodes: ExistingNode[]):

  1. Build adjacency from newEdges
  2. Identify "anchored" edges: one end is an existing node, the other is unplaced

  Case: SPLICE
    - Exactly 1 unplaced node
    - Has an incoming edge from an existing node AND an outgoing edge to an existing node
    - The existing nodes were previously connected by an edge (that edge was removed)
    → return { type: 'splice', from, to, newNode }

  Case: PIPELINE
    - Unplaced nodes form a linear chain (each has at most 1 in-edge + 1 out-edge within the batch)
    - Optionally anchored: first node has an incoming edge from an existing node
    → return { type: 'pipeline', nodes: topoSorted, edges }

  Case: FAN_OUT
    - One existing node (source) has outgoing edges to multiple unplaced nodes
    - Optionally: the unplaced nodes converge to a single unplaced target
    → return { type: 'fan_out', source, parallel, target }

  Case: DISCONNECTED (fallback)
    - Nodes with no edges, or mixed topologies that don't match above patterns
    → return { type: 'disconnected', nodes }
```

---

## Build Phases

### Phase 1: Core Placement (covers 80% of cases)

- `OccupancyIndex` — padded rect collection, collision queries
- `PipelinePlacer` — left-to-right chain
- `FreeSpaceFinder` — spiral fallback
- `TopologyClassifier` — route to correct placer
- `PlacementEngine` — orchestrator
- Burst buffer hook (`usePlacementBatcher`)
- Tests for all pure functions

### Phase 2: Fan-Out + Splice

- `FanOutPlacer` — vertical stack with optional convergence
- `SplicePlacer` — insert-between with neighbor shift
- `pinned` flag on node data + drag transition
- Extend classifier for fan-out and splice detection

### Phase 3: Manager Node Integration

- Manager variant in `registry.ts`
- Singleton constraint (auto-create, prevent delete, prevent duplicates)
- Initial position: far left of canvas
- Chat tab with permanent session history

### Phase 4: Polish

- Light fade-in animation (CSS class toggle)
- Edge cases: empty canvas, single node, overlapping bursts
- Performance profiling with 50+ nodes

---

## Open Questions (Future)

These are deferred and not part of the current spec:

1. **Camera behavior** — Should the viewport auto-pan to show new nodes? Deferred per user decision.
2. **Full re-layout button** — An "auto-arrange everything" option using ELK or a full Sugiyama pass. Only for explicit user action, never automatic.
3. **Database / Repo node types** — Future node variants placed via context menu. The placement engine's `FreeSpaceFinder` handles them automatically.
4. **Undo/redo** — Should placement be undoable? Depends on broader undo system.
5. **Layout direction toggle** — Currently hardcoded to left-to-right. Could become configurable later.
