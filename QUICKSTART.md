# Quickstart

> Read this first. 60 seconds to understand the system.

---

## What Role Are You?

### Orchestrator?

You decompose milestones into tickets.

```
YOUR TASK: Milestone 1
PLEASE SEE: ORCHESTRATOR.md
```

**Your workflow**:
1. Read `ROADMAP.md` → find your milestone
2. Read `ORCHESTRATOR.md` → follow the process
3. Create files in `decomp/M{n}/` → one per ticket
4. Use `templates/ticket.md` → copy and fill in

**Your output**: `decomp/M1/1.1.md`, `decomp/M1/1.2.md`, etc.

---

### Worker?

You implement tickets slice by slice.

```
YOUR TASK: Ticket 1.2
PLEASE SEE: WORKER.md, decomp/M1/1.2.md
```

**Your workflow**:
1. Read `WORKER.md` → understand the process
2. Read your decomp file → `decomp/M{n}/{ticket}.md`
3. Check `PROGRESS.md` → make sure you're not blocked
4. Implement slice by slice → verify each one
5. Update `PROGRESS.md` → mark done when complete

**Your output**: Working code, updated progress.

---

## Key Documents

| Document | What's In It | When to Read |
|----------|--------------|--------------|
| `ROADMAP.md` | All milestones and tickets | Planning |
| `PROGRESS.md` | Current status | Before starting |
| `CONVENTIONS.md` | How to write code | While coding |
| `decomp/M{n}/*.md` | Detailed ticket specs | While implementing |

---

## The Flow

```
ROADMAP.md (spec)
     ↓
ORCHESTRATOR reads milestone
     ↓
Creates decomp/M{n}/*.md files
     ↓
WORKER reads decomp file
     ↓
Implements slice by slice
     ↓
Updates PROGRESS.md
     ↓
Done
```

---

## Quick Rules

1. **One slice at a time** - verify before moving on
2. **Trust the decomp** - it has what you need
3. **Update progress** - others depend on it
4. **Follow conventions** - consistency matters
5. **Don't guess** - if unclear, document and move on

---

## Status Values

| Status | Meaning |
|--------|---------|
| `pending` | Not started |
| `in_progress` | Being worked on |
| `blocked` | Waiting on dependency |
| `done` | Complete and verified |

---

## Need More Detail?

| Topic | Read This |
|-------|-----------|
| Why this system? | `PHILOSOPHY.md` |
| Product vision | `PRD.md` |
| Full technical spec | `ROADMAP.md` |
| Code patterns | `CONVENTIONS.md` |
| Decomposition guide | `ORCHESTRATOR.md` |
| Implementation guide | `WORKER.md` |
