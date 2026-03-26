# Clean Up Dead Code in DAG Merge System

## Objective

Remove unused types and fields from the merge system that generate clippy warnings.

---

## Issues

All in `src/server/hub/dag/merge/`:

1. **Unused enum `VerifyResult`** — `types.rs:171-175` — defined but never instantiated
2. **Unused struct fields** (11 clippy warnings total):
   - `FileClassification::step_id` (3 variants)
   - `ConflictHunk::marker_range`
   - `MergeFileDetail::path` and `action`
   - `MergeAction::hunks` and `reason`
   - `StepInfo::step_id`
3. **`VerifyOutcome::Warning` unused field** — `verify.rs:12`
4. **Unused import `warn`** — `persist.rs:10`
5. **Minor clippy fixes**: `&PathBuf` → `&Path` (5 occurrences in classify.rs, context.rs, mod.rs), collapsible if statements, loop indexing antipatterns

## Impact

Eliminates ~11 of the 27 total clippy warnings. No behavior change.

## Verification

- `cargo clippy` — warning count drops from 27 to ~16
- `cargo test hub::dag::merge::` — merge tests pass
