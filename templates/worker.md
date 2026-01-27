# Worker Task

## Assignment

**ROLE**: You are a worker. Your job is to implement a ticket slice by slice, verifying each step before moving on.

**TICKET**: 2.2

**READ THESE FILES**:
- `WORKER.md` - Your process guide
- `decomp/` - Find your ticket file (e.g., ticket 1.2 → `decomp/M1/1.2.md`)
- `CONVENTIONS.md` - Code style and patterns
- `PROGRESS.md` - Verify dependencies are complete

---

## What You Do

1. Read your ticket spec in the decomp file
2. Check `PROGRESS.md` - make sure you're not blocked
3. For each slice (in order):
   - Read the slice requirements
   - Create/modify the files listed
   - **Write tests** - every slice needs tests
   - Run `cargo check` and `cargo test`
   - Do not proceed until ALL tests pass
4. When all slices complete:
   - `cargo check` passes
   - `cargo test` passes ← **Required, not optional**
   - Update `PROGRESS.md` → status = `done`
5. Create PR (if applicable):
   - **Target the parent branch** (branch you created from, not necessarily `main`)
   - **PR title format**: `{What you built} [Ticket X.Y]`
   - Example: "Add user authentication endpoint [Ticket 2.3]"

---

## Your Output

- Working code that matches the spec
- **Tests for all new functionality**
- All verification steps passing
- `PROGRESS.md` updated
- PR created (if applicable) targeting the parent branch

---

## Rules

- **One slice at a time** - verify before moving on
- **Write tests** - code without tests is incomplete
- **Trust the spec** - the decomp file has what you need
- **Follow conventions** - see `CONVENTIONS.md`
- **Don't add extras** - only build what's in the spec
- **Update progress** - others depend on knowing your status

---

## If Blocked

1. Document what's blocking you
2. Update `PROGRESS.md` with status = `blocked` and notes
3. Use `templates/blocker.md` if it's complex

---

## If Handing Off Mid-Work

1. Use `templates/handoff.md` to capture context
2. Save to `.nexor/work/handoff.md`
3. Update `PROGRESS.md` with current slice progress
