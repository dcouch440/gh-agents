# Convert `#[path = ...]` Test Hacks to Proper Modules

## Objective

Convert 9 files using the `#[path = "..._tests.rs"]` antipattern to standard folder-based modules with colocated `tests.rs` files.

---

## Problem

These files use `#[cfg(test)] #[path = "foo_tests.rs"] mod tests;` instead of the project convention of folder-based modules with separate `tests.rs`. This was already fixed for `file_executor.rs` — the same pattern should be applied to the remaining 9.

## Files

| Source file | Test file |
|------------|-----------|
| `src/server/hub/dag/pipeline/runner.rs` | `runner_tests.rs` |
| `src/server/hub/board/serializer/rasterize_png.rs` | `rasterize_png_tests.rs` |
| `src/server/hub/protocols/context.rs` | `context_tests.rs` |
| `src/server/services/board/executor.rs` | `executor_tests.rs` |
| `src/server/services/board/instruction.rs` | `instruction_tests.rs` |
| `src/server/services/system_node/file_reader.rs` | `file_reader_tests.rs` |
| `src/server/services/system_node/sync.rs` | `sync_tests.rs` |
| `src/server/services/system_node/state.rs` | `state_tests.rs` |
| `src/server/services/system_node/validate.rs` | `validate_tests.rs` |

## Pattern

For each file, apply the same conversion as `file_executor`:
1. Create `foo/` directory
2. Move `foo.rs` → `foo/mod.rs`, replace `#[path]` with `#[cfg(test)] mod tests;`
3. Move `foo_tests.rs` → `foo/tests.rs`
4. Delete old flat files
5. Parent `mod.rs` declaration stays the same (`mod foo;`)

## Note

The 4 `system_node/` files (`file_reader`, `sync`, `state`, `validate`) are all siblings in the same directory. Converting them all at once keeps the module consistent. Some of these may naturally get picked up when touching those files for other tickets.

## Verification

- `cargo check` — compiles
- `cargo test` — all tests pass (no test logic changes, only file moves)
