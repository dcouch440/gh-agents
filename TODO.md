Change Assistants notes to "Plan". Remove Required reading from Assistants notes.
Update src/server/hub/dag/workforce/mod.rs to use better algorithms.

Multi input, Multi output ports.

# room:

❯ Is there any usful code in belief capture? Something we may want to add into room? It  
 looks like a mess.

For refecnce. THe workforce team is assembled via conversation with the assistant. THe  
 assistant dispatches the goals to the dispatcher who updates the roster and the agents  
 notes.

With that said, what I think a "room" should be...

- User talks to the assistant with all context contected to the agent.
- This meeting could be about anything, the user could connect a workforce and say...  
  Hey I want to know what exactly this agent did step by step. The meeting should be able  
  to create an agent that has the capabilities to actually pull up the entire log (maybe a  
   Tier 3) and summerize step by step what happened for the meeting memebers. Then, once  
  the pre-phase is done. The user should be able to actually talk with agents that have  
  the knowedge and can ansewer questions.
- It would be a turn based system too if theres more than 1 user. The meeting should  
  have a gatekeeper that activates between turn cycles allowing new users to speak.  
  Possibly even a "conclude system" allowing users to talk to agents more than once. This  
  would require a deep system allowing for long time step activation with lots of turns.


ALways update protocol name
