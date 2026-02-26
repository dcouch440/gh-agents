# Agent Framework Context Management Research

## 1. Anthropic (3 major blog posts)

**[Effective context engineering for AI agents](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)** (Sep 2025) -- The definitive post. Four concrete techniques:

- **Scratchpad/file-based external memory**: "Scratchpads can be implemented as a tool call that simply writes to a file. Models can write to files like NOTES.md or TODO.txt, then reload them later, enabling long-horizon coherence without overloading the context window."
- **Trimming vs. summarization**: Trimming uses hard-coded heuristics (remove older messages). Summarization uses an LLM to distill. Claude Code uses both -- "preserving architectural decisions and unresolved bugs while discarding redundant tool outputs or messages."
- **Tool result eviction**: "Agents should store tool results externally by saving full tool results to the filesystem (not in context) and accessing them on demand with utilities like glob and grep. Newer tool results remain in full." Tool results and definitions can consume 50,000+ tokens before the agent even reads a request.
- **Subagent isolation**: "Each subagent might explore extensively, using tens of thousands of tokens or more, but returns only a condensed, distilled summary of its work (often 1,000-2,000 tokens)."

**[Effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents)** (Nov 2025) -- The initializer + progress file pattern. An initializer agent sets up a `claude-progress.txt` file. Subsequent sessions read this file to understand state. "The key insight was finding a way for agents to quickly understand the state of work when starting with a fresh context window, which is accomplished with the claude-progress.txt file alongside the git history."

**[How we built our multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system)** (Jun 2025) -- Context isolation via subagents. "One of the most effective uses for subagents is isolating operations that produce large amounts of output. By delegating these to a subagent, the verbose output stays in the subagent's context while only the relevant summary returns." Multi-agent outperformed single-agent by **90.2%** on internal evaluations.

