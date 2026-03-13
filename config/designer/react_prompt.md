{{.ReactDesigner.prior_design}}

<plan>
{{.ReactDesigner.plan}}
</plan>

<roster>
{{.ReactDesigner.roster}}
</roster>

<builder_action>
{{.ReactDesigner.builder_action}}
</builder_action>

For each agent listed above, write a config file to
design/agents/{slug}.json with tools, system_prompt,
assignment, and expected_output. Then call complete_design.
