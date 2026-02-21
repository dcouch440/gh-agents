# Inter-Agent Messaging & System Notifications — Research

> Research on how LLM agent frameworks handle message formatting between
> agents, system notifications, and mid-conversation injection patterns.
> Conducted Feb 2026 to inform Nexor's manager node dispatch chain.

---

## The Problem

Nexor's manager node creates a 4-layer dispatch chain:

```
L1 (Manager Assistant) → L2 (Manager's Builder) → L3 (Node Assistant) → L4 (Node's Builder)
```

Each layer dispatches async (fire-and-forget). When a child task completes,
the parent needs to be "woken up" via an injected message. Three distinct
message types must coexist in a single conversation:

1. **User messages** — actual human input
2. **System notifications** — "Task complete", "State changed"
3. **Agent messages** — instructions/reports from other layers

The LLM must distinguish all three and respond appropriately to each.

---

## API Constraints (Claude/Anthropic)

| Constraint | Implication |
|-----------|-------------|
| System prompt is top-only | Cannot inject new system instructions mid-conversation |
| Messages alternate user/assistant | No third-party "agent" role available |
| No `name` field on messages | Unlike OpenAI, can't tag message source in metadata |
| Prefilling deprecated on Opus/Sonnet 4.x | Cannot pre-seed assistant responses |

**Bottom line**: All injected messages (notifications, agent messages) MUST
use the `user` role. Differentiation happens at the **content level** using
structural markers the LLM is trained to recognize.

---

## Industry Survey

### AutoGen (Microsoft)

**Approach**: Relied on OpenAI's `name` field on messages to identify speakers.

```json
{ "role": "user", "name": "SecurityAuditor", "content": "The scan found 3 vulnerabilities..." }
```

**Problem**: The `name` field is OpenAI-only. Not supported by Anthropic,
Mistral, Groq, or local models. AutoGen GitHub Issue #2989 explored
alternatives:

- **Content prefix**: `"SecurityAuditor said:\n The scan found..."` —
  LLM learns the format and reproduces it in responses, requiring
  post-processing to strip prefixes.
- **Resolution** (June 2025): Kept `name` field, delegated handling to
  individual client implementations. No universal solution.

**Takeaway**: Content prefixes cause format leakage. The LLM mimics the
pattern in its own output.

### OpenAI Swarm

**Approach**: No inter-agent message formatting at all. On handoff:

- System prompt swaps entirely to the new agent's instructions
- Chat history persists but only active agent's system prompt is present
- Context passed via explicit `context_variables` dict, not message content

**Handoff history**: When `nest_handoff_history` is enabled, prior
conversation is collapsed into a `<CONVERSATION HISTORY>` XML block.
Wrapper text is customizable via `set_conversation_history_wrappers()`.

**Takeaway**: Swarm avoids the problem by swapping agents entirely rather
than having multiple agents share a conversation. Uses XML for history blocks.

### OpenAI Agents SDK

**Approach**: Handoffs collapse prior transcript into a summary message
wrapped in `<CONVERSATION HISTORY>` tags, injected as an `assistant` role
message (not `user` or `system`).

**Actual source code** (`src/agents/handoffs/history.py`):

```python
_DEFAULT_CONVERSATION_HISTORY_START = "<CONVERSATION HISTORY>"
_DEFAULT_CONVERSATION_HISTORY_END = "</CONVERSATION HISTORY>"

def _build_summary_message(transcript):
    summary_lines = [
        f"{idx + 1}. {_format_transcript_item(item)}"
        for idx, item in enumerate(transcript_copy)
    ]
    start_marker, end_marker = get_conversation_history_wrappers()
    content = "\n".join([
        "For context, here is the conversation so far between the user and the previous agent:",
        start_marker, *summary_lines, end_marker,
    ])
    return {"role": "assistant", "content": content}
```

**What the injected message looks like**:

```
For context, here is the conversation so far between the user and the previous agent:
<CONVERSATION HISTORY>
1. user: I need help with my billing
2. assistant: I'd be happy to help. Let me transfer you to our billing specialist.
3. function_call: {"name": "transfer_to_billing", "arguments": "{}"}
4. function_call_output: {"result": "transferred"}
</CONVERSATION HISTORY>
```

Key decisions: uses `assistant` role (avoids user/assistant alternation issues),
numbered lines with role prefixes, XML tags as parseable delimiters.
Wrappers customizable via `set_conversation_history_wrappers()`.

**Takeaway**: XML tags used for structural boundaries. Numbered role-prefixed
lines give receiving agent structured context about who said what.

### CrewAI

**Approach**: Tool-mediated delegation. Inter-agent messages arrive as
tool call inputs, not injected conversation messages.

**Literal delegation tool prompt** (from `translations/en.json`):

```
Delegate a specific task to one of the following coworkers: {coworkers}
The input to this tool should be the coworker, the task you want them to do,
and ALL necessary context to execute the task, they know nothing about the
task, so share absolutely everything you know, don't reference things but
instead explain them.
```

