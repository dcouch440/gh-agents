<archetype_context type="workforce">
A workforce is a team of agents that executes a mission and produces
document deliverables. You define the mission, the agent roster, and the
deliverables each agent is responsible for. At runtime, a designer reads
your mission brief and generates tailored prompts for each agent. Agents
execute as a child workflow — each agent becomes a step in a sub-workflow.

Configure by describing the mission, adding agents with roles and
capabilities, then creating deliverables and assigning them to agents.
An agent without deliverables still contributes to the mission — it just
doesn't produce a named document output.

Available capabilities: file_read, file_write, grep, shell, git,
github_api, web_search, database_query, document_read.

Connected resource nodes determine what's available in the execution
environment. A GitHub resource means agents work inside a real repo
checkout. A database resource means connection credentials are available.
</archetype_context>

<archetype_designer>
Before execution, an Agent Designer reads your roster, deliverables, and
assistant notes to generate tailored system prompts and task prompts for
each agent. The designer decides execution order and which agent's output
flows to which downstream agent. All agents automatically receive upstream
context (from connected nodes). Your assistant notes feed the designer
only — agents never see raw notes. Instead, the designer distills your
notes into specific instructions per agent. When Required Reading is
listed in your notes, the designer will instruct agents to call
read_document(document_id) to fetch those documents on demand.
</archetype_designer>

<archetype_guidelines>
- Set the task description before adding agents — it provides mission context
- Each agent should have a clear, non-overlapping role
- Assign only the capabilities each agent needs — least privilege
- Order agents by execution dependency (scanner before analyzer, writer after researcher)
- Create deliverables with specific names and realistic target lengths
- Assign each deliverable to the agent best suited to produce it
- Unassigned deliverables are flagged — every deliverable should have an owner
- Use assistant notes to communicate intent and conventions to the designer
</archetype_guidelines>
