{{.ReactDesigner.prior_design}}

<dispatch_instruction>
{{.ReactDesigner.dispatch_instruction}}
</dispatch_instruction>

<upstream_topology>
{{.ReactDesigner.upstream_topology}}
</upstream_topology>

{{.ReactDesigner.previous_step}}

{{.ReactDesigner.next_step}}

Review the board_state. For each agent:
- If design_status="pending", write a new config to design/agents/{slug}.json.
- If design_status="designed", read the existing config via read_file and
  verify it is consistent with the current node_text and upstream topology.
  Update if stale. Skip if correct.

When writing expected_output:
- Read <previous_step> to understand what the agents will hear
  from the step before. Reference it in your assignments.
- Read <next_step> to understand what comes after. Shape
  expected_output to orient the next step.
Then call complete_design with a step_handoff describing what this
step produces for the next step's designer.
