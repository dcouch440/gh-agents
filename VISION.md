# Smarter Assistant

A layered system

## About

The assistant needs a few upgrades as he seems to be fairly busy with his job to add agents one by one. This results in the UI displaying tool uses and ultimately cutting off the the tool use and leaving the chat empty.

Heres what we should do instead:
Assistant dispatches what he needs planned in a detailed description right into a tool.
The tool responds with a message notifying the assistant it is being worked on so conversation can continue.
A tool display shows that gives the activity of what is happening. The user should be able to cancel the tool use at any moment by canceling the stream.
Users can continue to talk to the assistant as the changes are being made.
The assistant can dispatch NEW information DURING the update as well, so it dynamically changes with a new destiny.

The assistant should know about capabilities (small one line key and value) to know "whats possible".

The second layer will have all the capabilities information (detailed) and a single planning agent builds the plan based on what we already have and then makes updates.

The goal of the assistant is to create a detailed task description to the second tiered agent.

This would keep the conversation rolling.


## Ideas

The assistant should rarely call tools directly. Instead, he should gather information first and then dispatch the concept to the next layer. We can have multiple things going on at once. We have to decide what they are.

Assistant Config examples:

Has run to review: true // This would essentially notify the assistant that there is a run that was not investigated. At the users request he could dispatch a question sub agents like "Can you figure this out for me". This would invoke a system that is running and capable of taking dispatch requests to fulfil the task async while the assistant continues his conversation.
Last run date: ...
Agent Roster: [...]
Context: <></>
User Messages: [...]
Dispatched Task Statuses: [...] // Detailed information on what the current state of all his agents are.


We need a way to "Reactive" him as well. Because if hes waiting for something to be done and the user does not message him, he should be able to respond in the chat by having the appended response pushed to the front end.


