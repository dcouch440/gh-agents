# Milestone 21: CLI Branch Switching

> Detect branch changes (both explicit `/branch` command and auto-detection via git) and smoothly transition the feed with a visual divider. Branch selector overlay with filtering and keyboard navigation.

## Goal

Users can switch between branches to view different agent activity feeds. Switching is explicit via `/branch` or automatic when `git checkout` is detected. Transitions are smooth — a styled divider separates old and new feed content, old tool displays gracefully stop, and the status bar updates.

**Checkpoint**: `/branch` opens selector, selecting a branch shows divider + new feed. Running `git checkout` in another terminal auto-triggers the same transition.

---

## Scope

- 5 tickets, ~18 slices
- 4 new files created
- 2 existing files modified (MainView, api/client)
- 4 test files created
- 1 backend ticket dependency (GET /branches endpoint — can stub)

## Dependency Graph

```
21.1 (Git HEAD Watcher Hook)
  └→ 21.2 (Branch Selector Component)
      └→ 21.3 (Branch Transition UX)
          └→ 21.4 (MainView Integration)
              └→ 21.5 (Tests)
```

## Tickets

| Ticket | Title | Slices | Dependencies |
|--------|-------|--------|-------------|
| 21.1 | Git HEAD Watcher Hook | 4 | M20 complete |
| 21.2 | Branch Selector Component | 4 | 21.1 |
| 21.3 | Branch Transition UX | 3 | 21.2 |
| 21.4 | MainView Integration | 4 | 21.3 |
| 21.5 | Integration Tests | 3 | 21.4 |

## Key Design Decisions

1. **fs.watch on `.git/HEAD`** — Lightweight, no polling. Debounce 100ms for rapid changes.
2. **Divider, not clear** — Old feed stays visible above the divider. No screen wipe.
3. **Stale markers** — In-progress tool displays above divider stop updating (visual dim).
4. **Filter-as-you-type** — Branch selector supports typing to narrow results.
5. **Stub backend** — `GET /branches` endpoint doesn't exist yet. Selector uses `git branch` locally as fallback.
6. **Main window unchanged** — Branch switch only affects feed content + status bar. No layout disruption.
