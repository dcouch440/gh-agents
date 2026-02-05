# Ticket {X.Y}: {Title}

> {One-line description of what this ticket accomplishes}

## Goal

{What does "done" look like? Be specific and measurable.}

**Checkpoint**: {How to verify the whole ticket is complete}

---

## Context

{What does the worker need to know before starting?}

**Key files**:
- `path/to/relevant/file.rs` - {why it's relevant}

**Dependencies**:
- Requires ticket {X.Y} to be complete
- Needs {specific type/function} from {module}

**References**:
- See `ROADMAP.md` section {X} for full spec
- See `PRD.md` section {Y} for data models

---

## Slices

### Slice {X.Y.1}: {Title}

**Do this**:
- {Specific action with file path}
- {Another specific action}

**Create/modify**:
- `src/path/to/file.rs`

**Verify**:
- [ ] `cargo check` passes
- [ ] {Specific verification step}

---

### Slice {X.Y.2}: {Title}

**Do this**:
- {Specific action}

**Create/modify**:
- `src/path/to/file.rs`

**Verify**:
- [ ] `cargo check` passes
- [ ] `cargo test {test_name}` passes

---

### Slice {X.Y.3}: {Title}

**Do this**:
- {Specific action}

**Create/modify**:
- `src/path/to/file.rs`

**Verify**:
- [ ] `cargo check` passes
- [ ] {Integration verification}

---

## Notes

{Any gotchas, decisions, or context that doesn't fit above}

- {Note 1}
- {Note 2}

---

## Completion Checklist

Before marking this ticket done:

- [ ] All slices verified
- [ ] `cargo check` passes
- [ ] `cargo test` passes (if tests exist)
- [ ] Code follows `CONVENTIONS.md`
- [ ] `PROGRESS.md` updated
