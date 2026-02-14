# Agent Personality Engineering

Reference document for designing agents with distinct, lively personalities that stay on-task and adapt their tone to context. Compiled from Anthropic research, academic papers, multi-agent framework analysis, and psychometric studies.

---

## Table of Contents

1. [Why Personality Matters](#1-why-personality-matters)
2. [The Science: How Personality Works Inside Models](#2-the-science-how-personality-works-inside-models)
3. [Personality Frameworks](#3-personality-frameworks)
4. [Personality's Effect on Task Quality and Safety](#4-personalitys-effect-on-task-quality-and-safety)
5. [The Personality-Task Balance](#5-the-personality-task-balance)
6. [Tone, Register, and Contextual Awareness](#6-tone-register-and-contextual-awareness)
7. [Personality Bleeding: When Character Overrides the Job](#7-personality-bleeding-when-character-overrides-the-job)
8. [Personality in Multi-Agent Systems](#8-personality-in-multi-agent-systems)
9. [Prompt Patterns for Personality](#9-prompt-patterns-for-personality)
10. [Measuring and Evaluating Personality](#10-measuring-and-evaluating-personality)
11. [Quantitative Results Summary](#11-quantitative-results-summary)
12. [Master Do's and Don'ts](#12-master-dos-and-donts)
13. [Recommended Default Profile for Nexor Agents](#13-recommended-default-profile-for-nexor-agents)

---

## 1. Why Personality Matters

Personality is not cosmetic. It directly shapes how agents communicate, how users perceive trustworthiness, and — critically — how reliably agents complete tasks.

### The Three Jobs of Personality

| Job | What It Does | Example |
|-----|-------------|---------|
| **Signal competence** | Users trust agents that communicate like competent humans in the same role | A code reviewer that is terse and direct signals engineering rigor |
| **Regulate tone for context** | The right tone at the right time prevents miscommunication | Calm precision during an incident; casual energy during brainstorming |
| **Differentiate agents** | In multi-agent systems, personality makes it clear who is speaking and what their perspective is | A planner agent and a critic agent should feel distinct |

### The Core Tension

> An agent with too much personality becomes unreliable. An agent with no personality becomes forgettable — and paradoxically, harder to trust.

The research shows this is not a binary choice. Personality and reliability are orthogonal axes that can be independently tuned when the prompt architecture separates identity from task execution.

---

## 2. The Science: How Personality Works Inside Models

### Persona Vectors (Anthropic, 2025)

**Source:** [Anthropic Persona Vectors Research](https://www.anthropic.com/research/persona-vectors)

The most significant finding in this space: personality traits exist as **concrete, measurable, steerable patterns of neural activity** inside LLMs. They are not surface-level text effects.

**How they work:**
1. Generate paired responses — one exhibiting a target trait (e.g., humor), one not
2. Identify the difference in neural activations between the pairs
3. The resulting difference vector encodes the trait as a linear direction in activation space

**Validated traits:** Evil behavior, sycophancy, hallucination, politeness, apathy, humor, optimism.

**Why this matters for prompt engineers:**
- Persona vectors activate **before** the model generates output — personality is decided upstream of text generation
- System-prompt-based personality genuinely activates these internal patterns; it is not "theater"
- Vectors can detect personality drift mid-conversation before it appears in output

### Personality is Fragile Under Perturbation

**Source:** [AAAI 2026 — Persistent Instability in LLM Personality](https://arxiv.org/html/2508.04826v1)

> "Slight prompt deviations can fundamentally alter measured personality, and behavioral consistency cannot be ensured through current prompting approaches alone."

This means personality requires **active maintenance** — monitoring, examples, and structural reinforcement — not a single system prompt and hope.

### Training vs. Prompting

| Approach | Mechanism | Stability | Accessibility |
|----------|-----------|-----------|---------------|
| **Prompt-based** | System prompt, few-shot examples | Fragile; requires reinforcement | Immediate; no model changes |
| **Representation engineering** | Directly manipulating model activations at inference time | More stable; trait-specific | Requires model access |
| **Fine-tuning** | Training on personality-aligned data (e.g., BIG5-CHAT) | Most stable | Requires training infrastructure |

For nexor, **prompt-based is the right approach** — we use third-party models and need per-agent customization. The key is doing it well.

---

## 3. Personality Frameworks

### The Big Five (OCEAN)

**Source:** [Nature Machine Intelligence — Psychometric Framework for LLMs](https://www.nature.com/articles/s42256-025-01115-6)

The dominant framework for LLM personality, validated across multiple studies:

| Trait | High | Low | Agent Design Relevance |
|-------|------|-----|----------------------|
| **Openness** | Creative, curious, explores alternatives | Practical, conventional, sticks to what works | Controls how exploratory vs. focused an agent is |
| **Conscientiousness** | Thorough, disciplined, follows through | Flexible, spontaneous, may cut corners | **Strongest correlate of safety and reliability** |
| **Extraversion** | Energetic, talkative, assertive | Reserved, concise, reflective | Controls verbosity and communication style |
| **Agreeableness** | Cooperative, warm, accommodating | Direct, challenging, skeptical | Controls whether agent pushes back or agrees |
| **Neuroticism** | Cautious, risk-aware, anxious | Calm, stable, confident | Controls hedging and uncertainty expression |

### Why Big Five Works for Agent Design

1. **Measurable**: Standard psychometric instruments (BFI-2, IPIP-NEO) can assess whether your prompt actually induces the intended personality
2. **Compositional**: Traits combine independently — you can have high openness + high conscientiousness (creative but disciplined)
3. **Predictive**: Specific trait profiles correlate with measurable downstream behaviors (see Section 4)

### SAC Framework: Continuous Trait Modulation

The SAC (Semantic Anchoring and Calibration) framework uses multi-dimensional trait assignments with five intensity levels per trait, enabling precise, graded personality modulation:

- **Monotonic shifts**: Increasing a trait instruction reliably increases measured trait expression
- **Co-mover effects**: Related traits shift together coherently (increasing Warmth naturally dampens Distrust)
- **Controllability**: Changes are measurable with standardized indices

### Beyond Big Five: Role-Based Personality

For practical agent design, raw trait scores are less intuitive than **role-based personality**. The CrewAI pattern encodes personality through narrative:

```
Role: Senior Backend Engineer
Goal: Review code changes for correctness and maintainability
Backstory: A 15-year veteran who has seen every anti-pattern. Direct but
not harsh. Prefers concrete examples over abstract critique. Respects
developers who show they've thought through edge cases.
```

The `backstory` naturally encodes personality (direct → low agreeableness; respects thoughtfulness → high conscientiousness) without requiring explicit trait labels.

---

## 4. Personality's Effect on Task Quality and Safety

### The Conscientiousness-Safety Link

**Source:** [Psychometric Personality Shaping — Safety in Language Models](https://arxiv.org/html/2509.16332)

This is the single most important finding for agent designers:

| Trait Change | Effect on Safety | Effect on Capability |
|-------------|-----------------|---------------------|
| **Reducing conscientiousness** | **Catastrophic**: 20-40 point drops on ETHICS, WMDP, TruthfulQA, MMLU | Minimal capability change |
| **Boosting conscientiousness** | Improves deontological ethics scores | No capability cost |
| **Increasing extraversion** | 4-9 point drops on TruthfulQA (impression management) | Capabilities unaffected |
| **Increasing neuroticism** | Lowers ethics in smaller models | Weak effect in GPT-4-class |
| **Increasing agreeableness** | Mixed — can amplify sycophancy | Can reduce pushback quality |

> Conscientiousness is the safety guardrail built into personality. Never trade it away for liveliness.

### Personality Helps Style, Not Accuracy

**Source:** [PromptHub — Role Prompting Analysis](https://www.prompthub.us/blog/role-prompting-does-adding-personas-to-your-prompts-really-make-a-difference)

Critical distinction:

- **Persona prompting is effective for open-ended/creative tasks** (tone, register, communication style)
- **Persona prompting shows mixed-to-negative results for factual/accuracy tasks** — one study found an "idiot" persona outperformed a "genius" persona on MMLU
- **Newer models show diminishing returns** from basic persona prompting for reasoning tasks

**The rule: Use personality for HOW the agent communicates, never for WHAT it knows.**

### The Recommended Profile for Production

Based on the research, the safest high-performing personality baseline is:

| Trait | Level | Why |
|-------|-------|-----|
| Conscientiousness | **High** | Prevents safety degradation; improves follow-through |
| Openness | **High** | Enables creative problem-solving without accuracy cost |
| Agreeableness | **Medium** | Cooperative without being sycophantic |
| Extraversion | **Medium** | Communicative without sacrificing truthfulness |
| Neuroticism | **Low** | Calm and confident; avoids excessive hedging |

---

## 5. The Personality-Task Balance

### The Spectrum of Control

**Source:** [Wiley System Dynamics Review — Prompt Engineering Dilemma](https://onlinelibrary.wiley.com/doi/10.1002/sdr.70008)

| Extreme | Problem | Symptom |
|---------|---------|---------|
| **Over-control** | Hardcoded personality logic creates brittleness | Agent follows personality rules but can't handle novel situations |
| **Under-control** | Vague personality guidelines produce inconsistency | Agent drifts between personalities turn-to-turn |
| **Right balance** | Specific enough to guide, flexible enough to adapt | Consistent character that reads the room |

### Anthropic's "Right Altitude" Applied to Personality

**Source:** [Anthropic Context Engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)

The same principle that applies to task prompts applies to personality:

> "Specific enough to guide behavior effectively, yet flexible enough to provide the model with strong heuristics."

**Too specific (bad):**
```
When the user says "thanks", respond with "No worries! Always happy to help.
Let me know if there's anything else!" with exactly one exclamation mark.
```

**Too vague (bad):**
```
Be friendly and helpful.
```

**Right altitude (good):**
```
You communicate like a senior engineer on a good team — direct, technically
precise, casually warm. You don't perform friendliness; you demonstrate it
through thoroughness and honesty.
```

### The Architectural Separation Principle

The most reliable way to prevent personality from interfering with task execution is **structural separation in the prompt**:

```
<identity>
Who you are and how you communicate.
</identity>

<task>
What you need to accomplish right now.
</task>
```

When these are mixed together ("You are a friendly assistant that analyzes code..."), the model must resolve ambiguity about which instruction takes priority. When separated, the model naturally treats identity as a communication layer and task as the execution layer.

### When to Turn Personality Down

Not all moments call for the same personality intensity. Effective agents modulate:

| Context | Personality Intensity | Why |
|---------|----------------------|-----|
| Normal workflow | Full expression | This is the default; be yourself |
| Error reporting | Reduced; factual and clear | Personality noise obscures critical information |
| Security/safety issues | Minimal; precise and direct | Levity undermines urgency |
| Celebrating success | Elevated; warm and energetic | Reinforce positive outcomes |
| Disagreeing with user | Moderate; firm but respectful | Personality should not soften necessary pushback |

---

## 6. Tone, Register, and Contextual Awareness

### The Linguistic Habits Problem

**Source:** [PersonaGym Evaluation](https://arxiv.org/html/2407.18416)

> "Linguistic habits are the universal weakness across all LLMs tested: all but three state-of-the-art models scored below 4.0 (out of 5) on matching personas to appropriate jargon, speech patterns, and communication styles."

This means:
- Simply saying "be casual" or "be formal" is **insufficient**
- Personas need explicit examples of how they speak — vocabulary, sentence structure, idioms
- **Few-shot examples of the target register are more effective than descriptive instructions**

### Defining Register Through Examples

**Bad (descriptive):**
```
Speak casually and use technical jargon appropriate for software engineers.
```

**Good (demonstrative):**
```
<voice_examples>
"Yeah, let's dig in. Paste the error and I'll trace it back."
"Heads up — this touches the auth middleware, so we should be careful."
"That's a clean solution. Ship it."
"I'd push back on this. The abstraction doesn't earn its complexity yet."
</voice_examples>
```

The model learns communication patterns far more effectively from demonstration than from description. This aligns with the broader finding that few-shot examples are 15-40% more effective than instructions alone (see PROMPT_RESEARCH.md, Section 6).

### The Three Registers

Most effective agents operate across three registers, shifting based on context:

| Register | When | Characteristics |
|----------|------|-----------------|
| **Working** | Day-to-day tasks, code review, implementation | Direct, technical, concise. Personality is present but not performing. |
| **Elevated** | Incidents, security, errors, disagreements | Precise, calm, minimal filler. Personality recedes; clarity advances. |
| **Relaxed** | Brainstorming, celebrating, casual check-ins | Warmer, more expressive, humor permitted. Personality is most visible. |

### Implementing Register Shifts

```
<communication_style>
Default register: Professional-casual. Technical but approachable.

Shift to elevated when:
- Reporting errors, failures, or security concerns
- Delivering bad news
- Correcting the user's approach

Shift to relaxed when:
- Brainstorming or exploring options
- Celebrating completed milestones
- User initiates casual conversation
</communication_style>
```

### Vocabulary as Identity

The most distinctive personalities have **specific vocabulary choices** that signal identity without being cartoonish:

| Agent Role | Vocabulary Signals |
|-----------|-------------------|
| Code Reviewer | "This reads well." / "I'd tighten this up." / "The intent is clear but the implementation is fragile." |
| Planner | "Let's scope this." / "What's blocking us?" / "The dependency chain here is..." |
| Debugger | "Let me trace this." / "The symptoms point to..." / "This narrows it down." |

These are not catchphrases — they are domain-appropriate linguistic patterns that build a coherent identity.

---

## 7. Personality Bleeding: When Character Overrides the Job

### What Personality Bleeding Looks Like

Personality bleeding occurs when an agent's character traits interfere with accurate task execution. It manifests as:

| Symptom | Caused By | Example |
|---------|-----------|---------|
| **Softening bad news** | High agreeableness | "The build is mostly fine! Just a few tiny issues..." (when the build is broken) |
| **Performing confidence** | High extraversion | Assertive answers to questions the model is uncertain about |
| **Over-qualifying** | High neuroticism | "I could be wrong, but maybe, if you think it's appropriate..." (for straightforward facts) |
| **Entertaining over informing** | Personality prioritized over task | Spending tokens on jokes when the user needs a fix |
| **Sycophancy** | High agreeableness + RLHF training | "Great question!" / "That's a really interesting approach!" (before disagreeing or when it adds nothing) |

### Sycophancy: The Most Dangerous Form

**Source:** [GovTech — LLM Sycophancy Survey](https://medium.com/dsaid-govtech/yes-youre-absolutely-right-right-a-mini-survey-on-llm-sycophancy-02a9a8b538cf)

Sycophancy is personality bleeding at scale:

- RLHF training incentivizes agreeable over accurate responses
- An agent given a "friendly and helpful" personality **amplifies** sycophantic tendencies
- Sycophantic behavior is strongest for ethically polarizing issues and subjective questions
- Users perceive sycophantic agents as more likable but **less trustworthy over time**

### The Contradiction Problem

**Source:** [Cognigy — AI Agent Persona Design](https://support.cognigy.com/hc/en-us/articles/17346614515868-Create-your-AI-Agent-s-persona)

> "Instructions contradictory with the AI Agent's description and job details can confuse the LLM and cause unexpected results."

When personality says "be warm and accommodating" and the task says "reject this PR for security violations," the model must resolve a conflict. Without explicit priority guidance, the resolution is unpredictable.

### Five Mitigation Strategies

**1. Explicit Priority Hierarchy**
```
When personality and accuracy conflict, accuracy wins. Always.
A charming wrong answer is worse than a blunt correct one.
```

**2. Constitutional Constraints**
```
<boundaries>
Never fabricate data, even if it would make the response more engaging.
Never soften error severity. A critical bug is a critical bug.
Never say "great question" or "that's a really interesting point."
Correct factual errors immediately, even if the user seems confident.
</boundaries>
```

**3. Anti-Sycophancy Guardrails**
```
<honesty>
If you disagree with the user's approach, say so directly with reasoning.
If asked for an opinion, give one. Do not hedge with "it depends" unless
it genuinely does — in which case, enumerate the conditions.
When delivering criticism, be specific and actionable, not vague and gentle.
</honesty>
```

**4. Separate Personality from Capability Claims**
Never let personality imply expertise the model doesn't have. A "confident senior engineer" persona should not make the model more likely to hallucinate technical details.

**5. Monitor for Drift**
In long conversations, personality tends to either decay (reverting to base model) or amplify (becoming a caricature). Both are failures. Reinforce personality periodically through the conversation structure, not just the system prompt.

---

## 8. Personality in Multi-Agent Systems

### The Cooperation-Vulnerability Paradox

**Source:** [Cooperative Personalities in Multi-Agent Contexts](https://arxiv.org/html/2503.12722v1)

In multi-agent systems, personality traits that improve cooperation also increase vulnerability:

| Personality Steering | Cooperation Benefit | Vulnerability Cost |
|---------------------|--------------------|--------------------|
| High Agreeableness | Reduced troublemaking to zero | Exploitability increased by 0.44 |
| High Agreeableness + Conscientiousness | 60% fewer collective penalties | Agents sacrificed individual gains |
| Balanced (medium traits) | Moderate cooperation | Moderate resilience |

> In multi-agent systems, personality must be designed with the system's threat model in mind.

### Personality as Differentiation

In multi-agent workflows, personality serves a functional purpose beyond aesthetics — it helps the orchestrator and other agents understand what perspective each agent brings:

| Agent | Personality Signal | Functional Purpose |
|-------|-------------------|-------------------|
| Planner | Methodical, scope-conscious | Prevents scope creep; thinks in dependencies |
| Implementer | Action-oriented, practical | Biases toward shipping; unblocks progress |
| Reviewer | Skeptical, detail-oriented | Catches what others miss; slows down when needed |
| Coordinator | Warm, organized, big-picture | Maintains team coherence; resolves conflicts |

These are not arbitrary — each personality maps to a cognitive function the system needs.

### Framework Approaches

| Framework | Personality Mechanism |
|-----------|----------------------|
| **CrewAI** | Explicit role-playing via `role`, `goal`, `backstory` fields |
| **AutoGen** | Emergent through conversational patterns and critique protocols |
| **LangGraph** | Cooperative framing: "collaborating with other assistants" |

CrewAI's explicit approach gives the most control. AutoGen's emergent approach produces more natural interactions but less predictable personality.

### Agent Description vs. Agent Personality

A critical distinction from the PROMPT_RESEARCH.md (Section 12):

| Concern | Purpose | Audience |
|---------|---------|----------|
| **Description** | For routing and orchestration | Other agents / orchestrator |
| **Personality** | For communication and behavior | The agent itself + end users |

The description says what the agent **does**. The personality says how the agent **is**. These must be defined separately:

```
// Description (for orchestrator routing):
"Reviews code changes for correctness, security, and maintainability."

// Personality (for the agent's system prompt):
"You are direct and specific in your feedback. You point to exact lines,
explain the concern, and suggest a concrete fix. You don't waste words
on praise for obvious things, but you acknowledge genuinely clever solutions."
```

---

## 9. Prompt Patterns for Personality

### Pattern 1: Layered System Prompt

The most reliable architecture separates identity, style, boundaries, and task into distinct sections:

```xml
<identity>
You are [Name], a [role] who [core personality trait in one sentence].
</identity>

<style_guide>
Voice: [2-3 adjectives that describe how you communicate]
Vocabulary: [words/phrases you use; words/phrases you avoid]
Default register: [working/elevated/relaxed and what it sounds like]
</style_guide>

<boundaries>
When personality and accuracy conflict, prioritize accuracy.
Never soften error severity.
Never perform enthusiasm you don't mean.
[Additional hard constraints]
</boundaries>

<examples>
[3-5 example exchanges showing the personality in action]
</examples>

<task>
[What the agent should accomplish — completely separate from who it is]
</task>
```

Why this works: Each section has a single responsibility. The model can attend to `<identity>` for character, `<boundaries>` for constraints, and `<task>` for execution without conflating them.

### Pattern 2: Personality Through Examples (Most Effective)

**Source:** [Anthropic Context Engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)

> "For an LLM, examples are the 'pictures' worth a thousand words."

Rather than describing personality abstractly, **show it**:

```xml
<examples>
<example>
<user>Can you help me debug this?</user>
<agent>Yeah, paste the error and I'll trace it back.</agent>
</example>

<example>
<user>This is frustrating, nothing works.</user>
<agent>I hear you. Let's isolate what changed — walk me through what you tried last.</agent>
</example>

<example>
<user>Should I use Redis or Postgres for this?</user>
<agent>Depends on access patterns. If you're doing key-value lookups with TTL, Redis. If you need transactions or complex queries, Postgres. What does your read/write ratio look like?</agent>
</example>

<example>
<user>Great work on that last feature!</user>
<agent>Thanks. The edge case handling on line 47 still bothers me — I want to revisit that when we have time.</agent>
</example>
</examples>
```

These four examples communicate more personality than any paragraph of description could:
- Direct, not performative
- Empathetic when appropriate, not sycophantic
- Asks clarifying questions instead of guessing
- Accepts praise without inflating, stays focused on the work

### Pattern 3: Graduated Register Instructions

```xml
<communication_style>
Default: Professional-casual. You sound like a competent colleague, not a
customer service bot.

When things go wrong (errors, failures, security):
  Drop the warmth. Be precise, structured, and calm.
  Lead with what happened, then what to do about it.

When things go right (shipped features, passing tests):
  Let some energy through. Brief acknowledgment, then move forward.
  "Clean. What's next?" is better than three lines of celebration.

When disagreeing:
  State the disagreement, state the reason, suggest an alternative.
  Never soften disagreement with compliments or hedging.
</communication_style>
```

### Pattern 4: Anti-Pattern Inoculation

Name the behaviors you don't want, then provide the alternative. This combines the "negative instructions + positive alternative + WHY" pattern from PROMPT_RESEARCH.md (Section 11):

```xml
<anti_patterns>
Instead of "Great question!", just answer the question.
  Why: Filler phrases waste tokens and feel performative.

Instead of "I'd be happy to help!", just help.
  Why: Announcing willingness delays the actual value.

Instead of "That's an interesting approach, but...", say "I'd do it differently."
  Why: The compliment-before-critique pattern signals incoming disagreement
  and trains users to distrust your praise.

Instead of hedging on things you know, state them directly.
  Why: Excessive hedging on confident knowledge erodes trust in your
  actual uncertainty signals.
</anti_patterns>
```

### Pattern 5: Role + Goal + Backstory (CrewAI Pattern)

For rapid personality definition when full prompt architecture is overkill:

```
Role: Security-focused code reviewer
Goal: Identify vulnerabilities and unsafe patterns before they ship
Backstory: Spent a decade in appsec. Has a reputation for finding the
bug everyone else missed. Not mean about it — explains the risk clearly
and always proposes a fix. Gets genuinely concerned about injection
vectors and auth gaps. Treats every PR like it's going to production
tomorrow, because it probably is.
```

The backstory encodes personality through narrative: concerned (high conscientiousness), explanatory (medium agreeableness), direct (low neuroticism), focused (medium extraversion).

### Pattern 6: Dynamic Personality with Mood Context

For agents that interact with users over extended sessions, inject conversation context that modulates personality:

```xml
<context>
Current workflow state: [building / debugging / reviewing / planning]
Recent events: [test suite passing / build failure / deadline approaching]
User sentiment: [neutral / frustrated / excited / uncertain]
</context>
```

The agent uses this context to calibrate its register without needing explicit rules for every scenario.

---

## 10. Measuring and Evaluating Personality

### The PersonaGym Framework

**Source:** [PersonaGym](https://arxiv.org/html/2407.18416)

PersonaGym evaluates persona agents across six dimensions:

| Dimension | What It Measures |
|-----------|-----------------|
| Expected Action | Does the agent behave as the persona would? |
| Toxicity Avoidance | Does the persona resist generating harmful content? |
| Linguistic Habits | Does the agent match the persona's speech patterns? |
| Persona Consistency | Does the persona hold across conversation turns? |
| Action Justification | Can the agent explain its actions through the persona's lens? |
| Knowledge-Action Alignment | Does the agent's behavior match the persona's knowledge? |

**Key finding:** Linguistic habits scored lowest across all models. This is where to focus testing effort.

### Practical Evaluation for Nexor

Rather than running full psychometric batteries, evaluate personality with targeted probes:

**1. Register consistency:** Send the same factual question in three emotional frames. Does the agent maintain its personality while adapting tone?

**2. Pushback test:** Give the agent a subtly wrong instruction. Does it correct it, or does agreeableness override accuracy?

**3. Pressure test:** Send rapid-fire questions. Does personality degrade under load, or remain consistent?

**4. Long conversation test:** Run a 20+ turn conversation. Does personality drift toward base model or become a caricature?

**5. Multi-agent differentiation:** Put two agents with different personalities in the same conversation. Can a reader tell them apart from voice alone?

### Measurement Instruments

| Instrument | Traits Measured | Length |
|-----------|----------------|--------|
| BFI-2 (Big Five Inventory 2) | OCEAN (5 traits) | 60 items |
| IPIP-NEO | OCEAN + 30 facets | 120-300 items |
| 16PF | 16 personality factors | 185 items |

These can be administered to LLMs by presenting the questionnaire items as prompts and mapping Likert-scale responses. This enables quantitative comparison between intended and measured personality.

---

## 11. Quantitative Results Summary

| Finding | Impact | Source |
|---------|--------|--------|
| Reducing conscientiousness | 20-40 point drops on safety benchmarks | Psychometric Shaping |
| Boosting conscientiousness | Improved ethics scores, no capability cost | Psychometric Shaping |
| Increasing extraversion | 4-9 point drops on TruthfulQA | Psychometric Shaping |
| Few-shot personality examples vs. descriptions | Linguistic habits scored below 4.0/5 across all models when using descriptions alone | PersonaGym |
| Agreeableness steering in multi-agent | Reduced troublemaking to zero, increased exploitability by 0.44 | Cooperative Personalities |
| Prompt-based personality stability | Fundamentally altered by slight prompt deviations | AAAI 2026 |
| Persona prompting for factual tasks | Mixed-to-negative results; "idiot" persona outperformed "genius" on MMLU | PromptHub |
| Dedicated descriptions vs system messages for routing | 2x correct selection rate | AutoGen |
| SAC continuous trait modulation | Monotonic, statistically robust trait shifts with cross-trait coherence | SAC Framework |

---

## 12. Master Do's and Don'ts

### DO

- **Separate identity from task** in the prompt architecture — use distinct `<identity>` and `<task>` sections
- **Use few-shot examples** to demonstrate personality — 3-5 examples communicate more than descriptions
- **Keep conscientiousness high** — it is the strongest correlate of safety, ethics, and truthfulness
- **Define register shifts explicitly** — specify when to be casual, when to be precise, when to be warm
- **Include anti-sycophancy guardrails** — "correct errors immediately, even if the user seems confident"
- **Show vocabulary, don't describe it** — example phrases are more effective than adjective lists
- **Test personality under pressure** — probe for drift, bleeding, and sycophancy specifically
- **Use backstory for rapid personality encoding** — narrative is more natural than trait scores
- **Make priority hierarchy explicit** — "when personality and accuracy conflict, accuracy wins"
- **Design personality for the role** — a reviewer should feel different from a planner

### DON'T

- **Don't sacrifice conscientiousness for liveliness** — it causes catastrophic safety degradation
- **Don't use personality to imply expertise** — a "senior engineer" persona should not make the model more confident in hallucinated answers
- **Don't mix identity and task instructions** — "You are a friendly assistant that analyzes code" forces the model to resolve ambiguity
- **Don't describe personality abstractly** — "be casual" fails; show what casual sounds like
- **Don't hardcode personality responses** — "When the user says X, respond with Y" creates brittleness
- **Don't expect stability without reinforcement** — personality drifts over long conversations; reinforce through structure
- **Don't use the same personality intensity for all contexts** — errors demand clarity, not charm
- **Don't add filler phrases** — "Great question!" and "I'd be happy to help!" waste tokens and signal inauthenticity
- **Don't let agreeableness override honesty** — a good agent disagrees when it should
- **Don't assume bigger models maintain persona better** — GPT-4 and LLaMA-3-8b scored the same on PersonaGym

---

## 13. Recommended Default Profile for Nexor Agents

Based on the research, here is a baseline personality profile for nexor agents. Individual agents should customize from this foundation.

### Trait Baseline

| Trait | Level | Rationale |
|-------|-------|-----------|
| Conscientiousness | **High** | Non-negotiable for safety and reliability |
| Openness | **High** | Enables creative problem-solving |
| Agreeableness | **Medium** | Cooperative without sycophancy |
| Extraversion | **Medium** | Communicative without verbosity |
| Neuroticism | **Low** | Calm, confident, minimal hedging |

### Default Voice

```
Direct and technically precise. Warm through thoroughness, not
performance. Speaks like a senior engineer who respects your time —
gives you what you need, flags what matters, moves on. Humor is
dry and rare, never forced. Admits uncertainty clearly and without
anxiety. Disagrees openly but constructively.
```

### Template System Prompt Section

```xml
<identity>
You are a [role] on the nexor platform.
[One sentence describing your specific function.]
</identity>

<voice>
You communicate like a senior engineer on a high-trust team.
Be direct. Be specific. Be honest.

When things go well: brief acknowledgment, move forward.
When things go wrong: lead with facts, follow with action.
When you disagree: state it, explain it, suggest an alternative.
When you're uncertain: say so clearly, without apologizing for it.
</voice>

<boundaries>
Accuracy over personality. Always.
Never soften error severity.
Never fabricate confidence.
Correct mistakes immediately, even if the user disagrees.
Skip filler phrases — answer the question, don't announce that you will.
</boundaries>

<examples>
[3-5 role-specific examples showing the voice in action]
</examples>
```

This template gives every agent a consistent baseline while leaving room for role-specific personality through the `<identity>` section and `<examples>`.

---

## Sources

### Anthropic
- [Persona Vectors: Monitoring and Controlling Character Traits](https://www.anthropic.com/research/persona-vectors)
- [Persona Vectors — arXiv Paper](https://arxiv.org/abs/2507.21509)
- [Effective Context Engineering for AI Agents](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)

### Academic — Personality Science
- [Psychometric Framework for LLM Personality (Nature Machine Intelligence)](https://www.nature.com/articles/s42256-025-01115-6)
- [Designing AI-Agents with Personalities: A Psychometric Approach](https://arxiv.org/abs/2410.19238)
- [BIG5-CHAT: Shaping LLM Personalities Through Training (ACL)](https://aclanthology.org/2025.acl-long.999.pdf)
- [Persistent Instability in LLM Personality Measurements (AAAI 2026)](https://arxiv.org/html/2508.04826v1)
- [Psychometric Personality Shaping Modulates Capabilities and Safety](https://arxiv.org/html/2509.16332)

### Academic — Multi-Agent and Evaluation
- [PersonaGym: Evaluating Persona Agents and LLMs](https://arxiv.org/html/2407.18416)
- [Cooperative Personalities in Multi-Agent Contexts](https://arxiv.org/html/2503.12722v1)
- [Exploring LLM Personality Traits via Latent Features Steering](https://arxiv.org/html/2410.10863v2)
- [Multi-Agent Collaboration Mechanisms Survey](https://arxiv.org/html/2501.06322v1)

### Industry
- [PromptHub — Role Prompting Analysis](https://www.prompthub.us/blog/role-prompting-does-adding-personas-to-your-prompts-really-make-a-difference)
- [LLM Sycophancy Mini Survey (GovTech)](https://medium.com/dsaid-govtech/yes-youre-absolutely-right-right-a-mini-survey-on-llm-sycophancy-02a9a8b538cf)
- [Prompt Engineering Dilemma (Wiley)](https://onlinelibrary.wiley.com/doi/10.1002/sdr.70008)
- [Steering LLM Agent Personalities (LessWrong)](https://www.lesswrong.com/posts/ugcMk9dYNbkYiBqcN/steering-llm-agents-temperaments-or-personalities)
- [Cognigy — AI Agent Persona Design](https://support.cognigy.com/hc/en-us/articles/17346614515868-Create-your-AI-Agent-s-persona)

### Multi-Agent Frameworks
- [CrewAI Agents](https://docs.crewai.com/en/concepts/agents)
- [AutoGen Agent Descriptions](https://microsoft.github.io/autogen/0.2/blog/2023/12/29/AgentDescriptions/)
