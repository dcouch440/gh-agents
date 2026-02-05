# Review: Ticket {X.Y}

**Date**: {YYYY-MM-DD}
**Reviewer**: {Agent/Role}
**Verdict**: {approved | changes_requested | blocked}

---

## Summary

{One paragraph summary of what was implemented}

---

## Checklist

### Code Quality
- [ ] Follows `CONVENTIONS.md`
- [ ] No obvious bugs
- [ ] Error handling is appropriate
- [ ] No panics in library code

### Completeness
- [ ] All slices implemented
- [ ] All verification steps pass
- [ ] Matches spec in decomp file

### Testing
- [ ] `cargo check` passes
- [ ] `cargo test` passes
- [ ] New functionality has tests

### Documentation
- [ ] Public APIs have doc comments
- [ ] Complex logic has explanatory comments
- [ ] `PROGRESS.md` updated

---

## Issues Found

### Critical (must fix)

| File | Line | Issue |
|------|------|-------|
| `src/path/file.rs` | {line} | {description} |

### Suggestions (optional)

| File | Line | Suggestion |
|------|------|------------|
| `src/path/file.rs` | {line} | {description} |

---

## Verdict

**{approved | changes_requested | blocked}**

{Explanation of verdict}

---

## Next Steps

{If approved}: Ready to merge/proceed
{If changes requested}: Address issues above and re-submit
{If blocked}: {What needs to happen}
