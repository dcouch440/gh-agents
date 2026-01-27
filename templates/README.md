# Templates

Copy-paste templates for common documents.

## Task Assignment Templates

| Template | Purpose | Used By |
|----------|---------|---------|
| `orchestrator.md` | Assign a milestone to decompose | Task assigner |
| `worker.md` | Assign a ticket to implement | Task assigner |

## Work Product Templates

| Template | Purpose | Used By |
|----------|---------|---------|
| `ticket.md` | Detailed ticket breakdown | Orchestrators |
| `milestone.md` | Milestone summary with dependency graph | Orchestrators |
| `decision.md` | Architectural decision record (ADR) | Anyone |
| `handoff.md` | Context capture when pausing work | Workers |
| `blocker.md` | Document blockers clearly | Workers |
| `review.md` | Code review feedback | Reviewers |
| `report.md` | Work completion report | Workers |

## Usage

1. Copy the template to your target location
2. Replace `{placeholders}` with actual values
3. Delete sections that don't apply
4. Fill in the rest

## When to Use Each

**ticket.md**
- Orchestrator decomposing a ticket from ROADMAP.md
- Output goes to `decomp/M{n}/{ticket}.md`

**milestone.md**
- Orchestrator summarizing a milestone after decomposition
- Output goes to `decomp/M{n}/README.md`

**decision.md**
- Any significant architectural or technical decision
- Output goes to `decisions/` or inline in decomp files

**handoff.md**
- Pausing work mid-ticket
- Output goes to `.nexor/work/handoff.md`

**blocker.md**
- Can't proceed, need help
- Output goes to `.nexor/work/blocker.md` or inline in PROGRESS.md

**review.md**
- Reviewing completed work
- Output inline or attached to PR

**report.md**
- Completed a ticket or significant piece of work
- Output goes to `.nexor/reports/{ticket}.md` or inline in PR
