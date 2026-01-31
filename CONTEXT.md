We need to design a react context architecure that is 1 to 1 with our database schema. That way we can match the web socket events in the UI in the same order. We dont want every event on the same stream.
It should be layered to role and we need to find a home for its subscriptions.

For example:
Subscriber to statistics, chat messages made for users. A stream of critical information for UI such as agent count orchestration count, possibly tokens per our for all agents active. These will need a home and we should define an architecture around it.

We want events to be smooth and for it to be reactive. Shared state where required and setup well. Production ready code.


For tool use:
we want a layered system that flows in the following.
llm match case for the tool for the job.

Say, the primary agent that you are speaking with for a case like "Product Design", presents a plan to you and has all the context they need. You open a chat and start the conversation with him as his proposal being the last message. The introduction of the conversation is your review to that person. His goal and config is to be consistent with you and think of how he can change his product desicion with the context you are providing.

The primary agent wont have tools, rather an ability to phone a friend using a clearly written (instructions in his context) explation passed down. That way we can get confirmation with the experts and the right tools.

The agent layer that picks up the the task is bassicly a switch case "expert in all fields" and responds with a few values.

Task Description: "The details it takes to get the job done"
Task Identifier: "a simple key to write thats easy to identify"
Additional Values: "suggestions needed"


This could return a response that 