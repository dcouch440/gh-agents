# Goal: Conversational Agent Creation & Wiring

Users type into the chat window to create agents, configure their roles, and wire them up to work together.

Examples of what this looks like:
- "Spin up a worker to handle the frontend tests"
- "Create a reviewer agent and have it watch the worker's output"
- "Add a utility agent for formatting"

## Approach

Every implementation decision serves this end state. Step one is connecting the chat orchestrator to the dispatcher/agent pool so that chat messages flow through the agent infrastructure instead of calling the LLM directly.
