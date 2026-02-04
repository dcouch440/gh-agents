# The main tool loop of interactive agents.

The primary agent will be provided with a tool and a detailed description.
{
    "tool_id": "request_assistance",
    "tool_description": "You can request assistance from agents, see parameters description ...",
    "parameters": {...}
}

This would provide us with a waterfall case match to clusters that would run to fulfill the goal.

Tool calls routing agent -> Agent routes it to finitely named cluster {"tool_id":"search_database": "query": "user requires database entries for table "..." column "..." use command (complex command)."}.

Agent router would get context for file tree, database tables and would have one goal to repond and call the tool himself.
This way tool use can be handles by sonnet and opus would only be used if the tool sonnet called required it. This would cut down on context for natural conversation because the main agent would only have the job of calling the single tool to request help rather than having a giant context of tools. This would help for the concept of documentation focused writing. If we could possibly mount the agents document its going to use to run the next set of tasks we could collaberate and design it together. Maybe its on the sidebar open to view during this.

THe UI elements that display events should should be though of like a livestream. Subscribed responsibly, with save clean code. I want to make sure our server is setup and ready for these conditions.