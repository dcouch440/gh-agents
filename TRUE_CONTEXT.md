# True Context

Here I describe a concept of "True Context".
For a response to occur to the agent who they are speaking to. We may give the blocks of code to speak about why they are making their decision.

Below is a bare bone example of context transfer:
PROMPT: WILL YOU ADD THE NUMBERS TOGETHER WITH THE NUMBER ABOVE.

Context For Scope (Specific topic that is important for solving the problem):
-> ChatLog[] -> Truncate long chat log 30+ messages -> THe user seems to require a function justing by the message.
<ctx_scop>I am picking the function add because the user is asking me to add numbers.</ctx_scop>

Context For Application ():
-> ChatLog[] -> Truncate long chat log 30+ messages -> ...the user seems frustrated that we are missing the messages above, I should look closer...
</ctx_vibe>The user clearly stated that they wanted to add two numbers to together.</ctx_vibe>


1. Few things, fan_out is gone, lets have it called for each.
2. I think that "for each" needs to be added to the cluster system instead. A STAGE is just a bunch of agents and clusters.

Lets try and imagine this:
Stage 1:
- Cluster 1: Review the project requirements and 

Cluster in in stage 2:
SYSTEM (Assigned in Agent):
You are a fea

Prompt:
{{stage_1.cluster_id.task_name}}

