# Protocol Design Guide

Practical application of research findings to each nexor protocol. This document bridges theory to implementation — what each protocol's prompts should look like, how they connect, and where the research applies.

Required Reading: AGENT_PERSONALITY.md, PROMPT_RESEARCH.md, AGENT_GOVERNANCE.md, AGENT_EXECUTION_PATTERNS.md

---

## Table of Contents

1. [The Protocol Ecosystem](#1-the-protocol-ecosystem)
2. [The Belief Pipeline](#2-the-belief-pipeline)
3. [The Agent Designer: Research Distributor](#3-the-agent-designer-research-distributor)
4. [Node Assistant Protocol](#4-node-assistant-protocol)
5. [Documenter Protocol](#5-documenter-protocol)
6. [Task Force Protocol](#6-task-force-protocol)
7. [Room / Meeting Protocol](#7-room--meeting-protocol)
8. [Belief Capture Protocol](#8-belief-capture-protocol)
9. [Single Step & For-Each](#9-single-step--for-each)
10. [Required Reading Flow](#10-required-reading-flow)
11. [Cross-Protocol Patterns](#11-cross-protocol-patterns)
12. [Research Application Matrix](#12-research-application-matrix)

---

## 1. The Protocol Ecosystem

### How the Protocols Connect

```
User
  ↕ (conversation)
Node Assistant ─── chat_belief_extraction → User Beliefs
  │ (notes + config)                          │
  ↓                                           │
Agent Designer ← research beliefs ←───────────┘
  │ (generates prompts)
  ↓
┌──────────────────────────────────────┐
│ Workflow Execution                    │
│                                      │
│  Context/Input ──→ Documenter        │
│       │              │ (produces     │
│       │              │  reference    │
│       │              │  documents)   │
│       ↓              ↓              │
│  Task Force ←── reads docs           │
│       │         (required reading)   │
│       ↓                              │
│  Belief Capture                      │
│       │ (distills runtime events     │
│       │  into structured beliefs)    │
│       ↓                              │
│  Room ←── beliefs as context         │
│       │   (agents know what happened)│
│       ↕                              │
│     User (can participate)           │
└──────────────────────────────────────┘
```

### Each Protocol's Job

| Protocol | Job | Input | Output |
|----------|-----|-------|--------|
| **Node Assistant** | Configure nodes; capture user intent | User conversation | Notes, config changes, chat beliefs |
| **Agent Designer** | Generate optimized prompts per agent | Roster + context + research | System prompts, task prompts, tool assignments |
| **Documenter** | Produce reference documents for agents | User notes + upstream context | Structured documents (required reading) |
| **Task Force** | Execute a multi-agent mission | Mission brief + roster + docs | Per-agent structured output |
| **Belief Capture** | Compress runtime events into beliefs | Upstream outputs (docs, agent work) | Structured beliefs with tags + confidence |
| **Room** | Multi-agent discussion with user | Beliefs + agent perspectives | Discussion output, decisions |

### The Design Principle

Each protocol has a different relationship with the research:

| Protocol | Primary Research Source | Key Concern |
|----------|----------------------|-------------|
| Node Assistant | AGENT_GOVERNANCE (observe-report-wait) | Not overstepping; capturing intent accurately |
| Agent Designer | PROMPT_RESEARCH + AGENT_GOVERNANCE | Distributing research patterns to generated prompts |
| Documenter | PROMPT_RESEARCH (structured output) | Document quality; convention adherence |
| Task Force | AGENT_GOVERNANCE (full stack) | Reliable execution; scope control; decision tracing |
| Room | AGENT_PERSONALITY (differentiation) | Distinct voices; grounded debate; belief-informed |
| Belief Capture | AGENT_EXECUTION_PATTERNS (memory) | Accurate compression; no information loss |

---

## 2. The Belief Pipeline

### Two Belief Systems, One Knowledge Graph

Nexor has two distinct belief extraction paths that feed into a unified knowledge base:

**Path 1: Chat Beliefs (User Intent)**
```
User ↔ Node Assistant conversation
  ↓ chat_belief_extraction
Beliefs about: what the user wants, their priorities, decisions, constraints
Source: USER messages only (not assistant responses)
Purpose: Inform other nodes about user intent across the board
```

**Path 2: Runtime Beliefs (What Happened)**
```
Workflow execution outputs (documents, agent work, tool results)
  ↓ belief_capture step
Beliefs about: what was produced, key findings, decisions made, risks found
Source: Agent outputs and documents
Purpose: Compress runtime context so rooms have full awareness
```

### Why Two Paths Matter

| Chat Beliefs | Runtime Beliefs |
|-------------|----------------|
| What the user **wants** | What the system **did** |
| Extracted from conversation | Extracted from execution outputs |
| Board-wide awareness | Workflow-scoped awareness |
| Survive across sessions | Generated per run |
| Inform all nodes | Inform rooms and downstream steps |

### The Convergence Point: Rooms

Rooms receive **both** belief types:
- **Chat beliefs** tell agents what the user cares about, their preferences, their mental model
- **Runtime beliefs** tell agents what happened during execution, what was produced, what went right or wrong

This is how room agents can have an intelligent conversation with the user about the work — they know both the intent (chat beliefs) and the outcomes (runtime beliefs).

### Belief Quality Standards

From AGENT_EXECUTION_PATTERNS.md (Section 6, Context Compression):

> "Structured summarization retained more 'continue-the-task' information... In debugging scenarios, those summaries were more likely to preserve the relationship between an error code, the affected endpoint, and the underlying cause."

Beliefs must preserve **relationships**, not just facts:
- BAD: "An error was found." (what error? where? what caused it?)
- GOOD: "The auth endpoint (/api/v1/auth/login) returns 500 when the JWT secret is missing from env — the token signing call panics on unwrap." (preserves the causal chain)

### Cross-Source Tension: The Supersede Pattern

The chat belief extractor already handles this well — when a user changes direction, new beliefs supersede old ones with explicit `cross_source_tension: "SUPERSEDED: {old belief}"`. This pattern should extend to runtime beliefs:

- If a task force agent discovers that a documenter's research was wrong, the belief capture should emit a belief with `cross_source_tension` marking the stale finding
- Rooms receive the superseded belief and know which information is current

---

## 3. The Agent Designer: Research Distributor

### The Multiplier Role

The Agent Designer is the most leveraged point in the system. Every research finding that gets encoded into the designer's beliefs propagates to every agent it designs. Currently, the designer has 19 BOCA-style beliefs from PROMPT_RESEARCH.md. The governance and execution pattern research should be encoded the same way.

### Recommended New Beliefs for the Agent Designer

These beliefs encode findings from AGENT_GOVERNANCE.md and AGENT_EXECUTION_PATTERNS.md:

```
[scope_boundaries | 0.85] Agents that receive explicit scope boundaries
("you may modify files in /src/api/, you must not touch /src/core/")
produce fewer out-of-scope actions — scope creep starts when boundaries
are implicit rather than stated.

[instruction_as_checklist | 0.85] Complex multi-step instructions broken
into numbered checklist items achieve ~80% compliance per item, versus
~27% when delivered as prose paragraphs — current models follow fewer
than 30% of complex instructions perfectly.

[convention_citation | 0.80] Agents instructed to cite the convention
they're following ("per API_CONVENTIONS section 2.1") produce more
consistent output than agents merely told to "follow conventions" —
citation forces active engagement with the reference material.

[self_assessment | 0.75] Agents that output a brief confidence assessment
after their main work ("confidence: 0.85 — followed established pattern;
low risk area: the error mapping for edge case X") catch their own errors
before downstream agents compound them.

[narrative_handoff | 0.80] When injecting upstream agent outputs, framing
them as third-party context ("Agent Scanner found: ...") rather than raw
output prevents the receiving agent from treating upstream work as its own
— reducing compounding errors and false attribution.

[required_reading | 0.85] When agents have document_read capability and
Required Reading is specified, instruct them to read those documents FIRST
before starting their task — agents that consult reference material before
acting produce output that adheres to project conventions.

[refusal_over_guessing | 0.80] An agent that says "I need clarification
on X before proceeding" is more valuable than one that guesses wrong —
the cost of asking is always lower than the cost of executing the wrong
interpretation.

[decision_tracing | 0.75] Agents that include brief reasoning for their
key choices ("chose cursor pagination over offset because API_CONVENTIONS
section 3.2 specifies cursor-based pagination for list endpoints")
produce auditable output the assistant can evaluate.
```

### How the Designer Uses Beliefs

The designer already processes beliefs as internalized research findings. When designing prompts for a task force agent, beliefs like `[scope_boundaries | 0.85]` cause the designer to include scope sections in the generated system prompt. The confidence weight (0.85) tells the designer how strongly to apply the pattern — high-confidence beliefs become near-mandatory prompt elements.

### Archetype-Specific Guidance

The designer receives `archetype_guidance` that varies by protocol type. This is where protocol-specific research application lives:

| Archetype | Guidance Focus |
|-----------|---------------|
| Task Force | Scope boundaries, decision tracing, convention citation, self-assessment |
| Documenter | Structured output, convention adherence, document quality criteria |
| Room | Personality differentiation, grounded debate, belief citation |

---

## 4. Node Assistant Protocol

### Current State

The node assistant already implements several research patterns well:
- Voice guidelines match AGENT_PERSONALITY.md recommendations (direct, precise, anti-sycophancy)
- Notes guidance implements structured session memory from AGENT_EXECUTION_PATTERNS.md
- Board overview provides cross-node awareness
- Examples demonstrate the voice (few-shot from PROMPT_RESEARCH.md)

### What the Research Adds

**From AGENT_GOVERNANCE.md — The Observe-Report-Wait Protocol:**

The node assistant is the per-node version of the workshop assistant described in the vision. It already does observe (board_overview) and report (tool calls for config). The gap is the **wait** discipline and **run observation** capability from the vision.

**Enhanced Assistant Capabilities (Vision Alignment):**

The vision specifies the assistant should be able to:
1. Execute agents at a given step (run a protocol)
2. See agent output and grade it
3. Run entire steps with snapshotted observation windows
4. Take notes about what it observes during runs
5. Update notes for the next run
6. Talk to the user during runs
7. Look up mentioned agents or context and see their executions

### Recommended Enhancements to Node Assistant System Prompt

```xml
<run_observation>
When observing a protocol execution:

1. BEFORE the run:
   - Review your notes for relevant context
   - Check if there are past run reflections for this protocol
   - Note what you expect to see based on the current config

2. DURING the run (between steps):
   - Check for user messages and respond
   - Note observations about agent behavior, quality, timing
   - Flag convention violations or unexpected patterns immediately

3. AFTER the run:
   - Grade the output: did it meet the mission objective?
   - Note what worked and what didn't
   - Update your notes with learnings for next time
   - If the user asks, provide specific feedback with evidence

You are the quality gate. If an agent's output doesn't meet standards,
say so directly with specifics. Don't soften — the user needs honest
assessment to improve the workflow.
</run_observation>
```

### Required Reading Integration

The node assistant already handles required reading well via the notes guidance:

```
## Required Reading — document IDs agents should read at runtime
- Document Name (document_id: <uuid>)
```

The enhancement: when the user shares a document, the assistant should not just record it — it should **read it** and confirm understanding:

```xml
<required_reading_behavior>
When the user shares a document for required reading:
1. Record the document ID in your notes
2. Read the document yourself to understand its conventions
3. Briefly confirm what you found: "Got it — this defines [X] convention
   for [domain]. I'll make sure the agents reference it."
4. When configuring agents later, reference specific sections of the
   document to ensure they're applied correctly
</required_reading_behavior>
```

---

## 5. Documenter Protocol

### Current Architecture

```
Strategist → Researchers (parallel) → Writers (parallel) → Assistant
```

The documenter produces **reference material for AI agents**, not human-facing deliverables. This is a critical distinction — the documents are the "required reading" that downstream agents consume.

### Where Research Applies

**Primary concern:** Document quality and convention adherence. The documents the documenter produces become the ground truth that task force agents follow. Bad documents produce bad agent behavior.

**From PROMPT_RESEARCH.md — Structured Output:**
- The strategist already produces structured JSON (document_plans array)
- The writer's prompt is minimal: "You are a technical writer." This is the biggest improvement opportunity.

**From AGENT_GOVERNANCE.md — Required Reading:**
- The documenter's researcher role already has `document_read` capability
- When Required Reading is in the assistant's notes, researchers should read those documents first
- This creates a chain: previous documents inform new documents

**From AGENT_EXECUTION_PATTERNS.md — Episodic Memory:**
- The documenter should learn from past runs: "Last time we generated API docs, the strategy phase underspecified section boundaries, causing writers to overlap."
- Store run reflections as episodic memory keyed by document type

### Enhanced Writer System Prompt

The current writer prompt is one line. Research shows this is where the most improvement is possible — the writer determines the quality of the entire protocol's output:

```xml
<identity>
You are a technical writer producing reference documentation for AI agents.
Your documents will be consumed by other agents during workflow execution —
they are the ground truth that shapes agent behavior and output quality.
</identity>

<quality_standards>
Your documents must be:
- **Specific**: Concrete rules, not vague guidelines. "Use cursor-based
  pagination with a max page size of 100" not "implement pagination."
- **Structured**: Headers, sections, and consistent formatting. Agents
  parse structure to find relevant sections.
- **Actionable**: Every convention should tell the reader exactly what to do.
  Include examples of correct AND incorrect patterns.
- **Scoped**: Each document covers one domain completely. No cross-referencing
  to information not in this document.

When writing conventions or specifications:
- State the rule
- Show an example of correct usage
- Show an example of incorrect usage (what to avoid)
- Explain WHY — agents that understand the reasoning generalize better
</quality_standards>

<audience>
Your reader is an AI agent with no prior knowledge of this project.
It will use your document as its sole reference for conventions in
this domain. Everything it needs must be in your document.
</audience>
```

### Enhanced Strategist Guidance

The strategist determines what the researchers look for and how writers approach each document. Adding research-informed guidance:

```xml
<strategy_principles>
When planning document research:
- Researchers have tools (file_read, grep, shell). Direct them to
  discover rather than assume — "scan the codebase for auth patterns"
  not "the auth module probably uses JWT."
- Writers produce better output with specific instructions. Instead of
  "write about the API," say "document each endpoint with method, path,
  request body schema, response schema, error cases, and an example curl."
- Each document should have a single clear purpose. If a document tries
  to cover too much, split it. The target_length guides scope.
- When Required Reading documents exist, instruct researchers to read
  them first — existing conventions should inform new documents, not
  contradict them.
</strategy_principles>
```

### Documenter Self-Assessment

Add a quality check between research and writing phases. The existing assistant role could serve this purpose:

```
Phase 2.5 (between research and write):
- Assistant reviews researcher findings
- Checks: Is there enough material to write each document?
- Checks: Do findings contradict existing required reading?
- If gaps found: flag for user or request additional research
```

### Episodic Memory for Documenter

After each documenter run, store:

```json
{
  "run_id": "uuid",
  "document_type": "api_reference | convention_guide | architecture_doc",
  "documents_produced": ["doc1", "doc2"],
  "strategy_quality": "Did the strategy phase give writers enough guidance?",
  "research_quality": "Did researchers find what was needed?",
  "writer_quality": "Did writers produce usable, convention-compliant docs?",
  "key_learning": "What would make the next run better?"
}
```

---

## 6. Task Force Protocol

### Current Architecture

```
Agent Designer → Agent 1 → Agent 2 → ... → Agent N (sequential)
```

Each agent receives: role description, mission brief, team roster, previous agents' outputs. The Agent Designer generates optimized prompts, tool assignments, and output routing.

### Where Research Applies — FULL GOVERNANCE STACK

The task force is the protocol where AGENT_GOVERNANCE.md applies most comprehensively. Each agent is a worker (L4 autonomy) executing within defined scope.

**From AGENT_GOVERNANCE.md:**
- Instruction hierarchy: mission brief (P1) > agent role (P2) > discovered context (P3)
- Scope boundaries: each agent must know what it can and cannot do
- Required reading: agents with `document_read` must read before acting
- Self-assessment: after execution, confidence + convention compliance
- Decision tracing: reasoning for key choices, referenced in output

**From AGENT_EXECUTION_PATTERNS.md:**
- Narrative handoff: upstream outputs framed as third-party context
- Structured envelopes: typed output for downstream consumption
- Scratchpad: agents should reason before acting
- Episodic memory: past task force runs inform future ones

### Enhanced Task Force Agent System Prompt

The current system prompt is a template. Research suggests these additions:

```xml
<identity>
You are **{{.TaskForce.agent_name}}**, a specialist agent executing
as part of a task force.
</identity>

<role>
{{.TaskForce.role_description}}
</role>

<authority>
You operate at Level 4 autonomy within this mission:
- Execute your assigned role fully and thoroughly
- If you discover something outside your scope: report it in your
  output under "out_of_scope_findings", do not act on it
- If instructions are ambiguous: state your interpretation before
  proceeding. If confidence is below 3/5, include it in
  "clarification_needed" rather than guessing
- If you can't complete your assignment: explain what's blocking
  you specifically, don't produce partial work silently
</authority>

<required_reading>
If you have the document_read tool and Required Reading is listed:
1. Read each required document BEFORE starting your task
2. Reference specific sections when they apply to your work
3. Follow conventions from the documents — they override your defaults
</required_reading>

<mission>
{{.TaskForce.task_description}}
</mission>

<team>
{{.TaskForce.team_roster}}
</team>

<upstream_context>
These are results from previous agents on your team.
This is THEIR work, not yours. Build on it, verify it if needed,
but do not repeat it.

{{.TaskForce.previous_outputs}}
</upstream_context>

<output_requirements>
Execute your assigned role. Produce structured output that downstream
agents can consume directly.

Include in your output:
- Your primary deliverable (the work product)
- Key decisions and reasoning (for audit trail)
- Convention references (which docs guided your approach)
- Confidence assessment (1-5, with brief justification)
- Any out-of-scope findings (observations outside your role)
</output_requirements>
```

### Task Force Agent Designer Guidance

The Agent Designer's `archetype_guidance` for task force should encode governance patterns:

```xml
<archetype_guidance type="task_force">
When designing prompts for task force agents:

SCOPE CONTROL:
- Each agent's system prompt must include explicit scope boundaries
- Tell agents what they CAN do and what they CANNOT do
- Include: "If you discover something outside your scope, report it
  but do not act on it"

REQUIRED READING:
- When notes include Required Reading, instruct agents with document_read
  to call read_document() FIRST
- Reference specific document sections in the task prompt when applicable

HANDOFF QUALITY:
- Frame upstream outputs as third-party context: "Agent [Name] found: ..."
- Each agent's task prompt should specify what format downstream agents
  expect from their output
- Include: "Your output will be consumed by [downstream agent role]"

DECISION TRACING:
- Instruct agents to include brief reasoning for key choices
- Pattern: "I chose X because [convention/finding/reasoning]"
- This makes the assistant's job of grading output possible

SELF-ASSESSMENT:
- End each agent's task prompt with: "After completing your work, rate
  your confidence 1-5 and note any areas of uncertainty."
</archetype_guidance>
```

### Episodic Memory for Task Force

```json
{
  "run_id": "uuid",
  "mission_type": "code_review | implementation | security_scan | research",
  "agents_in_roster": ["Scanner", "Analyzer", "Reporter"],
  "outcome": "success | partial | failure",
  "per_agent_assessment": {
    "Scanner": {"quality": 4, "note": "Found all major issues, missed one edge case in auth"},
    "Analyzer": {"quality": 5, "note": "Strong prioritization, clear reasoning"},
    "Reporter": {"quality": 3, "note": "Report was verbose; next time constrain to top-10 findings"}
  },
  "key_learning": "The Reporter needs a max_findings constraint to avoid noise",
  "convention_violations": [],
  "designer_prompt_quality": "Good tool assignment; routing was clean"
}
```

---

## 7. Room / Meeting Protocol

### Current Architecture

```
Gatekeeper (speaker selection) → Agent turns → User participation (optional)
```

Room agents have beliefs as context (from upstream belief capture). The gatekeeper decides who speaks and in what order.

### Where Research Applies — PERSONALITY-HEAVY

The room is where AGENT_PERSONALITY.md matters most. Room agents need:
- **Distinct voices** — each agent must feel different (differentiation)
- **Grounded positions** — arguments based on beliefs, not training data
- **Honest disagreement** — agents should challenge each other
- **Register awareness** — tone matches the discussion context

**From AGENT_PERSONALITY.md:**
- Use the Big Five to differentiate room members (Section 3)
- A reviewer should feel different from a planner (Section 8)
- Personality bleeding is dangerous in rooms — agents softening positions to agree (Section 7)
- Anti-sycophancy guardrails prevent false consensus (Section 7)

**From AGENT_GOVERNANCE.md:**
- Disagreement resolution: compare confidence, escalate to orchestrator (Section 6)
- The gatekeeper serves as the orchestrator/moderator

**From AGENT_EXECUTION_PATTERNS.md:**
- Beliefs as structured context prevent telephone-game degradation (Section 8)
- Decision tracing: each agent's position should be traceable to evidence

### Room Member Prompt Design

Each room member needs a personality that serves their functional role:

```xml
<identity>
You are {{member.name}}, a {{member.role}} participating in a discussion
about {{room.purpose}}.

Your perspective: {{member.perspective}}
</identity>

<voice>
{{member.voice_examples — 3-5 example statements showing how this agent communicates}}
</voice>

<grounding>
Your positions must be grounded in the beliefs provided to you.
When you make a claim, reference the specific belief or evidence.

Pattern: "Based on [belief/finding], I think [position] because [reasoning]."

Do not argue from general knowledge. Argue from the specific context
of this project and what the team has discovered.
</grounding>

<disagreement>
If you disagree with another agent:
- State the disagreement directly
- Reference the evidence that supports your position
- Acknowledge the other agent's evidence if it's valid
- Propose a resolution or identify what information would settle the debate

Never agree just to be agreeable. False consensus is worse than
productive disagreement.
</disagreement>

<beliefs>
{{injected beliefs from upstream belief capture}}
</beliefs>
```

### Gatekeeper Enhancement

The gatekeeper currently selects speakers by relevance. Research suggests adding:

```xml
<speaker_selection_criteria>
Beyond relevance, consider:
1. **Diverse perspectives first** — if two agents agree, prioritize the
   dissenting voice. Homogeneous responses waste turns.
2. **Evidence-holders first** — agents with beliefs directly relevant to
   the current topic should speak before those with general knowledge.
3. **Build on disagreement** — when agents disagree, schedule the
   respondent next to create productive dialogue.
4. **User priority** — if the user has spoken, select agents that can
   directly address the user's point.
</speaker_selection_criteria>
```

### Room Personality Templates

Based on AGENT_PERSONALITY.md (Section 8), functional roles map to personality profiles:

| Room Role | Big Five Profile | Voice Characteristics |
|-----------|-----------------|----------------------|
| **Advocate** | High Openness, Medium Agreeableness | Exploratory, proposes alternatives, "What if we..." |
| **Critic** | Low Agreeableness, High Conscientiousness | Skeptical, detail-oriented, "The evidence doesn't support..." |
| **Synthesizer** | High Agreeableness, High Openness | Connects ideas, finds common ground, "Both points suggest..." |
| **Pragmatist** | Low Openness, High Conscientiousness | Action-oriented, practical, "The simplest path is..." |
| **Domain Expert** | Medium all, narrative-driven backstory | Authoritative in specialty, defers outside it, "In my experience with [domain]..." |

### Room Self-Assessment

After a room completes, capture discussion quality:

```json
{
  "run_id": "uuid",
  "topic": "Discussion topic",
  "turns_used": 12,
  "agents_participated": ["Advocate", "Critic", "Pragmatist"],
  "consensus_reached": true,
  "key_disagreements": [
    {
      "topic": "Pagination approach",
      "positions": {"Advocate": "GraphQL cursors", "Pragmatist": "REST offset"},
      "resolution": "Pragmatist's position adopted — simpler for current scale"
    }
  ],
  "beliefs_cited": 8,
  "beliefs_challenged": 2,
  "quality_assessment": "Productive debate; all positions grounded in evidence"
}
```

---

## 8. Belief Capture Protocol

### Current Architecture

Two extractors serve different purposes:

**Chat Belief Extraction** (user conversations):
- Extracts from USER messages only
- Board-aware: knows what other nodes believe
- Handles topic evolution: captures latest intent, not history
- Detects cross-source tension with SUPERSEDED pattern

**Runtime Belief Capture** (workflow outputs):
- Extracts from agent outputs, documents, and tool results
- Per-source extraction (one LLM call per upstream source)
- Confidence-based filtering
- Tag vocabulary for semantic categorization

### Where Research Applies

**From AGENT_EXECUTION_PATTERNS.md — Context Compression:**
- Beliefs ARE the structured summarization that Factory's research found most effective
- They preserve relationships (causal chains, dependencies) not just facts
- They're typed (fact, opinion, assumption, requirement, etc.) which enables filtering

**From AGENT_GOVERNANCE.md — Instruction Following:**
- The extractor must follow the extraction focus precisely
- Tag vocabulary defines the output categories — tool constraints apply (26% compliance from AGENTIF)
- Structured JSON output with schema enforcement

### Chat Belief Extraction: Already Strong

The chat belief extraction system prompt is already well-designed:
- Priority on project scope (most important category)
- Specific examples of good vs bad beliefs
- Board awareness for cross-source tension
- Topic evolution handling (latest intent only)

**One enhancement — Belief Confidence Calibration:**

```xml
<confidence_calibration>
Confidence levels should reflect how explicitly the user stated the belief:

HIGH: User directly stated this. "We need cursor-based pagination."
MEDIUM: User implied this through discussion. The conversation about
  API design assumed REST, but they never explicitly said "use REST."
LOW: Inferred from context. The user discussed scalability, suggesting
  they expect high load, but didn't state a number.

Do not inflate confidence. A medium-confidence belief that accurately
reflects its uncertainty is more valuable than a high-confidence belief
that overstates what the user actually said.
</confidence_calibration>
```

### Runtime Belief Capture: Enhancement Opportunities

The runtime belief extractor processes agent outputs. The main enhancement from the research:

**Preserve Causal Chains:**

```xml
<extraction_quality>
When extracting beliefs from agent outputs, preserve relationships:

BAD: "A security vulnerability was found."
GOOD: "The auth endpoint (/api/v1/auth/login) is vulnerable to timing
attacks because the password comparison uses string equality (==) instead
of constant-time comparison — an attacker can measure response time
differences to enumerate valid passwords."

The causal chain matters: WHAT → WHERE → WHY → IMPACT.
Room agents need the full chain to have informed discussions.
</extraction_quality>
```

**Source Attribution:**

```xml
<source_attribution>
Each belief must trace back to its source:
- Which agent produced the finding?
- Which step in the workflow?
- What evidence supports it (file path, line number, tool output)?

This enables room agents to assess credibility: a belief from a
security scanner with file path evidence is more trustworthy than
a belief from a generalist agent's speculation.
</source_attribution>
```

### Belief as Context Engineering

From AGENT_EXECUTION_PATTERNS.md: beliefs solve the context compression problem. Instead of passing raw outputs (thousands of tokens) to room agents, beliefs provide high-signal compressed context (~50-100 tokens per belief).

The belief capture step should aim for:
- **Completeness**: Every significant finding is captured
- **Atomicity**: Each belief is one claim, testable and citable
- **Specificity**: Concrete details preserved, not abstracted away
- **Source tracing**: Every belief points back to its origin

---

## 9. Single Step & For-Each

### Single Steps

Single steps are the most straightforward execution mode. Research applies through the Agent Designer — when the designer generates the system prompt for a single-step agent, it should apply the same governance beliefs (scope boundaries, decision tracing, self-assessment).

**Enhancement — Convention Injection:**

For single steps that have Required Reading in the assistant's notes, the `compose_prompt()` function should inject convention reminders:

```xml
<conventions>
The following conventions apply to your task. Follow them precisely.
Cite the convention when your output follows a specific rule.

{{injected convention content from required reading documents}}
</conventions>
```

### For-Each Steps

For-each steps iterate over arrays. Each iteration is essentially a single step. Research applies per-iteration:

**Label-Based Routing Enhancement:**

The routing instruction block that gets injected into upstream steps could include governance guidance:

```xml
<routing_instructions>
Each item MUST include a "category" field set to exactly one of:
{{routing rules with descriptions}}

When categorizing, consider:
- If uncertain between categories, choose the more conservative option
- Include a "confidence" field (0.0-1.0) for your categorization
- If an item genuinely doesn't fit any category, flag it as "unroutable"
  with an explanation rather than forcing a bad fit
</routing_instructions>
```

---

## 10. Required Reading Flow

### The Full Pipeline

```
1. User shares documents with Node Assistant
   ↓
2. Node Assistant records document IDs in notes under "Required Reading"
   ↓
3. Documenter produces new reference documents
   ↓ (these can ALSO become required reading for other nodes)
4. Agent Designer sees Required Reading in notes
   ↓
5. Designer instructs agents with document_read to call
   read_document(document_id) BEFORE starting their task
   ↓
6. Agents read documents and reference conventions in their work
   ↓
7. Compliance validator checks output against conventions
```

### What Makes Required Reading Effective

From AGENT_GOVERNANCE.md (Section 4):

| Approach | Mechanism | Reliability |
|----------|-----------|-------------|
| "Read the docs" (instruction) | Agent may skip | Low |
| Inject content into prompt (forced) | Agent sees it | Medium |
| Require citation in output (engagement) | Agent must reference docs | High |
| Post-execution validation (verification) | System checks compliance | Highest |

**Nexor's current approach:** The Agent Designer instructs agents to call `read_document()` — this is the "inject content" approach (medium reliability).

**Recommended enhancement:** Add the citation requirement. When the designer generates task prompts for agents with required reading, include:

```
After reading the required documents, reference specific sections
in your output when they guided your decisions. Pattern:
"Per [document name] section [X]: [what the convention says] → [what I did]"
```

This moves from medium to high reliability by forcing active engagement.

### Cross-Documenter Required Reading

When one documenter's output becomes required reading for another protocol's agents:

```
Documenter A (produces API_CONVENTIONS document)
  ↓ document stored with UUID
Node Assistant records: "Required Reading: API_CONVENTIONS (doc_id: abc123)"
  ↓
Task Force execution:
  Agent Designer sees Required Reading in notes
  Generates prompt: "Use document_read to read doc abc123 before starting"
  Task Force agent reads API_CONVENTIONS and follows its rules
```

This is how the documenter feeds the rest of the system — its documents become the conventions that all other agents follow.

---

## 11. Cross-Protocol Patterns

### Pattern 1: The Feedback Loop

```
Task Force produces code → Belief Capture extracts results
  → Room discusses quality → User provides direction
  → Node Assistant updates notes → Next Task Force run improves
```

Each protocol in the chain adds signal. The key is that **beliefs compress the signal** so downstream protocols receive high-signal context, not raw output.

### Pattern 2: The Quality Gate

```
Agent produces output → Self-assessment (in output)
  → Belief Capture rates quality → Room reviews (optional)
  → Assistant grades → User approves or requests revision
```

Quality is assessed at multiple levels:
1. Agent self-assessment (embedded in output)
2. Belief capture (structured extraction of findings + quality)
3. Room discussion (multi-perspective evaluation)
4. Assistant observation (pattern matching against conventions)
5. User review (final authority)

### Pattern 3: The Convention Cascade

```
User defines requirements → Documenter produces conventions
  → Conventions become Required Reading
  → Agent Designer injects conventions into prompts
  → Workers follow conventions → Compliance validated
  → Belief Capture extracts adherence metrics
  → Room discusses convention effectiveness
  → User refines requirements → Documenter updates conventions
```

Conventions flow through the system and evolve based on feedback. The documenter doesn't produce static documents — it produces living conventions that the system learns to follow better over time.

### Pattern 4: The Sub-DAG Opportunity

When a single protocol step is too complex, it can be decomposed into a sub-DAG:

```
Parent DAG step (sub_workflow mode)
  ↓ port inputs
Sub-DAG:
  Context → Documenter → Task Force → Belief Capture → Room
  ↓ envelope output
Parent DAG continues with sub-DAG results
```

This enables the designer to create micro-pipelines for complex tasks without flattening everything into the parent workflow.

---

## 12. Research Application Matrix

### Which Research Applies Where

| Research Finding | Node Asst | Designer | Documenter | Task Force | Room | Belief Cap |
|-----------------|-----------|----------|------------|------------|------|------------|
| **PERSONALITY** | | | | | | |
| Big Five profiling | Voice | — | — | Moderate | **Critical** | — |
| Anti-sycophancy | Yes | — | — | Yes | **Critical** | — |
| Register shifts | Yes | Encodes | — | — | Yes | — |
| Voice examples | Yes | Generates | — | — | **Critical** | — |
| Personality bleeding prevention | — | — | — | — | **Critical** | — |
| **PROMPTS** | | | | | | |
| XML structuring | Yes | **Critical** | Yes | Yes | Yes | Yes |
| Few-shot examples | Yes | **Critical** | Strategy | Generated | — | — |
| CoT / structured thinking | — | — | Strategy | Generated | — | — |
| Positive framing | Yes | **Critical** | Yes | Generated | — | Yes |
| Queries at bottom | — | **Critical** | Prompts | Generated | — | Prompts |
| Schema design | — | Output | Strategy+Response | Generated | Gatekeeper | Response |
| **GOVERNANCE** | | | | | | |
| Autonomy levels | L2-L3 | — | Pipeline | L4 workers | Gatekeeper | Pipeline |
| Instruction hierarchy | Yes | Encodes | Yes | **Critical** | Moderate | Yes |
| Required reading | Manages | Distributes | Produces | **Critical** | Via beliefs | — |
| Scope boundaries | Yes | Encodes | Phase scope | **Critical** | Topic scope | Focus scope |
| Observe-report-wait | **Critical** | — | — | — | — | — |
| Compliance verification | Grades | — | QA phase | Post-exec | — | — |
| Anti-overreach | Yes | Encodes | — | **Critical** | — | — |
| Instruction as checklist | — | Encodes | Strategy | **Critical** | — | — |
| **EXECUTION** | | | | | | |
| Three-memory model | Session | — | Per-run | Per-run | Per-session | — |
| Scratchpad | Notes | — | — | Generated | — | — |
| Self-reflection | Run grading | — | Phase check | Self-assess | Position review | — |
| Episodic memory | Run history | — | Doc quality | Mission quality | Discussion quality | — |
| Structured compression | — | — | — | — | — | **IS this** |
| Narrative handoff | — | Encodes | Phase→Phase | Agent→Agent | — | — |
| Decision tracing | Grades it | — | — | **Critical** | Position tracing | Source tracing |
| Structured envelopes | — | — | Doc output | Agent output | Per-speaker | Belief JSON |

### Priority by Protocol

**Node Assistant:** GOVERNANCE > PERSONALITY > PROMPTS > EXECUTION
The assistant's primary job is governance — observing, grading, not overstepping. Personality makes it trustworthy. Prompts make it clear. Execution patterns (notes, history) make it effective over time.

**Agent Designer:** PROMPTS > GOVERNANCE > EXECUTION > PERSONALITY
The designer's primary job is generating effective prompts. Governance beliefs get encoded into generated prompts. Execution patterns inform handoff design. Personality is not relevant to the designer itself.

**Documenter:** PROMPTS > GOVERNANCE > EXECUTION > PERSONALITY
Document quality depends on prompt quality (writer instructions, strategy structure). Governance ensures conventions are followed. Execution patterns (episodic memory) improve quality over time. Personality is minimal — writers should be clear and structured, not characterful.

**Task Force:** GOVERNANCE > EXECUTION > PROMPTS > PERSONALITY
Task force agents need governance most — scope control, decision tracing, instruction following, required reading. Execution patterns (handoffs, scratchpads) prevent compounding errors. Prompts (generated by designer) enable the work. Personality is moderate — enough to differentiate agents, not enough to interfere.

**Room:** PERSONALITY > GOVERNANCE > EXECUTION > PROMPTS
Room agents need distinct personalities above all — the room's value comes from diverse perspectives. Governance prevents sycophancy and false consensus. Execution patterns (belief grounding) ensure arguments are evidence-based. Prompts are the vehicle for personality expression.

**Belief Capture:** EXECUTION > PROMPTS > GOVERNANCE > PERSONALITY
Belief capture is fundamentally an execution pattern — structured context compression. Prompt quality determines extraction accuracy. Governance ensures the extraction focus is followed. Personality is irrelevant — the extractor should be invisible.

---

## Sources

All sources are referenced in the primary research documents:
- [AGENT_PERSONALITY.md](./AGENT_PERSONALITY.md) — Identity, voice, traits, differentiation
- [PROMPT_RESEARCH.md](./PROMPT_RESEARCH.md) — Prompt structure, techniques, patterns
- [AGENT_GOVERNANCE.md](./AGENT_GOVERNANCE.md) — Authority, compliance, autonomy control
- [AGENT_EXECUTION_PATTERNS.md](./AGENT_EXECUTION_PATTERNS.md) — Memory, coordination, tracing
