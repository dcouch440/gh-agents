# Milestone 1: Foundation

> Project compiles, core types exist, config loads, database works.

## Goal

Establish the foundational infrastructure for nexor. After this milestone, a developer can:
- Run `cargo run` and have the application start cleanly
- Load configuration from TOML files
- Connect to a SQLite database with all required tables
- See structured logs in console and files

**Checkpoint**: `cargo run` starts, loads config, connects to SQLite, and exits cleanly with log output.

---

## Tickets

| Ticket | Title | Slices | Dependencies | Est. Complexity |
|--------|-------|--------|--------------|-----------------|
| 1.1 | Project Scaffolding | 3 | None | Low |
| 1.2 | Core Type Definitions | 6 | 1.1 | Medium |
| 1.3 | Configuration System | 4 | 1.2.6 | Medium |
| 1.4 | Database Setup | 8 | 1.2 (all) | Medium |
| 1.5 | Logging Infrastructure | 3 | 1.1 | Low |

**Total Slices**: 24

---

## Dependency Graph

```
          ┌─────────────────────────────────────────┐
          │                                         │
          ▼                                         │
       [1.1 Project Scaffolding]                    │
          │                                         │
          ├──────────────┬──────────────┐           │
          │              │              │           │
          ▼              ▼              ▼           │
   [1.2 Core Types]  [1.5 Logging]     ...         │
          │                                         │
          │ (specifically 1.2.6)                    │
          ├──────────────┐                          │
          │              │                          │
          ▼              ▼                          │
 [1.3 Config System] [1.4 Database] ◄──────────────┘
                         │             (needs all 1.2.x)
                         │
```

**Simplified view:**

```
1.1 ──┬──► 1.2 ──┬──► 1.3 (needs 1.2.6)
      │         │
      │         └──► 1.4 (needs all 1.2.x)
      │
      └──► 1.5 (independent)
```

---

## Parallelization

**Can run in parallel:**
- 1.1 must complete first (sets up project)
- Then: 1.2 and 1.5 can run simultaneously
- 1.3 can start once 1.2.6 (config types) is done
- 1.4 can start once all of 1.2 is done

**Optimal execution order:**
1. Start with 1.1 (scaffolding)
2. After 1.1: Start 1.2 and 1.5 in parallel
3. After 1.2.6: Start 1.3
4. After 1.2 complete: Start 1.4

**Agent tier recommendations:**
| Ticket | Recommended Tier | Reason |
|--------|------------------|--------|
| 1.1 | Utility | Boilerplate, straightforward |
| 1.2 | Worker | Type definitions need care |
| 1.3 | Worker | Config logic, error handling |
| 1.4 | Worker | Database setup, migrations |
| 1.5 | Utility | Logging setup is templated |

---

## File Changes Summary

### New Files Created

```
nexor/
├── Cargo.toml                          ← 1.1.1
├── src/
│   ├── main.rs                         ← 1.1.3, 1.5.2
│   ├── lib.rs                          ← 1.1.2, 1.5.1
│   ├── logging.rs                      ← 1.5.1, 1.5.3
│   ├── config/
│   │   ├── mod.rs                      ← 1.3.1
│   │   ├── global.rs                   ← 1.3.1
│   │   ├── project.rs                  ← 1.3.2
│   │   └── validation.rs               ← 1.3.4
│   ├── types/
│   │   ├── mod.rs                      ← 1.2.1
│   │   ├── task.rs                     ← 1.2.1
│   │   ├── agent.rs                    ← 1.2.2
│   │   ├── message.rs                  ← 1.2.3
│   │   ├── ticket.rs                   ← 1.2.4
│   │   ├── cost.rs                     ← 1.2.5
│   │   └── config.rs                   ← 1.2.6
│   ├── db/
│   │   ├── mod.rs                      ← 1.4.1
│   │   ├── migrations.rs               ← 1.4.2
│   │   └── queries.rs                  ← 1.4.8
│   ├── llm/
│   │   └── mod.rs                      ← 1.1.2 (placeholder)
│   ├── agents/
│   │   └── mod.rs                      ← 1.1.2 (placeholder)
│   ├── orchestration/
│   │   └── mod.rs                      ← 1.1.2 (placeholder)
│   ├── execution/
│   │   └── mod.rs                      ← 1.1.2 (placeholder)
│   ├── github/
│   │   └── mod.rs                      ← 1.1.2 (placeholder)
│   └── tui/
│       ├── mod.rs                      ← 1.1.2 (placeholder)
│       └── views/
│           └── mod.rs                  ← 1.1.2 (placeholder)
├── migrations/
│   ├── 001_create_tasks.sql            ← 1.4.2
│   ├── 002_create_task_events.sql      ← 1.4.3
│   ├── 003_create_agents.sql           ← 1.4.4
│   ├── 004_create_messages.sql         ← 1.4.5
│   ├── 005_create_cost_records.sql     ← 1.4.6
│   └── 006_create_tickets.sql          ← 1.4.7
└── tests/
    └── config_integration.rs           ← 1.3.3
```

### Runtime Files Created

```
.nexor/
├── config.toml                         ← User creates (optional)
├── state.db                            ← 1.4.1 (auto-created)
└── logs/
    └── nexor.log.YYYY-MM-DD        ← 1.5.2 (auto-created)
```

---

## Verification Checklist

After all tickets complete, verify:

- [ ] `cargo check` passes with no errors
- [ ] `cargo test` passes for all modules
- [ ] `cargo run` starts and exits cleanly
- [ ] Config loads from `~/.config/nexor/config.toml` (or uses defaults)
- [ ] Config loads from `.nexor/config.toml` (if present)
- [ ] Database file created at `.nexor/state.db`
- [ ] All 6 migration tables exist in database
- [ ] Logs appear in console
- [ ] Logs written to `.nexor/logs/`
- [ ] `RUST_LOG=debug cargo run` shows debug output

---

## Notes

- This milestone is entirely about infrastructure - no business logic yet
- All types are defined but not connected to business operations
- Database tables exist but application doesn't use them yet
- Logging is set up but span usage is minimal until agents exist
- The milestone creates a solid foundation for all subsequent milestones

---

## Next Milestone

After M1, proceed to:
- **M2: LLM Layer** - Requires M1 types for request/response structures
- **M4: Prompt Engineering** - Can start design work in parallel (no code dependencies)
