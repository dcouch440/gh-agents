# nexor

> AI Agent Orchestration TUI for GitHub Workflows

A Rust-based terminal application that orchestrates multiple AI agents to handle software engineering tasks.

---

## Documentation Flow

This project uses a structured documentation system designed for AI agent consumption:

```
PRD.md                    ← Product Requirements (the "what")
    ↓
ROADMAP.md                ← Technical Spec (the "how")
    ↓
PROGRESS.md               ← Work Tracking (the "status")
    ↓
decomp/                   ← Detailed Breakdowns (the "do this")
```

### Document Purposes

| Document | Purpose | Who Uses It |
|----------|---------|-------------|
| `QUICKSTART.md` | 60-second overview, read first | Everyone |
| `PHILOSOPHY.md` | Why this system works, core principles | Understanding |
| `PRD.md` | Product vision, data models, architecture | Reference |
| `ROADMAP.md` | Milestones, tickets, slices, dependencies | Source of truth |
| `PROGRESS.md` | Current status, what's done, what's blocked | Track progress |
| `CONVENTIONS.md` | Code style, patterns, naming | Workers |
| `templates/ticket.md` | Template for decomp files | Orchestrators |
| `decomp/M{n}/` | Detailed ticket breakdowns for implementation | Workers |

---

## Agent System

This project is built by AI agents following a simple two-role system:

### Orchestrator

**Decomposes milestones into actionable tickets.**

```
YOUR TASK: Milestone 1
PLEASE SEE: ORCHESTRATOR.md
```

The Orchestrator:
- Reads the milestone from `ROADMAP.md`
- Creates detailed ticket files in `decomp/M{n}/`
- Ensures each ticket has clear slices and verification steps

### Worker

**Implements tickets slice by slice.**

```
YOUR TASK: Ticket 1.2
PLEASE SEE: WORKER.md, decomp/M1/1.2.md
```

The Worker:
- Reads the decomp file for their ticket
- Implements each slice in order
- Verifies each slice before proceeding
- Updates `PROGRESS.md` when done

---

## Quick Start

### For Humans

1. Review `PRD.md` for product vision
2. Check `ROADMAP.md` for technical roadmap
3. See `PROGRESS.md` for current status

### For Orchestrators

1. Receive a Milestone assignment
2. Read `ORCHESTRATOR.md` for instructions
3. Create decomp files in `decomp/M{n}/`

### For Workers

1. Receive a Ticket assignment
2. Read `WORKER.md` for instructions
3. Read your decomp file (e.g., `decomp/M1/1.2.md`)
4. Implement and verify each slice

---

## Project Structure

```
nexor/
├── README.md              ← You are here
├── QUICKSTART.md          ← 60-second overview (read first)
├── PHILOSOPHY.md          ← How and why this system works
├── PRD.md                 ← Product Requirements Document
├── ROADMAP.md             ← Technical roadmap with all milestones
├── PROGRESS.md            ← Work tracking and status
├── CONVENTIONS.md         ← Code style, patterns, naming
├── ORCHESTRATOR.md        ← Guide for decomposition agents
├── WORKER.md              ← Guide for implementation agents
├── CLAUDE.md              ← AI assistant guidelines
├── templates/
│   ├── orchestrator.md    ← Task assignment for orchestrators
│   ├── worker.md          ← Task assignment for workers
│   ├── ticket.md          ← Detailed ticket breakdown
│   ├── milestone.md       ← Milestone summary
│   ├── decision.md        ← Architectural decision record
│   ├── handoff.md         ← Work handoff context
│   ├── blocker.md         ← Blocker documentation
│   └── review.md          ← Code review feedback
├── decomp/                ← Detailed ticket breakdowns
│   ├── M1/                ← Milestone 1 tickets
│   ├── M2/                ← Milestone 2 tickets
│   └── ...
└── src/                   ← Source code (coming soon)
```

---

## Current Status

See `PROGRESS.md` for detailed status.

**Summary:**
- 9 Milestones planned
- 55 Tickets defined
- 222 Slices total

---

## The Workflow

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│   Human writes PRD.md                                       │
│        ↓                                                    │
│   Human/AI creates ROADMAP.md (milestones + tickets)        │
│        ↓                                                    │
│   ┌─────────────────────────────────────────────────────┐   │
│   │  ORCHESTRATOR                                       │   │
│   │  - Receives: Milestone N                            │   │
│   │  - Reads: ROADMAP.md, ORCHESTRATOR.md               │   │
│   │  - Creates: decomp/M{N}/*.md                        │   │
│   └─────────────────────────────────────────────────────┘   │
│        ↓                                                    │
│   ┌─────────────────────────────────────────────────────┐   │
│   │  WORKER                                             │   │
│   │  - Receives: Ticket X.Y                             │   │
│   │  - Reads: WORKER.md, decomp/M{X}/{X.Y}.md           │   │
│   │  - Implements: slice by slice                       │   │
│   │  - Updates: PROGRESS.md                             │   │
│   └─────────────────────────────────────────────────────┘   │
│        ↓                                                    │
│   Code complete, tests pass                                 │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## License

MIT
