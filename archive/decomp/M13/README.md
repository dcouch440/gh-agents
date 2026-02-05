# Milestone 13: Agent View (Factorio-Inspired Monitor)

> Terminal grid of agent stations with real-time status, progress animations, and Unicode art — accessible as a secondary screen from the CLI.

## Goal

A dedicated Agent View screen showing all agents as fixed "stations" on a grid, organized by tier (orchestrators, workers, utilities). Each station displays live status, current task, and animated progress bars via WebSocket updates. Accessible by Tab keybind from the main chat screen.

**Checkpoint**: Press Tab → see a grid of agent stations with live status updates and animated progress bars → press Tab to return to Chat.

---

## Architecture

```
┌─────────────────────────────────────────┐
│  Terminal (Ink)                          │
│  ┌──────────┐  ┌─────────────────────┐  │
│  │ Chat View│  │ Agent View (grid)   │  │
│  │ (M12)    │  │ (M13 - this)        │  │
│  └──────────┘  └─────────────────────┘  │
│  [Tab] switches between views           │
├─────────────────────────────────────────┤
│  WebSocket Client (channels: agents,    │
│  tasks) + REST initial fetch            │
├─────────────────────────────────────────┤
│  Rust Backend (existing)                │
└─────────────────────────────────────────┘
```

## Tickets

| Ticket | Title | Slices | Dependencies |
|--------|-------|--------|--------------|
| 13.1 | WebSocket Client | 4 | M12 (CLI foundation) |
| 13.2 | Agent Types & State | 3 | 13.1 |
| 13.3 | Agent Station Widget | 4 | 13.2 |
| 13.4 | Agent Grid Layout | 3 | 13.3 |
| 13.5 | Screen Navigation | 3 | 13.4 |
| 13.6 | Polish & Animations | 4 | 13.5 |

---

## Design Notes

**Visual style** — Claude Code professional monochrome:
- Colors: white, dim gray, cyan accent only. No bright colors except red for errors.
- Unicode box-drawing: `┌─┐│└─┘` for station borders
- Progress bars: `[████░░░░░░]` using `█` and `░`
- Status icons: `●` (busy), `○` (idle), `✖` (offline)
- Spinner: braille dots `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` for working state
- Section headers: `─── ORCHESTRATORS ───`

**Station box layout** (~32 chars wide, 5 lines):
```
┌─ agent-worker-01 ──────── ●┐
│ task: Implement auth flow   │
│ [████████░░░░░░░░░░░░] 42% │
│                             │
└─────────────────────────────┘
```

**Idle station** (dimmed):
```
┌─ agent-worker-03 ──────── ○┐
│                             │
│           idle              │
│                             │
└─────────────────────────────┘
```

## Dependencies

- **M12** must be complete (CLI scaffolding, API client, auth, chat view)
- Backend WebSocket (`GET /ws`) and REST (`GET /api/agents`) already exist
- No backend changes needed

## New npm Dependencies

- `ws` — WebSocket client for Node.js
- `@types/ws` — TypeScript definitions
