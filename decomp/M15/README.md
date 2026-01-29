# Milestone 15: Repo Management & Power User Workspace

> Standalone workspace features for managing repositories, prompts, code editing, and agent reports — turning nexor into a daily-driver development tool.

## Overview

This milestone transforms nexor from an agent orchestration viewer into a **full power-user workspace**. The core additions:

1. **Multi-Repo Management** — Add, switch, configure, and monitor multiple repositories from one interface
2. **Prompt Library** — Create, edit, tag, version, and quick-launch saved prompts
3. **Full Code Editor** — Monaco-based editor with VS Code keybindings, tabs, split panes, and integrated terminal
4. **Report Viewer & Submission** — Review agent-generated reports on-screen, approve/reject/edit before submitting
5. **Pivotal Points Dashboard** — Track key decision points, bookmarks, and milestones across repos

## Architecture

All features build on the existing Axum server (M10) + React frontend (M11-M13).

```
┌──────────────────────────────────────────────────────────────┐
│                    React Frontend (M15)                        │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────────────┐   │
│  │  Repos   │ │ Prompts  │ │  Editor  │ │    Reports     │   │
│  │ Manager  │ │ Library  │ │ (Monaco) │ │ Review/Submit  │   │
│  └──────────┘ └──────────┘ └──────────┘ └────────────────┘   │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │              Pivotal Points Dashboard                     │ │
│  └──────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
                        │ HTTP + WebSocket
                        ▼
┌──────────────────────────────────────────────────────────────┐
│                   Axum Server (new endpoints)                 │
│  /api/repos, /api/prompts, /api/files, /api/reports,         │
│  /api/pivots                                                  │
└──────────────────────────────────────────────────────────────┘
                        │
                        ▼
┌──────────────────────────────────────────────────────────────┐
│           Existing Core (agents, orchestration, git, etc.)    │
└──────────────────────────────────────────────────────────────┘
```

## Dependencies

- Requires M10 (Server Layer) — complete
- Requires M11 (React Foundation) — in progress
- Can begin backend tickets (15.1, 15.2, 15.6) immediately
- Frontend tickets (15.3, 15.4, 15.5, 15.7) need M11.4 (Layout) complete

---

## Tickets

| Ticket | Title | Slices | Priority |
|--------|-------|--------|----------|
| 15.1 | Multi-Repo Backend | 6 | P0 |
| 15.2 | Prompt Library Backend | 6 | P0 |
| 15.3 | Multi-Repo Frontend | 5 | P0 |
| 15.4 | Prompt Library Frontend | 5 | P1 |
| 15.5 | Full Code Editor | 7 | P0 |
| 15.6 | Report Management Backend | 5 | P1 |
| 15.7 | Report Viewer & Submission UI | 6 | P1 |
| 15.8 | Pivotal Points | 5 | P2 |
| 15.9 | System Prompt Admin | 6 | P1 |

---

## Ticket Summaries

### 15.1: Multi-Repo Backend
Database schema and API endpoints for managing multiple repositories. CRUD for repos, active repo switching, per-repo config, git clone/pull integration.

### 15.2: Prompt Library Backend
Database schema and API for saved prompts. CRUD, tagging, versioning, categories, quick-launch to chat.

### 15.3: Multi-Repo Frontend
Repo list view, add/remove repo UI, repo switcher in header, per-repo status indicators (branch, dirty state, last sync).

### 15.4: Prompt Library Frontend
Prompt browser with search/filter, prompt editor with syntax highlighting, tag management, one-click launch to agent chat.

### 15.5: Full Code Editor
Monaco editor integration with VS Code keybindings. Tabbed editing, split panes, file tree sidebar, minimap, command palette (Ctrl+Shift+P), integrated find/replace, save with Ctrl+S, language detection, git gutter indicators.

### 15.6: Report Management Backend
Database schema and API for agent reports. Store generated reports, support review status (pending/approved/rejected), edit before submission, link reports to tasks and repos.

### 15.7: Report Viewer & Submission UI
On-screen report rendering (markdown), approve/reject buttons, inline editing, submission flow with confirmation, report history timeline.

### 15.8: Pivotal Points Dashboard
Bookmark key decisions, branch points, and milestones. Timeline view across repos, link to commits/PRs/reports, searchable and filterable.

### 15.9: System Prompt Admin
Seed all agent prompts (orchestrator, worker, utility) into the database on first boot from compiled defaults. Super-admin UI to edit prompts live, view diffs against defaults, version history, reset to default. Runtime reads from DB with in-memory cache.

---

## Key Design Decisions

### Editor: Monaco vs CodeMirror
**Choice: Monaco** — VS Code keybindings are native, not emulated. Users expect Ctrl+P (quick open), Ctrl+Shift+P (command palette), Ctrl+D (multi-cursor), Ctrl+/ (toggle comment), etc. Monaco delivers this out of the box.

### Report Workflow
Reports go through a lifecycle: `draft → pending_review → approved → submitted` or `draft → pending_review → rejected → revised → pending_review → ...`. Users can edit at any stage before submission.

### Repo Isolation
Each repo gets its own workspace context. Switching repos changes the file tree, git status, active branch, and scopes prompts/reports to that repo.

---

## Keyboard Shortcuts (Editor)

| Shortcut | Action |
|----------|--------|
| `Ctrl+S` | Save file |
| `Ctrl+P` | Quick open file |
| `Ctrl+Shift+P` | Command palette |
| `Ctrl+D` | Select next occurrence |
| `Ctrl+Shift+K` | Delete line |
| `Ctrl+/` | Toggle comment |
| `Ctrl+\` | Split editor |
| `Ctrl+W` | Close tab |
| `Ctrl+Tab` | Switch tab |
| `Ctrl+Shift+F` | Search across files |
| `Ctrl+G` | Go to line |
| `Ctrl+F` | Find in file |
| `Ctrl+H` | Find and replace |
| `Alt+Up/Down` | Move line up/down |
| `Ctrl+Shift+[/]` | Fold/unfold |

---

*Created: 2026-01-29*
