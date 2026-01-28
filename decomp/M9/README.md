# Milestone 9: Polish & Production

> Production-ready, fully-featured nexor with complete documentation and observability.

## Goal

Finalize nexor for production use:
- Complete remaining TUI views (/tasks, /agents, /costs)
- Add headless mode for CI/CD and automation
- Polish error handling throughout
- Package for Docker deployment
- Write comprehensive documentation
- Add observability for debugging agent decisions

**Checkpoint**: All views work, headless mode works, documentation complete.

---

## Tickets

| Ticket | Title | Slices | Dependencies | Status |
|--------|-------|--------|--------------|--------|
| 9.1 | Remaining TUI Views | 3 | M6 (TUI Basic) | pending |
| 9.2 | Headless Mode | 4 | M5 (Orchestration Core) | pending |
| 9.3 | Error Handling Polish | 3 | M1-M8 (all prior milestones) | pending |
| 9.4 | Docker Packaging | 3 | M1-M8 | pending |
| 9.5 | Documentation | 4 | M1-M8 | pending |
| 9.6 | Observability & Replay | 5 | M2 (LLM), M5 (Orchestration) | pending |
| 9.7 | Refactor Mode Foundation | 4 | M1, M5 | **done** |
| 9.8 | Refactor Agent | 4 | 9.7, M4 | **done** |
| 9.9 | TUI Integration | 3 | 9.7, 9.8, M6 | **done** |
| 9.10 | Menu Types & Data | 3 | 9.7, 9.9 | pending |
| 9.11 | Menu Widget & Rendering | 3 | 9.10 | pending |
| 9.12 | App Integration | 3 | 9.10, 9.11, 9.9 | pending |

**Total Slices**: 42

---

## Dependency Graph

```
M1-M8 (All Prior Milestones)
    │
    ├─────────────────────────────────────────────┐
    │                                             │
    ▼                                             │
M6 (TUI) ────────► 9.1 Remaining TUI Views        │
                                                  │
M5 (Orchestration) ──► 9.2 Headless Mode          │
                                                  │
M2 (LLM) + M5 ───────► 9.6 Observability          │
                                                  │
    ├──────────────────────────────────────────────┤
    │                                              │
    ▼                                              ▼
   9.3 Error Handling         9.4 Docker    9.5 Documentation
        Polish                 Packaging

Menu System Chain (9.10-9.12):
9.7 (Refactor Foundation) + 9.9 (TUI Integration)
    │
    ▼
9.10 Menu Types & Data
    │
    ▼
9.11 Menu Widget & Rendering
    │
    ▼
9.12 App Integration
```

---

## Parallelization

**Can run in parallel** (once dependencies are met):
- 9.1 Remaining TUI Views (after M6)
- 9.2 Headless Mode (after M5)
- 9.6 Observability & Replay (after M2, M5)
- 9.10-9.12 Menu System (after 9.7, 9.9) - sequential chain

**Should be done last** (need complete system):
- 9.3 Error Handling Polish
- 9.4 Docker Packaging
- 9.5 Documentation

**Recommended execution order**:
1. 9.1, 9.2, 9.6 in parallel (different dependencies)
2. 9.10 → 9.11 → 9.12 (menu system, sequential)
3. 9.3 Error Handling Polish
4. 9.4 Docker Packaging
5. 9.5 Documentation (last, describes final system)

---

## File Structure

New files for this milestone:

```
src/
├── tui/
│   ├── views/
│   │   ├── tasks.rs        ← /tasks view (9.1)
│   │   ├── agents.rs       ← /agents view (9.1)
│   │   └── costs.rs        ← /costs view (9.1)
│   ├── menu/
│   │   ├── mod.rs          ← Menu module root (9.10)
│   │   ├── types.rs        ← MenuItem, Menu, MenuAction (9.10)
│   │   ├── builder.rs      ← Menu tree construction (9.10)
│   │   ├── widget.rs       ← Ratatui widget (9.11)
│   │   └── controller.rs   ← Input handling (9.11)
│   └── terminal.rs         ← Minimal terminal wrapper (9.11)
├── cli.rs              ← CLI argument parsing (9.2)
├── headless.rs         ← Headless mode runner (9.2)
├── error.rs            ← Centralized error handling (9.3)
└── observability/
    ├── mod.rs          ← Observability module (9.6)
    ├── logging.rs      ← LLM call logging (9.6)
    ├── replay.rs       ← Decision replay (9.6)
    └── export.rs       ← Session export (9.6)

docker/
├── Dockerfile          ← Production container (9.4)
└── docker-compose.yml  ← Easy deployment (9.4)

docs/
├── installation.md     ← Setup guide (9.5)
├── configuration.md    ← Config reference (9.5)
├── usage.md            ← User guide (9.5)
└── commands.md         ← Command reference (9.5)
```

---

## Notes

- **Polish milestone**: This is about making existing functionality production-ready
- **Documentation timing**: Write docs last so they reflect the final system
- **Error handling**: Focus on user-facing errors - internal errors should be logged
- **Docker**: Enables deployment without Rust toolchain
- **Observability**: Critical for debugging agent behavior in production
- **Headless mode**: Enables CI/CD integration and scripting
- **Menu system**: Interactive popup menu with `/menu` or Esc, arrow-key navigation. Provides unified access to production control, refactor mode, change management, and navigation. Persists milestone limits in DB.
