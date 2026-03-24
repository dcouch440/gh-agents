1. The container ↔ filesystem bridge. The system node agent writes files via run_command inside a Docker container. Those files need to appear on the host for sync_to_db and file_reader
  to read them. We're relying on the JuiceFS volume mount path matching between container and host — but we're using step_id as the run_id in the container config, which is a hack. If the
  workspace mount resolves differently than expected, the host reads an empty directory.
  2. The run_command heredoc pattern. The system node agent writes JSON via cat > file.json << 'EOF'. If the LLM produces JSON with certain characters (single quotes, backslashes, EOF
  literal), the heredoc breaks silently. We have no write-time validation interceptor — that was deferred.
  3. First run vs re-run state. On first dispatch, base_dir is empty. On re-dispatch, it has previous files. The agent's <current_state> shows what exists. But if a previous run failed
  partway (wrote topology.json but not all agent files), the agent sees an inconsistent state. The complete_system validation catches this, but the agent might get confused.
  4. The propagation instruction format. The system prompt examples show <task> and <change> blocks. The board instruction builder produces <user_text> and <change>. The propagation
  formats yet another variant. The LLM is flexible, but we haven't tested that it responds correctly to each format variation.
  5. Session history across the rewrite. Steps that were previously configured by the old builder now get dispatched to the system node agent. The session history has old builder passdowns
   (from complete_task). The system node agent expects complete_system summaries in <prior_work>. Old history might confuse it.
  6. The designer service. The agent gutted run_standalone_design to return an error, but that function might be called from an API endpoint or another service path we didn't trace.
  7. PipelineExecutionContext is now dead but the lifecycle.rs file still defines it and PipelinePhase. If anything references these types transitively, it could cause issues at runtime
  even if it compiles.