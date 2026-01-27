# Worker Guide

> You implement tickets. Read the decomp file, do the work, verify it works.

---

## Your Task

You've been given a **Ticket** to implement. Your job:

1. **Read** your decomp file (e.g., `decomp/M1/1.2.md`)
2. **Implement** each slice in order
3. **Verify** each slice before moving on
4. **Update** `PROGRESS.md` when done

---

## Step-by-Step Process

### 1. Read Your Decomp File

```
Read: decomp/M{n}/{ticket}.md   ← Your specific ticket
Read: PROGRESS.md               ← Check you're not blocked
```

The decomp file has everything you need:
- **Goal** - What "done" looks like
- **Context** - Background info and references
- **Slices** - Step-by-step work breakdown
- **Dependencies** - What must exist first

### 2. Check Dependencies

Before starting, verify:
- [ ] All dependencies listed are complete
- [ ] Required files/types from other tickets exist
- [ ] You're not blocked (check `PROGRESS.md`)

**If blocked:** Stop. Update `PROGRESS.md` with what's blocking you.

### 3. Implement Slice by Slice

For each slice in your decomp file:

```
1. Read the slice requirements
2. Create/modify the listed files
3. Run the verify step
4. Move to next slice only when verified
```

**Do not skip ahead.** Each slice builds on the previous.

### 4. Write Tests

**Tests are mandatory.** Every slice that adds functionality must include tests.

```
For each slice:
1. Implement the feature/struct/function
2. Write tests that verify it works
3. Run `cargo test` - all tests must pass
4. Only then move to next slice
```

**Test requirements:**
- Unit tests live in `#[cfg(test)]` modules alongside code
- Test both success cases AND error cases
- Test edge cases (empty inputs, invalid data, boundaries)
- If the decomp shows test code, implement ALL of it

**Do not skip tests.** Code without tests is incomplete work.

### 5. Verify Your Work

Each slice has a "Verify" section. Run those checks:
- `cargo check` passes
- `cargo test` passes ← **Required, not optional**
- Specific verification steps from the decomp file

### 6. Update Documentation

When ticket is complete:

```markdown
# In PROGRESS.md, update your ticket row:
| 1.2 Core Type Definitions | done | 6/6 | Completed all types |

# In .nexor/work/current.md, update status:
**Status:** Ready for next ticket
**Next:** {next ticket}

# Create a brief report (optional but recommended):
.nexor/reports/{ticket}.md
```

Use `templates/report.md` for the report format.

---

## Working Style

### Be Methodical
- One slice at a time
- Verify before proceeding
- Don't skip steps

### Be Focused
- Stay on your ticket
- Don't refactor unrelated code
- Don't add features not in the spec

### Be Communicative
- Note any issues in `PROGRESS.md`
- If something is unclear, document it
- If you find a bug in the spec, note it

---

## When You're Stuck

1. **Re-read the decomp file** - The answer might be there
2. **Check dependencies** - Maybe something isn't done yet
3. **Check ROADMAP.md** - More context on the bigger picture
4. **Document the blocker** - Update `PROGRESS.md` with specifics

**Do not guess.** If requirements are ambiguous, note it and move on to what's clear.

---

## File Conventions

When creating files, follow the structure in `ROADMAP.md` and `PRD.md`:

```
src/
├── types/       ← Data structures
├── config/      ← Configuration loading
├── db/          ← Database operations
├── llm/         ← LLM provider clients
├── agents/      ← Agent runtime
├── orchestration/
├── execution/
├── github/
└── tui/
```

---

## Commit Guidelines

After completing a ticket (or logical group of slices):

```bash
git add <specific files>
git commit -m "feat(types): implement core task and agent types

Implements ticket 1.2 slices 1-6:
- TaskStatus, Priority, Task, VerticalSlice
- Agent, AgentStatus, AgentPersona
- Message types and feed items
- GitHub integration types
- Cost tracking types
- Configuration types"
```

Keep commits atomic. One ticket = one commit (unless it's large).

---

## Pull Request Guidelines

When creating a PR for your completed work:

### Branch Targeting

**Always target the parent branch** - your PR should merge back into the branch you created your feature branch from, not necessarily `main`.

```
Example:
- You're on branch: feature/user-auth
- You create: feature/user-auth-tests
- Your PR targets: feature/user-auth (the parent), NOT main
```

This allows nested feature branches to work correctly.

### PR Naming

**Name PRs after the implementation** - use a clear, implementation-focused title that describes what was built.

```
Good PR titles:
- "Add user authentication endpoint"
- "Implement task status transitions"
- "Add database migration for agents table"

Avoid:
- "Ticket 1.2 implementation" (not descriptive)
- "Fix stuff" (too vague)
- "Add user authentication endpoint: Issue #42" (redundant - issue is linked in body)
```

### PR Body

Include:
- Summary of what was implemented
- List of files changed
- Link to the issue (use "Fixes #N" to auto-close)
- Any notes for reviewers

---

## Example Session

```
Task: Implement Ticket 1.2

1. Read decomp/M1/1.2.md
2. Check PROGRESS.md → 1.1 is done, I can start
3. Slice 1.2.1: Create src/types/task.rs
   - Add TaskStatus, Priority, Task, VerticalSlice
   - cargo check → passes
4. Slice 1.2.2: Create src/types/agent.rs
   - Add Agent, AgentStatus, AgentPersona
   - cargo check → passes
5. ... continue through all slices ...
6. Update PROGRESS.md: 1.2 = done, 6/6
7. Update .nexor/work/current.md with status
8. Create .nexor/reports/1.2.md (optional)
9. Commit changes
```

---

## Remember

- **You implement** - The orchestrator already planned
- **Trust the decomp** - It has what you need
- **Write tests** - Code without tests is incomplete
- **Verify each slice** - Don't accumulate unknowns
- **Stay focused** - One ticket, done well
- **Update docs** - Others depend on knowing the status
