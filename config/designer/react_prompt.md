{{.ReactDesigner.prior_design}}

<dispatch_instruction>
{{.ReactDesigner.dispatch_instruction}}
</dispatch_instruction>

<upstream_topology>
{{.ReactDesigner.upstream_topology}}
</upstream_topology>

Review the board_state. For each agent:
- If design_status="pending", write a new config to design/agents/{slug}.json.
- If design_status="designed", read the existing config via read_file and
  verify it is consistent with the current node_text and upstream topology.
  Update if stale. Skip if correct.
Then call complete_design.
