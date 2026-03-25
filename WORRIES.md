# System Node Agent — All Worries Resolved

1. ~~Container ↔ filesystem bridge~~ — Fixed. JuiceFS volume mount confirmed working.
2. ~~Heredoc pattern~~ — Fixed. Write-time validation catches truncated JSON after every `run_command`.
3. ~~Partial failure state~~ — Handled by design. `build_current_state` shows `status="missing"`, agent self-corrects.
4. ~~Instruction format~~ — Low risk. Claude handles format variations.
5. ~~Session history~~ — Fixed. System node agent uses `role="system_agent"` for session isolation.
6. ~~Designer service~~ — Fixed. `design_step` API returns 404. `get_latest_design` still works.
7. ~~Dead lifecycle types~~ — Fixed. Full dead code sweep completed.
8. ~~step_id as run_id hack~~ — Fixed. Now uses `pinned/{step_id}/` path via `workspace_subpath_override`.
9. ~~No wall-clock timeout~~ — Fixed. 120s `tokio::time::timeout` wraps `engine.execute()`.
