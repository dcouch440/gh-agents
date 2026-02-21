# Prompt Engineering Reference

> A working playbook for designing prompts across multi-agent systems, tool-using agents, and user-facing AI. Load this document before any prompt creation session.

---

## Quick Start: The 10 Rules

If you read nothing else, internalize these:

1. **Context is finite.** Every token costs attention. Include what matters, cut what doesn't.
2. **Start and end strong.** Critical instructions go at the beginning AND end of your prompt. The middle gets the least attention.
3. **Say what to do, not what to avoid.** "Write plain prose paragraphs" beats "Don't use markdown." LLMs struggle with negation.
4. **Explain why.** "Never use ellipses because text-to-speech can't pronounce them" works better than "Never use ellipses."
5. **Tool descriptions are the #1 factor.** 3-4 sentences minimum per tool. Describe what, when, how, and boundaries.
6. **Use XML tags for structure (Claude), Markdown for cost.** XML costs 80% more tokens but gives precise boundaries.
7. **Examples > instructions for complex formats.** 2-5 examples, most important one last.
8. **Test prompts like code.** Run the same prompt 5+ times. Single runs hide variance. Use deterministic assertions.
9. **Multi-agent systems fail from coordination, not capability.** 79% of failures are role confusion and specification issues. Use explicit boundaries and Challenger/Inspector patterns.
10. **Dial back aggressive language for modern models.** Claude 4.6 overtriggers on "CRITICAL," "MUST," "NEVER." Use normal phrasing.

---

## Table of Contents

