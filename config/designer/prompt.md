<archetype>{{.Designer.archetype}}</archetype>

<context>
{{.Designer.context_description}}
</context>

<agents>
{{.Designer.agent_definitions}}
</agents>

<upstream_context>
{{.Designer.upstream_context}}
</upstream_context>

<available_capabilities>
These are the tools authorized for this execution. Assign a subset to each
agent based on their role — not every agent needs every tool.

{{.Designer.available_tools}}
</available_capabilities>

<archetype_guidance>
{{.Designer.archetype_guidance}}
</archetype_guidance>

For each agent listed above, assign tools from the available pool and
produce a system prompt and assignment. The runtime handles mission
context, upstream output injection, and execution ordering — focus on
agent identity and specific task instructions.
