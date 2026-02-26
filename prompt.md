# Research: Context Management for LLM Builder Agents with Incremental Changes

## The Problem We're Solving

We have a builder agent that configures AI workflow nodes. When a user makes changes to their workflow (editing node descriptions, rewiring connections, adding annotations), the builder agent needs to understand what changed and update the node's configuration accordingly.

Right now the builder receives the full node content on every message, and it accumulates conversation history. Context grows exponentially. The builder also has 13 tools already, so adding more tools (like "read_config" helpers) makes tool selection harder — we just researched this exact problem.

We need a pattern for delivering incremental changes to an LLM agent that:
- Keeps per-turn context small and focused
- Doesn't require the agent to re-read the full document every turn
- Works without adding tools to an already tool-heavy agent (13 tools)
- Preserves enough context for the agent to make good decisions about config changes

## The Specific Constraints

- The builder agent has 13 tools. Adding read/fetch tools is undesirable.
- Node configs include: system prompt, tool list, agent roster, routing rules, annotations. Moderate size (500-3000 tokens per node).
- The builder may work on the same node across multiple turns as the user iterates.
- A board serializer already produces structural diffs (new/updated/deleted nodes, before/after text). We have the raw diff data.
- We have access to cheap fast models (Grok fast, Haiku) for preprocessing.

## What We Need Researched

### How do AI coding tools handle this?
This is essentially the same problem code editors solve — an AI agent needs to understand a codebase but can't hold it all in context. Research how these systems manage incremental context:

- **Cursor** — how does it decide what file context to include per edit? How does it handle multi-file changes? What goes in the system prompt vs the user message?
- **Claude Code** — how does it manage context across a session? When does it re-read files vs rely on cached understanding? How does autocompact work?
- **Aider** — it uses a "repo map" concept. How does that work? What's the compression ratio? How does it decide what to include?
- **GitHub Copilot Workspace** — how does it plan and execute multi-file changes with limited context?
- **Continue.dev, Cody, other AI code tools** — any novel approaches to context management?

Search X (via Grok) for developer discussions about context window management in AI coding tools — what patterns are people finding work well, what breaks down?

### How do agent frameworks handle growing context?
- **Anthropic's guidance** — any published patterns for multi-turn agent context management?
- **OpenAI's agent patterns** — how do they recommend handling long-running agents?
- **LangChain/LangGraph** — memory management patterns, conversation summarization, sliding windows
- **Google ADK** — any context management primitives?

Search X for discussions about "agent memory", "context window management LLM", "conversation summarization agents" — real experience reports from builders.

### Diff-in-prompt patterns
- Are there established patterns for showing before/after diffs directly in LLM prompts?
- How do code review bots (like PR review agents) present diffs to models?
- What format works best — unified diff, side-by-side, annotated, or just "here's the new version"?
- Does showing a diff actually help the model vs just showing the current state with highlights?
- Any research on LLM comprehension of different diff formats?

### Summarization-as-dispatch
- Using a cheap model as a preprocessor to summarize changes before the builder sees them
- The summarizer reads the full context, produces a focused instruction, builder gets just that
- How do multi-agent systems handle the handoff — what gets summarized, what gets passed verbatim?
- What's the information loss? How do you detect when the summarizer drops something important?
- Are there "summarize then verify" patterns where the agent can request more context if the summary is insufficient?

## Deliverable

A detailed report with specific findings, links, and quotes. For each approach found, explain:
1. How it works mechanically (not just "it uses embeddings" — explain the actual flow)
2. What tradeoffs it makes (what information is lost, what's preserved)
3. Whether it applies to our case (moderate-size configs, 13 tools, incremental edits)

End with a concrete recommendation: given our constraints, what's the simplest pattern that solves the exponential context growth while keeping the builder effective?
