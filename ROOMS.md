# Rooms

A collabarative mix.

## Change requirements

- Adds an llm layer to rooms executions.

## What it is

At some point, we will be able to place rooms on the board. Rooms should be an item that takes the context in and is sorted. If you look at our concept of the documenter, it has a concept of selecting values and asinging them to a lower layer.

The rooms can have three layers:
Interpreter, Gatekeeper, Responder (Human or Agent)

## Interpreter

The interpreter is an agent that creates context for the responder.
The interpreter is actually only coming up with one line per response.
THis creates a stacking like pattern that we could use for the actual responder.

### Example:

#### Debate Agent

You are in the middle of a conversation between people. Your role is to
respond for the user to make valid arguments.

Your name: Bob

**_Coversation Summery_**: The conversation starts with the user mentioning a bug in the front end.
**_Conversation Summery_**: First response was Gen, She described the amount of money something like that would cost.

-> Now this spots actually the split, its hidden, it does not have own agents context.
**_Conversation Summery_**: Bob didn't know where to find the problem so he suggested looking it up.
-> Back to non hidden from the agent.

**_...It Continues..._**

##### All Users Messages

- "Theres a bug in the front end."
- "I dont care how much it wil cost."
- "Do you know where its at Bob"
- "I'll go do that"

### Example End

Note on how to implement: The view above is a system -prompt for the agent debater. The actual prompt is the most recent incoming message. Or the last output.

Now heres where it gets interesting.
Because with the messages sliced up like this intermixed. We can weave user messages into the agents as long as we discard the previous generations lines. This would free up the user being able to send however many messages they want.

Any example of discarding messages might be:
"Hey I want you to go away" -> Interpreter: "David didn't like that Jon said rude things"
Now two user messages between an actual new agent debate call because one is still running:
"Just kidding" -> Interpreter: "David made a joke about Jon needing to go away but he didn't mean it and was seen to be humouros."

This is additional weaving because we can discard the previous interpetations and use the final one because the job of the interpreter is to judge based on the last messages in the window are. How do we do it? Todo
