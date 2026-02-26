# Sidebar Tree — Vision

## What It Is

The sidebar tree is a unified view that merges workflow structure and execution output into a single scrollable surface. There is no separate output panel. The tree IS the output view — nodes expand inline to reveal their content, styled with zero chrome. File-tree lines are the only structure. Text hangs beneath nodes like it's draped there.

This is an app for creative people, not developers. No config panels, no gear icons, no technical details. Click to open, click to close. Read the output. That's it.

## Layout

The tree tab takes the full sidebar. One scrollable view.

```
├── ▼ Gather Requirements        ● success
│
│   Identified 4 target competitors
│   and 3 pricing dimensions to
│   track across Q3 and Q4...
│                               ▼ expand
│
├─┬─ ▼ Web Search              ● success
│ │
│ │  Scraped 4 competitor sites.
│ │  Found current pricing for all
│ │  tiers. CompetitorA raised...
│ │                             ▼ expand
│ │
│ ├─ ▼ Paper Review            ● success
│ │
│ │  Analyzed quarterly earnings
│ │  report. Key finding: margin
│ │  compression in enterprise...
│ │                             ▼ expand
│ │
│ └─ ▼ Fact Check              ◐ running
│
│    ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
│
│    ━━━━━━━━━━━━━━━━━━━━━
│
│    ━━━━━━━━━━━━━━━━━━━━━━━━━
│
└── ▶ Write Final Report       ○ pending
```

## Design Principles

### No chrome

Output text has no borders, no cards, no background. It hangs directly below the tree node, indented to match the tree depth. The tree gutter lines (`│`) are the only visual structure. The text is the content. Nothing else.

### File tree lines are the spine

Box-drawing characters (`├──`, `└──`, `│`) run continuously down the left side. They provide hierarchy, padding, and visual grouping. Every piece of content is visually owned by its parent node through these lines.

### Minimal styling

Dark mode: white text on black background. Light mode: dark text on light background. No decorative elements. The skeleton, the text, and the tree lines. That's the entire visual language.

## Behaviors

### Click to expand/collapse

Click a node row to toggle its children and output content.

- `▼` — expanded, showing output or children below
- `▶` — collapsed, just the header row

Users can leave any combination of nodes expanded. Expand state persists within the session.

### Before any run

All nodes are collapsed. The tree is a table of contents — just node names with `○ pending` status. No output areas, no skeletons. A clean outline of the workflow structure.

```
├── ▶ Gather Requirements       ○ pending
├── ▶ Web Search                ○ pending
├── ▶ Paper Review              ○ pending
├── ▶ Fact Check                ○ pending
└── ▶ Write Final Report        ○ pending
```

### Running — skeleton lines

When a node is running, its output area shows thin horizontal bars at varying widths with generous vertical spacing between them. The bars pulse opacity (easing between ~0.3 and ~0.7) on a slow cycle. On dark mode these are soft white lines gently breathing on black.

```
│ └─ ▼ Fact Check              ◐ running
│
│    ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
│
│    ━━━━━━━━━━━━━━━━━━━━━
│
│    ━━━━━━━━━━━━━━━━━━━━━━━━━
```

When output arrives, the skeleton is replaced with real text. No transition — the lines disappear, text is there.

### Success — output preview

Completed outputs show a ~200px max-height preview rendered with `TerminalBlock` (the existing markdown renderer). A subtle "expand" affordance at the bottom lets the user see the full output inline. Expanding pushes everything below it down.

```
├── ▼ Gather Requirements        ● success
│
│   Identified 4 target competitors
│   and 3 pricing dimensions to
│   track across Q3 and Q4...
│                               ▼ expand
```

### Error — red text

Errors render as red-colored text hanging the same way as normal output. Same position, same indent, same lack of chrome. The status dot is red. The text is red. The user sees something went wrong without needing to parse technical UI.

```
├── ▼ Fact Check                ● error
│
│   Failed to connect to search API.
│   Request timed out after 30s.
```

### Expanding full output

When the user clicks "expand", the 200px max-height constraint is removed. The full output renders inline via `TerminalBlock`, which handles markdown: headings, code blocks, lists, tables, bold/italic. Everything flows within the tree gutter.

