# Orchestrator Guide

> You decompose milestones into actionable tickets for workers.

---

## Your Task

You've been given a **Milestone** to decompose. Your job:

1. **Read** the milestone spec in `ROADMAP.md`
2. **Decompose** into detailed ticket files
3. **Output** to `decomp/M{n}/` directory
4. **Update** `PROGRESS.md` with any notes

---

## Step-by-Step Process

### 1. Understand the Milestone

```
Read: ROADMAP.md → Find your milestone section
Read: PRD.md → Understand the bigger picture (if needed)
Read: PROGRESS.md → Check dependencies and current state
```

**Ask yourself:**
- What is the goal of this milestone?
- What checkpoint proves it's complete?
- What are the dependencies from other milestones?

### 2. Create Decomposition Files

For each ticket in the milestone, create a detailed file:

```
decomp/
└── M1/
    ├── 1.1.md   ← Ticket 1.1: Project Scaffolding
    ├── 1.2.md   ← Ticket 1.2: Core Type Definitions
    ├── 1.3.md   ← Ticket 1.3: Configuration System
    └── ...
```

### 3. Ticket File Format

Each `decomp/M{n}/{ticket}.md` file must follow this structure:

```markdown
# Ticket {n.m}: {Title}

> {One-line description}

## Goal

{What does "done" look like? Be specific.}

## Context

{What does the worker need to know? Reference files, patterns, dependencies.}

## Slices

### Slice {n.m.1}: {Title}

**Do this:**
- {Specific action}
- {Specific action}

**Files to create/modify:**
- `path/to/file.rs`

**Verify:**
- {How to test this slice works}

### Slice {n.m.2}: {Title}
...

## Dependencies

- {What must be done before this ticket?}
- {Reference other tickets if needed: "Requires 1.2.6"}

## Notes

{Any gotchas, decisions made, or context for the worker}
```

### 4. Update Progress

After creating decomp files:
- Update `PROGRESS.md` with any dependency notes discovered
- Add entries to the Decisions Log if you made architectural choices

---

## Quality Checklist

Before you're done, verify each ticket file:

- [ ] **Goal is clear** - Worker knows exactly what "done" means
- [ ] **Slices are vertical** - Each slice works independently
- [ ] **Slices are small** - Each can be done in one session
- [ ] **Verify steps exist** - Worker knows how to test each slice
- [ ] **Files are listed** - Worker knows what to create/modify
- [ ] **Dependencies noted** - Worker knows what to do first

---

## Example Output

For Milestone 1, you would create:

```
decomp/M1/
├── 1.1.md   ← Project Scaffolding (3 slices)
├── 1.2.md   ← Core Type Definitions (6 slices)
├── 1.3.md   ← Configuration System (4 slices)
├── 1.4.md   ← Database Setup (8 slices)
└── 1.5.md   ← Logging Infrastructure (3 slices)
```

Each file is self-contained. A worker reading `1.3.md` should have everything they need.

---

## Remember

- **You don't implement** - You plan and decompose
- **Be specific** - Vague tickets create confusion
- **Think vertically** - Each slice must be deployable alone
- **Reference the spec** - ROADMAP.md is the source of truth
- **Workers depend on you** - Your clarity = their productivity
