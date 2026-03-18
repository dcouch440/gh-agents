{{.ReactDesigner.prior_design}}

<current_design>
{{.ReactDesigner.current_design_handoff}}
</current_design>

<task>
{{.ReactDesigner.task}}
</task>

<step_order>
{{.ReactDesigner.step_order}}
</step_order>

{{.ReactDesigner.previous_step}}

{{.ReactDesigner.next_step}}

Review the board_state. For each agent:
- If design_status="pending", write a new config.
- If design_status="designed", read and verify consistency.

When writing expected_output:
- Read <previous_step> to understand what the agents will hear
  from the step before. Reference it in your assignments.
- Read <next_step> to understand what comes after. Shape
  expected_output to orient the next step.
Then call complete_design.
