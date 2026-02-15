Make group hover only occur if the user hovers for more than a set time seconds.
Create test: When selecting a

New architype, the agent builds simulations of agent orchestration, this is done via a folder structure, PAPER.md, README.md and a folder structure for the simulations.

results/
app/
shared_modules/
app_v1/
app_v2/
...
PAPER.md
README.md

It creates a python application with the goals in mind. The conversation with the assistant IS the simulation creator.
The user should be able to see what the agents doing. Once the plan is set, a crew of agents plan it out.
There should be specially curated prompt engineering agents just as good as the designer.
Really, the designing IS the main source of knowledge in this because we would always need the crew definer.

This is more or less an instance control. Heres the thing, users can save the simulation as prototypes (the resulting agent config). Then they can use them in other places.

You can have multiple open conversations going on with the assistants on the board. The user should feel like he is having a chat with agents who are aware of each other. We need to use prompt injections in a way where agents know they are connected with eachother. Its as if we should have hiaku summeries of conversations to the connected nodes.
Earlier we designed a system where the user has there "current config" injected right into the users prompt. We could have useful information present that can be fetched from every node, this could be summeries of why the user chose that, what the user said etc... It should always be framed in a certain way. Instead of The user decided to go with this because x. It should be:

The following header can be split after event message in the AI's chat log, while messages stay. Strange idea...

You are continuing from a previous conversation, Before you is a description of my node setup on the board.
<Pleas refrain from mentioning these directly unless specifically requested>
// note: only see connected items.
Node 01:
Node Name: Research Task Force
Description: I chose this because I wanted to know what agents did when they watched tv.
Capabilities: [...]
Assigned Agents:
Agent: Researcher
Description: I chose this because I wanted to ...
...
Node 02:
Node Name: ...
</Pleas refrain from mentioning these directly unless specifically requested>

See if you can make the chat window feel a bit more premium.  
 Allow user input size greoth be dependent on the size of the box (use good practices, important),



RESEARCH APP PLUS ONLINE THEN DESIGN -> IMPLEMENT


Assistant personality:
- Instead of responding with an expanation about what your conntected to, talk to the user like their your friend.

Planning agent loads up docker container.
Creates a plan in the docker container.

new protocol: PLANNER
Backstory: Sends messages to other agents on the board after assessing it. Can help build board information.

Detect recenectly delted chat logs for context sweep.
When a user clicks a node it should automatically enter them in chat.

Tools stay on chat board not disapear. They should be full width instead of small, decorative but fitting the scene.



Ensure Assistants NEVER include any information including the actual contents of a document. Only definitions. This is because the assistant should understand that intermediate runs are for creation pourposes only. The assistant should always be setting things up for future runs and they should understand they are there to asssist the user for their next final run.
The assistant should be able to view the content to help currate its affectivness giving feedback to the user on how to improve their document for the upcoming job.

Example -> Document -> Task force is required to read document.
Assistant conversates about its validity and effectivness for the job.
The assistant is the all knowing expert for how the information will be handles in the lifecycle of a run.




USE MORE SIMPLE LANGUAGE.