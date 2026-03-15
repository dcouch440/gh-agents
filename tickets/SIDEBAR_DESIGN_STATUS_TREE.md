# Sidebar Design-Status Tree

## Overview

Replace the flat dispatch accordion with a **tree-first sidebar** that shows the full agent hierarchy with live status indicators. The tree updates in real-time as the builder configures rosters and the designer writes per-agent prompts — then transitions to execution tracking when the user runs the workflow.

**Why this matters:**
- The current DispatchPanel shows flat accordion rows per dispatch — no hierarchy, no agent-level visibility
- The backend already emits three granular events (`workforce_designer_progress`, `designer_agent_designed`, `workforce_agent_progress`) but the frontend silently ignores them
- Users need to see at a glance: which nodes are designed, which agents have prompts, what's still pending — before they hit run
- The vision doc (vision-system-store.md §Designer Frontend Events) specifies this exact UX

**Entry points:** Board submit (Phase 0 → builder → designer) and workflow execution (run).

---

## Status Lifecycle

Every node and agent moves through a single linear status progression:

```
○ pending          Phase 0 created the node, nothing else yet
◑ building...      Builder dispatch is configuring the roster
○ built (3)        Roster ready, agents visible in tree, no prompts yet
◐ designing (1/3)  Designer writing per-agent prompts to store
● designed         All prompts written, ready for user review
▶ running (1/3 ✓)  Execution in progress, agents completing
✓ completed 3.2s   Done, artifacts in store
✗ failed           Agent or step errored
```

Agent-level indicators (nested under nodes):

```
○ pending          Not yet reached by designer or executor
● designed         Prompt written to store
◐ running...       Agent executing (tokens streaming)
✓ completed 420ms  Agent finished successfully
✗ failed           Agent errored
```

---

## Temporal Walkthrough — Design Phase

User draws 3 nodes, hits submit.

### T0 — Phase 0 complete (instant, from POST response)

```
── Research Competitors          ○ pending
── Analyze Findings              ○ pending
── Write Report                  ○ pending
```

### T1 — Builder dispatched for Research

```
── Research Competitors          ◑ building...
── Analyze Findings              ○ pending
── Write Report                  ○ pending
```

### T2 — Builder completes Research, roster appears

```
── Research Competitors          ○ built (3 agents)
│  ├── Scanner                   ○ pending
│  ├── Deep Researcher           ○ pending
│  └── Fact Checker              ○ pending
── Analyze Findings              ◑ building...
── Write Report                  ○ pending
```

### T3 — Designer starts for Research, Builder finishes Analyze

```
── Research Competitors          ◐ designing (0/3)
│  ├── Scanner                   ○ pending
│  ├── Deep Researcher           ○ pending
│  └── Fact Checker              ○ pending
── Analyze Findings              ○ built (2 agents)
│  ├── Trend Analyst             ○ pending
│  └── Statistician              ○ pending
── Write Report                  ◑ building...
```

### T4–T6 — Designer writes agent configs one by one

```
── Research Competitors          ◐ designing (2/3)
│  ├── Scanner                   ● designed
│  ├── Deep Researcher           ● designed
│  └── Fact Checker              ○ pending       ← next
── Analyze Findings              ◐ designing (1/2)
│  ├── Trend Analyst             ● designed
│  └── Statistician              ○ pending
── Write Report                  ○ built (1 agent)
│  └── Report Writer             ○ pending
```

### T7 — All design complete

```
── Research Competitors          ● designed
│  ├── Scanner                   ● designed
│  ├── Deep Researcher           ● designed
│  └── Fact Checker              ● designed
── Analyze Findings              ● designed
│  ├── Trend Analyst             ● designed
│  └── Statistician              ● designed
── Write Report                  ● designed
│  └── Report Writer             ● designed
```

User clicks any agent → config panel shows system prompt, assignment, expected output, tools. All editable.

---

## Temporal Walkthrough — Execution Phase

User clicks Run after reviewing designs.

### T8 — DAG executor starts

```
── Research Competitors          ▶ running
│  ├── Scanner                   ◐ running...
│  ├── Deep Researcher           ○ waiting
│  └── Fact Checker              ○ waiting
── Analyze Findings              ○ queued
── Write Report                  ○ queued
```

### T9 — Agents execute sequentially within levels, parallel across levels

```
── Research Competitors          ▶ running (1/3 ✓)
│  ├── Scanner                   ✓ completed  420ms
│  ├── Deep Researcher           ◐ running...  ░░
│  └── Fact Checker              ◐ running...  ░░    ← parallel level
── Analyze Findings              ○ queued
── Write Report                  ○ queued
```

### T10 — DAG advances, Research complete, Analyze starts

