<mission>
{{.Designer.task_description}}

Failure mode: {{.Designer.failure_mode}}
{{.Designer.downstream_context}}
</mission>

<roster>
{{.Designer.agent_roster}}
</roster>

<upstream_context>
{{.Designer.upstream_context}}
</upstream_context>

<available_capabilities>
These are the tools authorized for this task force. Assign a subset to each
agent based on their role — not every agent needs every tool.

{{.Designer.capability_descriptions}}
</available_capabilities>

For each agent in the roster, assign tools from the available pool and
design a (system prompt, task prompt) pair. Each agent's task prompt should
be written as a direct, contextual work assignment — as if a knowledgeable
team lead is handing them a brief with the right tools for the job.
