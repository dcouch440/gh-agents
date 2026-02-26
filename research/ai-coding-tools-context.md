# How AI Coding Tools Manage Context Windows

Research into how production AI coding tools handle incremental code changes, growing context, and multi-file edits. Focused on mechanical details relevant to the builder agent problem: an agent that receives full node content every message and accumulates history, causing exponential context growth.

---

## 1. Cursor

### Prompt Construction: Full Rebuild Every Turn

Cursor rebuilds the entire prompt sent to the LLM on every user message. The prompt is not incrementally updated -- it is reconstructed from scratch each turn. Static sections (system instructions, tool schemas) are cached via prompt caching to avoid reprocessing unchanged portions. Dynamic sections (user input, selected chat history, code snippets) are rebuilt or retrieved fresh each turn.

Source: [BuildSomethingAI/Cursor-Context-Management](https://github.com/BuildSomethingAI/Cursor-Context-Management/blob/main/README.md)

### Automatic Context Attachment

Each time the user sends a message, Cursor automatically attaches state information without the user requesting it:
- Currently open files
- Cursor position
- Recently viewed files
- Edit history in the current session
- Linter errors

This is "state context" -- the current world state. It is distinct from "intent context" (system prompt, what the user wants).

Source: [How Cursor Works Internally](https://adityarohilla.com/2025/05/08/how-cursor-works-internally/)

### File Reading: 250-Line Chunks

In Agent mode, Cursor reads the first 250 lines of a file by default. It can extend by another 250 lines if needed. Specific searches return a maximum of 100 lines. This is a deliberate trade-off to conserve context length and reduce costs. Even with `should_read_entire_file = True`, the agent still only receives 250 lines unless the file is manually attached.

Community recommendation: keep files under 500 lines so the agent can read the whole file in two attempts. Document function and implementation logic in the first 100 lines.

Source: [Cursor Forum - Read >250 lines in agent mode](https://forum.cursor.com/t/read-250-lines-in-agent-mode/83618)

### Dynamic Context Discovery (December 2025)

Cursor's most significant architectural advancement. The core principle: provide fewer details up front and let the agent pull relevant context on its own.

**The problem it solves:** With static context, every MCP tool description, every skill definition, every file reference is included in the system prompt. As users install more MCP servers, the static context grows linearly, filling the window with potentially irrelevant information.

**The solution -- five techniques:**

1. **MCP Tool Descriptions Synced to Files:** Instead of including all MCP tool descriptions in the system prompt, Cursor syncs them to a folder on disk. The agent receives only tool names as static context. When a task requires a specific tool, the agent reads the tool description file using `grep`, `rg`, or `jq`. This reduced total agent tokens by **46.9%** in an A/B test (statistically significant, high variance based on number of MCPs installed).

2. **Long Tool Outputs Written to Files:** Instead of returning large JSON responses directly into the context, agent outputs are written to files. The agent reads them incrementally using `tail`, `head`, etc. This eliminates unnecessary summarization and preserves data fidelity.

3. **Chat History Files During Summarization:** When context fills and summarization triggers, the pre-summarization history is saved to a file. If the agent needs details lost in summarization, it can read the file.

4. **Agent Skills Standard:** Skills have a name and short description included as static context. The full skill definition is loaded dynamically only when the agent determines it is relevant.

5. **Terminal Sessions as Files:** Terminal output is stored in files rather than injected directly into context.

**The common pattern:** Files as the primary interface for LLM tools. Content is stored on disk and fetched by the agent on demand rather than front-loaded into the context window.

Source: [Cursor Blog - Dynamic Context Discovery](https://cursor.com/blog/dynamic-context-discovery), [InfoQ coverage](https://www.infoq.com/news/2026/01/cursor-dynamic-context-discovery/)

### Summarization on Context Fill

When the context window fills, Cursor triggers a summarization step that gives the agent a fresh context window with a summary of its work so far. Cursor acknowledges this is lossy: "the agent's knowledge can degrade after summarization since it's a lossy compression of the context."

---

## 2. Claude Code

### Auto-Compact: Automatic Conversation Compaction

Claude Code automatically compacts conversation history when approaching context limits. The process:

1. Detects when input tokens exceed a configured threshold (approximately 75-95% of the context window)
2. Analyzes the conversation to identify key information worth preserving
3. Creates a concise summary of previous interactions, decisions, and code changes
4. Replaces old messages with the summary
5. Continues the session with compacted context

Source: [ClaudeLog - What is auto-compact](https://claudelog.com/faqs/what-is-claude-code-auto-compact/)

### The Buffer Problem

Claude Code reserves a significant buffer for the compaction process itself. Until recently, this buffer was **45,000 tokens (22.5% of the 200K window)**. It has since been reduced to approximately **33,000 tokens (16.5%)**. This buffer is consumed before the user writes a single line of code.

The `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE` environment variable (1-100) controls when compaction fires. The `/compact` command allows manual compaction with custom instructions (e.g., "summarize only to-do items").

Source: [Claude Code Context Buffer: The 33K-45K Token Problem](https://claudefa.st/blog/guide/mechanics/context-buffer-management)

### What Gets Preserved vs. Lost

**Preserved:** General shape of conversation, topics discussed, broad conclusions, overall direction of work.

**Lost:** Exact numbers, specific code snippets, precise variable names, nuanced reasoning chains, carefully worded constraints. Example: "API endpoint accepts max 512KB payload with 30-second timeout and specific header format" might become "discussed API constraints" in the summary.

**Mitigation:** Use CLAUDE.md files to inject fundamental requirements every session. Core app features, tech stacks, project notes, and constraints live there and are never compacted away because they are part of the static system context.

Source: [How Claude Code Got Better by Protecting More Context](https://hyperdev.matsuoka.com/p/how-claude-code-got-better-by-protecting), [Claude Compaction API Docs](https://platform.claude.com/docs/en/build-with-claude/compaction)

### Compaction API (for custom implementations)

The `compact-2026-01-12` beta header enables compaction in raw API calls. When enabled:
- Claude detects when input tokens exceed the trigger threshold
- Generates a summary of the current conversation
- Returns a compaction block containing the summary
- Subsequent turns use the compacted context

You can customize what gets preserved: `"Focus on preserving code snippets, variable names, and technical decisions."`

Source: [Anthropic Compaction API Docs](https://platform.claude.com/docs/en/build-with-claude/compaction), [Automatic context compaction cookbook](https://platform.claude.com/cookbook/tool-use-automatic-context-compaction)

### File Re-reading Strategy

Claude Code does not maintain a persistent in-memory file cache. It re-reads files when needed using tool calls (`Read`, `Bash cat`, etc.). Each read is a fresh fetch from disk. This means files are always current (no stale cache), each read consumes context tokens, and the agent decides when to re-read based on its own judgment.

---

## 3. Aider

### The Repo Map: Compressed Codebase Awareness

Aider's core innovation. A concise map of the entire git repository that includes the most important classes and functions along with their types and call signatures. Sent to the LLM with every change request.

**What it contains:**
- List of files in the repo
- Key symbols defined in each file
- Critical lines of code for each definition (signatures, not implementations)

**What it does NOT contain:**
- Function bodies
- Comments
- Import statements (unless they define symbols)
- File contents beyond signatures

Source: [Aider - Repository map](https://aider.chat/docs/repomap.html)

### Tree-Sitter for Symbol Extraction

Aider uses tree-sitter to parse source code into Abstract Syntax Trees (ASTs). From the AST, it extracts function definitions and signatures, class definitions, type definitions, variable declarations, and method signatures. Tree-sitter supports language-specific grammars, so the extraction is syntax-aware. This replaced an earlier ctags-based approach.

Source: [Building a better repository map with tree sitter](https://aider.chat/2023/10/22/repomap.html)

### PageRank-Based Ranking

Not all symbols are equally important. Aider uses a graph-based ranking algorithm:

1. **Build a graph:** Each source file is a node. Edges connect files that have dependencies (file A references a symbol defined in file B).
2. **Build it with NetworkX:** The `RepoMap` class builds a `NetworkX MultiDiGraph` of file relationships.
3. **Run PageRank with personalization:** Ranks nodes using PageRank. Personalization biases the ranking toward files that are already in the chat (files the user has added for editing).
4. **Select top-ranked definitions:** Formats the highest-ranked definitions into a token-limited string.

The result: symbols that are most referenced across the codebase rank highest. These are the "important identifiers" -- APIs, shared types, core functions.

Source: [DeepWiki - Repository Mapping](https://deepwiki.com/Aider-AI/aider/4.1-repository-mapping)

### Token Budget and Binary Search

The `--map-tokens` flag controls the repo map budget (default: **1,024 tokens**). The `get_ranked_tags_map()` method uses **binary search** to find the maximum number of tags that fit:

- Binary search starts with `middle = min(max_map_tokens // 25, num_tags)`
- Targets output within **15% of max_map_tokens**
- Token counting uses sampling for efficiency: texts under 200 characters use exact counting; longer texts sample every Nth line and estimate

The map size is **dynamic** -- Aider expands it significantly when no files have been added to the chat, to give the LLM maximum codebase awareness. When files are added, the map shrinks because the LLM has direct file content.

Source: [DeepWiki - Repository Mapping](https://deepwiki.com/Aider-AI/aider/4.1-repository-mapping)

### Chat History Summarization

Aider automatically summarizes chat history to avoid exhausting the context window. Key details:

- Uses the "weak model" (configurable via `--weak-model`) for summarization
- Triggers when chat history exceeds a soft token limit
- Above approximately **25,000 tokens of context**, most models "start to become distracted and become less likely to conform to their system prompt"
- Users can also `/clear` to manually reset history

Source: [Aider FAQ](https://aider.chat/docs/faq.html)

### Edit Formats: Minimizing Output Tokens

Aider uses different "edit formats" to minimize the tokens the LLM must produce:

- **"Whole" format:** LLM returns the complete updated file. Simple but expensive.
- **"Diff" format (search/replace blocks):** LLM specifies edits as search/replace pairs. Far fewer output tokens.
- **"Unified diff" format:** Standard unified diff format. Made GPT-4 Turbo "3X less lazy" (produced more complete edits).

Source: [Aider - Edit formats](https://aider.chat/docs/more/edit-formats.html)

---

## 4. GitHub Copilot Workspace

### Spec-Plan-Implement Pipeline

Copilot Workspace decomposes every task into three explicit, human-editable phases:

**Phase 1 -- Specification:**
- Generates two bullet-point lists: "current state of the codebase" and "desired state after the change"
- Identifies relevant files using a combination of LLM techniques and traditional code search
- The contents of the highest-ranked files are used as context for all subsequent steps
- If resulting context is too large, only the most relevant parts are kept

**Phase 2 -- Plan:**
- Generates a list of files to modify (edit, create, delete, move, rename)
- Each file gets a list of specific steps describing exact changes needed
- Human-editable: the user can modify the plan before implementation

**Phase 3 -- Implementation:**
- Generates updated file contents one by one
- Renders diff views for each file
- Diffs are editable

Source: [GitHub Next - Copilot Workspace](https://githubnext.com/projects/copilot-workspace), [Copilot Workspace User Manual](https://github.com/githubnext/copilot-workspace-user-manual/blob/main/overview.md)

### Context Window Strategy

Uses a combination of LLM-based relevance scoring and traditional code search to identify which files matter. Ranks files by relevance; top files become the context for all phases. If context exceeds the window, it is truncated to the most relevant portions. At 95% of token limit, Copilot automatically compresses history.

---

## 5. Continue.dev

### Context Provider Architecture

Continue uses a pluggable "context provider" system. Users type `@` to see a dropdown of available context sources. Each provider retrieves and injects specific context into the prompt.

**Built-in providers:** `@file`, `@code` (functions/classes), `@diff` (branch changes), `@open` (current file), `@terminal` (last command + output), `@docs`, `@codebase` (embeddings search).

**Custom providers:** `HttpContextProvider` (POST to external URLs), MCP context providers (any Model Context Protocol server).

The key architectural choice: context is **user-directed** (explicit `@` references) rather than automatically attached.

Source: [Continue Docs - Context Providers](https://docs.continue.dev/customize/deep-dives/custom-providers)

---

## 6. Sourcegraph Cody

### Multi-Layered Context Architecture

Context is organized into layers cached separately:

**Layer 1 -- Perma-layer:** Repository-level metadata, architectural patterns. Cached and prefetched.

**Layer 2 -- Action History (append-only):** Files opened, navigation actions, edits. Periodically cached because the append-only nature means older actions never change.

**Layer 3 -- Long-Range Retrievers (dynamic):** Code search results, distant function definitions, cross-repo references. Small but critical, fresh each request.

Source: [Sourcegraph Blog - Toward infinite context for code](https://sourcegraph.com/blog/towards-infinite-context-for-code)

### Two-Stage Retrieval Pipeline

**Stage 1 -- Retrieval (recall-optimized):** Sparse vector search (keyword + LLM-enhanced ranking) combined with dense vector retrieval (embeddings). Complementary sources retrieving distinct sets.

**Stage 2 -- Ranking (precision-optimized):** Expand and Refine method on the Repo-level Semantic Graph. Graph expansion + link prediction.

**Prefetch + Cache:** Using Gemini 1M-token models, prefetching dropped time-to-first-token from **30-40 seconds to ~5 seconds** for ~1MB context.

Source: [Sourcegraph Blog - How Cody understands your codebase](https://sourcegraph.com/blog/how-cody-understands-your-codebase)

---

## 7. Additional Systems

### Augment Code
Full-repository indexing (400K+ files) with semantic embeddings. 200K token context window. Persistent "Memories" across conversations.

Source: [How Augment Code Solved the Large Codebase Problem](https://blog.codacy.com/ai-giants-how-augment-code-solved-the-large-codebase-problem)

### Goose (Block)
Auto-compact at 80% context usage (configurable). `.goosehints` for persistent instructions. Memory extension for cross-session state.

Source: [Goose - Understanding Context Windows](https://block.github.io/goose/blog/2025/08/18/understanding-context-windows/)

### Devin
Self-aware context management -- proactively summarizes as it approaches limits. No long-term memory across sessions. "Session Insights" for inter-session learning.

Source: [Cognition - Rebuilding Devin for Claude Sonnet 4.5](https://cognition.ai/blog/devin-sonnet-4-5-lessons-and-challenges)

---

## 8. Key Research

### JetBrains: "The Complexity Trap" (NeurIPS 2025)

**Observation masking** replaces tool outputs with placeholders after the agent processes them, while preserving full action and reasoning history. A typical agent turn heavily skews toward observation (file contents, grep output, build errors).

**Results (SWE-bench Verified, 5 model configs):** Halves cost vs raw agent. With Qwen3-Coder 480B: **52% cheaper AND +2.6% solve rate**. LLM summarization could not consistently outperform simpler masking. Hybrid approach saves an additional 7-11%.

Source: [JetBrains Research Blog](https://blog.jetbrains.com/research/2025/12/efficient-context-management/), [arXiv](https://arxiv.org/abs/2508.21433)

### Anthropic: "Effective Harnesses for Long-Running Agents"

Two-agent pattern: initializer (sets up progress file, git baseline, feature list) + coding agent (reads progress, picks next feature, commits, updates progress). The progress file + git history let the agent understand state with a fresh context window.

Source: [Anthropic Engineering](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents)

### Factory.ai: Anchored Iterative Summarization

Persistent structured summary with explicit sections (session intent, file modifications, decisions, next steps). Only newly dropped spans are summarized and merged into the persistent summary. Scored **3.70 vs 3.44 (Anthropic) and 3.35 (OpenAI)** on 36K production messages.

Source: [Factory.ai - Compressing Context](https://factory.ai/news/compressing-context)

### Martin Fowler: Context Engineering for Coding Agents

Subtasking: each subtask gets its own context window. Multi-agent with isolated contexts outperforms single-agent because each subagent window is allocated to a narrower task.

Source: [Martin Fowler - Context Engineering for Coding Agents](https://martinfowler.com/articles/exploring-gen-ai/context-engineering-coding-agents.html)

---

## 9. Patterns Across All Tools

| Pattern | Tools Using It | Mechanism |
|---------|---------------|-----------|
| Never front-load full content | Cursor, Aider, Cody, Copilot | Read limits, structural maps, file ranking |
| Compressed structural awareness | Aider, Cody, Cursor | Repo map, semantic graph, search index |
| Demand-pull over supply-push | Cursor, Cody, Claude Code | Agent requests content; not provided upfront |
| Observation masking > summarization | JetBrains research | Replace tool outputs with placeholders after processing |
| Structured persistent state | Anthropic, Factory, Augment, Goose | Progress files, anchored summaries, memories |
| Layered caching | Cody, Cursor | Static/semi-static/dynamic layers cached separately |

---

## 10. Application to the Builder Agent Problem

Strategies ordered by likely impact:

**A. Observation Masking:** After the builder processes tool output, mask it in subsequent turns. Keep reasoning and actions. Expected ~50% cost reduction (JetBrains numbers).

**B. Demand-Pull Node Content:** Send structural summary, not full content. Let builder request specific nodes. No new tools needed -- dispatch instruction includes a manifest.

**C. Anchored Iterative Summarization:** Structured summary with explicit sections (task intent, nodes modified, decisions, remaining work). Merge new spans rather than regenerating.

**D. Compressed Structural Awareness:** Build a "node map" -- workflow DAG with names, types, connections, key attributes -- not full system prompts. Entire workflow in ~500 tokens.

**E. Subtasking with Isolated Context:** Each node gets its own builder invocation with fresh context. Dispatcher provides only relevant node content + neighbor summaries.

**F. Protected Context Layer:** Define information that survives all compaction: topology, beliefs, constraints. Analogous to CLAUDE.md.
