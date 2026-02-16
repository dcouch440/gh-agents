# Improved Assistant

## The assistant should be:

- the all knowing beign on their domain.∫
- have new tools to be able to execute agents at a given step.
- able to Test the designer by having the designer step accessable to call.
- able to see the output of the Designer and grade it.
- able to run the entire step, using a tool to view it as it runs out showing snap shotted windows timing out after 30 seconds so the assistant can take notes about what he seas.
- their thoughts after the run and be able to update his notes for the next run.
- able to talk to the user DURING runs (outside of tool sleeps).
- able to recieve mentions of agents or context and be able to look them up and see thier executions.

## The system for agents should:

- Respond with usable traceable actions through the assistants tool use.
- be capable of a clean UI, sending messages through to the main window (front end workflows chat box + full screen).
- be debuggable
- be on a sub dag system (Thats a big one)?

## Last statements

Its clear that the application has taken a new direction. The addition of having an assistant that truly understands the system.
The scope of the assistant is only the workshop they live in.
This DOES mean, that we may want to convert Documenter to a dynamically created (Designer to agent) node. This is because we want to give the assistant freedom over the execution flow. With the introduction of input routing from the designer, the task can be handles in a multi faceted system that has a mission statement (User Notes), a way to do it (Assistant), a knowledgeable delegator (Designer), and competent workers (Agents). With this, we want to ensure that each protocol consistently follows this demanding concept.

Final Output
 |
 Agent
 |
 Agent
 |
 Agent -> Document            ...
 |                            |
 Agent -> Document            Agent
 |                            |
 Agent                        Agent
 |                            |
 Designer                     Designer
 |                            |
 Protocol ------------------> Protocol


We should have meta data loading connectors. Agent to agent or Document to agent.
Instead of putting it as context in the agents memory. It should be a task the designer requires of the agent.
"Simply put, designer sees: Connected Document ID(123) requires investigation by agent before his or her work start"

Designer assigns task to agent via ID and agent reads document by simply calling their tools and receiving the output.