# Front end design

Database requirements:
Push all events to the UI through websocket.

Front end:
Watch progression unfold, whats being said, what is happening.
The user should be able to view the stages they created and click on the configurations of them.
For example, they are designing the prompts all the way down with access to variables.

We should be able to block at an agent who is responsible for project design.
Or maybe how the code is written... Context may look like.

Agent: look at the project front end component design and return a list of reusable components for the project that can be shared and used to meet to projects requirements.
Output: ChatWindow, ChatWindowInput, ...

in use...

Application Goals:
...

Components To Design (Injected):
ChatWindow, ChatWindowInput, ...

Agent Guidelines and rules:
Help the user design the components above in the context of the application goals.
Use your best judment by ... . Work toward helping the user with their needs.
You cannot search the codebase but you are provided with the tool "request_help". Which will aid in questions you may have in the database. example: ({"request_help": "'short summery of what is required' | 'the user needs to know if their is a global context manager in the front end.", | ...}). You may be provided with a response where the agent cannot help you. # this would more or less invoke a routing tool for a list of tools that can be used and each one would have a specific set of tasks that are invoked in order.

Your Responsibilities:
{{Context.project_designer}}

Prompt:
Please review the the implementation goal below:
{{ticket}} # Formatted task
