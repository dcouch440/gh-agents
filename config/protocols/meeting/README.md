# todo: Meetings

## A Theory of Routing

To have a conversation with many is hard. You have the plain options of presenting the entire chat to agents or sumerizing it all.

Heres a question, how can we deal with this in slices of context. The main concept of this is first aggregating a point made, then identify those involved, then identify the necessary context.

In the application, we will have a node called a meeting. It will be hooked up to other protocols that also have data.
We have a complete map of everything that has happened. We cant cram all this in one agents mind on every message while also having every other agents point being made at the same time.

We need a complete system, we need to have agents that can search the connected nodes, or even search further than that to gather context. The user will define what the meeting is about. Before starting cheap agents will search local protocol nodes for context.
They should have a complete mental map of the application before the meetings start. "searching agents" name in progress.

Possible Agents:
Search: Agents that find relevant information about the meeting before it starts through search.
        We would almost need a filetree like tool that can show the agents quickly what they need to see.
        The tool responses should be formatted and easy to read large amounts of data in one go.
Gatekeeper: Selects context from board and speakers to go. Each round is the same agent with a slightly different config.
            The gatekeeper decides if we need to look for more context or not.
            The gate keeper assigns speaking order and context selection.
            THe gate keeper has access to all messages (search) incase the user references something it does not recodnize ("example: "Can you copy down that graph").
            In this situation, the gatekeeper would be required to search old messages for this.
Speaker: The agents that are responding to you. They are provided selected context, not sure about the messages.
