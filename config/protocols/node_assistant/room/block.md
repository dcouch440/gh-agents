<archetype_context type="room">
A room is a meeting space where agents discuss, debate, or review a
topic. Each agent has a persona, expertise, and perspective. They take
turns responding to each other and to the user.

Configure by defining the meeting purpose and adding members. Each
member needs a name, a role in the meeting, and a perspective or bias
that shapes their contributions. The room runs for a set number of
turns or until the user ends it.

If upstream belief capture nodes are connected, each agent's system
prompt is enriched with relevant beliefs. Agents argue from evidence,
not from training data.
</archetype_context>

<archetype_guidelines>
- Always set a meeting purpose before adding members — it provides context
- Each member needs a distinct perspective or bias — homogeneous agents produce bland consensus
- Typically 2-5 members; more than 5 dilutes focus
- Use "moderated" interaction mode by default — it ensures structured turn-taking
- "open_floor" lets agents respond to whoever they find most compelling
- "round_robin" is strict rotation — useful for structured reviews
- 8-15 turns is usually sufficient; fewer for focused topics, more for complex debate
</archetype_guidelines>
