# Diff-in-Prompt Patterns: How LLMs Understand Changes

Research into how language models comprehend diffs, how production systems present before/after changes to models, and what formats work best for showing "what changed" in multi-turn agent interactions.

**Problem context:** A builder agent needs to understand what changed in a node's configuration (system prompt, tools, routing rules, description text) without re-reading the full config every turn.

---

## 1. Diff Formats in Prompts: What LLMs Actually Understand

### The Core Finding: Format Choice Matters Enormously

Aider's benchmarks demonstrate that edit format selection alone can swing performance dramatically. GPT-4 Turbo scored 20% on a coding benchmark using search/replace blocks but 61% using unified diffs -- a 3x improvement with zero model changes. However, GPT-3.5 scored only 19% with that same unified diff format because it could not reliably produce valid diffs. The format must match the model's capabilities.

Sources:
- [Unified diffs make GPT-4 Turbo 3X less lazy](https://aider.chat/docs/unified-diffs.html)
- [Aider edit formats](https://aider.chat/docs/more/edit-formats.html)
- [The Harness Problem](https://blog.can.ac/2026/02/12/the-harness-problem/)

### Aider's Four Design Principles for Edit Formats

Aider arrived at four principles through extensive benchmarking:

1. **FAMILIAR** -- Choose a format the model has seen heavily in training data. Unified diff syntax appears in millions of GitHub commits, READMEs, and Stack Overflow posts. The model has been extensively trained to generate conforming text.

2. **SIMPLE** -- Avoid escaping, syntactic overhead, and brittle specifiers like line numbers or line counts. GPT is "terrible at working with source code line numbers" -- this is backed by quantitative benchmark experiments.

3. **HIGH LEVEL** -- Encourage the model to structure edits as new versions of substantive code blocks (functions, methods), not surgical minimal changes to individual lines.

4. **FLEXIBLE** -- Be maximally flexible when interpreting the model's output. LLMs will produce nearly-correct but not perfectly-formatted diffs.

The key psychological insight: "With unified diffs, GPT acts more like it's writing textual data intended to be read by a program. Diffs are usually consumed by the `patch` program, which is fairly rigid, and this seems to encourage rigor -- making GPT less likely to leave informal editing instructions in comments or be lazy about writing all the needed code."

Source: [Aider unified diffs documentation](https://aider.chat/docs/unified-diffs.html)

### The Diff-XYZ Benchmark (JetBrains, NeurIPS 2025)

The first systematic benchmark for evaluating how LLMs understand diffs. Three tasks tested on 1,000 real-world code edits from CommitPackFT:

- **Apply**: old code + diff -> new code
- **Anti-apply**: new code - diff -> old code (reverse the diff)
- **Diff generation**: old code + new code -> produce the diff

**Key results:**
- **Search-replace format performs best for larger models** across most tasks
- **Structured udiff variants** offer similar but slightly weaker performance
- **Smaller open models benefit little from any formatting choice** -- the model must be capable enough to leverage the format
- For **diff generation**, search-replace is a strong default for larger models, while udiff-l (line-tagged unified diff) works better for smaller models
- For **apply/anti-apply**, search-replace *underperforms* more structured formats -- "highlighting a trade-off between ease of generating edits and faithfulness of application"

**Format variants tested:**
- `udiff` -- standard unified diff
- `udiff-h` -- avoids committing to exact line numbers before the hunk body is produced
- `udiff-l` -- replaces single-character `+`/`-` markers with explicit tags, reducing ambiguity
- `search-replace` -- find/replace blocks

The takeaway: **no single diff format dominates across all models and use cases**. The optimal format depends on the model size and the task (reading diffs vs. producing diffs vs. applying diffs).

Sources:
- [Diff-XYZ: A Benchmark for Evaluating Diff Understanding](https://arxiv.org/abs/2510.12487)
- [HuggingFace dataset](https://huggingface.co/datasets/JetBrains-Research/diff-xyz)

### Line Numbers Are Poison

Multiple independent sources converge on this: **LLMs are bad with line numbers**.

- Aider: "GPT is terrible at working with source code line numbers" -- backed by many quantitative experiments
- LLMs generate diffs with incorrect hunk header line numbers; the modified lines are correct but headers don't align, causing `patch` failures
- The `ln-diff` project was specifically created to address this, using a "line-numbered patch format" designed for LLM attention patterns
- OpenAI's `apply_patch` format uses `@@` hunk headers but the content-based matching is more important than the line numbers
- The `udiff-h` variant from Diff-XYZ specifically avoids committing to line numbers before the hunk body, improving generation quality

**Implication for our builder agent:** If showing diffs of configuration changes, never use line-number-based addressing. Use content-anchored formats (search-replace, before/after blocks) instead.

Sources:
- [Context Over Line Numbers](https://medium.com/@surajpotnuru/context-over-line-numbers-a-robust-way-to-apply-llm-code-diffs-eb239e56283f)
- [ln-diff](https://github.com/dceluis/ln-diff)

### What About Non-Code Content?

Most research focuses on code diffs. Our builder agent deals with configuration diffs -- system prompts (natural language), tool lists (structured), routing rules (structured). This is closer to "document editing" than "code editing."

The `chopdiff` library (Joshua Levy) addresses this gap for LLM applications: diff filtering, text mapping, and windowed transforms for documents. Key pattern: diff two versions of a document, then filter the diff to only accept changes that match specific criteria. This allows controlled incremental changes.

For natural language diffs, the before/after pattern with explicit change markers may work better than unified diff syntax, since the model has less training data for "diffs of English prose" than for "diffs of code."

Source: [chopdiff](https://github.com/jlevy/chopdiff)

---

## 2. How PR Review Bots Present Diffs to Models

### CodeRabbit: 1:1 Code-to-Context Ratio

CodeRabbit's core architectural decision: **for every line of code under review, feed the LLM an equal weight of surrounding context**. This 1:1 ratio includes:

- User intent (PR description, linked Jira tickets)
- File dependencies (code graph)
- Past PRs and learned patterns (stored in LanceDB)
- Linter/analyzer results (40+ tools)
- Chat conversation history

**How diffs reach the model:**
1. Clone the full repo into an isolated sandbox
2. Run 40+ static analysis tools on the changes
3. Build a lightweight map of definitions and references
4. Scan commit history for files that frequently change together
5. Chunk code intelligently, prioritize changed areas
6. Adjust review depth based on file complexity and importance
7. Pack the diff + equal-weight context into the prompt
8. Verify every suggestion post-generation to reduce hallucinations

**Key insight:** CodeRabbit does not just send raw diffs. It sends diffs *surrounded by structured context* that helps the model understand why the change matters. The diff is the centerpiece but context is the enabler.

Sources:
- [How CodeRabbit delivers accurate AI code reviews on massive codebases](https://www.coderabbit.ai/blog/how-coderabbit-delivers-accurate-ai-code-reviews-on-massive-codebases)
- [Context Engineering: Level up your AI Code Reviews](https://www.coderabbit.ai/blog/context-engineering-ai-code-reviews)
- [CodeRabbit on Google Cloud Run](https://cloud.google.com/blog/products/ai-machine-learning/how-coderabbit-built-its-ai-code-review-agent-with-google-cloud-run)
- [CodeRabbit LanceDB case study](https://lancedb.com/blog/case-study-coderabbit/)

### PR-Agent (Qodo): Compression Strategy

PR-Agent's core innovation is its "PR Compression Strategy" -- converting arbitrarily long code diffs into manageable LLM prompts.

**Mechanical process:**
1. Get the full git diff between source and target branches
2. Organize file patches within each language
3. Sort patches by number of tokens (descending -- most significant changes first)
4. Add patches to the prompt until reaching a buffer from max token length
5. Remaining patches go into a compressed list called "other modified files"
6. Each hunk gets configurable extra context lines (`patch_extra_lines_before`, `patch_extra_lines_after`)
7. Dynamic context extension: expand the context window to include function/class definitions up to `max_extra_lines_before_dynamic_context` lines

**Critical constraint:** Each tool (/review, /improve, /ask) uses a **single LLM call** with the goal of getting an answer in ~30 seconds affordably. This means the compression must be aggressive -- no multi-turn reasoning about diffs.

**Implication for our builder agent:** PR-Agent proves that prioritized, compressed diffs work for single-call reasoning. Sort changes by significance. Include the most important changes with full context. Summarize the rest.

Sources:
- [PR-Agent compression strategy](https://qodo-merge-docs.qodo.ai/core-abilities/compression_strategy/)
- [PR-Agent dynamic context](https://qodo-merge-docs.qodo.ai/core-abilities/dynamic_context/)
- [PR-Agent GitHub](https://github.com/qodo-ai/pr-agent)

### GitHub Copilot Code Review

GitHub Copilot's PR review combines the diff with contextual information (PR title, body, custom instructions) into a prompt sent to a "carefully tuned mix of models, prompts, and system behaviors." Details are sparse since this is a closed system, but the key pattern is the same: diff + structured context, not diff alone.

Source: [About GitHub Copilot code review](https://docs.github.com/en/copilot/concepts/agents/code-review)

### Baz: AST-Aware Diffing

Baz identified a fundamental problem: "State-of-the-art models were trained on vast volumes of code so they recognize diffs and diff-related signatures like `@@`, `+`/`-` except they are not necessarily able to deduce what the *impact* of the diff would be on a given code snippet."

**Their solution:** Use Difftastic (a language-aware syntax diff tool that parses code with Tree-sitter) instead of text-based git diff. This produces AST-level diffs that map changes to real-world impacts like functionality and readability, rather than superficial formatting tweaks.

**Key insight:** Text-based diffs tell the model *what lines changed*. AST-based diffs tell the model *what structures changed*. The latter is much more useful for reasoning about impact.

**Implication for our builder agent:** When diffing configuration objects (tool lists, routing rules), a structured/semantic diff (showing which tools were added/removed, which rules changed) will be more comprehensible than a text-level diff of the serialized config.

Sources:
- [Why Your Code Gen AI Doesn't Understand Diffs](https://baz.co/resources/why-your-code-gen-ai-doesnt-understand-diffs)
- [Building an AI Code Review Agent](https://baz.co/resources/building-an-ai-code-review-agent-advanced-diffing-parsing-and-agentic-workflows)

### Faire ("Fairey"): RAG + Diff

Faire built an in-house reviewer called "Fairey" for ~300 engineers. Architecture:

1. Retrieve the code diff
2. Use RAG to gather: modified files, surrounding code, related tests, documentation
3. Send diff + RAG context to LLM for review generation
4. Self-evaluation loop: a *different* LLM instance assesses each suggestion for quality
5. Post comments to PR via GitHub API

Now processes ~3,000 reviews/week. The key learning: off-the-shelf solutions lacked domain-specificity. They needed to tailor the context around diffs for their specific business domain.

Source: [How Faire's platform team built an AI code review agent](https://getdx.com/blog/how-faire-platform-team-built-an-ai-code-review-agent/)

### Sourcebot: The Honest Assessment

Sourcebot's learning from building a review agent: "The core problem isn't really about code review agents themselves, but about making an LLM actually understand your code. A 'code review agent' is just a component above this that chunks up diffs and sends suggestions to your PR."

Without sufficient context, the model "provides unhelpful suggestions, likely because it doesn't understand the implementation of the function being called." Diff without context is insufficient.

Source: [Sourcebot review agent learnings](https://www.sourcebot.dev/blog/review-agent-learnings)

---

## 3. Research on LLM Diff Understanding

### "What a Diff Makes" (arXiv 2511.00160, 2025)

This paper directly tests whether LLMs can comprehend diff outputs and use them for code migration tasks.

**Core finding:** "Contexts containing diffs can significantly improve performance against out-of-the-box LLMs and, in some cases, perform better than using [full] code."

This is significant: showing the model a diff of what changed between library versions performed *better* than showing it the full source code of both versions. The diff format compressed the information into a signal the model could act on more effectively.

**Methodology:** Used zero-shot prompting with large context window models (128k tokens for gpt-4o). Paired standard diff utilities (which find the longest common subsequence) with LLM reasoning.

Source: [What a diff makes: automating code migration with LLMs](https://arxiv.org/abs/2511.00160)

### "How Accurately Do Large Language Models Understand Code?" (2025)

The first large-scale empirical investigation into LLMs' code comprehension. Uses proxy debugging tasks (localizing faulty lines) to assess understanding. Key finding: specific code properties (structure, naming, patterns) significantly affect LLM comprehension. Models understand structured, well-named code far better than obfuscated or poorly structured code.

**Implication:** The format in which we present diffs matters because it affects the structural clarity the model can leverage.

Source: [How Accurately Do LLMs Understand Code?](https://arxiv.org/html/2504.04372v2)

### MemoryAgentBench (ICLR 2026)

Evaluates memory in LLM agents via incremental multi-turn interactions. Identifies four core competencies for memory agents:

1. **Accurate retrieval** -- find relevant information from accumulated context
2. **Test-time learning** -- integrate new information with existing knowledge
3. **Long-range understanding** -- connect information across distant turns
4. **Selective forgetting** -- drop outdated information when updates arrive

The benchmark simulates real multi-turn scenarios by splitting long texts into chunks and feeding them incrementally. Key design: "inject once, query multiple times."

**Implication:** When our builder agent receives incremental config updates, it needs to be able to (a) integrate the update with its existing understanding and (b) selectively forget the old version of whatever changed.

Source: [MemoryAgentBench](https://arxiv.org/abs/2507.05257)

---

## 4. Edit Instruction Formats: How Tools Express Changes

### The Format Landscape

| Format | Used By | Mechanism | Strength | Weakness |
|--------|---------|-----------|----------|----------|
| Search/Replace blocks | Aider, Claude Code | Find exact old text, swap in new text | Simple, no line numbers | Requires perfect character reproduction |
| Unified diff | Aider (GPT-4), git | `+`/`-` markers with context lines | Familiar, compact | LLMs mess up hunk headers |
| `apply_patch` (V4A) | OpenAI Codex | Custom diff envelope with `*** Begin/End Patch` | Model trained specifically for it | Other models fail catastrophically |
| Whole file | Aider (simple mode) | Return entire updated file | Easiest for model | Token-expensive, file size limited |
| `str_replace` | Claude Code | Find old string, replace with new | Very simple conceptually | Requires exact match including whitespace |
| Neural merge | Cursor | Fine-tuned 70B model merges draft edit into file | Handles imprecise edits | Requires dedicated model |
| Semantic edit | MorphLLM | Code-understanding-based transforms | 98% accuracy claimed | Complex infrastructure |

Source: [Code Surgery by Fabian Hertwig](https://fabianhertwig.com/blog/coding-assistants-file-edits/)

### The Harness Problem

Can Boluk's analysis ("The Harness Problem," Feb 2026) makes the case that edit format is a *harness* problem, not a model problem. An 8% improvement in Gemini's success rate was achieved purely by changing the edit format -- "bigger than most model upgrades deliver, and it cost zero training compute."

But formats are not portable: "Codex uses `apply_patch` with an OpenAI-flavored diff format, but when given to other models completely unaware of it, patch failures go through the roof -- Grok 4's patch failure rate was 50.7%, GLM-4.7's was 46.2%."

**Key principle:** The format the model was trained on matters more than the theoretical superiority of the format. OpenAI trained Codex specifically on `apply_patch`. Claude was trained with `str_replace`. Using the "wrong" format for a model degrades performance catastrophically.

Source: [The Harness Problem](https://blog.can.ac/2026/02/12/the-harness-problem/)

### What This Means for Showing Changes (Not Expressing Them)

The edit format literature is about *how models express changes* (output format). Our problem is *how models read changes* (input format). These are related but different:

- **Output:** The model needs to produce syntactically valid, mechanically applicable edits. Format strictness matters.
- **Input:** The model needs to *understand* what changed and reason about implications. Comprehension matters.

The Diff-XYZ benchmark tests both directions and finds they diverge: search-replace is best for generating diffs but underperforms for applying them. The optimal input format (for comprehension) may be different from the optimal output format (for generation).

---

## 5. Incremental Prompt Patterns

### Agentic Context Engineering (ACE)

ACE (arXiv 2510.04618) is the most directly relevant framework. It treats context as an "evolving playbook" that accumulates, refines, and organizes strategies through generation, reflection, and curation.

**The Delta Update Mechanism:**

The Curator produces compact "delta" updates -- new or modified bullets merged into the existing playbook using lightweight, non-LLM logic. This is the key pattern:

1. Each bullet has metadata: unique ID, counters tracking helpfulness/harmfulness
2. New bullets are appended (new ID)
3. Existing bullets are updated in place (increment counters, modify text)
4. Periodic deduplication removes redundancy using semantic embeddings
5. The result is a compact, evolving context that costs 83.6% fewer tokens than baseline methods

**Three-role architecture:**
- **Generator** -- produces reasoning trajectories (runs the task)
- **Reflector** -- distills concrete insights from successes and errors
- **Curator** -- integrates insights into structured context updates (the delta producer)

**Performance:** +10.6% improvement on AI agent tasks, +8.6% on specialized domains (finance), at 83.6% lower token cost compared to monolithic context rewrites.

**Implication for our builder agent:** Instead of showing the full config every turn, maintain a "playbook" of the node's configuration as structured bullets. When something changes, produce a delta update: "Tool `web_search` was removed. Routing rule to `Fact Check` was updated: condition changed from `confidence < 0.8` to `confidence < 0.9`." The model reads the current playbook + the delta, not the full config + the full diff.

Sources:
- [ACE paper](https://arxiv.org/abs/2510.04618)
- [ACE GitHub](https://github.com/ace-agent/ace)
- [ACE on SambaNova](https://sambanova.ai/blog/ace-open-sourced-on-github)

### Context Engineering (Broader Pattern)

The broader "context engineering" movement (as distinct from "prompt engineering") emphasizes that what goes *into* the prompt matters more than how you phrase the instruction. Key principles relevant to incremental updates:

- **Compress aggressively.** PR-Agent's compression strategy proves that prioritized, token-budgeted context works for single-call reasoning.
- **Structure over prose.** Structured bullets with metadata (ACE) outperform unstructured summaries.
- **Context = code + intent + history.** CodeRabbit's 1:1 ratio shows that surrounding the change with context about *why* it matters improves comprehension.
- **Verify post-generation.** CodeRabbit and Faire both use post-generation verification to catch hallucinations from incomplete context.

Source: [Beyond Prompting: The Power of Context Engineering](https://towardsdatascience.com/beyond-prompting-the-power-of-context-engineering/)

---

## 6. Synthesis: Patterns for the Builder Agent

### The Problem Restated

The builder agent (Board Dispatcher -> Per-Node Builder path) needs to understand what changed in a node's configuration without re-reading everything. The configuration includes:

- System prompt (natural language, potentially long)
- Tool list (structured, list of tool names + configs)
- Routing rules (structured, conditions + targets)
- Description text (natural language, user-facing)
- Agent roster (structured, hierarchy of agents)

### Recommended Approach: Structured Delta with Before/After

Based on the research, here is the recommended format:

**Do not use unified diffs for configuration changes.** Unified diffs are optimized for code (line-oriented, character-precise). Configuration changes are semantic (a tool was added, a routing condition changed, a system prompt paragraph was rewritten).

**Do not use line numbers.** Every source agrees: LLMs are bad with line numbers.

**Use a structured delta format** inspired by ACE's playbook updates and PR-Agent's compression strategy:

```
## What Changed (Node: "Research Team")

### Tools
- ADDED: `aggregate_companies` -- aggregates company data by ID
- REMOVED: `basic_search` -- replaced by aggregate tool

### Routing Rules
- UPDATED: Rule "to Fact Check"
  - Before: trigger when `confidence < 0.8`
  - After: trigger when `confidence < 0.9`

### System Prompt
- UPDATED: Paragraph 2 (data handling)
  - Before: "Query the database for new website visits"
  - After: "Query the database for new website visits and aggregate the
    company information by first selecting the companies then placing
    their IDs in your final response."
  - Change summary: Added aggregation logic with company ID extraction

### Agent Roster
- No changes

### Description
- UPDATED:
  - Before: "Search for competitor pricing"
  - After: "Search for competitor pricing across Q3 and Q4, compare
    year-over-year trends, flag anomalies above 10%."
```

### Why This Format

1. **Semantic, not textual.** Each section describes what structurally changed (tool added, rule updated), not what characters changed. This aligns with Baz's finding that AST-level diffs outperform text-level diffs for LLM comprehension.

2. **Before/after for prose changes.** For system prompts and descriptions (natural language), show the specific changed section with before/after. The "What a Diff Makes" paper found that diffs can outperform full code for comprehension -- but only when the diff is clean and the context is clear.

3. **Change summary for long prose.** When a system prompt paragraph changes significantly, add a one-line summary of what the change means. This mirrors CodeRabbit's context engineering: the diff alone is not enough, the model needs to understand intent.

4. **Explicit "no changes" for unchanged sections.** This eliminates ambiguity. The model knows it has complete information about what changed.

5. **No line numbers.** Content-anchored throughout.

6. **Structured sections match the config structure.** The diff format mirrors the configuration object's structure, making it easy for the model to map changes to their location in the config.

### When to Show Full Config vs. Delta

Based on ACE's playbook pattern and PR-Agent's compression strategy:

- **First encounter:** Show the full config. The model needs a baseline.
- **Small changes (1-3 fields):** Show only the delta in the structured format above.
- **Large changes (>50% of config modified):** Show the full new config with change markers. At some point, a delta becomes harder to comprehend than the whole thing.
- **Complete rewrite:** Show the full new config. No delta -- just "Configuration was completely rewritten."

### Token Budget Heuristic

PR-Agent's approach: sort changes by significance, include the most important with full context, compress the rest. For our builder agent:

1. Compute a rough "significance score" for each change (e.g., system prompt rewrite > tool addition > routing threshold tweak)
2. Include full before/after for the top changes
3. Summarize remaining changes as one-liners
4. If total delta exceeds ~2000 tokens, switch to full config with change markers

### The Playbook Pattern (For Multi-Turn Builder Sessions)

If the builder agent works on a node across multiple turns (e.g., user refines the config iteratively):

1. Maintain a "configuration summary" -- a compressed, structured representation of the current config (ACE's playbook)
2. Each turn, produce a delta update to the summary
3. The model receives: current summary + latest delta + user instruction
4. After each turn, update the summary (non-LLM, mechanical merge)

This mirrors ACE's 83.6% token savings while maintaining comprehension. The summary acts as the model's "memory" of the full config without needing to re-read it.

---

## 7. Tool-Specific Patterns Worth Noting

### PR-Agent's Numbered Hunks

PR-Agent converts diffs to numbered hunks with `convert_to_hunks_with_lines_numbers`, then dynamically extends context to include function/class definitions. This is not about line numbers in the diff itself -- it is about numbering the *hunks* so the model can reference them ("issue in hunk 3"). This is a useful pattern for our builder agent: number the changes so the model can reference them in its reasoning.

### CodeRabbit's Post-Generation Verification

Every suggestion is verified before posting. For our builder agent, this means: after the model processes a delta and produces updated configuration, verify the result against the delta to ensure all changes were incorporated. Simple mechanical check, not LLM-based.

### Cursor's Neural Merge

Cursor trained a separate 70B model whose job is to take a draft edit and merge it into the file correctly. This is overkill for our use case but illustrates the principle: if the model produces imprecise edits, a dedicated merge step can clean them up. For configuration updates, a schema-aware merge (JSON merge, structured merge) would serve the same purpose without a dedicated model.

---

## 8. Open Questions

1. **How well do models understand diffs of natural language** (system prompts) vs. diffs of structured data (tool lists)? The research is heavily code-focused. We may need to run our own experiments.

2. **What is the optimal delta granularity?** ACE uses bullets. PR-Agent uses file-level patches. For configuration objects, the natural granularity is the field level (tool list, system prompt, routing rules). Is this too coarse or too fine?

3. **Does the builder agent need to see the full config periodically** to prevent drift? ACE's deduplication step suggests yes -- periodic full-context refreshes prevent the accumulated deltas from diverging from reality.

4. **How does this interact with the beliefs layer?** Beliefs are already a compressed, structured representation. The delta format could be unified with belief updates -- "belief X was strengthened by this config change."