```
── Research Competitors          ✓ completed  3.2s
│  ├── Scanner                   ✓ 420ms
│  ├── Deep Researcher           ✓ 1.8s
│  └── Fact Checker              ✓ 1.4s
── Analyze Findings              ▶ running (0/2)
│  ├── Trend Analyst             ◐ running...
│  └── Statistician              ○ waiting
── Write Report                  ○ queued
```

### T11 — All complete

```
── Research Competitors          ✓ completed  3.2s
│  ├── Scanner                   ✓ 420ms
│  ├── Deep Researcher           ✓ 1.8s
│  └── Fact Checker              ✓ 1.4s
── Analyze Findings              ✓ completed  2.8s
│  ├── Trend Analyst             ✓ 1.6s
│  └── Statistician             ✓ 1.2s
── Write Report                  ✓ completed  2.1s
│  └── Report Writer             ✓ 2.1s
```

---

## Run Tab — Execution Output Stream

Below the tree, the Run tab shows a streaming output view per step/agent:

```
┌─ Run ───────────────────────────────────────┐
│ ▶ Research Competitors      ✓  3.2s  2.1k▼ │  ← collapsed, expandable
│   3 agents · 2 artifacts                    │
│                                             │
│ ▼ Analyze Findings          ✓  2.8s  1.4k▼ │  ← expanded
│                                             │
│   Trend Analyst                  ✓  1.6s    │
│   ┊ Identified 3 key pricing trends...     │
│   ┊ → trend_analysis.json                  │
│                                             │
│   Statistician                   ✓  1.2s    │
│   ┊ Computed statistical significance...   │
│   ┊ → stats_report.json                    │
│                                             │
│ ▼ Write Report              ✓  2.1s  890▼  │
│                                             │
│   Report Writer                  ✓  2.1s    │
│   ┊ ## Competitive Pricing Analysis — Q4   │
│   ┊ Three of four competitors increased    │
│   ┊ enterprise pricing by 12-18%...        │
│   ┊ → final_report.md                      │
│                                             │
│                          Total: 8.1s  4.4k▼ │
└─────────────────────────────────────────────┘
```

Parallel agents show with a parallel marker:

```
│  ┌─ parallel ──────────────────────────┐    │
│  │ Deep Researcher          ◐ running  │    │
│  │ ┊ Cross-referencing Q3 earnings...  │    │
│  │                                     │    │
│  │ Fact Checker             ◐ running  │    │
│  │ ┊ Verifying claim: "CompetitorA..." │    │
│  └─────────────────────────────────────┘    │
```

---

## Event → State Mapping

### Events consumed (all already emitted by backend, types already defined in `frontend/src/types/ws.ts`)

| Event | Topic | Store action |
|-------|-------|-------------|
| `dispatch_started` | session | Set node status → `building` |
| `dispatch_completed` | session | Set node status → `built` |
| `roster_changed` | workflow | Refetch roster, populate agent children in tree |
| `workforce_designer_progress` | workflow | `started` → node `designing(0/N)`, `completed` → node `designed`, `failed` → node `error` |
| `designer_agent_designed` | workflow | Increment counter, set agent status → `designed` |
| `step_started` | workflow | Set node status → `running` |
| `workforce_agent_progress` | workflow | `started` → agent `running`, `completed` → agent `completed` + duration, `failed` → agent `failed` |
| `step_completed` | workflow | Set node status → `completed` + duration + token count |
| `step_failed` | workflow | Set node status → `failed` |
| `step_name_updated` | workflow | **BUG FIX** — update step name in `workflowStore`. Backend emits this from `chat/broadcast.rs` when builder calls `set_node_name`, but frontend has no constant and no handler. Tree (and canvas) show stale names until page refresh. |

### New store: `designStatusStore`

Tracks per-step and per-agent design/execution status. Separate from `workflowStore` (which owns step config data) and `dispatchStore` (which owns dispatch traces).

```typescript
type NodeStatus =
  | { phase: 'pending' }
  | { phase: 'building' }
  | { phase: 'built'; agentCount: number }
  | { phase: 'designing'; designed: number; total: number }
  | { phase: 'designed' }
  | { phase: 'designed_edited' }
  | { phase: 'running'; completed: number; total: number }
  | { phase: 'completed'; durationMs: number }
  | { phase: 'failed'; error: string }

type AgentStatus =
  | { phase: 'pending' }
  | { phase: 'designed' }
  | { phase: 'running' }
  | { phase: 'completed'; durationMs: number }
  | { phase: 'failed' }

type DesignStatusState = {
  byStep: Record<string, NodeStatus>
  byAgent: Record<string, AgentStatus>  // keyed by roster_agent_id or agent_name
}
```

