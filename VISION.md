# Auto Block UI

A block of conversations.

              Agent
              |
              Agent
              |
              Agent -> Document     ...
              |                     |
              Agent -> Document     Agent
              |                     |
              Agent                 Agent
              |                     |
              Designer              Designer
              |                     |
Input-Context-Protocol -----------> Protocol
(Instead of wires, we have auto piping between gaps, like hard pipes)

## About

I was noticing the board would look great if every single block was the same size and lined up next to each other. Imagine for a second if we had a special algorithm, coded with precision. The algorithm would make the blocks "notch" to their size. Additionally, when blocks get close to each other they stick. Pipes run a special piping algorithm that can weave them in between pathways. This is a profound concept and may require online research into algorithms. How it would be accomplished etc... We want to ensure that our code is the best in the business while using our Collections static primitive methods.

The "pipes" can be auto snapped at time of attachment. I think it would be cool to have pipes that linen up next to each other nice and unified. The user is not always going to want to have things the default size so we want to be smart about how we approach the alignment.

We have a pretty nice store already in the front end so we should be able to build off that and maybe refactor one of the UI sections to have a truly effective system that routes events only to the subscribing functions so we do not have a lot of renders.

Theres another issue to consider. What happens when some one resizes a block ON TOP of another one. Do we shove the other ones out of the way and break the sleek seem? It almost seems like we might be able to resize many nodes on the way over to "perfect the block". Like automatic resizing. Very cool...

This also means the need to be mindful of the ports on the end of the block. We must ensure the pipes are coming out of the right ones.

Assistants notes: Always top
Protocol: All sides (Left: input, context), (Top: Agents), (Right: Next protocol), (Bottom: Notes)
Users Notes (context): Always Left
Input: Always Left (one use only)
Agent (Dynamic Node): All sides (Left: Possible parallel agent), (Right: Document), (Top: Agent), (Bottom: Protocol, Agent)
Document: Always left


With that picture painted. You can see that we will need to line the pipes up to run in parallel lines real sleek like.

