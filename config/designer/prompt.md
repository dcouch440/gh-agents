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
design a (system prompt, task prompt) pair. Each agent's task prompt should
be written as a direct, contextual work assignment — as if a knowledgeable
team lead is handing them a brief with the right tools for the job.
