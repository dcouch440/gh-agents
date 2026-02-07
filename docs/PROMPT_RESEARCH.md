# Prompt Engineering Research

Reference document for upgrading nexor's framework-level prompts to enterprise grade. Compiled from Anthropic docs, OpenAI docs, academic papers, and multi-agent framework analysis.

---

## Table of Contents

1. [Core Principles (Anthropic)](#1-core-principles-anthropic)
2. [XML Tag Structuring](#2-xml-tag-structuring)
3. [System Prompt vs User Message](#3-system-prompt-vs-user-message)
4. [Structured Output Techniques](#4-structured-output-techniques)
5. [Chain-of-Thought Patterns](#5-chain-of-thought-patterns)
6. [Few-Shot / Multishot Prompting](#6-few-shot--multishot-prompting)
7. [Task Decomposition Prompts](#7-task-decomposition-prompts)
8. [Routing / Classification Prompts](#8-routing--classification-prompts)
9. [LLM-as-Judge / Review Prompts](#9-llm-as-judge--review-prompts)
10. [Multi-Agent Debate / Verification](#10-multi-agent-debate--verification)
11. [Negative Instructions (Anti-Pattern)](#11-negative-instructions-anti-pattern)
12. [Agent Description vs System Message](#12-agent-description-vs-system-message)
13. [Context Engineering for Agents](#13-context-engineering-for-agents)
14. [Quantitative Results Summary](#14-quantitative-results-summary)
15. [Master Do's and Don'ts](#15-master-dos-and-donts)

---

## 1. Core Principles (Anthropic)

**Source:** [Anthropic Prompt Engineering Docs](https://platform.claude.com/docs/en/docs/build-with-claude/prompt-engineering/overview)

### The Golden Rule
> Show your prompt to a colleague with minimal context. If they're confused, Claude will be too.

### Be Clear and Direct
- Provide context: what the results will be used for, what audience, what workflow
- Be specific about what you want — if you want only code, say so
- Use sequential numbered steps for exact execution order
- Tell Claude **WHY**, not just **WHAT** — Claude generalizes from explanations

### Claude 4.x Specific
- More responsive to system prompts — dial back from "CRITICAL: You MUST" to "Use X when..."
- Pays extremely close attention to details and examples
- Naturally recognizes when tasks benefit from sub-agents
- Use `effort` parameter to control thinking depth
- Sensitive to the word "think" — use "consider," "evaluate," "analyze" instead

### Recommended Technique Priority (Most to Least Effective)
1. Be clear and direct
2. Use examples (multishot)
3. Let Claude think (chain of thought)
4. Use XML tags
5. Give Claude a role (system prompts)
6. Chain complex prompts
7. Long context tips

---

## 2. XML Tag Structuring

**Source:** [Anthropic XML Tags Guide](https://platform.claude.com/docs/en/docs/build-with-claude/prompt-engineering/use-xml-tags)

### Why XML Tags
- **Clarity**: Clearly separate different prompt parts
- **Accuracy**: Reduce misinterpretation
- **Flexibility**: Easy to find, add, remove, modify parts
- **Parseability**: Makes post-processing easier

### No Canonical Tags
There are no "official" tag names. Use names that make sense with the content they surround.

### Best Practices
- Be consistent — use the same tags throughout and refer to them ("Using the contract in `<contract>` tags...")
- Nest tags for hierarchy: `<outer><inner></inner></outer>`
- Combine with other techniques: `<examples>` + `<thinking>` + `<answer>` = "super-structured, high-performance prompts"

### Demonstrated Impact
Without XML tags, Claude misunderstood a financial report task and generated wrong structure/tone. With XML tags wrapping data, instructions, and formatting examples separately, output was concise and correctly structured.

---

## 3. System Prompt vs User Message

**Source:** [Anthropic System Prompts Guide](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/system-prompts)

### The Split
- **System prompt**: Identity, role, behavioral rules (cacheable)
- **User message**: Task instructions, dynamic data (per-execution)

### Role Prompting Impact
- Generic legal analysis: "The agreement seems standard."
- Role-prompted (General Counsel): "I have serious concerns that could expose our company to significant risks..."
- The role-prompted version caught issues the generic version dismissed

### Specificity Matters
"A `data scientist` might see different insights than a `marketing strategist`. A `data scientist specializing in customer insight analysis for Fortune 500 companies` might yield different results still."

---

## 4. Structured Output Techniques

### API-Level Constrained Decoding (Gold Standard)

**Sources:** [Anthropic Structured Outputs](https://platform.claude.com/docs/en/build-with-claude/structured-outputs), [JSONSchemaBench](https://arxiv.org/html/2501.10868v1)

Both Anthropic and OpenAI now offer constrained decoding — the model literally cannot produce tokens that violate your schema. Zero JSON parsing errors, guaranteed schema compliance.

**When NOT to use:** Complex reasoning tasks can lose 27 percentage points of accuracy with structural constraints (GPT-4o-mini on Shuffled Objects: 92.68% unconstrained → 65.85% structured). Always benchmark.

### Schema Design (Single Biggest Lever)

**Source:** [Instructor Library Research](https://python.useinstructor.com/blog/2024/09/26/bad-schemas-could-break-your-llm-structured-outputs/)

| Technique | Impact |
|-----------|--------|
| Field naming (`final_choice` → `answer`) | 4.5% → 95% accuracy |
| Adding `reasoning` field before `answer` | 33% → 92% accuracy |
| Field ordering (reasoning BEFORE answer) | Critical — defeats purpose if reversed |
| Field descriptions with ranges/examples | Significant compliance improvement |

**Key rules:**
- Use clear, domain-appropriate field names
- Put reasoning/chain-of-thought fields FIRST in the schema
- Include descriptions with valid ranges, format expectations, examples
- JSON Mode has 50% more variation than tool calling — prefer tool/function calling

### Prompt-Based JSON (When No API Constraints Available)

**Priority order:**
1. Frame positively: "Return raw JSON parsed by JSON.parse()" not "Do NOT wrap in markdown"
2. Add context: "The consumer parses raw JSON directly, so wrapper text causes parsing errors"
3. Provide 1-3 examples for 15-40% accuracy improvement
4. Set temperature 0.0-0.2 for format compliance
5. Implement retry with error feedback as safety net

### Tool Use Examples (Anthropic)

**Source:** [Anthropic Advanced Tool Use](https://www.anthropic.com/engineering/advanced-tool-use)

1-5 realistic examples per tool improved accuracy from **72% → 90%**. Examples should show:
- Minimal, partial, and full specification patterns
- Format conventions ("YYYY-MM-DD")
- ID conventions ("USR-12345")
- Parameter correlations

---

## 5. Chain-of-Thought Patterns

**Source:** [Anthropic CoT Guide](https://platform.claude.com/docs/en/docs/build-with-claude/prompt-engineering/chain-of-thought)

### Three Levels

**1. Basic:** "Think step-by-step" (lacks guidance on HOW)

**2. Guided:** Outline specific thinking steps
```
Think before you write the email. First, think through what messaging might appeal
to this donor given their donation history. Then, think through what aspects of the
program would appeal to them. Finally, write the personalized email.
```

**3. Structured (Best):** Use XML tags to separate thinking from answer
```
Think before you write the email in <thinking> tags. First, think through what
messaging might appeal... Finally, write the personalized donor email in <email> tags.
```

### Critical Rule
**Always have Claude OUTPUT its thinking.** Without outputting the thought process, no thinking occurs.

### When NOT to Use
- Simple extraction/classification tasks (adds latency with no benefit)
- Tasks a human wouldn't need to "think through"

### Extended Thinking (Claude-Specific)
- Start with general instructions, NOT prescriptive step-by-step
- Claude's creativity in approaching problems may exceed your ability to prescribe the optimal process
- Grammar state resets between thinking and response — thinking is unconstrained, output follows schema
- Minimum budget: 1024 tokens

---

## 6. Few-Shot / Multishot Prompting

**Source:** [Anthropic Multishot Guide](https://platform.claude.com/docs/en/docs/build-with-claude/prompt-engineering/multishot-prompting)

### Core Finding
**3-5 diverse, relevant examples. More = better, especially for complex tasks.**

### Why It Works
- Reduces misinterpretation of instructions
- Enforces uniform structure and style
- Boosts complex task handling

### Crafting Examples
- **Relevant**: Mirror actual use case
- **Diverse**: Cover edge cases; vary enough to avoid unintended patterns
- **Clear**: Wrap in `<example>` tags, nest multiple in `<examples>`

### Quantitative Impact
- 15-40% accuracy improvement on structured output
- Fortune 500 company: +20% accuracy with scratchpad + few-shot + SME guidance
- Diminishing returns beyond 3-5 examples

### When Few-Shot Hurts
- Can anchor models too strongly on specific patterns
- Examples must be accurate — wrong examples teach wrong behavior

---

## 7. Task Decomposition Prompts

### How Leading Frameworks Do It

**CrewAI agent definition:**
```
Role: {professional identity}
Goal: {objective guiding decisions}
Backstory: {context establishing expertise}
```
Key insight: **80% of effort should go into designing tasks, only 20% into defining agents.**

**LangGraph collaboration prompt:**
```
You are a helpful AI assistant, collaborating with other assistants. Use the provided
tools to progress towards answering the question. If you are unable to fully answer,
that's OK, another assistant with different tools will help where you left off. Execute
what you can to make progress. If you or any of the other assistants have the final
answer or deliverable, prefix your response with FINAL ANSWER so the team knows to stop.
```

Key language patterns:
- "collaborating with other assistants" — establishes cooperative context
- "that's OK, another assistant will help" — prevents over-reaching
- "Execute what you can to make progress" — encourages incremental contribution
- "prefix with FINAL ANSWER" — explicit termination signal

### Decomposition Techniques

**DecomP (Decomposed Prompting):**
Uses explicit operation markers: `[split]`, `[merge]`, `(foreach)`, `[EOQ]`

**Calibration principle:**
> "Decompose enough to make tasks tractable, but not so much that coordination overhead dominates."

**ADAPT (As-Needed Decomposition):**
Recursively decomposes only as needed, adapting to both task complexity and LLM capabilities.

### Cognitive Verbs (Research Findings)

**Source:** [Prompt Vocabulary Research](https://arxiv.org/html/2505.17037v1)

LLMs perform best with **moderately specific verbs** (8.08-10.57 on specificity scale). Maximally specific verbs **degrade reasoning** significantly (correlations of -0.89 and -0.87).

| Good | Bad |
|------|-----|
| "Analyze" | "Microscopically dissect" |
| "Evaluate" | "Adjudicate the merits of" |
| "Review" | "Forensically examine" |
| "Consider" | "Exhaustively enumerate" |

**Effective verb patterns by role:**
- **Planner**: "analyze," "break down," "identify dependencies"
- **Reflector**: "review," "verify that the answer is based on," "criticize"
- **Refiner**: "carefully consider where you could go wrong," "using insights from previous attempts"

### Preventing Over/Under-Decomposition

- Multi-granularity approach: explicitly mention decomposition strategies in the prompt
- Result: significant improvement in accuracy, reduction in redundant tasks
- ACONIC framework (constraint-based): **10-40 percentage point improvements**

---

## 8. Routing / Classification Prompts

### Reliability Techniques

**Source:** [LLM Agent Routing Survey](https://arxiv.org/html/2502.00409v2)

**Key findings:**
- Domain-based routing declined to **53% accuracy** in unconstrained "Other" categories
- Routing works best with well-defined, bounded categories
- Hybrid: semantic search for broad categorization, then classifier LLM for fine-grained

### Agent Description Best Practices

**Source:** [Google Agent Development Kit](https://google.github.io/adk-docs/agents/llm-agents/), [AutoGen](https://microsoft.github.io/autogen/0.2/blog/2023/12/29/AgentDescriptions/)

Format: `"[Verb] the [object] for a given [input]"`
Example: `"Retrieves the capital city for a given country"`

**Using dedicated descriptions vs system messages for routing roughly doubled correct selection rates and reduced distraction callouts by ~50%.**

**Description rules:**
- Third-person (avoid "I" or "You")
- State capabilities relevant for routing decisions
- Under ~20 words
- Describe what the agent does, not how it instructs itself
- Specific enough to differentiate from peers

### Quantitative Routing Results

| Approach | Result |
|----------|--------|
| RouteLLM (preference weighting) | 80% GPT-4 quality with 30% of calls |
| FrugalGPT (confidence thresholding) | 59-98% cost savings vs GPT-4 |
| Supervised BERT classifiers | 87.7% MMLU |
| Knowledge graph routing | 4-21 point improvements |

---

## 9. LLM-as-Judge / Review Prompts

### The G-Eval Framework

**Source:** [G-Eval Guide](https://www.confident-ai.com/blog/g-eval-the-definitive-guide)

Three-step process:
1. LLM transforms criterion into structured evaluation steps
2. Steps become chain-of-thought guidance for judging
3. Judgments weighted by log-probabilities for final score

### Preventing Rubber-Stamping and Nitpicking

**Source:** [Monte Carlo](https://www.montecarlodata.com/blog-llm-as-judge/), [Evidently AI](https://www.evidentlyai.com/llm-guide/llm-as-a-judge), [Arize AI](https://arize.com/blog/evidence-based-prompting-strategies-for-llm-as-a-judge-explanations-and-chain-of-thought/)

**7 key techniques:**

1. **Few-Shot**: Include one scored example. Diminishing returns with multiple.

2. **Step Decomposition**: Break complex judgments into smaller reasoning steps.

3. **Single Criterion Per Judge**: Never combine multiple dimensions in one prompt.

4. **Integer Rubric with Score Definitions**:
   | Score | Label | Description |
   |-------|-------|-------------|
   | 3 | Highly Relevant | Directly answers without extraneous information |
   | 2 | Mostly Relevant | Addresses core but may include minor irrelevant details |
   | 1 | Not Relevant | Fails to answer or provides unrelated information |

5. **Structured JSON Output**: `{"judgment": "[LABEL]", "reasoning": "[EXPLANATION]"}`

6. **Explanation-First (Recommended Default)**: Reason tied to rubric, then output score. Explanations **reduce variance** and increase human agreement.

7. **Score Smoothing**: Aggregate over time, re-run soft failures.

### Critical Finding on CoT in Judging

> "There is little evidence to favor CoT over simpler prompting strategies for NLG evaluation."

CoT only helps for tasks requiring multi-hop factual verification. For qualitative evaluations, it had **neutral or negative effects**. What DOES help: requiring **explanations** (not CoT). The explanation-first pattern is the recommended default.

### Bias Mitigation
- **Position bias**: Randomize response order
- **Verbosity bias**: Models prefer longer answers; use direct scoring
- **Self-enhancement bias**: Models favor own outputs

### Concrete Templates

**Binary classification:**
```
Evaluate the following for [CRITERION]. A [POSITIVE] response [description].
A [NEGATIVE] response [description]. Return: '[POSITIVE]' or '[NEGATIVE]'.
```

**Faithfulness:**
```
Evaluate RESPONSE for faithfulness to CONTEXT. A faithful response includes only
context-present information, avoids inventing details, doesn't contradict context.
Return: 'Faithful' or 'Not Faithful'.
```

### Quantitative Benchmark
Properly configured LLM judges: **80%+ agreement** with human evaluators (MT-Bench). Prompt quality matters more than model selection.

---

## 10. Multi-Agent Debate / Verification

### Debate Prompt Structure

**Source:** [Multiagent Debate Paper](https://arxiv.org/abs/2305.14325)

**Initial round:**
```
Can you solve the following math problem?
{problem}
Explain your reasoning. Your final answer should be a single numerical number,
in the form of {{answer}}
```

**Refinement round:**
```
These are the solutions to the problem from other agents:
<other agent responses>
Using the solutions from other agents as additional information, can you provide
your answer to the math problem?
```

**Configuration**: 3 agents, 2-3 rounds. Performance improves with more agents and rounds.

### Agent Role Design

- Assigned personas ("affirmative"/"negative", "verifier"/"solver")
- Heterogeneous roles (verifier + solver) outperform homogeneous debates
- **Moderate disagreement** achieves best performance — "tit for tat" controlled disagreement
- Judge/moderator manages rounds in discriminative, extractive, or adjudicating mode

### Quantitative Results
- Tool-MAD framework: up to **35.5% improvement** over standard debate
- Multi-agent debate improves accuracy across six benchmarks
- Multi-agent orchestration: **100% actionable recommendation rate** vs 1.7% for single-agent (incident response)
- Performance scales with agents and rounds

---

## 11. Negative Instructions (Anti-Pattern)

### The Pink Elephant Problem

> Telling someone "don't think of a pink elephant" makes them think of it.

**Findings:**
- Users report "NEVER create duplicate files" rules being violated
- LLMs "produce worse output" with more "DO NOTs"
- Changing "do not make new versions" → "Make all updates in current files" saw immediate improvement

### Anthropic's Official Stance

> "Tell Claude what to do instead of what not to do."

Instead of "NEVER use ellipses," say: "Your response will be read aloud by a text-to-speech engine, so never use ellipses since the text-to-speech engine will not know how to pronounce them." — WHY context helps Claude generalize.

### Transformation Pattern

| Bad (Negative) | Good (Positive) | Best (Positive + WHY) |
|----------------|-----------------|----------------------|
| "Do NOT wrap JSON in markdown" | "Return raw JSON only" | "Return raw JSON only. The output is parsed by JSON.parse(), so wrapper text causes parsing errors." |
| "Do NOT include explanatory text" | "Output only the JSON object" | "Output only the JSON object. A downstream parser reads your response directly." |
| "NEVER use values not in the list" | "Use exactly one value from the list below" | "Use exactly one value from the list below. Unknown values cause routing failures." |

### When Negatives Work
- Hard safety/ethical boundaries
- System prompts for firm limits (not soft preferences)
- When paired with a positive alternative and WHY context

---

## 12. Agent Description vs System Message

### Two Separate Concerns

**Source:** [AutoGen Agent Descriptions](https://microsoft.github.io/autogen/0.2/blog/2023/12/29/AgentDescriptions/)

| Concern | Purpose | Audience | Style |
|---------|---------|----------|-------|
| Description | For routing/orchestration | Other agents / orchestrator | Third-person, capability-focused, <20 words |
| System message | For behavior/identity | The agent itself | Second-person, detailed instructions |

### Impact
Using dedicated descriptions (vs system messages) for orchestration **roughly doubled correct speaker selection** and **reduced distraction callouts by ~50%**.

### Good Description Examples
- `"A helpful AI assistant with strong language skills, Python skills, and Linux command line skills."`
- `"Retrieves the capital city for a given country"`
- `"Handles inquiries about current billing statements"`

### Bad Description Examples
- `"I am a helpful assistant"` (first person)
- `"Billing agent"` (too vague to differentiate)
- `"You should handle billing queries and also sometimes do refunds and..."` (instruction-style, too long)

---

## 13. Context Engineering for Agents

**Source:** [Anthropic Context Engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)

### Core Definition
> "The set of strategies for curating and maintaining the optimal set of tokens during LLM inference."

### Context Rot
As token count increases, recall accuracy decreases. LLMs have finite "attention budgets."

### System Prompt Altitude
- **Too specific**: Hardcoded if-else logic creates fragility
- **Too vague**: High-level guidance fails to signal desired outputs
- **Ideal**: "Specific enough to guide behavior, flexible enough to provide strong heuristics"

### Tool Design
- Self-contained, robust to error, clear about intended use
- Input parameters must be descriptive and unambiguous
- **Human-engineer test**: "If a human engineer can't definitively say which tool should be used, an AI agent can't either"

### Long Context Tips
- Put long documents at TOP, query/instructions at BOTTOM
- **Queries at end improve quality by up to 30%**
- Structure documents with `<document index="1"><source>...</source><document_content>...</document_content></document>`
- Ask Claude to quote relevant parts FIRST, then analyze

### Guiding Principle
> "Find the smallest set of high-signal tokens that maximize the likelihood of your desired outcome."

---

## 14. Quantitative Results Summary

| Technique | Impact | Source |
|-----------|--------|--------|
| Schema field naming (`final_choice` → `answer`) | 4.5% → 95% accuracy | Instructor |
| Chain-of-thought field in schema | 33% → 92% accuracy | Instructor |
| Tool use examples (1-5 per tool) | 72% → 90% accuracy | Anthropic |
| Few-shot prompting | +15-40% accuracy | Multiple |
| Scratchpad + few-shot + SME guidance | +20% accuracy | Fortune 500 case study |
| Queries at end of long context | +30% quality | Anthropic |
| Dedicated agent descriptions | 2x correct selection | AutoGen |
| Multi-agent orchestration (incident response) | 1.7% → 100% actionable | Academic |
| Prompt optimization vs adding agents | +6% (more cost-effective) | Academic |
| Constrained decoding speed | +50% generation speed | JSONSchemaBench |
| Tool search (dynamic loading) | 49% → 74% accuracy | Anthropic |
| Multi-agent debate | +35.5% over baseline | Tool-MAD |
| Moderate verb specificity | -0.89 correlation with max specificity | Academic |

---

## 15. Master Do's and Don'ts

### DO
- Define success criteria and tests before prompt engineering
- Put long documents at TOP, instructions at BOTTOM
- Use XML tags to separate prompt components
- Provide 3-5 diverse, relevant examples
- Tell Claude WHY, not just WHAT
- Use structured CoT with `<thinking>` and `<answer>` tags
- Use the `system` parameter for role assignment
- Chain multi-step tasks into separate prompts
- Put reasoning fields BEFORE answer fields in schemas
- Use moderately specific verbs ("analyze" not "microscopically dissect")
- Design agent descriptions separately from system messages
- Use one criterion per judge prompt
- Require explanations (not CoT) in judge prompts
- Frame instructions positively with WHY context
- Start with minimal prompt, add instructions based on failure modes

### DON'T
- Don't handle everything in a single massive prompt
- Don't use vague instructions without specifying format/length
- Don't rely on Claude to infer norms and preferences
- Don't put queries before long documents (up to 30% quality loss)
- Don't use "CRITICAL: You MUST" language in Claude 4.x (overtriggering)
- Don't use maximally specific verbs for reasoning tasks
- Don't combine multiple evaluation criteria in one judge prompt
- Don't use negatives without positive alternatives and WHY context
- Don't use system messages for routing decisions (use descriptions)
- Don't assume structured outputs always help (can hurt reasoning by 27%)

---

## Sources

### Anthropic Official
- [Prompt Engineering Overview](https://platform.claude.com/docs/en/docs/build-with-claude/prompt-engineering/overview)
- [Be Clear and Direct](https://platform.claude.com/docs/en/docs/build-with-claude/prompt-engineering/be-clear-and-direct)
- [Multishot Prompting](https://platform.claude.com/docs/en/docs/build-with-claude/prompt-engineering/multishot-prompting)
- [Chain of Thought](https://platform.claude.com/docs/en/docs/build-with-claude/prompt-engineering/chain-of-thought)
- [XML Tags](https://platform.claude.com/docs/en/docs/build-with-claude/prompt-engineering/use-xml-tags)
- [System Prompts](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/system-prompts)
- [Chain Complex Prompts](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/chain-prompts)
- [Long Context Tips](https://platform.claude.com/docs/en/docs/build-with-claude/prompt-engineering/long-context-tips)
- [Extended Thinking Tips](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/extended-thinking-tips)
- [Claude 4.x Best Practices](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-4-best-practices)
- [Structured Outputs](https://platform.claude.com/docs/en/build-with-claude/structured-outputs)
- [Context Engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)
- [Advanced Tool Use](https://www.anthropic.com/engineering/advanced-tool-use)

### Multi-Agent Frameworks
- [AutoGen Agent Descriptions](https://microsoft.github.io/autogen/0.2/blog/2023/12/29/AgentDescriptions/)
- [AutoGen Multi-Agent Debate](https://microsoft.github.io/autogen/stable/user-guide/core-user-guide/design-patterns/multi-agent-debate.html)
- [CrewAI Agents](https://docs.crewai.com/en/concepts/agents)
- [LangGraph Multi-Agent Workflows](https://blog.langchain.com/langgraph-multi-agent-workflows/)
- [Google Agent Development Kit](https://google.github.io/adk-docs/agents/llm-agents/)
- [OpenAI Agents SDK](https://openai.github.io/openai-agents-python/multi_agent/)

### Academic
- [Multi-Agent Prompt Optimization](https://arxiv.org/html/2502.02533v1)
- [Multiagent Debate](https://arxiv.org/abs/2305.14325)
- [LLM Routing Survey](https://arxiv.org/html/2502.00409v2)
- [JSONSchemaBench](https://arxiv.org/html/2501.10868v1)
- [Prompt Vocabulary Research](https://arxiv.org/html/2505.17037v1)
- [Instructor Library](https://python.useinstructor.com/blog/2024/09/26/bad-schemas-could-break-your-llm-structured-outputs/)

### Judge / Evaluation
- [G-Eval Guide](https://www.confident-ai.com/blog/g-eval-the-definitive-guide)
- [Monte Carlo LLM-as-Judge](https://www.montecarlodata.com/blog-llm-as-judge/)
- [Evidently AI LLM-as-a-Judge](https://www.evidentlyai.com/llm-guide/llm-as-a-judge)
- [Arize AI Evidence-Based Judging](https://arize.com/blog/evidence-based-prompting-strategies-for-llm-as-a-judge-explanations-and-chain-of-thought/)
