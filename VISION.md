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


- Call order, the steps display the call order on the header. 1, ..., 5a, 5b

- Output schema examples on the steps.
