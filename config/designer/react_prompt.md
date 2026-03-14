{{.ReactDesigner.prior_design}}

<dispatch_instruction>
{{.ReactDesigner.dispatch_instruction}}
</dispatch_instruction>

<node_text>
{{.ReactDesigner.node_text}}
</node_text>

<upstream_topology>
{{.ReactDesigner.upstream_topology}}
</upstream_topology>

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