**Conversation history injection**:

```
You are a member of a crew collaborating to achieve a common goal. Your task
is a specific action that contributes to this larger objective. For additional
context, please review the conversation history between you and the user that
led to the initiation of this crew.
```

Key decisions: context from other agents arrives as structured tool call
inputs (`coworker`, `task`, `context` fields). System prompt set once per
agent invocation — no mid-conversation system notifications.

**Takeaway**: Tool-mediated delegation avoids the message formatting problem
entirely. But doesn't solve async notifications.

### LangGraph

**Approach**: `SystemMessage` prepended per turn, not accumulated in history.

```python
def agent_node(state):
    system_msg = SystemMessage(content="You are a helpful research assistant.")
    messages = [system_msg] + state["messages"]
    response = model.invoke(messages)
    return {"messages": [response]}
```

For notifications, LangGraph uses graph state — a node writes to shared
state, the next node reads it and constructs messages. No built-in
"inject notification between turns" primitive.

**Caveat**: `SystemMessage` objects added mid-conversation were sometimes
ignored by downstream agents that only saw `HumanMessage` types. Recommendation:
prepend fresh system messages at each node invocation rather than accumulating.

**Takeaway**: State-based, not message-based. Notifications are implicit
via state changes, not explicit injected messages.

### Google ADK

**Approach**: Template variable resolution. State values injected via
`{key}` placeholders resolved on each model call.

```python
instruction = "You are helping user {user_name}. Current order status: {order_status}"
# ADK resolves from session.state before each LLM call
```

State modifications are event-driven through `EventActions.state_delta`.
No separate "notification message" — state changes reflected in the next
prompt resolution automatically.

**Takeaway**: Same philosophy as our board_state approach — state IS the
notification. No explicit messages needed if the system prompt is rebuilt
each turn with fresh state.

### Claude Code (Anthropic's own product)

**Approach**: Uses `<system-reminder>` XML tags injected as `user` role
messages throughout the conversation.

```xml
<system-reminder>
Agent a6b365a progress: 5 new tools used, 14505 new tokens.
The agent is still running.
</system-reminder>
```

Key characteristics:
- ~40 different system reminder types (task status, plan mode, todo changes, etc.)
- Injected at multiple points: tool results, between messages, after tool calls
- Self-documenting via consistent tag name
- Claude trained to treat these as system-level, not user-level
- **Reinforcement**: system prompt defines behavior, tags reinforce mid-conversation

**Takeaway**: XML tags work at production scale. Anthropic uses this pattern
in their flagship developer product. The LLM does not reproduce or confuse
these with user messages.

---

## Anthropic's Published Guidance

### Building Effective Agents (Dec 2024)

> "Designing good prompts turned out to be the single most important way
> to guide how the agents behaved."

Recommends orchestrator-worker pattern. Each subagent needs:
- An objective
- An output format
- Guidance on tools and sources
- Clear task boundaries

No specific message formatting guidance published.

### Effective Context Engineering (2025)

Recommends organizing prompts into "distinct sections" using:
- XML tags (`<background_information>`, `<instructions>`)
- Markdown headers
- Clear delineation between sections

Focus on "the smallest set of high-signal tokens that maximize the
likelihood of some desired outcome."

### Multi-Agent Research System (2025)

Orchestrator-worker with parallel subagents. No published details on
inter-agent message formatting. Architecture-level guidance only.

---

## Taxonomy of Injection Strategies

| Strategy | Role Used | Marker Format | Frameworks |
|----------|-----------|---------------|------------|
| XML tags in user content | `user` | `<system-reminder>` | Claude Code |
| XML tags in assistant message | `assistant` | `<CONVERSATION HISTORY>` | OpenAI Agents SDK |
| Structured tool inputs | `tool` result | JSON fields | CrewAI |
| Prepended SystemMessage | `system` | LangChain object | LangGraph |
| Dynamic system_message update | `system` (rebuilt) | Updated prompt string | AutoGen |
| Template variable resolution | `system` (resolved) | `{variable}` placeholders | Google ADK |
| `developer` role messages | `developer` | Plain text | OpenAI o1+ models |

### Nexor Constraint

Our `Role` enum is currently `{ User, Assistant }` only (see `src/llm/types/mod.rs:37`).
System prompt is a separate field on `LLMRequest`, not a message. This matches
the Anthropic API exactly — no mid-conversation system role available.

---

## Key Insight: Nobody Has a Standard

No major framework has published a universal inter-agent message format.
The closest things are:
- OpenAI's `name` field (not portable)
- Claude Code's `<system-reminder>` (production-proven but undocumented as a pattern)
- OpenAI Agents SDK's `<CONVERSATION HISTORY>` (handoff-specific)

Everyone rolls their own. XML tags emerge as the most common structural
marker across frameworks.

---

## Recommendation for Nexor

### Use XML Tags with Distinct Tag Names