### WS handler additions

Wire into `WsStoreRouter.tsx`:

```typescript
subscribe(WS_TOPIC.WORKFLOW, designStatusStore.handleWsEvent)
```

The handler switches on `workforce_designer_progress`, `designer_agent_designed`, `workforce_agent_progress`, `step_started`, `step_completed`, `step_failed` and updates the status maps.

---

## Dispatch Tab — Unified Builder + Designer Stream

The current `DispatchAccordionRow` shows builder dispatch only. Extend to show designer progress lines within the same section:

```
┌─ Dispatch ──────────────────────────────────┐
│ ▼ Research Competitors          designed ●  │
│   Builder: ✓ Configured 3-agent pipeline    │
│   Designer: Scanner designed                │
│   Designer: Deep Researcher designed        │
│   Designer: Fact Checker designed           │
│   Designer: ✓ All agents designed           │
│                                             │
│ ▼ Analyze Findings             designing... │
│   Builder: ✓ Configured 2-agent pipeline    │
│   Designer: Trend Analyst designed          │
│   Designer: designing...                    │
│                                             │
│ ▶ Write Report                  waiting...  │
└─────────────────────────────────────────────┘
```

Designer progress lines are appended to the existing dispatch trace as `designer_agent_designed` events arrive. No separate accordion, no nesting — one continuous stream per node.

---

## Component Structure

```
frontend/src/components/board/dispatch/
├── DispatchPanel.tsx              # Existing — hosts tabs
├── DispatchTab.tsx                # Existing — extend with designer lines
├── RunTab.tsx                     # Existing — extend with agent-level output
├── DesignStatusTree.tsx           # NEW — tree rendering with status indicators
├── DesignStatusNode.tsx           # NEW — single tree node row with indicator
└── hooks/
    └── useDesignStatus.ts         # NEW — selector hooks for tree state

frontend/src/stores/
├── designStatusStore.ts           # NEW — per-step/agent status tracking
└── ws/
    └── WsStoreRouter.tsx          # MODIFY — add designStatusStore subscription
```

### DesignStatusTree

Renders workflow steps as a tree with nested agents. Uses box-drawing characters (inspired by `AsciiTree` but rendered as DOM elements for interactivity — click to select, hover for detail).

Each row: `[indent + connector] [status icon] [name] [status label] [duration/count]`

Tree data comes from:
- Step list from `workflowStore` (topology/ordering)
- Roster from `workflowStore` (agent children per step)
- Status from `designStatusStore` (phase indicators)

---

## Implementation Order

### Part 1: designStatusStore + WS wiring
- Create `designStatusStore` with `NodeStatus`/`AgentStatus` types
- Implement `handleWsEvent` for the 6 event types
- Wire into `WsStoreRouter`
- **Bug fix:** Add `STEP_NAME_UPDATED` constant to `ws.ts`, handle in `workflowStore/wsHandler.ts` to patch step name on the fly (backend already emits it, frontend ignores it — builder renames show stale until refresh)
- **Test:** Submit board, verify store state updates via devtools

### Part 2: DesignStatusTree component
- Build tree renderer with status icons
- Subscribe to `designStatusStore` + `workflowStore` (steps + rosters)
- Render in sidebar (above or replacing the dispatch accordion header area)
- **Test:** Visual verification — submit board, watch tree populate

### Part 3: Designer lines in DispatchTab
- Append `designer_agent_designed` events as progress lines within dispatch accordion rows
- Show designer phase status alongside builder completion
- **Test:** Submit board, verify builder ✓ then designer progress lines appear

### Part 4: Execution status in tree
- Handle `step_started`, `workforce_agent_progress`, `step_completed` in tree
- Add duration display on completion
- **Test:** Run workflow, verify tree transitions through running → completed

### Part 5: Run tab agent-level output
- Group streaming output by agent within each step
- Parallel agent rendering with visual grouping
- Collapsed/expanded step summaries
- **Test:** Run workflow, verify per-agent streaming and collapsed summaries

---

## What Already Exists (no rebuilding)

- **WS event types + data shapes**: `frontend/src/types/ws.ts` — all 3 designer/execution events already typed
- **WS routing infrastructure**: `WsStoreRouter.tsx` — just add one subscription line
- **Backend event emission**: `designer.rs`, `react_designer/mod.rs`, `agent_executor.rs` — all 3 events already broadcast
- **Step + roster data**: `workflowStore` — steps, rosters, topology all available
- **Dispatch trace rendering**: `DispatchTraceView` — extend, don't replace
- **AsciiTree utility**: `frontend/src/utils/AsciiTree.ts` — reference for tree logic (DOM tree may not use it directly but follows the same structure)