```
├── ▼ Gather Requirements        ● success
│
│   Identified 4 target competitors
│   and 3 pricing dimensions to
│   track across Q3 and Q4.
│
│   ## Competitors
│   - CompetitorA (Enterprise tier)
│   - CompetitorB (Growth tier)
│   - CompetitorC (Startup tier)
│   - CompetitorD (Enterprise tier)
│
│   ## Dimensions
│   1. Base pricing per seat
│   2. Volume discount thresholds
│   3. Annual vs monthly spread
│                               ▲ collapse
```

## Status indicators

| Status | Dot | Output area |
|---|---|---|
| `idle` / `pending` | `○` hollow | None (collapsed) |
| `running` | `◐` half-fill, animated | Skeleton lines pulsing |
| `success` | `●` green | Text preview (200px) with expand |
| `error` | `●` red | Red text |
| `skipped` | `○` grey | None |

## DAG Topology — Complete Gutter Reference

The tree gutter represents DAG topology, not just depth. When steps run in parallel, the gutter forks. When they converge, it merges back. Every possible DAG shape maps to a specific gutter pattern.

### Gutter vocabulary

| Character | Meaning |
|---|---|
| `├──` | Sequential step (more steps follow) |
| `└──` | Sequential step (last step / merge point) |
| `│` | Continuation line (something below is connected) |
| `├─┬─` | Fork point — this step starts a parallel group |
| `│ ├─` | Parallel sibling (more siblings follow) |
| `│ └─` | Last parallel sibling |
| `┬─` | Root-level parallel (no preceding fork, used for fan-in from multiple roots) |

---

### 1. Sequential

The simplest DAG. A straight chain.

**Edges:** A → B → C

```
├── A
├── B
└── C
```

---

### 2. Parallel fan-out, single merge

One step fans out to multiple parallel steps, which all converge at one merge point.

**Edges:** A → B, A → C, A → D, B → E, C → E, D → E

```
├── A
├─┬─ B
│ ├─ C
│ └─ D
└── E
```

The `├─┬─` is the fork. The inner `├─` and `└─` are parallel siblings. The outer `│` continues past all of them to the merge point `└──`.

---

### 3. Multiple sequential forks

Two separate fork/merge sections in a row. The first fork resolves before the second begins.

**Edges:** A → B, A → C, B → D, C → D, D → E, D → F, E → G, F → G

```
├── A
├─┬─ B
│ └─ C
├── D
├─┬─ E
│ └─ F
└── G
```

Each fork opens and closes cleanly before the next one starts. The gutter never nests — it returns to the main spine between forks.

---

### 4. Nested forks

A fork within a branch of an outer fork. One parallel branch itself contains a sub-fork.

**Edges:** A → B, A → C, B → D, B → E, D → F, E → F, F → G, C → G

```
├── A
├─┬─ B
│ │  ├─┬─ D
│ │  │ └─ E
│ │  └── F
│ └─ C
└── G
```

The outer fork splits A into two branches: the B-branch (which contains its own sub-fork D/E merging at F) and the C-branch. Both converge at G. The gutter nests — two levels of `│` columns show the parallel depth.

---

### 5. Multiple independent roots

Two or more entry points that never share an ancestor. Completely independent chains.

**Edges:** A → B, C → D

```
├── A
├── B
│
├── C
└── D
```

Independent chains are top-level siblings separated by a gap. No fork/merge connectors — they aren't parallel, they're unrelated. The user drew them as separate workflows on the board.

---

### 6. Fan-in from independent roots

Multiple root nodes with no common ancestor that converge into a single step.

**Edges:** A → C, B → C

```
┬─ A
├─ B
└── C
```

The `┬─` at the top indicates parallel roots — no preceding fork point because there is no common ancestor. They converge at C with `└──`. This is visually similar to a fork/merge but starts at the root level.

With a continuation after the merge:

**Edges:** A → C, B → C, C → D

```
┬─ A
├─ B
├── C
└── D
```

---

### 7. Partial merge (sub-groups within a fork)

A fans out to B, C, D. But they don't all merge at the same point — B and C merge into E, D continues to F, then E and F merge at G.

