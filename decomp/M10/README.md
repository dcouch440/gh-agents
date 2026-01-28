# Milestone 10: In-TUI File Editor

> View and edit files directly within the TUI

## Goal

Users can view and edit files directly within nexor, including files that agents are currently working on. This provides a seamless development experience without leaving the TUI.

## Checkpoint

Can open a file from an agent's task context, edit it in-app with syntax highlighting, save changes, and optionally commit to the current branch.

## Dependencies

- **M6: TUI Basic** - Core TUI infrastructure, layout, views
- **M7: Execution Layer** - File operations, git operations

## Tickets

| Ticket | Title | Slices | Status |
|--------|-------|--------|--------|
| 10.1 | File Viewer Widget | 4 | pending |
| 10.2 | File Editor Widget | 5 | pending |
| 10.3 | File Browser Widget | 4 | pending |
| 10.4 | Diff Viewer | 4 | pending |
| 10.5 | Save & Commit Flow | 5 | pending |
| 10.6 | Slash Commands Integration | 5 | pending |
| 10.7 | Agent Integration | 4 | pending |

**Total Slices:** 31

## Key Features

### User Flow

```
Agent Task View                    File Editor
┌─────────────────────┐           ┌─────────────────────────────┐
│ Task: Implement     │           │ src/auth/login.rs    Ctrl+X │
│ login endpoint      │  ────►    ├─────────────────────────────┤
│                     │  /edit    │  1 │ use crate::auth::...   │
│ Files:              │           │  2 │ use crate::db::...     │
│  • src/auth/login.rs│           │  3 │                        │
│    [View] [Edit]    │           │  4 │ pub async fn login()   │
└─────────────────────┘           └─────────────────────────────┘
```

### Keybindings (nano-style)

| Key | Action |
|-----|--------|
| Ctrl+X | Exit (prompts to save) |
| Ctrl+O | Save file |
| Ctrl+G | Go to line |
| Ctrl+W | Search |
| Ctrl+K | Cut line |
| Ctrl+U | Paste |

### Slash Commands

| Command | Description |
|---------|-------------|
| `/view <path>` | Open file in read-only viewer |
| `/edit <path>` | Open file in editor |
| `/diff <path>` | Show diff for file |
| `/files` | Open file browser |

## Technical Stack

| Component | Crate |
|-----------|-------|
| Editor widget | `tui-textarea` |
| Syntax highlighting | `syntect` |
| Git operations | `git2` |
| File tree | Custom or `tui-tree-widget` |

## Parallelization

- 10.1 (Viewer) and 10.2 (Editor) can be worked in parallel
- 10.3 (Browser) and 10.4 (Diff) can be worked in parallel
- 10.5 (Save/Commit) needs 10.2
- 10.6 (Commands) needs 10.1-10.4
- 10.7 (Agent Integration) needs 10.1-10.2

## Notes

- This milestone enables human-AI collaborative editing
- Important for debugging agent output and making quick fixes
- Reduces context switching by keeping users in the TUI