**[Prompt caching](https://www.anthropic.com/news/prompt-caching)** -- Cache read tokens cost 0.1x base price (90% discount). Latency reduction up to 85%. System prompt + tool definitions should be a stable prefix.

## 2. OpenAI

**[Agents SDK context management](https://openai.github.io/openai-agents-python/context/)** -- Separates `RunContext` (local structured state, never sent to LLM) from session memory (handles history automatically).

**[Session memory](https://cookbook.openai.com/examples/agents_sdk/session_memory)** -- Two techniques: **context trimming** (drop older turns, keep last N -- deterministic, zero latency) and **context compression** (LLM-based summarization). `OpenAIResponsesCompactionSession` auto-compacts based on `should_trigger_compaction`.

**[Responses API compaction](https://developers.openai.com/api/docs/guides/compaction)** -- First-party API solution. "All prior user messages are kept verbatim, while prior assistant messages, tool calls, tool results, and encrypted reasoning are replaced with a single encrypted compaction item that preserves the model's latent understanding." Set `compact_threshold` for automatic server-side compaction. Not available for Anthropic models.

## 3. LangChain / LangGraph

**[Memory overview](https://docs.langchain.com/oss/python/langgraph/memory)** -- Short-term (thread-scoped message history) vs long-term (cross-thread). Legacy memory types deprecated in favor of LangGraph checkpointing.

Three strategies for short-term memory:
1. **RemoveMessage**: Targeted deletion from graph state. Delete all but last N messages.
2. **trim_messages**: Token-count-based trimming rather than message-count.
3. **LLM-based summarization**: Maintain rolling summary + last 2 messages. Periodically regenerate summary and prune old messages.

Source: [Message Handling and Summarization](https://deepwiki.com/langchain-ai/langchain-academy/5.2-message-handling-and-summarization)

## 4. Google ADK

**[Four-layer context framework](https://google.github.io/adk-docs/sessions/)** -- Working Context (what the LLM sees), Session (durable event log + key-value state), Memory (long-lived searchable knowledge), Artifacts (large data). These layers are NOT all dumped into the prompt -- the framework selects what goes into working context.

**[State as working memory](https://google.github.io/adk-docs/sessions/state/)** -- Key-value store with variable interpolation in agent instructions via `{key}` syntax. Tools modify state via `tool_context.state`. This is structured typed working memory, not message history.

## 5. CrewAI

**[Unified Memory](https://docs.crewai.com/en/concepts/memory)** -- Single `Memory` class using LLM to analyze content when saving (infers scope, categories, importance). Adaptive-depth recall with composite scoring (semantic similarity + recency + importance).

**[Automatic context management](https://docs.crewai.com/en/concepts/agents)** -- `respect_context_window=True` (default) auto-detects context overflow and calls `summarize_messages()` which splits messages into chunks, summarizes each via LLM, replaces with single summary. After each task, extracts discrete facts and stores them -- inter-task context is extracted facts, not raw history.

## 6. AutoGen (Microsoft)

**[BufferedChatCompletionContext](https://github.com/microsoft/autogen/discussions/5006)** -- Sliding window over message history. Simple message count limit.

**[Mem0 integration](https://microsoft.github.io/autogen/0.2/docs/notebooks/agentchat_memory_using_mem0/)** -- Delegates memory management to external service. Long-term, short-term, semantic, episodic memories per user/agent/session.

## 7. MemGPT / Letta

**[Virtual context management](https://arxiv.org/abs/2310.08560)** -- Foundational paper. Treats LLM context like OS physical memory with paging. Three-tier hierarchy: main context (in-window), recall storage (searchable conversation history), archival storage (vector-searchable knowledge). The LLM manages its own memory through tools: `memory_replace`, `memory_insert`, `archival_memory_insert`, `archival_memory_search`, `conversation_search`. "Page faults" trigger retrieval from external storage.

## 8. Manus

**[Context engineering lessons](https://manus.im/blog/Context-Engineering-for-AI-Agents-Lessons-from-Building-Manus)** (Jul 2025) -- Most operationally detailed production lessons:

- **KV-cache hit rate is the #1 metric**. Cached vs uncached: 10x cost difference. Rules: stable prefixes, append-only context, deterministic serialization. "Even a single-token difference can invalidate the cache from that token onward."
- **Three-tier compression**: Raw (keep recent turns full) > Compaction (mechanical reduction) > Summarization (LLM-based, last resort). "If context exceeds 128k tokens, summarize the oldest 20 turns using JSON structure while keeping the last 3 turns raw."
- **File system as extended context**: "Unlimited in size, persistent by nature, and directly operable by the agent itself."
- **Todo.md as attention management**: "By constantly rewriting the todo list, Manus is reciting its objectives into the end of the context. This pushes the global plan into the model's recent attention span, avoiding 'lost-in-the-middle' issues."
- **Tool masking vs. removal**: Mask token logits during decoding rather than changing tool definitions (which would invalidate KV-cache).
- **Error preservation**: Keep errors in context -- "they tell the model what went wrong and implicitly constrain the next action."

## 9. Community (X/Twitter)

Key voices confirming the problem:
- **[@MaryamMiradi](https://x.com/MaryamMiradi/status/1989377220381720873)**: "Your agent starts strong, performs a few tool calls, suddenly gets confused, outputs garbage."
- **[@femke_plantinga](https://x.com/femke_plantinga/status/1991131314511040519)**: "Everyone thinks bigger context windows solve AI's memory problem. They're wrong." Identifies context poisoning, context distraction, context confusion as failure modes.
- **[@IntuitMachine](https://x.com/IntuitMachine/status/1979266898257719564)**: "Context engineering has emerged as a critical discipline, addressing the paradox that while agents require extensive context, their performance degrades as the context grows."

## 10. Synthesis: Recommended Patterns for Our Builder Agent

Seven patterns identified across all sources. For our specific problem (builder agent, 13 tools, 500-3000 token node configs accumulating across turns), the recommended combination:

**1. Subagent isolation** (Anthropic, strongest evidence) -- Already matches our Board Dispatcher / Per-Node Builder architecture. Each node gets a fresh context. The dispatcher receives only summaries, never full configs. Evidence: 90.2% improvement in Anthropic's multi-agent system.

**2. Structured state file** (Anthropic harnesses, Manus, Google ADK) -- The dispatcher maintains a structured JSON/markdown file tracking what has been configured, what remains, and key decisions. Replaces message history as coordination mechanism. Manus variant: todo.md rewritten each turn pushes plan into strongest attention position.

**3. Tool result eviction** (Anthropic, Manus, OpenAI) -- When the builder must process multiple nodes in one context (chat path), evict old tool results after each node. Keep only the last node's results in full. Manus rule: "summarize the oldest N turns while keeping the last 3 raw."

**4. Cache-friendly prompt structure** (Manus, Anthropic) -- System prompt + tool definitions as stable prefix. No timestamps. Deterministic serialization. Append-only. 10x cost reduction on cached tokens.

The combination of **subagent isolation + structured state file** addresses the exponential accumulation at the architecture level. **Tool result eviction + cache optimization** handle the mechanics within any single agent's context.
