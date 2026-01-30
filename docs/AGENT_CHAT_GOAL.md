# Goal: Conversational Agent Creation & Wiring

Users type into the chat window to create agents, configure their roles, and wire them up to work together.

Examples of what this looks like:
- "Spin up a worker to handle the frontend tests"
- "Create a reviewer agent and have it watch the worker's output"
- "Add a utility agent for formatting"

## Agent Clusters & Tooling

Agents are freeform — an orchestrator or a worker, each with different tools.
The user defines what an agent can do by selecting tools and configuring triggers.
Groups of agents working together form **clusters**.

Tools come from the existing execution layer:
- File operations (read, write, list)
- Git operations (status, diff, commit, branch)
- Test runner (full suite, single test, coverage)
- Sandboxed command execution
- Agent management (create, list, assign, remove)

New tools are just a schema + an execute handler. Users should be able to
compose agents with whatever tool mix fits their workflow.

## Scheduled / Timed Agents & Pipelines

Agents support scheduling and event-driven triggers. Completion of one agent
can trigger the next, forming pipelines.

Examples:
- "Find bugs and commit fixes, then have a reviewer check them"
- "Run the test suite every hour and report failures"
- "After every ticket completion, add tests and fix bugs"
- A ticket only gets accepted after two agents sign off (worker + reviewer)
- Chain agents: worker → reviewer → merge agent

Trigger types:
- **Cron-like** — run on a schedule (hourly, daily, etc.)
- **Event-driven** — fire on completion events, commit events, ticket transitions
- **Approval gates** — ticket/PR only advances after N agents approve

This means agents need: scheduled execution, completion triggers that
spawn/notify downstream agents, pipeline definitions, and acceptance
criteria (e.g. "two agents must pass before merging").

## Approach

Every implementation decision serves this end state. The execution layer
(file ops, git, tests, sandbox) already exists. The next step is adding
native Anthropic tool use to the orchestrator so it can call tools and
manage agents on behalf of the user through the chat window.
