# Milestone 6: TUI Basic

> Functional terminal interface with feed, chat, and navigation.

## Goal

Build a working terminal user interface using ratatui and crossterm that allows users to:
- See a home screen with branding
- Chat with the orchestrator agent
- View real-time agent activity feed
- Navigate between views using slash commands
- View technical logs

**Checkpoint**: Can see agent activity in feed, chat with orchestrator, navigate with slash commands.

---

## Tickets

| Ticket | Title | Slices | Dependencies |
|--------|-------|--------|--------------|
| 6.1 | Terminal Setup | 3 | M1 (Foundation) |
| 6.2 | Layout System | 3 | 6.1 |
| 6.3 | Home Screen | 3 | 6.2 |
| 6.4 | Feed View (/feed) | 4 | 6.2 |
| 6.5 | Chat View (/main) | 5 | 6.2, M3 (Agent Runtime) |
| 6.6 | Slash Command Router | 4 | 6.2 |
| 6.7 | Logs View (/logs) | 3 | 6.2, M1.5 (Logging) |

**Total Slices**: 25

---

## Dependency Graph

```
M1 (Foundation)
    │
    ▼
   6.1 Terminal Setup
    │
    ▼
   6.2 Layout System
    │
    ├──────────┬──────────┬──────────┬──────────┐
    ▼          ▼          ▼          ▼          ▼
   6.3        6.4        6.5        6.6        6.7
  Home       Feed       Chat     Commands     Logs
 Screen     View       View      Router      View
                        │
                        ▼
                   M3 (Agents)
```

---

## Parallelization

**Can run in parallel** (after 6.2 is complete):
- 6.3 Home Screen
- 6.4 Feed View
- 6.6 Slash Command Router
- 6.7 Logs View

**Must be sequential**:
- 6.1 → 6.2 (layout needs terminal)
- 6.5 Chat View needs M3 Agent Runtime for orchestrator integration

**Recommended execution order**:
1. 6.1 Terminal Setup
2. 6.2 Layout System
3. 6.3, 6.4, 6.6, 6.7 in parallel
4. 6.5 Chat View (after M3 is ready)

---

## File Structure

All TUI code goes in `src/tui/`:

```
src/tui/
├── mod.rs              ← Public exports, TUI module root
├── app.rs              ← App struct, main event loop, state
├── layout.rs           ← Layout constraints and rendering
├── input.rs            ← Input handling, key events
├── commands.rs         ← Slash command parsing and routing
└── views/
    ├── mod.rs          ← View exports
    ├── home.rs         ← Home screen with logo
    ├── feed.rs         ← Agent activity feed
    ├── chat.rs         ← Orchestrator conversation
    └── logs.rs         ← Technical log viewer
```

---

## Key Types (from PRD.md)

```rust
// Feed items displayed in /feed view
struct FeedItem {
    id: Uuid,
    agent_id: AgentId,
    content: String,
    item_type: FeedItemType,
    verbosity_level: VerbosityLevel,
    timestamp: DateTime<Utc>,
}

enum FeedItemType {
    AgentReport,
    TaskStarted,
    TaskCompleted,
    Error,
    UserMessage,
    SystemNotice,
}

enum VerbosityLevel {
    Quiet,
    Normal,
    Verbose,
}
```

---

## Notes

- **UI Design**: See `PRD.md` section "UI Design" for mockups
- **Keybindings**: Standard keys (Arrow, Tab, Enter, Ctrl+C) - NOT Vim-style
- **Fixed layout**: Predictable panel arrangement, no resizing
- **ratatui + crossterm**: Use these crates for terminal handling
- **Color scheme**: Minimal chrome, content-first design
- Agent status format in header: `w[0/6] o[0/2]` (workers active/total, orchestrators active/total)
