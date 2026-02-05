# Milestone {N}: {Title}

> {One-line description of what this milestone accomplishes}

## Goal

{What does "done" look like for this milestone?}

**Checkpoint**: {Demo or test that proves the milestone is complete}

---

## Tickets

| Ticket | Title | Slices | Dependencies |
|--------|-------|--------|--------------|
| {N}.1 | {Title} | {count} | None |
| {N}.2 | {Title} | {count} | {N}.1 |
| {N}.3 | {Title} | {count} | {N}.1, {N}.2 |

---

## Dependency Graph

```
{N}.1 ──→ {N}.2 ──→ {N}.4
   │         │
   └──→ {N}.3 ┘
```

---

## Parallelization

**Can run in parallel**:
- {N}.1 and {N}.X (no dependencies between them)

**Must be sequential**:
- {N}.2 needs {N}.1 complete first
- {N}.4 needs {N}.2 and {N}.3

---

## Notes

{Any context, decisions, or gotchas for this milestone}

- {Note}
