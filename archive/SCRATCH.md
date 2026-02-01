# What do we do if we want to make changes part way through?
Referring to our github agents...
I am wondering if we could have another command called /refactor.

## What it would do
Essentially, the concept is that we already have the PRD, ROADMAP.md, and all of our decomp files.
Why don't we just have the system pause production and refactor the app?

For example.
If you look around the app, you will see we are currently in progress.
If I want to make changes, I stop creating tickets and have Claude plan out the changes I want.
This updates tickets because Claude updates them for me.
Then when I make tickets "Go do ticket 5.1", The ticket viewed is the one claude already planned out.

I wonder if we can do that for our application with our github agents.
When a /refactor is created... it submits a ticket to our refactor agent and production stops.
Then, when the refactor ticket is created all future tickets should now be changed and tickets continue.

Wow!