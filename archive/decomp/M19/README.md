# Milestone 19: Agent Tool Execution UI

> Make subagent tool execution visible, interactive, and polished — in both the web UI and CLI. Modeled after Claude Code's progressive-disclosure approach where tool calls appear inline with clear status, collapsible output, and real-time streaming.

## Goal

When an agent executes tools (Read, Write, Edit, Glob, Grep, Bash, Git), each tool call appears in real time as a structured block — not buried in a text feed. Users see what the agent is doing, what it found, and how long it took. The experience should feel as fluid as watching Claude Code work in a terminal.

**Checkpoint**: User sends a complex task. The chat view shows the orchestrator spawning an explorer subagent. The explorer's tool calls stream in: a Glob search with expanding results, a Read with syntax-highlighted file content, a Grep with matched lines highlighted. Each block is collapsible, shows duration, and color-codes by tool type. The CLI shows the same information in a terminal-native format.

---

## Scope

- 10 tickets, ~47 slices
- New WebSocket event types for structured tool execution data
- New React components: ToolCallBlock, DiffView, FileListOutput, SearchMatchesOutput, TerminalOutput, AgentCard, ActivityPanel, TaskCard, DelegationTree
- New CLI components for terminal-native tool output rendering
- Zustand store for agent activity state
- TasksPage and AgentsPage stubs replaced with real implementations
- Performance: virtualized lists, memoized components, keyboard navigation
- Fully wired: backend emits structured events → WS delivers → UI renders in real time

## Key Concepts

| Concept | Description |
|---------|-------------|
| `ToolCallEvent` | Structured WS event with tool name, args, output, duration, status |
| `ToolCallBlock` | React component that renders a single tool invocation with collapsible output |
| `AgentActivityPanel` | Side panel or inline section showing an agent's tool execution timeline |
| `DiffView` | Syntax-highlighted inline diff for Edit/Write operations |
| `SearchResults` | Structured display for Glob/Grep results with file links |
| `TerminalOutput` | Monospace scrollable output for Bash commands |
| `DelegationTree` | Parent→child agent tree showing orchestrator delegation |
| Progressive disclosure | Tool blocks start collapsed, expand on click, auto-expand on error |

## Design Language

Extends the existing nexor design system:

| Element | Treatment |
|---------|-----------|
| Tool header | Monospace, left-border accent by tool type, icon + tool name + duration badge |
| Read/Write | `--color-accent-secondary` (blue) left border |
| Edit | `--color-status-warning` (yellow) left border |
| Bash | `--color-text-secondary` (gray) left border |
| Search (Glob/Grep) | `--color-status-info` (blue) left border |
| Git | `--color-status-success` (green) left border |
| Error state | `--color-status-error` (red) left border + expanded by default |
| Spinner | Accent-colored pulse animation while tool is running |
| Collapse/expand | Chevron icon, 0.15s transition, remembers user preference per session |
| Timestamps | Tertiary text, relative format ("2s ago"), monospace |

## Dependency Graph

```
19.1 (Backend Events)
  └→ 19.2 (WS Protocol)
       ├→ 19.3 (Activity Store + Hook)
       │    ├→ 19.4 (ToolCallBlock)
       │    └→ 19.5 (Output Renderers)
       │         ├→ 19.6 (Agent Views)
       │         ├→ 19.8 (Chat Integration)
       │         └→ 19.9 (Tasks Page)
       └→ 19.7 (CLI Tool Display)
  19.4-19.9 ──→ 19.10 (Polish & Performance)
```

## Tickets

| Ticket | Title | Slices | Dependencies |
|--------|-------|--------|-------------|
| 19.1 | Tool Execution Events Backend | 5 | M18 (18.9) |
| 19.2 | WebSocket Tool Event Protocol | 4 | 19.1 |
| 19.3 | Agent Activity Store & Hook | 5 | 19.2 |
| 19.4 | ToolCallBlock Component | 6 | 19.3 |
| 19.5 | Output Renderer Components | 6 | 19.3 |
| 19.6 | Agent Activity Views | 4 | 19.4, 19.5 |
| 19.7 | CLI Tool Execution Display | 4 | 19.2 |
| 19.8 | Chat Integration & Streaming | 4 | 19.4, 19.5 |
| 19.9 | Tasks Page with Delegation View | 4 | 19.4, 19.5 |
| 19.10 | Polish, Animations & Performance | 4 | 19.4-19.9 |

## Key Design Decisions

1. **Structured events, not strings** — Tool calls emit structured `ToolCallEvent` data (tool name, typed args, typed output, duration, status), not flattened text in the feed. The feed still works but tool calls get their own rendering path.
2. **Progressive disclosure** — Tool blocks render collapsed by default (header + status only). Click to expand output. Errors auto-expand. This keeps the view scannable when an agent runs 20+ tool calls.
3. **Inline in chat** — Tool execution blocks appear inline in the chat conversation, nested under the agent's response — not in a separate page. This mirrors Claude Code where tool calls appear in the conversation flow.
4. **Real-time streaming** — Tool calls appear as soon as they start (with spinner), output fills in when complete. No waiting for the full agent response.
5. **Terminal-native CLI** — CLI renders tool calls using Ink components with box drawing, color, and collapsible sections — not a web view in a terminal.
6. **Existing design system** — All new components use the established CSS custom properties, spacing scale, and animation patterns. No new design tokens unless necessary.
7. **Diff view for edits** — Write and Edit operations show before/after diffs with syntax highlighting, not just the final content.
8. **Search results are navigable** — Glob/Grep results show file paths as clickable links (web) or navigable list (CLI) that can trigger a Read.

## Verification

1. `cargo check` + `cargo test` — backend compiles, new event types tested
2. `cd ui && npm run build` — UI compiles
3. `cd cli && npm test` — CLI tests pass
4. Manual: spawn explorer agent → watch tool calls stream into chat view inline
5. Manual: collapse/expand tool blocks, verify animations smooth at 60fps
6. Manual: trigger an error (bad file path) → verify error block auto-expands with red styling
7. Manual: CLI shows tool calls with color and structure
8. Manual: 20+ tool calls in a row → view stays scannable (collapsed by default)
9. Manual: TasksPage shows tasks with delegation tree expandable per task
10. Manual: AgentsPage shows active agents with live tool call streams
11. Manual: keyboard nav (j/k/Enter/Esc) works in chat and agents views
12. Manual: WS disconnect → reconnect → state preserved
