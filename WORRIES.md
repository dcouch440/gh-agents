# System Node Agent — Resolved Worries

All items below have been addressed. Kept for historical reference.

1. ~~Container ↔ filesystem bridge~~ — Fixed. JuiceFS volume mount confirmed working. Files persist via `runs/{step_id}/` path.
2. ~~Heredoc pattern~~ — Fixed. Write-time validation in `validate_written_files()` catches truncated JSON after every `run_command`.
3. ~~Partial failure state~~ — Handled by design. `build_current_state` shows `status="missing"`, `complete_system` validation catches inconsistencies, agent self-corrects.
4. ~~Instruction format~~ — Low risk. Claude handles `<user_text>` and `<task>` interchangeably.
5. ~~Session history~~ — Fixed. System node agent uses `role="system_agent"` for session isolation.
6. ~~Designer service~~ — Fixed. `design_step` API returns 404. `get_latest_design` (read-only) still works.
7. ~~Dead lifecycle types~~ — Fixed. `lifecycle.rs` deleted, full dead code sweep completed.

## Known debt (not urgent)

- `step_id` used as `run_id` for workspace path — semantically wrong but functionally correct. A proper `pinned_step_path` would be more resilient to run directory garbage collection.
- No wall-clock timeout on system node dispatch — relies on `max_rounds: 10` to cap runaway agents.