XML is the right choice because:
1. Already our system prompt pattern (XML throughout all prompts)
2. Claude trained to parse XML structurally — won't reproduce tags in responses
3. Distinct tag names prevent confusion between message types
4. Type attributes provide machine-parseable metadata
5. Production-proven by Claude Code at scale

### Proposed Message Types

```xml
<!-- Type 1: System notification — task lifecycle events -->
<notification type="task_complete" task_id="abc123">
  Configure Collector: DONE
</notification>

<!-- Type 2: Agent message — from another layer's agent -->
<agent_message from="Collector" node_id="nd-1f2a" layer="3">
  Configured for Acme + Widget. Need scraping URLs to finalize.
</agent_message>

<!-- Type 3: Dispatch result — builder completion report -->
<dispatch_result dispatch_id="d-7890" status="complete">
  Created 3-node pipeline. Dispatched initial instructions to all nodes.
</dispatch_result>
```

All three injected as `user` role messages. The XML tag itself tells the
LLM what kind of message it is.

### Reinforcement Strategy

Based on Claude Code's `<system-reminder>` pattern and AutoGen's format
leakage problem, reinforcement should happen at multiple layers:

**1. Top of system prompt — Define message types**
```xml
<message_types>
You receive messages from multiple sources in this conversation:

- **User messages**: Direct input from the human user. Always respond
  conversationally.
- **<notification>**: System events (task complete, state changed).
  Process silently unless the user should know.
- **<agent_message>**: Reports from other agents in the system.
  Incorporate their information into your awareness.
- **<dispatch_result>**: Completion reports from background work you
  initiated. Review and decide next action.

Never reproduce these XML tags in your responses. They are system-level
markers, not conversation format.
</message_types>
```

**2. Bottom of system prompt (last lines) — Brief reminder**
```xml
<reminder>
When you see <notification>, <agent_message>, or <dispatch_result> tags,
these are system-injected updates — not user messages. Read the board_state
for current truth. Respond to the user naturally.
</reminder>
```

**3. The tags themselves are self-documenting**
Attributes like `type=`, `from=`, `layer=` provide context without needing
the system prompt to explain every variant.

### Why Not Plain Prefixes?

AutoGen's experience with `"AgentName said:\n ..."` showed that LLMs
learn and reproduce content prefixes. The assistant starts producing
`[SYSTEM: ...]` in its own responses, requiring post-processing.

XML tags don't have this problem because Claude treats them as structural
markup, not conversational patterns to mimic.

---

## Open Questions

1. **Tag nesting**: Should `<dispatch_result>` contain `<agent_message>`
   sub-elements for rolled-up multi-node reports? Or keep flat?

2. **Verbosity vs signal**: How much detail goes in the tag content vs
   letting the agent read `<board_state>` for truth? The tag might just
   be a signal ("something changed") with minimal content.

3. **Rate of injection**: If 5 nodes complete in quick succession, does
   the manager get 5 separate `<notification>` messages or one batched
   update? Batching reduces noise but adds latency.

4. **Testing**: Need to verify Claude doesn't reproduce XML tags in
   responses with our specific tag names. May need prompt-level
   reinforcement like "Never output <notification> tags."

---

## Sources

- [AutoGen: Use of "name" field for messages (Issue #2989)](https://github.com/microsoft/autogen/issues/2989)
- [AutoGen: Transform to add agent's name into message content (PR #3334)](https://github.com/microsoft/autogen/pull/3334)
- [AutoGen 0.4 ConversableAgent Reference](https://microsoft.github.io/autogen/0.2/docs/reference/agentchat/conversable_agent/)
- [OpenAI Swarm: README](https://github.com/openai/swarm/blob/main/README.md)
- [OpenAI Agents SDK: Handoffs](https://openai.github.io/openai-agents-python/handoffs/)
- [OpenAI Agents SDK: Source (handoffs/history.py)](https://github.com/openai/openai-agents-python)
- [CrewAI: translations/en.json](https://github.com/crewAIInc/crewAI/blob/main/src/crewai/translations/en.json)
- [CrewAI: Customizing Prompts](https://docs.crewai.com/en/guides/advanced/customizing-prompts)
- [LangGraph: SystemMessage Discussion (Issue #635)](https://github.com/langchain-ai/langgraph/discussions/635)
- [Google ADK: State Documentation](https://google.github.io/adk-docs/sessions/state/)
- [MCP: Server-Injected System Prompts (Issue #148)](https://github.com/modelcontextprotocol/specification/issues/148)
- [Anthropic: Building Effective Agents](https://www.anthropic.com/research/building-effective-agents)
- [Anthropic: Effective Context Engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)
- [Anthropic: Multi-Agent Research System](https://www.anthropic.com/engineering/multi-agent-research-system)
- [Claude Code system-reminder patterns](https://github.com/Piebald-AI/claude-code-system-prompts)
- [Anthropic: Messages API](https://docs.anthropic.com/en/api/messages)