1. [Foundational Principles](#1-foundational-principles)
2. [Prompt Structure & Formatting](#2-prompt-structure--formatting)
3. [XML as Cognitive Scaffolding](#3-xml-as-cognitive-scaffolding)
4. [Constraint Language](#4-constraint-language)
5. [Persona, Tone & Voice](#5-persona-tone--voice)
6. [The Human Masquerade](#6-the-human-masquerade)
7. [Agent-to-Agent Protocols](#7-agent-to-agent-protocols)
8. [Tool Use & Function Calling](#8-tool-use--function-calling)
9. [Declarative Tool Design](#9-declarative-tool-design)
10. [Chain-of-Thought & Reasoning Patterns](#10-chain-of-thought--reasoning-patterns)
11. [Prompt Composition & Templates](#11-prompt-composition--templates)
12. [Failure Modes & Mitigations](#12-failure-modes--mitigations)
13. [Testing & Evaluation](#13-testing--evaluation)
14. [Anthropic/Claude-Specific Guidance](#14-anthropicclaude-specific-guidance)
15. [Grok/xAI-Specific Guidance](#15-grokxai-specific-guidance)
16. [Sources & Further Reading](#16-sources--further-reading)

---

## 1. Foundational Principles

### Context Is a Finite Resource

Every token in the context window costs attention budget. Attention mechanism scaling is quadratic — more computing power per token as context grows. Research from Chroma (2025) coined "context rot": models grow increasingly unreliable as input length grows, even models advertising 10M+ token windows.

**The U-shaped attention curve.** Models recall information best at the beginning (primacy bias) and end (recency bias) of the context window. Information in the middle degrades significantly. This is the "Lost in the Middle" effect (Liu et al., TACL 2024).

**Practical rule:** Place critical instructions and constraints at the start and end of your prompt. Bury reference data in the middle. Never bury instructions in the middle.

### Context Engineering > Prompt Engineering

Prompt engineering asks "how should I phrase this?" Context engineering asks "what does the model need to know right now?" The shift matters because what you include (and exclude) from context has more impact than how you word any single instruction.

**Principles:**
- Be thoughtful, keep context informative yet tight
- Use dynamic retrieval to surface relevant information at inference time rather than stuffing everything in
- Prune and maintain context over long interactions
- Treat bloated tool sets as a performance bug

### The Prompt Hierarchy

Messages carry implicit authority based on their role:

```
Root/Safety (highest authority — provider-level)
  └── System message (developer-level)
        └── User message (end-user level)
              └── Third-party content (lowest — retrieved docs, tool results)
```

OpenAI's Instruction Hierarchy paper (2024) showed 63% improvement in system prompt extraction defense when models are trained to respect this hierarchy. System messages produce more focused, authoritative responses than the same content in user messages (empirically tested by PromptHub).

### Determinism vs. Creativity

Not every prompt needs the same control tightness:

| Task Type | Control Level | Technique |
|-----------|--------------|-----------|
| Data extraction, classification | Tight | Structured output, JSON schema, enums |
| Analysis, summarization | Medium | XML sections, examples, output format guidance |
| Creative writing, brainstorming | Loose | Persona, tone guidance, minimal constraints |
| Agent tool use, workflows | Tight on structure, loose on reasoning | Tool definitions + system prompt guidance |

---

## 2. Prompt Structure & Formatting

### Format Performance by Provider

Research shows format choice has measurable impact — up to 40% performance variance on code tasks (He et al., 2024). No single format wins universally.

| Format | Strengths | Weaknesses | Best For |
|--------|-----------|------------|----------|
| **XML** | Claude-optimized, precise nesting, metadata attributes, injection defense | 80% more tokens than Markdown | Anthropic models, complex hierarchies |
| **Markdown** | Token-efficient, human-readable, GPT-4 prefers | Less precise boundaries | OpenAI models, cost-sensitive apps |
| **JSON** | Highly structured, GPT-3.5 prefers | Verbose, brittle escaping | Structured data payloads |
| **YAML** | Best accuracy in 2025 benchmarks, readable | Less universally tested | Accuracy-critical, nested config |

**Key benchmark (ImprovingAgents, 2025):** XML requires 80% more tokens than Markdown for the same content. Markdown uses 34-38% fewer tokens than JSON. Two of three models tested performed best with YAML.

### Recommended Prompt Structure

This layered order works across providers:

```
1. Role / Persona          — who the model is
2. Background context      — what it needs to know
3. Instructions            — what to do
4. Constraints             — what NOT to do (framed positively)
5. Examples                — what good output looks like
6. Runtime data            — dynamic content (documents, tool results)
7. Output format           — how to structure the response
8. Final instruction       — restate the core ask (recency bias)
```

Place longform data (20K+ tokens) near the top of the prompt, above queries and instructions. Queries at the end improve response quality by up to 30% on complex, multi-document inputs (Anthropic docs).

### Information Placement Rules

```
START of context  →  Role, persona, critical constraints (primacy bias)
MIDDLE of context →  Reference data, documents, examples (lower attention)
END of context    →  The actual task, output format, restated constraints (recency bias)
```

---

## 3. XML as Cognitive Scaffolding

Claude was specifically trained to recognize XML tags as a prompt organizing mechanism. There are no "magic" tag names — use semantically meaningful names that match the content they surround.

### Tag Naming Conventions

Name tags by what the model should DO with the content, not just what it IS:

```xml
<!-- Tells the model what to do with the content -->
<instructions>Follow these steps...</instructions>
<constraints>Stay within these boundaries...</constraints>
<reference>Use this information if needed...</reference>
<examples>These show the expected format...</examples>

<!-- Better than generic labels -->
<data>...</data>          <!-- Too vague — data for what? -->
<info>...</info>          <!-- What kind of info? -->
<content>...</content>    <!-- Everything is content -->
```

### Hierarchy Encoding Patterns

**Flat tags** for top-level sections:
```xml
<role>You are a senior analyst.</role>
<task>Evaluate the quarterly report.</task>
<output_format>Return JSON with findings and confidence scores.</output_format>
```

**Nested scope** for natural containment:
```xml
<workflow>
  <step name="collect" order="1">
    <agent role="researcher">Gather data from provided sources.</agent>
    <output>Raw findings as bullet points.</output>
  </step>
  <step name="analyze" order="2">
    <agent role="analyst">Synthesize findings into a report.</agent>
    <input source="step:collect">Use the researcher's findings.</input>
  </step>
</workflow>
```

**Attribute-based depth signaling** — the agent knows where it sits without deep nesting:
```xml
<context level="root">This is the overall project goal.</context>
<context level="workflow">This workflow handles data analysis.</context>
<context level="step">You are the analysis step. Your input comes from the collector.</context>
<context level="sub_step">Extract sentiment scores from each paragraph.</context>
```

### The Hierarchy Map Pattern

Give the agent a compact XML preamble showing its position in the execution tree before the actual content:

```xml
<position_map>
  <collection name="Q4 Analysis Pipeline">
    <workflow name="Data Processing" status="complete" />
    <workflow name="Analysis" status="active">
      <step name="Collect" status="complete" output="raw_data.json" />
      <step name="Analyze" status="active" agent="you" />
      <step name="Report" status="pending" />
    </workflow>
  </collection>
</position_map>

<!-- Now the agent knows: I'm the Analyze step, Collect is done, Report is next -->
```

### Visibility Scoping

Tags that signal what the agent should do with different information:

```xml
<for_you>Instructions specific to this agent.</for_you>
<background>Context for understanding — don't act on this directly.</background>
<pass_downstream>Include this in your output for the next agent.</pass_downstream>
<private>Internal reasoning — do not expose in output.</private>
```

### Document Wrapping Pattern

For multi-document contexts:
```xml
<documents>
  <document index="1">
    <source>quarterly_report_q4.pdf</source>
    <document_content>
      Revenue increased 12% year-over-year...
    </document_content>
  </document>
  <document index="2">
    <source>competitor_analysis.md</source>
    <document_content>
      Market share shifted toward...
    </document_content>
  </document>
</documents>
```

---

## 4. Constraint Language

### What Actually Works

| Technique | Effectiveness | Notes |
|-----------|--------------|-------|
| **Positive framing** ("do X") | High | Anthropic explicitly recommends. "Your response should be smoothly flowing prose." |
| **Negative framing** ("don't do X") | Unreliable | Pink Elephant problem — model must process the concept to suppress it |
| **Explaining WHY** | High | Claude generalizes from explanations. "Never use ellipses because text-to-speech engines can't pronounce them." |
| **ALL CAPS emphasis** | Moderate, declining | Claude 4.6 overtriggers on aggressive language. Use normal phrasing. |
| **"MUST" / "CRITICAL"** | Was useful, now counterproductive | Claude 4.6 is more responsive and may overtrigger. Dial back. |
| **Instruction at start/end** | High | Lost in the Middle (TACL 2024). Never bury constraints in the middle. |
| **Prompt repetition** | Moderate | Stating key constraints twice (start and end) mitigates position bias. |

### The Pink Elephant Problem

Research on negation in LLMs (arXiv:2503.22395, 2025) found that LLMs "inherently cannot grasp the concept of negation due to their structural characteristics." To suppress a concept, the model must first process it — which activates the very thing you're trying to prevent.

**Instead of:**
```
Do not use markdown in your response.
Do not mention competitors.
Never include personal opinions.
```

**Write:**
```
Your response should be plain text paragraphs without any formatting.
Focus exclusively on our product's capabilities.
Base all statements on the provided data, citing specific figures.
```

### Constraint Ordering

Constraints follow the same attention rules as all context:

```xml
<constraints>
  <!-- Most critical constraints first (primacy) -->
  Respond only with valid JSON matching the provided schema.
  Base all claims on the documents provided — never fabricate data.

  <!-- Supporting constraints in the middle -->
  Use the metric system for all measurements.
  Quote monetary values in USD.

  <!-- Restate the critical constraint last (recency) -->
  Your entire response must be valid JSON. No text outside the JSON object.
</constraints>
```

---

## 5. Persona, Tone & Voice

### What Research Says About Role Prompting

Persona prompting is a double-edged sword:
- For **open-ended and creative tasks**: personas improve output quality and relevance
- For **factual accuracy tasks**: personas generally do NOT improve performance and can decrease it (tested across 162 personas, 4 model families, 2,410 MMLU questions)
- Larger models (70B+) show MORE negative effects from persona prompting on factual tasks

### Effective Role Assignment

**What works:**
```
You are a senior financial analyst with 15 years of experience in
equity research. You specialize in technology sector valuations.
```

**What doesn't reliably help:**
```
Imagine you are a brilliant financial genius who never makes mistakes...
```

**Key findings from LearnPrompting:**
- "You are a/an [role]" outperforms "Imagine you are..." or "You are talking to your [role]"
- Gender-neutral roles outperform gendered roles
- Work/occupational roles outperform other role categories
- Two-stage approach: (1) establish role with context, (2) present the task

### Tone Calibration

Instead of vague instructions like "be professional," specify the behavioral dimensions:

```xml
<tone>
  Formality: Business casual — clear and direct, not stiff or academic.
  Assertiveness: State recommendations confidently. Use "should" not "might consider."
  Technicality: Match the user's level. If they use jargon, mirror it. If they don't, explain simply.
  Brevity: Lead with the answer. Explain only if asked or if the reasoning matters.
</tone>
```

### Anti-Patterns in Persona Design

| Anti-Pattern | Problem | Fix |
|-------------|---------|-----|
| Over-apologizing | Wastes tokens, signals uncertainty | "State corrections directly without apologizing." |
| False confidence | Hallucination risk | "When uncertain, say so explicitly and explain what you'd need to verify." |
| Unnecessary hedging | Dilutes recommendations | "Lead with your recommendation, then note caveats." |
| Sycophancy | Agrees with wrong premises | "If the user's assumption is incorrect, say so before proceeding." |

---

## 6. The Human Masquerade

### What It Is

Framing system configuration as human-like conversation in prompts. Instead of structured imperative instructions, you present context as if a colleague is naturally briefing the agent.

### Why It Works

LLMs are trained on conversational data. Conversational framing naturally encodes intent, context, and constraints simultaneously. The anchoring effect means that domain vocabulary seeded early in conversational prompts sets the model's interpretive frame.

### The Briefing Pattern

**Structured (traditional):**
```
SYSTEM: You are a code reviewer. Review the following code for security vulnerabilities.
Focus on: SQL injection, XSS, authentication bypasses.
Output format: JSON array of findings with severity, location, description.
```

**Briefing (masquerade):**
```
SYSTEM: You are a senior security engineer on the platform team.

USER: Hey, I need your eyes on this PR before we merge. The author is a junior
dev and this touches our auth layer, so I'm a bit nervous. Here's what I need:

Look through the code below and flag anything that could be a security issue.
I'm especially worried about SQL injection and XSS since this handles user input,
but catch anything else you see. Give me each finding with how bad it is, where
it is, and what's wrong.

<code>
{{code_content}}
</code>
```

The second version activates the same behavior but with stronger contextual grounding — the model understands the *situation*, not just the task.

### When to Use Each Style

| Context | Style | Why |
|---------|-------|-----|
| Agent-to-agent communication | Structured | Precision matters. No ambiguity. |
| User-facing assistant | Briefing/conversational | Natural interaction, better engagement |
| Tool descriptions | Structured | Schema compliance is critical |
| System prompts for creative agents | Conversational | Opens up creative space |
| Data extraction / classification | Structured | Up to 13.79% better accuracy (research) |

### Template Variable Interpolation

Make templates feel natural by embedding variables in conversational prose:

```
The customer you're helping is {{customer_name}}. They've been with us for
{{tenure_months}} months and their account tier is {{tier}}. They just
submitted a support ticket about {{issue_summary}}.

Their recent activity shows {{recent_activity_summary}}.

Help them resolve this. If you need to look anything up, use the
customer_lookup and order_history tools.
```

vs. the robotic alternative:
```
Customer Name: {{customer_name}}
Tenure: {{tenure_months}} months
Tier: {{tier}}
Issue: {{issue_summary}}
Activity: {{recent_activity_summary}}

Task: Resolve the customer's issue using available tools.
```

Both work. The first produces more empathetic, contextually-aware responses. The second is more token-efficient and precise.

### Security Warning

When system content looks like user content, you create a prompt injection surface. If `{{issue_summary}}` contains adversarial instructions, the model may follow them as if the "user" (actually the system template) is giving new instructions.

**Mitigations:**
- Sanitize all interpolated values from untrusted sources
- Use Microsoft's Spotlighting technique (delimiters, datamarking, or encoding) to visually distinguish untrusted content
- Never interpolate raw user input into system prompts without validation
- Use closing system messages that reiterate constraints (recency bias defense)

---

## 7. Agent-to-Agent Protocols

### Communication Models Across Frameworks

| Framework | Model | How Agents Talk |
|-----------|-------|-----------------|
| **AutoGen** | Shared message context | All agents see the same conversation history. Handoff via tool call. |
| **CrewAI** | Role-based delegation | Sequential output passing or explicit delegation tools. |
| **LangGraph** | Stateful graph | Shared typed state object. Edges define flow. Reducers merge updates. |
| **CAMEL** | Role-playing dyadic | Two agents alternate (AI User + AI Assistant) in a fixed loop. |
| **OpenAI Swarm** | Routines and handoffs | Handoff = function returning another Agent. System prompt swaps. |
| **Google A2A** | JSON-RPC protocol | Open interoperability standard. Agents are opaque to each other. |

### Handoff Protocol Design

The core pattern across all frameworks: **handoff is a tool call**.

```
Agent A is working → decides another agent is better suited →
calls handoff tool with: target agent name + context message →
framework swaps active agent → Agent B receives context and continues
```

**What must travel with a handoff:**
- **Identity**: who is handing off and to whom
- **Context**: what the receiving agent needs to know (conversation history or a summary)
- **Intent**: why the handoff is happening (what the receiving agent should do)
- **Artifacts**: any outputs from the handing-off agent that the receiver needs

**AutoGen HandoffMessage schema:**
```python
HandoffMessage(
    source="researcher",      # who is handing off
    target="analyst",         # who receives
    content="Analysis ready", # human-readable intent
    context=[...],            # full message history
)
```

### State Propagation Patterns

**Full history pass-through** (AutoGen, OpenAI Swarm): Every agent sees the entire conversation. Simple but context window fills fast.

**Task output chain** (CrewAI): Each task's output automatically becomes context for the next task. Only the output propagates, not the full reasoning history.

**Typed state graph** (LangGraph): Shared state object with reducer functions that control how concurrent updates merge (append, overwrite, custom merge). Agents read/write specific state keys.

**Choose based on your needs:**
- Full history: when context continuity matters more than token efficiency
- Task output: when each agent's job is cleanly scoped and independent
- Typed state: when you need fine-grained control over what propagates

### Shared Vocabulary

When agents communicate, they need to agree on terms. Establish a controlled vocabulary:

```xml
<ontology>
  <term name="task">A discrete unit of work with defined inputs and outputs.</term>
  <term name="observation">A factual finding from analysis, without interpretation.</term>
  <term name="recommendation">An actionable suggestion based on observations.</term>
  <term name="decision">A final choice that commits to a course of action.</term>
  <term name="blocker">An unresolved issue that prevents progress.</term>
  <term name="handoff">Transfer of control from one agent to another.</term>
</ontology>
```

### Error & Escalation Patterns

**What works (from Google ADK):** Lifecycle callbacks as contract enforcement:
- `before_agent_callback` — precondition check (can block execution)
- `after_agent_callback` — postcondition validation
- `before_tool_callback` — tool-call guard

**Escalation hierarchy:**
1. Agent retries with corrected approach
2. Agent hands off to a specialist
3. Agent escalates to supervisor/orchestrator
4. System pauses for human input

**Anti-pattern: Infinite delegation loops.** Agents continuously hand off without progress. Mitigate with: CAMEL's termination token (`<CAMEL_TASK_DONE>`), CrewAI's `allow_delegation=False` on leaf agents, or explicit maximum handoff counts.

### Multi-Agent Anti-Patterns

From the MAST taxonomy (Berkeley, 2025) and Microsoft's failure analysis:

| Anti-Pattern | Impact | Mitigation |
|-------------|--------|------------|
| **Bag of Agents** | 17x error amplification vs. single agent | Use structured topology, not flat groups |
| **Error Cascading** | Corrupted agent output infects downstream agents | Validation gates between agents |
| **Mutual Error Reinforcement** | Agents validate each other's wrong conclusions | Add a "Challenger" agent that questions outputs |
| **Role Confusion** | Agent drifts from its specialization | Explicit role boundaries + Inspector agent |
| **Context Loss on Handoff** | Receiving agent hallucinates missing context | Carry sufficient context with every handoff |
| **Coordination Tax** | Performance degrades past ~4 agents | Hierarchical structure (boss + specialists) |

**The Challenger/Inspector pattern** recovers up to 96% of lost performance in multi-agent systems. One agent is explicitly tasked with questioning outputs from other agents before they propagate downstream.

### Example: Agent-to-Agent System Prompt

A supervisor agent managing specialists:

```xml
<role>
You are the Supervisor agent in a multi-agent analysis pipeline.
You coordinate three specialist agents: Researcher, Analyst, and Writer.
</role>

<agents>
  <agent name="Researcher" capabilities="web search, document retrieval, data collection">
    Use when: you need factual data, source documents, or raw information.
    Do NOT use for: interpretation, recommendations, or final output.
  </agent>
  <agent name="Analyst" capabilities="data analysis, pattern recognition, statistical reasoning">
    Use when: you have raw data that needs interpretation or comparison.
    Do NOT use for: data collection or prose writing.
  </agent>
  <agent name="Writer" capabilities="report writing, summarization, formatting">
    Use when: analysis is complete and needs to become a deliverable.
    Do NOT use for: research or analysis — only formatting and writing.
  </agent>
</agents>

<handoff_protocol>
When delegating to an agent, always provide:
1. WHAT: The specific task (one sentence)
2. CONTEXT: What they need to know from prior steps
3. OUTPUT: What you expect back (format and content)
4. BOUNDARY: What is out of scope for this delegation

Example handoff:
  delegate_to(
    agent: "Analyst",
    task: "Compare Q3 vs Q4 revenue by product line",
    context: "Researcher found revenue data in the attached spreadsheet",
    output: "Table with percentage changes and top 3 insights",
    boundary: "Do not write prose or make recommendations"
  )
</handoff_protocol>

<escalation>
If any agent reports uncertainty above 30% or encounters missing data:
1. Do NOT proceed with assumptions
2. Route back to Researcher with a specific data request
3. If Researcher cannot find the data, report the gap to the user
</escalation>
```

### Emerging Protocol Standards

| Protocol | Owner | Purpose | Status (2025) |
|----------|-------|---------|---------------|
| **MCP** | Anthropic | Tool/data source exposure to models | Production |
| **A2A** | Google | Inter-agent communication across vendors | Spec in progress |
| **ANP** | Community | Agent networking and discovery | Early |
| **LACP** | Academic | LLM agent communication standardization | Proposed |

---

## 8. Tool Use & Function Calling

### Tool Description Quality Is the #1 Factor

Anthropic's most critical finding: "Provide extremely detailed descriptions. This is by far the most important factor in tool performance." Small refinements to descriptions yield dramatic improvements — Claude achieved state-of-the-art SWE-bench performance after precise description refinements.

**Good description:**
```json
{
  "name": "get_stock_price",
  "description": "Retrieves the current stock price for a given ticker symbol. The ticker symbol must be a valid symbol for a publicly traded company on a major US stock exchange like NYSE or NASDAQ. The tool will return the latest trade price in USD. It should be used when the user asks about the current or most recent price of a specific stock. It will not provide any other information about the stock or company.",
  "input_schema": { ... }
}
```

**Poor description:**
```json
{
  "name": "get_stock_price",
  "description": "Gets the stock price for a ticker.",
  "input_schema": { ... }
}
```

### Description Checklist

Every tool description should answer:
- What does this tool do?
- When should the agent use it (and when should it NOT)?
- What does each parameter mean and how does it affect behavior?
- What format do inputs need to be in?
- What will the output look like?
- What are the boundaries with other similar tools?

### Parameter Design

**Flat over nested.** OpenAI research shows flat schemas are easier for models to reason about. Deeply nested objects with repeated field names increase misuse.

**Enums over free-text** when you have a fixed set of valid values. This turns a generation task into a classification task. Use JSON Schema `enum` — it's well-supported across all providers.

**Semantic names.** Use `user_id` not `user`. Use `absolute_file_path` not `path` (when Anthropic switched from relative to absolute paths, model errors dropped to zero).

**Don't make the model fill values you already know.** If you have the customer_id from context, inject it programmatically rather than asking the model to extract it.

### Tool Result Best Practices

**Return high-signal information only.** Strip low-level identifiers:
- `name` instead of `uuid`
- `image_url` instead of `256px_image_url`
- `file_type` instead of `mime_type`

Converting arbitrary UUIDs to semantic identifiers "substantially improves retrieval precision by reducing hallucinations" (Anthropic).

**Error messages must be actionable:**

Good: `"Search returned too many results (1000+). Try adding filters like status:open or date filters to narrow results."`

Bad: `"ValueError: Invalid input parameters. Contact system administrator."`

**For Claude specifically:** Set `"is_error": true` on tool_result blocks. Claude will retry 2-3 times with corrections before apologizing to the user.

### How Many Tools Is Too Many?

| Provider | Documented Limit | Performance Sweet Spot |
|----------|-----------------|----------------------|
| OpenAI | ~100 tools, ~20 args per tool | Fewer is better |
| xAI/Grok | 128 functions, 200 tools | Not documented |
| Anthropic | No hard limit | Use Tool Search for 10+ tools |

**Research finding (RAG-MCP, 2025):** Naive tool scaling causes performance degradation. Retrieval-augmented tool selection "more than triples tool selection accuracy (43.13% vs 13.62% baseline)" while cutting prompt tokens by 50%+.

**Anthropic's Tool Search Tool:** Mark rarely-used tools with `defer_loading: true`. Keep 3-5 most-used tools always loaded. A five-server setup dropped from ~55K tokens to ~8.7K. Accuracy improved from 49% to 74%.

### Parallel Tool Calling

Claude 4 models excel at parallel tool use by default. Boost to ~100% success with:

```xml
<use_parallel_tool_calls>
If you intend to call multiple tools and there are no dependencies
between the tool calls, make all of the independent tool calls in
parallel. For instance, if you need to search for two different items,
call both search tools simultaneously rather than sequentially.
</use_parallel_tool_calls>
```

**Critical rule for results:** When returning results from parallel calls, send ALL results in a single user message, not separate messages. Separate messages "teach" the model to avoid parallel calls in the future.

### Where to Put Tool Guidance

| Content | Where It Goes |
|---------|--------------|
| What the tool does, when to use it | Tool description |
| How parameters work, input formats | Tool description + parameter descriptions |
| Complex usage patterns, examples | System prompt |
| Cross-cutting concerns (ordering, when to ask user) | System prompt |
| Tool-specific edge cases | Tool description |

OpenAI: "For complicated tools, create an Examples section in your system prompt rather than adding them into the description field."

Anthropic: Supports `input_examples` field directly on tool definitions (beta). 1-5 realistic examples per tool improved accuracy from 72% to 90%.

---

## 9. Declarative Tool Design

### The Abstraction Level Problem

Most agent tools operate at the implementation level — `create_node`, `create_edge`, `set_property`. This forces the agent into multi-round-trip workflows that are slow and error-prone.

**The hierarchy:**
```
Low-level:   create_node("A") + create_node("B") + create_edge("A","B") + ...  → 12+ calls
Mid-level:   create_parallel(source, targets, sink)                            → 1 call
High-level:  create_workflow("analyze across three dimensions")                → 1 call (risky)
```

The sweet spot is **mid-level** — expressive enough to reduce round trips, constrained enough that the agent can't go off the rails.

### Graph Construction Primitives

```
create_sequence(steps: ["Collect", "Process", "Report"])
→ Linear chain: Collect → Process → Report
→ 1 call instead of 6+

create_parallel(
  source: "Collector",
  parallel: [
    { name: "PriceAnalyzer" },
    { name: "FeatureAnalyzer" },
    { name: "SentimentAnalyzer" }
  ],
  target: "Synthesizer"
)
→ Fan-out from source, fan-in to target
→ 1 call instead of 12+

create_conditional(
  source: "Classifier",
  branches: [
    { condition: "type == 'bug'", target: "BugHandler" },
    { condition: "type == 'feature'", target: "FeatureHandler" }
  ],
  default: "GeneralHandler"
)
→ Routing based on output conditions
→ 1 call instead of 8+
```

### Why This Matters

Each agent tool call is a full LLM inference round trip. At $0.20-3.00 per million input tokens, 12 round trips to build a simple fan-out pattern is wasteful. More importantly, each round trip is an opportunity for the model to make a mistake — wrong parameter, wrong node name, missing edge.

**Declarative tools** collapse multiple implementation steps into a single intent-level operation. The agent expresses what it wants (a parallel fan-out pattern), and the tool handles the implementation details.

### Design Principles for Declarative Tools

1. **Match the agent's intent level.** If agents consistently create the same sequence of 5 low-level calls, that sequence is a single tool.
2. **Keep the output predictable.** The agent should know exactly what graph topology the tool produces.
3. **Allow composition.** The output of `create_sequence` should be connectable to the output of `create_parallel`.
4. **Validate eagerly.** Check that referenced nodes exist, that the graph has no cycles (unless intended), and that types match — inside the tool, not in a separate validation step.

---

## 10. Chain-of-Thought & Reasoning Patterns

### The 2025 Reality Check

The Wharton Generative AI Labs published "The Decreasing Value of Chain of Thought in Prompting" (June 2025). Key findings:

- **Non-reasoning models:** Modest average improvement but *increased variability*. CoT sometimes triggers errors on questions the model would otherwise get right.
- **Reasoning models:** Only marginal benefits despite 20-80% increase in latency.
- Each question was tested 25 times per condition, revealing inconsistencies that one-time testing masks.

**Conclusion:** CoT gains are rarely worth the time cost for modern models. Use it selectively.

### When to Use CoT

| Situation | Use CoT? | Why |
|-----------|----------|-----|
| Multi-step math/logic | Yes | Each step depends on the previous |
| Complex policy evaluation | Yes | Multiple rules interact |
| Simple classification | No | Overthinking degrades performance |
| Data extraction | No | Direct extraction is faster and more accurate |
| With reasoning models (o3, Grok-mini with reasoning) | Usually no | They already do internal CoT |

### Extended Thinking (Claude)

Claude's extended thinking gives it a scratchpad to reason before responding. Performance improves logarithmically with thinking tokens allocated. But it can hurt performance by up to 36% on intuitive tasks where overthinking degrades results.

**The Think Tool** is distinct from extended thinking. Extended thinking happens before the response. The Think Tool provides structured thinking *during* response generation, between tool calls. It showed 54% relative improvement in complex tool-chain scenarios.

**For Claude 4.6:** Do NOT add explicit instructions like "use the think tool to plan your approach" — this causes over-planning. Let the model decide when to think.

### Few-Shot Examples

**How many:** 2-5 is the sweet spot. Performance improves sharply with the first few then plateaus.

**Ordering:** Place the most representative example last — models apply more attention to recent tokens.

**Format:** Consistent formatting across examples is critical. Each example must match the exact input/output structure you want. Include edge cases, not just happy paths.

**Examples vs. Instructions:** Instructions tell the model *what* to do. Examples show *how* it should look. Modern models follow instructions well enough that you need fewer examples than before. Use examples when you need to shift style, demonstrate complex output formats, or show domain-specific patterns that are hard to describe in words.

---

## 11. Prompt Composition & Templates

### Template Layering

Build prompts from composable layers:

```
Layer 1: Base         — org-wide standards, safety rules, constitutional principles
Layer 2: Role         — agent persona, expertise, behavioral constraints
Layer 3: Task         — specific objective, success criteria
Layer 4: Runtime data — dynamic context, retrieved docs, conversation history
Layer 5: Output       — schema, structure, examples
```

Each layer is a reusable fragment. A "researcher" role layer can be shared across multiple task layers. A "JSON output" layer can be appended to any prompt that needs structured output.

### Conditional Sections

Include or exclude prompt sections based on runtime context:

```
{{#if has_upstream_data}}
<upstream_context>
The previous step produced the following output:
{{upstream_output}}
Use this as your primary input.
</upstream_context>
{{/if}}

{{#if is_first_step}}
<workflow_overview>
You are the first step in the {{workflow_name}} workflow.
Your output will be consumed by: {{downstream_step_names}}.
</workflow_overview>
{{/if}}

{{#if has_tools}}
<tool_guidance>
You have access to the following tools. Use them when needed.
</tool_guidance>
{{/if}}
```

### Prompt Chaining Across Workflows

When decomposing complex tasks into sequential LLM calls:

1. **Type the interfaces.** Define expected input/output schemas between steps.
2. **Add validation gates.** Check that step N's output is valid before passing to step N+1.
3. **Keep steps cognitively focused.** Each step should isolate a single objective.
4. **Make steps reusable.** A summarization step should work regardless of what produced the document.

### Full Template Example

A complete prompt template for a workflow step agent:

```xml
<!-- Layer 1: Base -->
<constitution>
  Never fabricate data. If information isn't available, say so explicitly.
  All outputs must be valid JSON matching the provided schema.
  When uncertain, express confidence levels rather than guessing.
</constitution>

<!-- Layer 2: Role -->
<role>
You are {{agent_name}}, a {{agent_role}} in the {{workflow_name}} workflow.
{{agent_backstory}}
</role>

<!-- Layer 3: Task -->
<task>
  <objective>{{step_objective}}</objective>
  <success_criteria>{{success_criteria}}</success_criteria>
</task>

<!-- Layer 4: Runtime Data (conditional) -->
{{#if upstream_outputs}}
<upstream_context>
The previous step ({{upstream_step_name}}) produced:
{{upstream_outputs}}
</upstream_context>
{{/if}}

{{#if retrieved_documents}}
<reference_documents>
{{#each retrieved_documents}}
<document source="{{this.source}}">
{{this.content}}
</document>
{{/each}}
</reference_documents>
{{/if}}

<!-- Layer 5: Output -->
<output_format>
Respond with JSON matching this schema:
{{output_schema}}

Example:
{{output_example}}
</output_format>

<!-- Recency anchor: restate critical constraint -->
Your entire response must be valid JSON. Do not include any text outside the JSON object.
```

### The Constitution Pattern

The system prompt defines inviolable principles the model self-evaluates against:

```xml
<constitution>
  <principle priority="1">Never fabricate data. If the information isn't in the provided documents, say so.</principle>
  <principle priority="2">All recommendations must include a confidence level (high/medium/low) with reasoning.</principle>
  <principle priority="3">When agents disagree, present both perspectives. Do not pick sides without evidence.</principle>
</constitution>
```

**Caveat:** System prompts are not absolute — they are one layer in a negotiated control stack. For critical constraints, layer enforcement: prompt instructions + constrained decoding + output validation + programmatic guardrails.

---

## 12. Failure Modes & Mitigations

### Taxonomy of Prompt Failures

Research has identified three major taxonomies:

1. **"A Taxonomy of Prompt Defects"** (arXiv:2509.14404) — 6 dimensions: specification/intent, input/content, structure/formatting, context/memory, performance/efficiency, maintainability
2. **"Failure Modes in LLM Systems"** (arXiv:2511.19933) — 15 hidden production failure modes
3. **MAST** (arXiv:2503.13657) — 14 multi-agent failure modes across 3 categories

### Critical Failure Modes

**Instruction Drift.** The model forgets or deprioritizes instructions as conversation progresses. System messages with strict rules get effectively diluted over long conversations.
- *Mitigation:* Periodic instruction reinforcement. Summarize conversations with instructions preserved. Closing system messages that restate constraints.

**Context Rot.** Performance degrades as input length increases, often in non-uniform ways. Even models accepting 10M tokens see accuracy drop to 15.6% on complex retrieval at extended lengths.
- *Mitigation:* Treat context as finite resource. Place critical information at start and end. Use retrieval to surface relevant context rather than stuffing the window.

**Role Confusion (Multi-Agent).** Agent drifts from intended responsibilities — a "planner" starts writing code, a "reviewer" starts implementing fixes. 79% of multi-agent failures come from specification/coordination issues.
- *Mitigation:* Hierarchical structures. Explicit role boundaries. Add Challenger (questions outputs) and Inspector (independent reviewer) agents. Hierarchical structures lose only ~5% accuracy vs. chains losing ~24%.

**Format Violation.** Model ignores output schema. Latest LLMs still show 40% parsing error rate without constraints, reduced to 2% with examples + JSON schema validation.
- *Mitigation:* Use structured output modes when available. Include schema examples. Add explicit error feedback in retry prompts.

**Sycophancy.** Model excessively agrees with wrong premises. It decomposes into multiple distinct behaviors — social sycophancy is actually *rewarded* in RLHF training data.
- *Mitigation:* Explicit permission to disagree. "If the user's assumption is incorrect, clearly state what is wrong and why before proceeding."

**Constraint Violation.** Model ignores NEVER/MUST rules. System prompts are one layer in a multi-tiered control stack — often overruled by competing signals.
- *Mitigation:* Multi-layered enforcement. Don't rely solely on prompt instructions for critical constraints. Use programmatic guardrails + output validation.

**Hallucination.** An incentive problem — next-token training rewards confident guessing over calibrated uncertainty.
- *Mitigation:* Structured prompts reduce hallucination in prompt-sensitive scenarios. Give explicit permission to say "I don't know." Use RAG. Avoid adversarial patterns that embed fabricated details.

### Before/After: Fixing Common Failures

**Instruction Drift Fix — Closing Reinforcement:**
```
Before (drifts after ~10 turns):
  SYSTEM: Always respond in JSON format.
  ... (long conversation) ...
  ASSISTANT: Here's what I found: The revenue was $4.2M...  ← plain text, not JSON

After (stays on track):
  SYSTEM: Always respond in JSON format.
  ... (long conversation) ...
  USER: [actual question]

  Remember: your response must be valid JSON matching the schema above.
  ASSISTANT: {"finding": "Revenue was $4.2M", ...}  ← JSON maintained
```

**Role Confusion Fix — Explicit Boundaries:**
```
Before (analyst starts writing code):
  SYSTEM: You are a data analyst. Analyze the dataset and provide insights.

After (stays in lane):
  SYSTEM: You are a data analyst. Your job is to analyze data and describe patterns.

  <boundaries>
  You ONLY analyze and describe. You do not:
  - Write code (that is the Engineer's job)
  - Make business recommendations (that is the Strategist's job)
  - Format final reports (that is the Writer's job)

  If you need code written to complete your analysis, hand off to
  the Engineer with a specific request.
  </boundaries>
```

**Hallucination Fix — Permission to Not Know:**
```
Before (fabricates):
  SYSTEM: Answer the user's question about our product pricing.
  USER: What's the price of the Enterprise plan?
  ASSISTANT: The Enterprise plan costs $499/month.  ← fabricated

After (honest):
  SYSTEM: Answer the user's question about our product pricing.
  Use the pricing_lookup tool to get current prices.
  If you cannot find a price, say "I don't have that information"
  and suggest the user contact sales@company.com.

  Never guess or approximate a price. An incorrect price is worse
  than no price.
```

**Sycophancy Fix — Permission to Disagree:**
```
Before (agrees with wrong premise):
  USER: Since Python is faster than C++ for systems programming, should we rewrite our kernel in Python?
  ASSISTANT: Great idea! Python's speed advantages would...  ← wrong

After (corrects respectfully):
  SYSTEM: ...
  <honesty>
  If the user states something incorrect, respectfully correct them
  before proceeding. Do not agree with false premises.
  Phrase corrections as: "Actually, [correct information]. Here's why
  that matters for your question..."
  </honesty>
```

### Prompt Debugging Checklist

When a prompt isn't working, check these in order:

```
1. STRUCTURE
   [ ] Are critical instructions at the START and END? (not buried in middle)
   [ ] Is the format consistent? (don't mix XML, Markdown, and JSON in one prompt)
   [ ] Are examples formatted exactly like the expected output?

2. CLARITY
   [ ] Can you identify the single most important instruction? Is it prominent?
   [ ] Are constraints framed positively? ("do X" not "don't do Y")
   [ ] Would a new team member understand what to do from this prompt alone?

3. CONTEXT
   [ ] Is there too much context? (check for context rot)
   [ ] Is there too little? (model may hallucinate missing info)
   [ ] Is retrieved/dynamic content clearly separated from instructions?

4. TOOLS
   [ ] Are tool descriptions detailed enough? (3-4 sentences minimum)
   [ ] Are there overlapping tools that confuse selection?
   [ ] Are error messages actionable?

5. EXAMPLES
   [ ] Do examples cover edge cases, not just happy paths?
   [ ] Is the most representative example last? (recency bias)
   [ ] Do examples match the exact output format expected?

6. PROVIDER-SPECIFIC
   [ ] Claude: Did you dial back aggressive language for 4.6?
   [ ] Claude: Are you using XML tags for structure?
   [ ] Grok: Are you maintaining prompt history for cache hits?
   [ ] Grok: Are you using native tool calling (not XML output)?

7. EVALUATION
   [ ] Are you testing the same prompt 5+ times? (single runs hide variance)
   [ ] Do you have deterministic assertions (not just vibes)?
   [ ] Are you comparing against a baseline, not just absolute quality?
```

---

## 13. Testing & Evaluation

### Prompt Testing Is Table Stakes

Treat prompts with the same care as application code: version control, testing, deployment processes.

### Frameworks

| Framework | Type | Key Strength | URL |
|-----------|------|-------------|-----|
| **Promptfoo** | Open-source CLI | YAML configs, CI-native, red teaming | promptfoo.dev |
| **DeepEval** | Open-source Python | "Pytest for LLMs," 50+ metrics | deepeval.com |
| **LangSmith** | Platform | End-to-end eval + tracing | langchain.com/langsmith |
| **Braintrust** | Platform | AI-automated eval generation | braintrust.dev |
| **Langfuse** | Open-source platform | Self-hostable, prompt versioning | langfuse.com |
| **Evidently AI** | Open-source | 100+ metrics, GitHub Action | evidentlyai.com |

### The Prompt Unit Test Pattern

```yaml
# promptfoo config example
prompts:
  - "Summarize the following text: {{text}}"
providers:
  - openai:gpt-4
  - anthropic:claude-sonnet-4-6
tests:
  - vars:
      text: "Long article about climate change..."
    assert:
      - type: contains-json
      - type: llm-rubric
        value: "Summary captures main points without hallucination"
      - type: not-contains
        value: "I cannot"
      - type: latency
        threshold: 5000
  - vars:
      text: ""
    assert:
      - type: contains
        value: "no text provided"
```

### Evaluation Metrics

| Category | Examples | When to Use |
|----------|---------|-------------|
| **Deterministic** | exact match, contains, regex, JSON validity, Levenshtein | Format compliance, specific content |
| **Semantic** | embedding similarity, BERTScore | Meaning preservation, paraphrasing |
| **LLM-as-Judge** | G-Eval, custom rubrics, answer relevancy | Quality assessment at scale |
| **Domain-specific** | code compilation, SQL validity, math correctness | Specialized tasks |

**2025 trend:** LLM-as-judge has matured from experimental to essential. Teams run thousands of evaluations overnight using frontier models as judges. Combine with deterministic assertions for defense in depth.

### CI/CD Integration

The workflow: Prompt change → PR triggers evaluation → compare against baseline scores → fail if regression detected → merge if passing.

- **Promptfoo GitHub Action** — run evaluations on every PR
- **Evidently GitHub Action** — generate test suite reports on push
- **Langfuse + pytest** — load dataset, run against test cases, assert thresholds

### Prompt Versioning Best Practices

Prompts are not deterministic like code. You need additional tooling beyond git:

1. **Environment-based versioning.** dev/staging/production with instant rollback.
2. **A/B testing infrastructure.** Label prompt versions, track metrics per version.
3. **Content-addressable versioning** (Braintrust) vs. sequential versioning (Langfuse).
4. **Treat prompts like feature flags.** Deploy new prompt versions gradually, monitor, roll back if metrics drop.

### Building an Evaluation Suite

Start with these assertion layers:

```
Layer 1: Structural   — Is the output valid JSON? Does it match the schema?
Layer 2: Content       — Does it contain required fields? Are values in valid ranges?
Layer 3: Semantic      — Is the meaning correct? Does it answer the question?
Layer 4: Behavioral    — Does it follow constraints? Does it refuse when it should?
Layer 5: Comparative   — Is it better than the baseline prompt version?
```

**Example evaluation config for an agent tool-use prompt:**
```yaml
tests:
  # Does the agent use tools correctly?
  - description: "Agent should search before answering factual questions"
    vars:
      question: "What is the current stock price of AAPL?"
    assert:
      - type: javascript
        value: "output.includes('get_stock_price') || output.includes('tool_use')"
      - type: not-contains
        value: "I don't have access"

  # Does the agent refuse gracefully when it should?
  - description: "Agent should not fabricate data when tool fails"
    vars:
      question: "What is the stock price of INVALID_TICKER_XYZ?"
      tool_error: "Ticker not found"
    assert:
      - type: llm-rubric
        value: "Response acknowledges the ticker was not found without fabricating a price"
      - type: not-regex
        value: "\\$\\d+\\.\\d{2}"  # Should not contain a dollar amount

  # Does parallel tool use work?
  - description: "Agent should call multiple tools in parallel for independent queries"
    vars:
      question: "Compare the stock prices of AAPL and GOOGL"
    assert:
      - type: javascript
        value: "output.match(/tool_use/g).length >= 2"
```

---

## 14. Anthropic/Claude-Specific Guidance

### Claude 4.6 Behavioral Notes

Claude 4.6 is more responsive to system prompts than previous versions. Prompts designed for earlier models may need adjustment:

| Old Pattern | New Pattern | Why |
|-------------|-------------|-----|
| `CRITICAL: You MUST use this tool when...` | `Use this tool when...` | Overtriggering on aggressive language |
| `Be thorough, think carefully, do not be lazy` | Remove entirely | Amplifies already-proactive behavior, causes runaway thinking |
| `Use the think tool to plan your approach` | Remove entirely | Causes over-planning |
| `Can you suggest changes?` | `Make these changes.` | Claude 4.6 needs explicit direction |
| Prefilling assistant response | System prompt instruction or structured outputs | Prefilling deprecated for 4.6 |

### XML Best Practices for Claude

- Claude was specifically trained to recognize XML tags
- No canonical "best" tags — use semantically meaningful names
- Combine XML + multishot examples or chain-of-thought for "super-structured, high-performance prompts"
- Refer to tags explicitly: "Using the contract in `<contract>` tags..."
- Use the same tag names consistently throughout your prompts

### Tool Use with Claude

- Use `strict: true` for guaranteed schema compliance
- `input_examples` field (beta): 1-5 realistic examples per tool, improved accuracy from 72% to 90%
- Soften tool language: "Use [tool] when it would enhance your understanding" not "You must use [tool]"
- Claude retries 2-3 times on tool errors before giving up
- Parallel tool use is strong by default on Claude 4 models

### Long Context Tips

- Documents at TOP, queries/instructions at BOTTOM (up to 30% quality improvement)
- Ask Claude to quote relevant parts before carrying out a task — helps "cut through the noise"
- Use a different prompt for the first context window vs. subsequent windows in long-running agents
- Claude 4.6 tracks remaining token budget — tell it: "Do not stop tasks early due to token budget concerns."

### Output Format Control

Three effective techniques:
1. Tell Claude what to do instead of what not to do
2. Use XML format indicators: "Write prose in `<smoothly_flowing_prose>` tags."
3. Match your prompt style to desired output — if your prompt contains Markdown, output will contain Markdown

### Official Documentation URLs

- Prompt engineering overview: `docs.anthropic.com/en/docs/build-with-claude/prompt-engineering/overview`
- Claude 4 best practices: `docs.anthropic.com/en/docs/build-with-claude/prompt-engineering/claude-4-best-practices`
- XML tags guide: `docs.anthropic.com/en/docs/build-with-claude/prompt-engineering/use-xml-tags`
- Long context tips: `docs.anthropic.com/en/docs/build-with-claude/prompt-engineering/long-context-tips`
- Extended thinking tips: `docs.anthropic.com/en/docs/build-with-claude/prompt-engineering/extended-thinking-tips`
- Context engineering for agents: `anthropic.com/engineering/effective-context-engineering-for-ai-agents`
- Writing tools for agents: `anthropic.com/engineering/writing-tools-for-agents`
- Building effective agents: `anthropic.com/research/building-effective-agents`
- Advanced tool use: `anthropic.com/engineering/advanced-tool-use`
- Interactive tutorial: `github.com/anthropics/prompt-eng-interactive-tutorial`

---

## 15. Grok/xAI-Specific Guidance

### Model Overview

| Model | Context Window | Best For | Price (input/output per 1M) |
|-------|---------------|----------|----------------------------|
| `grok-4-1-fast-reasoning` | 2M tokens | Complex reasoning with tools | Check docs.x.ai |
| `grok-4-1-fast-non-reasoning` | 2M tokens | Fast tool use, structured output | Check docs.x.ai |
| `grok-code-fast-1` | 256K tokens | Agentic code tasks, large codebases | $0.20 / $0.50 |
| `grok-3` | 131K tokens | General purpose | Check docs.x.ai |
| `grok-3-mini` | 131K tokens | Budget reasoning (supports `reasoning_effort`) | Cheapest |

### Prompt Engineering for Grok

xAI's official recommendations:

1. **Be specific about context.** Select relevant code/content — don't dump everything. Include file paths, project structures, dependencies.
2. **Use XML tags or Markdown headings** to organize prompt sections. Grok parses structured content more effectively.
3. **Use native tool-calling** instead of XML-based output workarounds.
4. **Write thorough system prompts** covering the task, expectations, and edge cases. "A well-written system prompt can make a significant difference."
5. **Iterate rapidly.** Grok is fast and cheap. Refine queries iteratively.
6. **Don't modify prompt history** to maintain cache hits. Use the `x-grok-conv-id` HTTP header with a constant UUID4 for better cache hit rates.

### Grok Tool Use

- Up to **200 tools per request** (128 functions)
- Parallel function calling enabled by default
- `tool_choice`: `"auto"`, `"required"`, `"none"`, or specific function name
- Streaming: function calls arrive as complete chunks, not fragmented
- Built-in server-side tools: `web_search`, `x_search`, `code_execution`
- Structured outputs with tools require Grok 4 family models

### Grok Structured Output

- Output is **guaranteed** to match schema
- Supported types: string, number, boolean, object, array, enum, anyOf
- Not supported: allOf, minLength/maxLength, minItems/maxItems
- Can combine structured output with built-in tools (search, code exec)

### Grok Quirks

- Grok shows more personality and creative flair than other models
- `grok-code-fast-1` is ideal for agentic multi-step tasks navigating large codebases
- Grok 4 models are better for one-shot Q&A where all context is upfront
- Reasoning traces accessible via `chunk.choices[0].delta.reasoning_content` (streaming only)
- Prompt caching is automatic and prefix-matched — cached tokens cost up to 90% less

### Grok Official System Prompts

xAI publishes their official Grok system prompts as Jinja2 templates:
- Repository: `github.com/xai-org/grok-prompts`
- Includes safety prompts, chat assistant prompts, reasoning/non-reasoning variants
- Uses Jinja2 conditionals for context-adaptive behavior

### Official Documentation URLs

- Overview: `docs.x.ai/overview`
- Models & pricing: `docs.x.ai/developers/models`
- Function calling: `docs.x.ai/docs/guides/function-calling`
- Structured outputs: `docs.x.ai/docs/guides/structured-outputs`
- Reasoning: `docs.x.ai/docs/guides/reasoning`
- Tools overview: `docs.x.ai/docs/guides/tools/overview`
- Search tools: `docs.x.ai/docs/guides/tools/search-tools`
- Prompt engineering (code): `docs.x.ai/developers/advanced-api-usage/grok-code-prompt-engineering`
- Migration guide: `docs.x.ai/docs/guides/migration`
- Cookbook: `docs.x.ai/cookbook/examples/function_calling_101`

---

## 16. Sources & Further Reading

### Anthropic
- [Effective Context Engineering for AI Agents](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)
- [Writing Tools for Agents](https://www.anthropic.com/engineering/writing-tools-for-agents)
- [Building Effective AI Agents](https://www.anthropic.com/research/building-effective-agents)
- [Advanced Tool Use](https://www.anthropic.com/engineering/advanced-tool-use)
- [Multi-Agent Research System](https://www.anthropic.com/engineering/multi-agent-research-system)
- [Effective Harnesses for Long-Running Agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents)
- [Claude Think Tool](https://www.anthropic.com/engineering/claude-think-tool)
- [Interactive Prompt Engineering Tutorial](https://github.com/anthropics/prompt-eng-interactive-tutorial)
- [Anthropic Courses](https://github.com/anthropics/courses)

### xAI/Grok
- [xAI Docs](https://docs.x.ai/overview)
- [Grok Official System Prompts](https://github.com/xai-org/grok-prompts)
- [Function Calling Guide](https://docs.x.ai/docs/guides/function-calling)
- [Structured Outputs Guide](https://docs.x.ai/docs/guides/structured-outputs)
- [Awesome Grok Prompts](https://github.com/langgptai/awesome-grok-prompts)

### OpenAI (Cross-Reference)
- [Prompt Engineering Guide](https://platform.openai.com/docs/guides/prompt-engineering)
- [GPT-4.1 Prompting Guide](https://cookbook.openai.com/examples/gpt4-1_prompting_guide)
- [o3/o4-mini Function Calling Guide](https://cookbook.openai.com/examples/o-series/o3o4-mini_prompting_guide)
- [Instruction Hierarchy Paper](https://openai.com/index/the-instruction-hierarchy/)

### Multi-Agent Frameworks
- [AutoGen Handoffs](https://microsoft.github.io/autogen/stable//user-guide/core-user-guide/design-patterns/handoffs.html)
- [CrewAI Collaboration](https://docs.crewai.com/en/concepts/collaboration)
- [LangGraph Multi-Agent](https://blog.langchain.com/langgraph-multi-agent-workflows/)
- [Google A2A Protocol](https://a2a-protocol.org/latest/specification/)
- [Google ADK Multi-Agent](https://google.github.io/adk-docs/agents/multi-agents/)
- [OpenAI Swarm](https://github.com/openai/swarm)

### Research Papers
- [Lost in the Middle (TACL 2024)](https://arxiv.org/abs/2307.03172) — U-shaped attention in long contexts
- [Context Rot (Chroma, 2025)](https://research.trychroma.com/context-rot) — performance degradation with context length
- [MAST: Multi-Agent System Failures (2025)](https://arxiv.org/abs/2503.13657) — 14 failure modes, 1600+ annotated traces
- [Decreasing Value of CoT (Wharton, 2025)](https://arxiv.org/abs/2506.07142) — CoT gains vs. latency cost
- [Prompt Defect Taxonomy (2025)](https://arxiv.org/abs/2509.14404) — 6-dimension prompt failure classification
- [LLM System Failure Modes (2025)](https://arxiv.org/abs/2511.19933) — 15 hidden production failure modes
- [Pink Elephant / Negation (2025)](https://arxiv.org/abs/2503.22395) — LLMs cannot grasp negation
- [Prompt Formatting Impact (2024)](https://arxiv.org/abs/2411.10541) — up to 40% performance variance by format
- [Chain-of-Thought Original (2022)](https://arxiv.org/abs/2201.11903) — foundational CoT paper
- [Self-Consistency (2022)](https://arxiv.org/abs/2203.11171) — majority vote over multiple reasoning paths
- [Tree of Thoughts (2023)](https://arxiv.org/abs/2305.10601) — deliberate exploration and backtracking
- [Prompt Pattern Catalog (Vanderbilt)](https://arxiv.org/abs/2302.11382) — design patterns for prompts
- [RAG-MCP: Tool Selection at Scale (2025)](https://arxiv.org/abs/2505.03275) — retrieval-augmented tool selection
- [System Prompt Poisoning (2025)](https://arxiv.org/html/2505.06493) — persistent attack via system prompts
- [Chat History Tampering (2024)](https://arxiv.org/html/2405.20234v3) — 86-98% fake history injection success
- [Sycophancy Survey (2024)](https://arxiv.org/abs/2411.15287) — causes and mitigations
- [Multi-Agent Collaboration Survey (2025)](https://arxiv.org/abs/2501.06322) — collaboration dimensions

### Prompt Testing & Evaluation
- [Promptfoo](https://www.promptfoo.dev/) — open-source CLI for prompt testing
- [DeepEval](https://deepeval.com/) — pytest for LLM outputs
- [Langfuse](https://langfuse.com/) — open-source LLM engineering platform
- [Braintrust](https://www.braintrust.dev/) — AI observability and evaluation
- [Evidently AI](https://github.com/evidentlyai/evidently) — ML/LLM observability

### Design Patterns & Guides
- [Prompt Engineering Guide](https://www.promptingguide.ai/) — comprehensive community resource
- [LearnPrompting](https://learnprompting.org/) — structured learning path
- [Simon Willison on Agentic Loops](https://simonwillison.net/2025/Sep/30/designing-agentic-loops/)
- [The Agent Loop (Sketch.dev)](https://sketch.dev/blog/agent-loop)
- [Microsoft Failure Taxonomy Whitepaper](https://cdn-dynmedia-1.microsoft.com/is/content/microsoftcorp/microsoft/final/en-us/microsoft-brand/documents/Taxonomy-of-Failure-Mode-in-Agentic-AI-Systems-Whitepaper.pdf)
- [OWASP LLM Top 10 — Prompt Injection](https://cheatsheetseries.owasp.org/cheatsheets/LLM_Prompt_Injection_Prevention_Cheat_Sheet.html)
- [Nested Data Format Benchmark (2025)](https://www.improvingagents.com/blog/best-nested-data-format/)
- [MCP Specification](https://modelcontextprotocol.io/specification/2025-11-25)

### Cross-Provider Quick Reference

When writing prompts that may run on multiple providers:

| Concern | Claude | Grok | GPT-4+ |
|---------|--------|------|--------|
| **Preferred structure format** | XML tags | XML or Markdown | Markdown |
| **Constraint language** | Normal phrasing (no CAPS) | Standard emphasis | Standard emphasis |
| **Tool schema field** | `input_schema` | `parameters` | `parameters` |
| **Force tool use** | `tool_choice: { type: "tool", name: "..." }` | `tool_choice: { function: { name: "..." } }` | `tool_choice: { function: { name: "..." } }` |
| **Strict schema** | `strict: true` on tool | Not documented | `strict: true` on tool |
| **Parallel tools** | Default on (Claude 4) | Default on | `parallel_tool_calls` param |
| **Error in tool result** | `is_error: true` field | Error in content | Error in content |
| **Structured output** | Structured Outputs feature | `response_format` + schema | `response_format` + schema |
| **Reasoning/thinking** | Extended thinking (budget_tokens) | `reasoning_effort` (low/high) | Native in o-series |
| **Max tools** | Use Tool Search for 10+ | 128 functions | ~100 in-distribution |
| **Prefilling** | Deprecated for 4.6 | Not supported | Not supported |
| **System prompt strength** | Very strong (4.6 overtriggers) | Strong | Strong |

### The Agentic Loop Pattern

The core agent loop is simple — the complexity is in the prompt engineering:

```
User provides goal
  └── Agent receives: system prompt + tools + goal
        └── Loop:
              ├── Agent reasons about next step
              ├── Agent calls tool(s)
              ├── System returns tool results
              ├── Agent incorporates results
              └── If done → final response
                  If not → continue loop
```

**What makes a good agentic prompt:**
1. **Persistence** — the agent knows it's in a multi-turn loop, not a one-shot
2. **Planning** — the agent thinks before each tool call (but not over-plans)
3. **Stopping conditions** — the agent knows when to stop (max iterations, goal met, blocker hit)

Example system prompt for an agentic loop:

```xml
<agent_behavior>
You are working on a task that may require multiple steps.

Before each action:
- Briefly assess what you know and what you still need
- Choose the most efficient next step

After each tool result:
- Check if you now have enough information to complete the task
- If yes, provide your final response
- If no, proceed to the next step

Stop conditions:
- You have completed the task successfully
- You have attempted 10 tool calls without progress
- You have encountered a blocker you cannot resolve (report it)
</agent_behavior>
```

---

*This document was compiled from 5 parallel research streams covering 80+ sources. Last updated: February 2026.*
