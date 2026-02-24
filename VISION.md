# Complexity Setting For Dispatch Agent

## About

Loads multi layer Designers.

For example: One agent -> Write me a story about a cat.
Complexity: LOW
Result -> Simple Designer
Two agent -> Write a scanner and documenter to document the repository.
Complexity: MEDIUM
Result -> ... You get the picture.

## Why?

The Designer is our MOST robust agent asside from the assistant. If a user wants to jump in an spit out some stuff to design and run multiple times they have to wait through the GIANT designer prompt.

## Do we really go through with it?

The Designer is our core founder. The back bone of everything.

====User Path====  
 Do this xyz...  
 Manager: Ok  
 Manager Tool: Dispatch xyz...  
 Dispatch Agent 1: Create topology 3-4 tool uses -> Passsdown  
 Chain Dispatch Agent 2: I see the topology has been created, let me now create  
 the content.  
 Dispatch Agent 3 (Per Node): (content and topology is now made, message is sent  
 to per node agent) Great, ill create the team!

==== Topology Path (The one we care about, simple) ====  
 User: Thinks "I just created everything, let me hit submit"  
 POST -> response: {new nodes on board)  
 Kickoff: Dispatch Agent 4 (Dispatching based on the change set...)  
 Dispatch Agent 3 (Per Node): Great, ill make the nessesary change!!

=== Legend ===  
 Dispatch Agent 1: Responsable for creating topology in a couple turns. No if this  
 or if that. Just a simple agent that creates topology.  
 Dispatch Agent 2: Creates text for the box container (canvas) representing the  
 job.  
 Dispatch Agent 3: Per node, the classic already made and ready.  
 Dispatch Agent 4 Simplified Manager builder, no topology tools, just a well  
 written focused job.
