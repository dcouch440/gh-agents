# Milestone 20: CLI Feed Infrastructure

> Build the authenticated main screen for the Ink CLI with a live WebSocket-driven feed, slash command system, and chat input.

## Goal

Replace the static "Authenticated" screen with a live feed page. The CLI connects to the backend via WebSocket, subscribes to feed/tasks/agents channels, and renders agent activity in real-time. Users can send chat messages and run slash commands from an input bar.

**Checkpoint**: CLI renders live feed updates from backend, `/help` lists commands, chat messages route to orchestrator.

---

## Scope

- 7 tickets, ~24 slices
- 1 existing file modified (`App.tsx`)
- 9 new files created
- 7 test files created
- 1 npm dependency added (`ws` or native WebSocket)

## Component Tree

```
<MainView>
  <StatusBar />          ← branch name + WS connection dot
  <FeedArea />           ← flex:1 scrollable feed
  <InputBar />           ← > prompt with text input
</MainView>
```

## Dependency Graph

```
20.1 (WS Types)
  ├→ 20.2 (WebSocket Client)    ← parallel
  └→ 20.3 (Feed Store)          ← parallel
      └→ 20.4 (Command System)
          └→ 20.5 (UI Components: StatusBar, FeedItemRow, FeedArea, InputBar)
              └→ 20.6 (MainView + App.tsx Wiring)
                  └→ 20.7 (Integration Tests)
```

## Tickets

| Ticket | Title | Slices | Dependencies |
|--------|-------|--------|-------------|
| 20.1 | WebSocket & Feed Types | 3 | None |
| 20.2 | WebSocket Client | 4 | 20.1 |
| 20.3 | Feed Store | 3 | 20.1 |
| 20.4 | Slash Command System | 3 | 20.3 |
| 20.5 | UI Components | 4 | 20.4 |
| 20.6 | MainView + App Wiring | 4 | 20.5 |
| 20.7 | Integration Tests | 3 | 20.6 |

## Key Design Decisions

1. **useSyncExternalStore** — Feed store uses module-level state with subscribe/getSnapshot, no Zustand needed.
2. **Native WebSocket or `ws`** — Use Node native WebSocket if targeting Node 21+, otherwise add `ws` package.
3. **Reconnect with backoff** — 2s initial, doubling to 30s cap, reset on successful connect.
4. **Slash commands as registry** — Simple map-based registry, each command is `{ name, description, execute }`.
5. **Feed items are append-only** — No editing/removing individual items. `/clear` resets entire feed.
6. **No branch filtering yet** — All WS feed items shown. Branch filtering is M21's concern.
