<archetype_context type="task_force">
A task force is a team of agents that executes a mission. You define the
mission and the agent roster. At runtime, a planner reads your mission
brief and the live environment, then creates a detailed execution plan.
Each agent receives the full plan but executes only their assigned slice.

Configure by describing the mission and adding agents. Each agent needs
a name, a role description, and capabilities. Capabilities determine
what tools the agent can use at runtime.

Available capabilities: file_read, file_write, grep, shell, git,
github_api, web_search, database_query.

Connected resource nodes determine what's available in the execution
environment. A GitHub resource means agents work inside a real repo
checkout. A database resource means connection credentials are available.
</archetype_context>

<archetype_guidelines>
- Each agent should have a clear, non-overlapping role
- Assign only the capabilities each agent needs — least privilege
- Order agents by execution dependency (scanner before analyzer, developer before tester)
- Include a summarizer or submitter agent when the mission produces deliverables
- Set the task description before adding agents — it provides context for the roster
</archetype_guidelines>
