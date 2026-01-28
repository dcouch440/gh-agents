# Milestone 12: React Features

> Core feature views - chat, feed, tasks, files.

## Goal

Full feature views that provide parity with the original TUI vision, but in a modern web interface.

**Checkpoint**: Can chat with orchestrator, see agent feed, manage tasks, browse files.

---

## Context

This milestone builds the main feature pages on top of the foundation from M11.

**Architecture**:
```
┌─────────────────────────────────────────────────────────────┐
│                    Feature Pages                             │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐          │
│  │  Chat   │ │  Feed   │ │  Tasks  │ │  Files  │          │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘          │
├─────────────────────────────────────────────────────────────┤
│                    Layout (M11)                              │
├─────────────────────────────────────────────────────────────┤
│                    API Client (M11)                          │
└─────────────────────────────────────────────────────────────┘
```

---

## Tickets

| Ticket | Title | Slices | Dependencies |
|--------|-------|--------|--------------|
| 12.1 | Chat View | 5 | M11, M10.3 |
| 12.2 | Feed View | 4 | M11, M10.4 |
| 12.3 | Tasks View | 5 | M11, M10.2 |
| 12.4 | Agents View | 4 | M11, M10.2 |
| 12.5 | File Browser & Editor | 5 | M11, M10.2 |
| 12.6 | Diff Viewer | 3 | 12.5, M10.2 |

---

## Design Notes

All views should follow the Claude Code-inspired design from `doc/DESIGN-SYSTEM.md`:
- Dark backgrounds
- Monospace for agent output
- Streaming text animations
- Minimal chrome

---

## Key Components to Build

| Component | Used In | Description |
|-----------|---------|-------------|
| `<Message>` | Chat | Chat message with streaming |
| `<FeedItem>` | Feed | Agent activity item |
| `<TaskCard>` | Tasks | Task status card |
| `<AgentCard>` | Agents | Agent status card |
| `<FileTree>` | Files | Directory tree |
| `<CodeEditor>` | Files | Monaco/CodeMirror editor |
| `<DiffView>` | Diff | Side-by-side diff |

---

## Completion Criteria

- [ ] Chat with streaming responses
- [ ] Live agent activity feed
- [ ] Task list with real-time updates
- [ ] Agent pool status
- [ ] File browser with syntax highlighting
- [ ] File editing capability
- [ ] Git diff viewer
