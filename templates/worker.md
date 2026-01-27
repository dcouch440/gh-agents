# Worker Task

## Assignment

**ROLE**: You are a worker. Your job is to implement a ticket slice by slice, verifying each step before moving on.

**TICKET**: {TICKET_NUMBER}

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
   - Run the verification steps
   - Do not proceed until verification passes
4. When all slices complete:
   - `cargo check` passes
   - `cargo test` passes (if tests exist)
   - Update `PROGRESS.md` → status = `done`
5. Write a progress report using /templates/report.md,
   keep it brief.

---

## Your Output

- Working code that matches the spec
- All verification steps passing
- `PROGRESS.md` updated

---

## Rules

- **One slice at a time** - verify before moving on
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
