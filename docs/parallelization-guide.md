# nexor Parallelization Guide

> Quick reference for running milestones and tickets in parallel.

---

## Milestone Flow

```
                              ┌─────────────────┐
                              │  M1: Foundation │
                              │    (REQUIRED)   │
                              └────────┬────────┘
                                       │
          ┌────────────┬───────────────┼───────────────┬────────────┐
          │            │               │               │            │
          ▼            ▼               ▼               ▼            ▼
    ┌──────────┐ ┌──────────┐   ┌──────────┐   ┌──────────┐  ┌──────────┐
    │ M2: LLM  │ │M4: Prompt│   │ M6: TUI  │   │M7: Exec  │  │M8: GitHub│
    │  Layer   │ │ (design) │   │  Basic   │   │  Layer   │  │Integration
    └────┬─────┘ └────┬─────┘   └────┬─────┘   └────┬─────┘  └────┬─────┘
         │            │              │              │             │
         └──────┬─────┘              │              └─────┬───────┘
                │                    │                    │
                ▼                    │                    │
          ┌──────────┐               │                    │
          │M3: Agent │               │                    │
          │ Runtime  │               │                    │
          └────┬─────┘               │                    │
               │                     │                    │
               └─────────┬───────────┘                    │
                         ▼                                │
                   ┌──────────┐                           │
                   │   M5:    │                           │
                   │  Orch.   │                           │
                   │   Core   │                           │
                   └────┬─────┘                           │
                        │                                 │
                        └─────────────┬───────────────────┘
                                      ▼
                              ┌──────────────┐
                              │  M9: Polish  │
                              │ & Production │
                              └──────────────┘
```

---

## Run Groups (What Can Run Together)

### Wave 1 - Must be first

| Milestone | Tickets |
|-----------|---------|
| M1: Foundation | 1.1 → 1.2 + 1.5 (parallel) → 1.3 + 1.4 (parallel) |

### Wave 2 - After M1 complete (ALL can run in parallel)

| Milestone | Can Start |
|-----------|-----------|
| M2: LLM Layer | Immediately |
| M4: Prompts (design) | 4.1 → 4.2/4.3/4.4/4.5/4.8/4.10 |
| M6: TUI Basic | 6.1 → 6.2 → rest |
| M7: Execution | 7.1/7.2/7.3 (parallel) |
| M8: GitHub | 8.1 → rest |

### Wave 3 - After M2 + some M4

| Milestone | Dependencies |
|-----------|--------------|
| M3: Agent Runtime | M1 + M2 |
| M4: Testing (4.7, 4.11) | M2 needed for LLM calls |

### Wave 4 - After M3 + M4

| Milestone | Dependencies |
|-----------|--------------|
| M5: Orchestration Core | M3 + M4 |

### Wave 5 - Everything ready

| Milestone | Dependencies |
|-----------|--------------|
| M9: Polish | M1-M8 mostly done |

---

## Ticket Parallelization (Detail)

### M1: Foundation

```
1.1 ─┬─► 1.2 (types) ─┬─► 1.3 (config)
     │                │
     │                └─► 1.4 (database)
     │
     └─► 1.5 (logging)  ◄── runs parallel with 1.2
```

### M2: LLM Layer

```
2.1 ─► 2.2 ─┬─► 2.3 (cost tracking)
            │
            └─► 2.4 (retry logic)
```

### M4: Prompts (high parallelism!)

```
       ┌─► 4.2 (orchestrator) ─┐
       │                       │
4.1 ──┼─► 4.3 (worker) ────────┼─► 4.6 (examples)
       │                       │
       ├─► 4.4 (utility) ──────┘
       │
       ├─► 4.5 (schemas) ─┬─► 4.9 (recovery)
       │                  │
       │                  └─► 4.7 (testing) ◄── needs M2
       │
       ├─► 4.8 (context)
       │
       └─► 4.10 (tools)

4.11 ◄── only needs M2, can run separately
```

### M6: TUI Basic

```
6.1 ─► 6.2 ─┬─► 6.3 (home)
            │
            ├─► 6.4 (feed)
            │
            ├─► 6.5 (chat) ◄── needs M3
            │
            ├─► 6.6 (commands)
            │
            └─► 6.7 (logs)
```

### M7: Execution

```
7.1 (files) ──┐
              │
7.2 (git) ────┼─► 7.4 (docker)
              │
7.3 (tests) ──┘

7.5 (approvals) ◄── needs M6 TUI
```

---

## Quick Reference: Max Parallel Workers

| Phase | Workers |
|-------|---------|
| M1 in progress | 2-3 |
| Wave 2 (M2/M4/M6/M7/M8) | 5-8 |
| Wave 3 (M3 added) | 4-6 |
| Wave 4+ | 3-4 |

---

## Critical Paths to Watch

```
FASTEST PATH TO WORKING AGENTS:
M1 → M2 → M3 → M5

FASTEST PATH TO TUI DEMO:
M1 → M6

FASTEST PATH TO GITHUB PR:
M1 → M7 → M8
```

---

## Blocking Dependencies Summary

| Ticket | Blocked By |
|--------|------------|
| 1.3 | 1.2.6 (config types) |
| 1.4 | 1.2 (all types) |
| 2.2-2.4 | 2.1 |
| 3.x | M1 + M2 |
| 4.6 | 4.2, 4.3, 4.4 |
| 4.7 | 4.5 + M2 |
| 4.9 | 4.5 |
| 4.11 | M2 |
| 5.x | M3 + M4 |
| 6.5 | M3 (for chat) |
| 6.7 | 1.5 (logging) |
| 7.4 | 7.1, 7.2, 7.3 |
| 7.5 | M6 |
| 8.2-8.4 | 8.1 |
| 9.x | M1-M8 |