**Edges:** A → B, A → C, A → D, B → E, C → E, D → F, E → G, F → G

```
├── A
├─┬─ B
│ ├─ C
│ ├── E
│ │
│ └─ D
│    └── F
└── G
```

The outer fork from A opens with `├─┬─`. B and C are parallel siblings that merge at E (shown as a sequential continuation `├──` within the fork). D is another parallel branch that continues to F. The entire group converges at G with `└──`.

The key: E and F sit at different positions within the fork because they belong to different sub-groups. E follows B+C. F follows D. The gutter shows this through indentation and continuation lines.

---

### 8. Wide fan-out (many parallel branches)

A single fork with many parallel branches. Same pattern as #2, just wider.

**Edges:** A → B, A → C, A → D, A → E, A → F, B → G, C → G, D → G, E → G, F → G

```
├── A
├─┬─ B
│ ├─ C
│ ├─ D
│ ├─ E
│ └─ F
└── G
```

The `├─` / `└─` pattern scales to any number of parallel siblings. The outer `│` runs the full height.

---

### 9. Diamond

The minimal fork/merge. One step splits to two, they rejoin at one.

**Edges:** A → B, A → C, B → D, C → D

```
├── A
├─┬─ B
│ └─ C
└── D
```

---

### 10. Sequential with a single parallel section in the middle

A common real-world pattern. Linear flow, then a parallel burst, then linear again.

**Edges:** A → B, A → C, A → D, B → E, C → E, D → E, E → F

```
├── A
├─┬─ B
│ ├─ C
│ └─ D
├── E
└── F
```

The fork opens after A, closes at E, then the pipeline continues sequentially to F.

---

### Reading the gutter

The left edge of the tree tells you the shape of the DAG at a glance without reading any content:

```
├──          sequential
├──          sequential
├─┬─         fork (parallel starts)
│ ├─         parallel sibling
│ │  ├─┬─    nested fork
│ │  │ └─    nested parallel end
│ │  └──     nested merge
│ └─         parallel end
├──          merge / sequential continues
├─┬─         another fork
│ └─         parallel end
└──          final merge / last step
```

Scan the left column: every `│` is a parallel group still open. Every `└` is a group closing. The depth of `│` nesting tells you how many parallel groups are active at that point in the pipeline.

## Scope

### What we're building

- Unified tree+output sidebar component replacing the current split layout (`StepTree` + `StepOutputPanel`)
- Reworked `buildStepTree` that reads DAG topology to produce fork/merge connectors
- Skeleton line component with pulse animation
- Output preview with 200px max-height and expand/collapse
- TerminalBlock integration for rendered output
- Status dot component with the five states

### What we're removing

- `StepOutputPanel` — output is now inline in the tree
- The split layout (tree top half, output bottom half)
- Config panel / node selection for configuration
- Sub-agent rows in the tree (deferred)

### What stays the same

- `WorkflowSidebar` container (tabs, resize handle)
- `sidebarStore` (tab state, width, drag)
- `workflowExecutionStore` (step states, outputs, status)
- `TerminalBlock` and the terminal renderer pipeline
- Chat tab

## Technical notes

### buildStepTree rework

The current `buildStepTree` does a DFS from root nodes and produces a flat list with `depth` and `isLast` flags. This works for sequential trees but cannot represent parallel branches.

The reworked version needs to:

1. Read the full edge topology (not just DFS parent-child)
2. Identify fork points — nodes with multiple outgoing edges
3. Identify merge points — nodes with multiple incoming edges
4. Produce connector metadata per row: which gutter lines are active at each depth, whether a row starts a fork (`├─┬─`), continues a parallel branch (`│ ├─`), or ends one (`│ └─`)

The output is still a flat `TreeEntry[]`, but each entry carries enough gutter metadata to render the correct box-drawing characters.

### Gutter rendering

Each tree row renders its left gutter as a series of fixed-width columns. Each column is either empty, a vertical line (`│`), a branch (`├──`), a terminator (`└──`), a fork start (`├─┬─`), a parallel branch (`│ ├─`), or a parallel terminator (`│ └─`). The gutter columns are determined by the entry's depth and connector metadata.

This replaces the current approach of simple depth-based left padding with a single connector character.
