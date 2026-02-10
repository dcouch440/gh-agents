# Ideas

## New protocols

We should have a protocol called documenter. Attach the document to protocol agent, have the agent save the document, then we can reference it anywhere on the board with a wire. THis would be handy for multi document saving. An example might be an gent that needs to write a document, it can show up directly on the screen and even be populated. This can be a static document (persists through runs) or it can be fresh document each time. The document can be fed directly into the next agent as context and they can have a system prompt and a prompt still.

First prompt:
Id like to talk about the protocol system. I have some visios about its future.

Heres somthing I wrote down: "We should have a protocol called documenter. Attach the document to protocol agent, have the agent save the document, then we can reference it anywhere on the board with a wire. THis would be handy for multi document saving. An example might be an gent that needs to write a document, it can show up directly on the screen and even be populated. This can be a static document (persists  
 through runs) or it can be fresh document each time. The document can be fed directly into the next agent as context and they can have a system prompt and a prompt still."

So what do we do with this? It makes me think alot about the future of the app. Puting nodes on the screen with system prompts and such is not as fun as its cracked up to be.  
 So what are we to do? Well, I have been thinking about what it would be like to have the protocols be a staple in the system.

If you take our decomp protocol step for example, you will notice it expects the user to assign agents to the possible ports. It makes things a little difficult to precieve.  
 If instead, the protocol box was an an agent that you prompt and it creates them automatically, that would be really cool.  
 You would just type into the box and it would see the incoming context, the users requests and create the next nodes itself.

This is a complex problem sure, but the idea of the document is what gave it away for me. We could have a "document" protocl. The protocl would be an agent responsable for creating the document on the board. The agent would define the documents that need to be created, either staticly or by run. Those documents can be shown on the page as a node and can be connected to any agent on the board.

How do we solve this? Well, I think we should creaete a some detailed tickets together to get this dream to come true. 

## Auto prompt
- Typing into the text box with language that describes what you want, automatically converts to ordered visually appealing markdown. Each agent gets context from the incoming context and converts the prompt to make more sense to the agent who is receiving it. This could be a dedicated feature actually, maybe the documenter does not even need the context himself. Maybe it is rather, a single layer of the prompt creator themsealves writing the prompt for the documentor. This would open the documentor up to searching for agents and specialist for the job. So the new flow could be: "Context resolver, "documentor" who is the agent searcher, agent action with context, documents created. We need to ensure that the 

## Next ideas
- use grok full time as a researcher agent.
- Prompt doctor (evaluator), looks at the step before and validates the ansewer, either returns the fixed ansewr or sends a message to the other llm to get the job done (you missed something). We need a "re-fire".
- Full activity on the side with message aggragation for multiple agents running of the same tier. Activity inside the panel in the front end with a message counter incrementing. LAST RUN PRESENT ON IDLE.

- We need to name the incoming context, the protocol agents, the agents should see their context like this:
<context><context_name>...</context_name>...</context>

This way when they are routing things they can assign the context to the agents.

This is because the protocol is going to be the main hub that routes the jobs to the sub agents. So the naming protocol section can be important context for them to feed to the sub agents.

There will be all different kinds of nodes that are not nessesary made to (go into another). Some might have a port only on the bottom and they are made to review and finalize certain steps. "Crew" can be another one.. Thats the workhorse. Its the large set of agents that works to get a job done. It could be sequancial, parallel.

- change context to workflow based item (Not step documents).

- We need a way to Indentify incoming context to present to the agent if the context is filled during runtime.

- We need to make is so ports have a unqique key on them.

- Call order, the steps display the call order on the header. 1, ..., 5a, 5b

- Output schema examples on the steps.
