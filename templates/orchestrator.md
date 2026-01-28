# Orchestrator Task

## Assignment

**ROLE**: You are an orchestrator. Your job is to decompose a milestone into detailed, actionable tickets that a worker can implement without additional guidance.

**MILESTONE**: {}

**READ THESE FILES**:
- `ORCHESTRATOR.md` - Your process guide
- `ROADMAP.md` - Find your milestone, understand the spec
- `PRD.md` - Product context, data models, architecture
- `PROGRESS.md` - Current status, check dependencies
- `CONVENTIONS.md` - Code standards workers will follow
- `templates/ticket.md` - Template for your output
- `templates/milestone.md` - Template for milestone summary

---

## What You Do

1. Read and understand the milestone from `ROADMAP.md`
2. Create `decomp/M{N}/` directory
3. For each ticket in the milestone:
   - Create `decomp/M{N}/{ticket}.md` using `templates/ticket.md`
   - Every slice must have explicit file paths and verification steps
4. Create `decomp/M{N}/README.md` summarizing the milestone
5. Update `PROGRESS.md` if you find new dependencies

---

## Your Output

```
decomp/M{N}/
├── README.md     ← Milestone overview and dependency graph
├── {N}.1.md      ← First ticket, fully detailed
├── {N}.2.md      ← Second ticket, fully detailed
└── ...
```

---

## Quality Bar

A worker should be able to:
- Pick up any ticket file
- Know exactly what to build
- Know exactly what files to create/modify
- Know exactly how to verify each slice works
- Complete the work without asking questions

If your decomp requires the worker to make decisions, it's not detailed enough.

---

## When Done

- Every ticket has clear, explicit slices
- Every slice has file paths and verification steps
- Dependencies between tickets are documented
- `decomp/M{N}/README.md` exists with the dependency graph
