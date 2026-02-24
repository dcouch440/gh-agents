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



# NEW agent dispatch tool

Dispatch Detail Mapping system.
Summerize all dispatches in the last x time, results, tasks, input descriptions, everything.
Create detail map where details given are on a scaling pattern for relevence to the applications completness.
1: Made Agent
2: Made an agent for the Analyzer Workforce.
3: Dispatched xyz, this, then that
4: ... full transcript.

This can be used simalarly to grep | tail. A way to investigate what has happend.

Assistant asks the question in a dispatch, which spawns a summerizer agent in the background.
notification sent to assistant that its complete, allowing the manager to read the outputs for conversations with the user or to track down an error in the system to make changes.

Tool UI example for the user to watch the exploration live.
Check Dispatch History L3:
  ├─ Dispatched -> User requirests that we...
  |                ...
  |                ...
  ├─ Builder -> ...
  └─ Task 1: ...
  ...

Builder built node without agents.

Add expansion layer between L2 and L3.


If the user was detailed in their description, the assistant should be detailed in their dispatch.

HALLUCINATION DETECTION

Make dispatch agent multi task capable. running tasks should show on the builder agents system prompt and it should do the task and then complete it.


I guess I just kind of felt like the designer can be more of the
program. hes hidden away, we wont see his response but we should
see the result. I Feel like we need endpoints for the designers
agents he creates and all their data for the given workflow.
We need web socket events too to notify the front end that hes done.
instead of before every run (optional) the user can run the designer in the menu to observe the prompts and debug their program (advanced users).