# Agent Execution Patterns

Reference document for designing agent memory, coordination, self-reflection, and inter-agent communication. Compiled from Anthropic engineering guides, academic papers (2024-2026), production system analysis, and multi-agent framework documentation.

---

## Table of Contents

1. [Why Execution Patterns Matter](#1-why-execution-patterns-matter)
2. [Agent Memory Architecture](#2-agent-memory-architecture)
3. [Scratchpad and Working Memory](#3-scratchpad-and-working-memory)
4. [Self-Reflection and Metacognition](#4-self-reflection-and-metacognition)
5. [Episodic Memory: Learning from Past Runs](#5-episodic-memory-learning-from-past-runs)
6. [Context Compression and Long-Horizon Agents](#6-context-compression-and-long-horizon-agents)
7. [Inter-Agent Communication Protocols](#7-inter-agent-communication-protocols)
8. [Handoff Patterns: Preventing Telephone-Game Degradation](#8-handoff-patterns-preventing-telephone-game-degradation)
9. [Decision Tracing and Accountability](#9-decision-tracing-and-accountability)
10. [Multi-Agent Coordination Architectures](#10-multi-agent-coordination-architectures)
11. [Sub-DAG Execution: Nested Workflows](#11-sub-dag-execution-nested-workflows)
12. [Quantitative Results Summary](#12-quantitative-results-summary)
13. [Master Do's and Don'ts](#13-master-dos-and-donts)

---

## 1. Why Execution Patterns Matter

### The Token Economics Problem

**Source:** [Anthropic: How We Built Our Multi-Agent Research System](https://www.anthropic.com/engineering/multi-agent-research-system)

Multi-agent systems use approximately **15x more tokens** than standard chat. Token usage alone accounts for **80% of performance variance**. Three factors explain 95% of performance differences between multi-agent configurations.

This means execution patterns — how agents think, remember, communicate, and coordinate — are not optional optimizations. They are the primary lever for quality and cost.

### The Coordination Overhead Problem

When agents pass information between each other, quality degrades at each hop (the telephone game). When agents maintain state across long sessions, context rot erodes accuracy. When agents act without tracing their decisions, debugging becomes impossible.

Execution patterns solve these problems through structure, not hope.

---

## 2. Agent Memory Architecture

### The Three-Memory Model

**Source:** [LIGHT Framework: Design Patterns for Long-Term Memory in LLM-Powered Architectures](https://serokell.io/blog/design-patterns-for-long-term-memory-in-llm-powered-architectures)

Effective agents need three distinct memory systems:

| Memory Type | Duration | Content | Access Pattern |
|-------------|----------|---------|----------------|
| **Working memory** | Current step/turn | Active task context, current inputs, tool outputs | Always in prompt; refreshed each step |
| **Session memory** | Current run | Accumulated observations, decisions, progress notes | Persisted to file/DB; loaded at checkpoints |
| **Episodic memory** | Across runs | Past run outcomes, reflections, learnings | Queried by similarity; injected as context |

### How Each Memory Type Maps to Nexor

| Memory Type | Nexor Mechanism | Where It Lives |
|-------------|----------------|----------------|
| **Working memory** | Step prompt context + port inputs + current envelope | In-flight; part of the LLM call |
| **Session memory** | `DagExecutionState` + run notes + content versions | `var_outputs`, `completed_envelopes`, `ContentVersionRow` |
| **Episodic memory** | Past run reflections and learnings | To be built — stored alongside `WorkflowExecutionRow` |

### The Memory Lifecycle

```
Run starts:
  → Load episodic memory (similar past runs)
  → Initialize session memory (empty progress notes)
  → Set up working memory (mission context)

Each step:
  → Working memory = step prompt + port inputs + upstream envelopes
  → After step: update session memory (observations, decisions)
  → After step: persist to content versions for audit

Run ends:
  → Generate run reflection (what worked, what didn't)
  → Store as episodic memory for future runs
  → Update session summary for human review
```

---

## 3. Scratchpad and Working Memory

### The ReAct Pattern

**Source:** [ReAct: Synergizing Reasoning and Acting in Language Models (Google Research)](https://arxiv.org/abs/2210.03629)

The ReAct pattern interleaves reasoning with action, using a scratchpad to maintain cognitive state:

```
Thought: [reasoning about what to do next]
Action: [the action to take with parameters]
Observation: [the result of the action]
... (repeat)
Thought: [final synthesis]
Answer: [final output]
```

Why this works: **reasoning traces help the model induce, track, and update action plans as well as handle exceptions**, while actions allow it to gather additional information from external sources.

### RAISE Enhancement: Short-Term + Long-Term

**Source:** [RAISE: ReAct with Memory](https://arxiv.org/abs/2210.03629)

RAISE enhances ReAct by adding:
- **Scratchpad** for short-term storage (current chain of reasoning)
- **Repository of similar past examples** for long-term retention (drawn from episodic memory)

This dual-memory approach prevents the agent from "forgetting" lessons learned in previous steps while maintaining focus on the current task.

### Implementing the Scratchpad for Nexor Agents

The scratchpad should be structured, not free-form:

```xml
<scratchpad>
<current_step>
  Step: {step_name}
  Objective: {what this step should accomplish}
  Inputs received: {summary of port inputs}
</current_step>

<observations>
  - {observation 1 from tool/action output}
  - {observation 2}
</observations>

<reasoning>
  Given observations, the best approach is {X} because {Y}.
  Convention {Z} from the required reading applies here.
  Risk: {any concerns about this approach}
</reasoning>

<decision>
  Action: {what I will do}
  Expected outcome: {what should happen}
  Fallback: {what to do if it fails}
</decision>
</scratchpad>
```

### Cognitive Workspace: Active Memory Management

**Source:** [Cognitive Workspace: Active Memory Management for LLMs (2025)](https://arxiv.org/html/2508.13171v1)

Key innovation: rather than passively accumulating context, the agent **actively curates** what stays in working memory:

| Metric | Cognitive Workspace | Traditional RAG |
|--------|-------------------|----------------|
| Memory reuse rate | **58.6%** | **0%** |
| Net efficiency gain | **17-18%** | Baseline |
| Operation count | 3.3x higher | Baseline |

The three core innovations:
1. **Active memory management** — deliberate information curation, not passive accumulation
2. **Hierarchical cognitive buffers** — enabling persistent working states across steps
3. **Task-driven context optimization** — dynamically adapts to cognitive demands

**Practical implication for nexor:** After each step, the agent should explicitly decide what to keep, what to summarize, and what to discard from working memory. This prevents context rot.

---

## 4. Self-Reflection and Metacognition

### The Reflexion Framework

**Source:** [Reflexion: Language Agents with Verbal Reinforcement Learning (NeurIPS 2023)](https://arxiv.org/abs/2303.11366)

Reflexion is the most validated self-improvement framework for LLM agents:

| Task Type | Without Reflexion | With Reflexion | Improvement |
|-----------|------------------|---------------|-------------|
| Code generation (HumanEval) | 80% (GPT-4) | **91%** | +11 pts |
| Reasoning (HotPotQA) | Baseline CoT | **+20%** | Significant |

How it works:
1. **Actor** generates output
2. **Evaluator** assesses the output
3. **Self-reflector** generates verbal feedback about what went wrong
4. Reflection is stored in **episodic memory buffer**
5. On next attempt, the actor receives its own prior reflections as context

> "Reflexion offers a lightweight alternative that doesn't require fine-tuning the underlying language model, making it more efficient in terms of data and compute resources."

### Implementing Self-Assessment

After each significant action, agents should produce a structured self-assessment:

```xml
<self_assessment>
<action_taken>{what the agent did}</action_taken>
<expected_outcome>{what was supposed to happen}</expected_outcome>
<actual_outcome>{what actually happened}</actual_outcome>
<assessment>correct | uncertain | likely_wrong</assessment>
<confidence>0.0-1.0</confidence>
<should_revise>true | false</should_revise>
<revision_plan>{if should_revise, what to change}</revision_plan>
<convention_compliance>
  - {convention X}: followed | violated (details)
  - {convention Y}: followed | violated (details)
</convention_compliance>
</self_assessment>
```

### When Self-Reflection Helps vs. Hurts

| Context | Self-Reflection Value | Why |
|---------|----------------------|-----|
| Multi-step reasoning | **High** | Catches compounding errors early |
| Code generation | **High** | Reflexion showed 11% improvement |
| Simple extraction | **Low** | Adds latency with no quality gain |
| Creative tasks | **Medium** | Useful for coherence, not for creativity |
| Convention compliance | **High** | Forces agent to check its own work |

### Safety Application

**Source:** [Safeguarding by Progressive Self-Reflection (EMNLP 2025)](https://aclanthology.org/2025.findings-emnlp.503.pdf)

A self-reflection prompt injected during generation allows the model to assess its own output:

> "Let's check if the generated text is harmful or harmless."

If the response is deemed harmful, the model backtracks and regenerates. This creates a safety net without external moderation.

---

## 5. Episodic Memory: Learning from Past Runs

### The Missing Piece

**Source:** [Position: Episodic Memory is the Missing Piece for Long-Term LLM Agents (Feb 2025)](https://arxiv.org/pdf/2502.06975)

> "Episodic memory captures experience-level knowledge documenting the goal, reasoning steps, actions, outcomes, and reflections."

Unlike semantic memory (facts) or procedural memory (how-tos), episodic memory records **what happened in specific past situations** — enabling agents to recall similar past episodes and adapt insights to new goals.

### The Episodic Memory Schema for Nexor

After each run, store:

```json
{
  "run_id": "uuid",
  "workflow_id": "uuid",
  "task_type": "code_review | documentation | implementation | design",
  "mission_objective": "what this run was trying to accomplish",
  "approach": "what strategy was used",
  "outcome": "success | partial | failure",
  "duration_ms": 45000,
  "total_tokens": 12500,
  "steps_completed": 5,
  "steps_failed": 1,
  "reflection": {
    "what_worked": "The three-phase decomposition was effective",
    "what_didnt": "The API convention doc was not loaded for step 3",
    "root_cause": "Step 3's agent didn't have the convention in its prompt",
    "key_insight": "Always inject API conventions for any step touching endpoints"
  },
  "convention_violations": [
    {"rule": "REST naming", "step": 3, "details": "Used /getUsers instead of /users"}
  ],
  "tags": ["api", "user-management", "rest-endpoint"]
}
```

### Retrieval and Injection

Before each new run:
1. Query episodic memory for the top 3 most similar past runs (by task type, tags, workflow)
2. Inject their reflections as context:

```xml
<past_experience>
In similar past tasks, you found that:
- {reflection.key_insight from run A}
- {reflection.what_didnt from run B} — avoid this approach
- {reflection.what_worked from run C} — this worked well

Common pitfalls for this task type:
- {aggregated convention_violations}
</past_experience>
```

### Amazon Bedrock's Implementation

**Source:** [Build Agents to Learn from Experiences Using Amazon Bedrock AgentCore Episodic Memory](https://aws.amazon.com/blogs/machine-learning/build-agents-to-learn-from-experiences-using-amazon-bedrock-agentcore-episodic-memory/)

Amazon's production episodic memory system:
- Captures experience-level knowledge including goals, reasoning steps, actions, outcomes, and reflections
- Agents recall similar past episodes and **adapt insights** to new goals rather than replaying prior trajectories
- The distinction is crucial: episodic memory provides **heuristics**, not playback

---

## 6. Context Compression and Long-Horizon Agents

### The Context Rot Problem

**Source:** [Anthropic Context Engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)

> "As token count increases, recall accuracy decreases. LLMs have finite 'attention budgets.'"

For long-running agents (multi-step workflows, extended sessions), the accumulated context eventually exceeds the model's ability to attend to all of it. Quality degrades silently.

### Factory's Evaluation of Compression Approaches

**Source:** [Evaluating Context Compression for AI Agents (Factory.ai, 2025)](https://factory.ai/news/evaluating-compression)

Factory evaluated three compression approaches across **36,000+ real engineering messages**:

| Approach | Compression Ratio | Task-Critical Retention | Interpretability |
|----------|-------------------|------------------------|------------------|
| Structured summarization (Factory) | High | **Highest** | High |
| OpenAI `/responses/compact` | **99.3%** (highest) | Lower | Low |
| Anthropic SDK compression | High | Lower | Medium |

Key finding: **all three achieved similar compression ratios. The difference was in what survived compression.**

> "Structured summarization retained more 'continue-the-task' information... In debugging scenarios, those summaries were more likely to preserve the relationship between an error code, the affected endpoint, and the underlying cause."

Traditional NLP metrics like ROUGE don't capture whether compressed context enables continued work. Factory uses **probe-based evaluation** — can the agent continue productively after compression?

### The Structured Progress File Pattern

**Source:** [Anthropic: Effective Harnesses for Long-Running Agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents)

Instead of generic summarization, maintain a structured progress file with explicit sections:

```json
{
  "session_intent": "Implement user profile API endpoints",
  "current_phase": "Writing handler for GET /users/:id",
  "completed": [
    "Created UserProfile struct with validation",
    "Implemented GET /users with pagination",
    "Added integration tests for list endpoint"
  ],
  "in_progress": "GET /users/:id handler — response serialization",
  "decisions_made": [
    {"decision": "Use Option<String> for nullable bio field", "reason": "Aligns with existing pattern in AgentRow"},
    {"decision": "Pagination via cursor, not offset", "reason": "API_CONVENTIONS.md section 3.2"}
  ],
  "blockers": [],
  "files_modified": ["src/server/api/users/mod.rs", "src/server/models/user.rs"],
  "open_questions": ["Should profile photos be stored as URLs or binary?"],
  "next_steps": [
    "Complete GET /users/:id response mapping",
    "Add tests for single-user endpoint",
    "Implement PATCH /users/:id for profile updates"
  ]
}
```

This file survives context compression intact because it is structured data, not prose. When the context window is refreshed, load this file to restore the agent's working state.

### Anthropic's Note-Taking Practice

> Agents should "regularly write notes persisted to memory outside of the context window" to maintain documentation references across extended interactions.

For nexor, this maps to content versioning — every step's prompt, system prompt, and envelope is already snapshotted via `ContentVersionRow`. The additional layer is **agent-written notes** that capture reasoning, not just outputs.

---

## 7. Inter-Agent Communication Protocols

### The Structured Envelope Pattern

Every message between agents should use a typed schema. This prevents ambiguity, enables machine-readable handoffs, and creates an audit trail:

```json
{
  "from_agent": "agent_id",
  "from_step": "step_id",
  "to_agent": "agent_id",
  "to_step": "step_id",
  "message_type": "task_result | observation | question | error | handoff",
  "content": {
    "summary": "1-2 sentence TL;DR of this message",
    "structured_data": {},
    "evidence": ["specific quotes, data points, or tool outputs"],
    "confidence": 0.85,
    "caveats": ["known limitations of this output"],
    "conventions_followed": ["API_CONVENTIONS.md section 2.1"]
  },
  "context_for_receiver": "Why this matters for your task"
}
```

### Nexor's Existing Envelope System

Nexor already has `StepExecutionEnvelope`:

```rust
pub struct StepExecutionEnvelope {
    pub status: ExecutionStatus,
    pub data: Option<serde_json::Value>,
    pub metadata: ExecutionMetadata,
    pub error: Option<ExecutionError>,
}
```

This is the transport layer. The **content layer** (what goes inside `data`) should follow the structured pattern above to prevent information loss.

### The Four Communication Protocols (2025 Landscape)

**Source:** [A Survey of Agent Interoperability Protocols (2025)](https://arxiv.org/html/2505.02279v1)

| Protocol | Creator | Purpose | Key Feature |
|----------|---------|---------|-------------|
| **MCP** (Model Context Protocol) | Anthropic | LLM ↔ tools/APIs | JSON-RPC 2.0; tool discovery |
| **ACP** (Agent Communication Protocol) | Community | Agent ↔ agent | Dynamic roles; any agent can initiate |
| **A2A** (Agent-to-Agent Protocol) | Google | Agent ↔ agent | Agent cards for capability discovery |
| **ANP** (Agent Network Protocol) | Community | Decentralized agent discovery | Cross-system agent communication |

For nexor's internal system, MCP and A2A concepts are most relevant:
- **MCP patterns** for how agents interact with tools
- **A2A patterns** for how agents describe themselves to each other (agent cards → agent descriptions)

### Structured Communication as Finite State Machines

**Source:** [Structured Inter-Agent Communication](https://www.emergentmind.com/topics/structured-inter-agent-communication)

Communication protocols modeled as finite-state machines (FSMs) provide explicit specification of:
- Message types and valid sequencing
- Agent roles and permitted actions at each state
- Transition conditions and error handling

This maps directly to nexor's edge system — edges define valid transitions between steps, with condition types controlling flow.

---

## 8. Handoff Patterns: Preventing Telephone-Game Degradation

### The Problem

When information passes through multiple agents, quality degrades at each hop:
- Agent A produces a detailed analysis
- Agent B receives a summary of A's work and loses nuance
- Agent C receives a summary of B's summary and loses critical context
- By Agent D, the original insight is gone

### Google ADK's Narrative Casting

**Source:** [Architecting Efficient Context-Aware Multi-Agent Framework for Production (Google Developers Blog)](https://developers.googleblog.com/architecting-efficient-context-aware-multi-agent-framework-for-production/)

Google's Agent Development Kit (ADK) solves this with **narrative casting**:

1. **Re-cast prior messages as narrative context**: Prior "Assistant" messages are re-framed as third-person narrative (e.g., `[For context]: Agent B found that...`) rather than appearing as the new agent's own outputs

2. **Action attribution**: Tool calls from other agents are marked and summarized so the new agent acts on results without confusing them with its own capabilities

3. **Fresh Working Context**: Each agent gets a fresh Working Context from its own perspective, while factual history is preserved in the Session

### Implementing Narrative Casting in Nexor

When passing context between steps:

```xml
<upstream_context>
Previous step "{step_name}" (Agent: {agent_name}) produced:

Summary: {envelope.data.summary}

Key findings:
{enumerate structured findings from envelope.data}

Confidence: {envelope.data.confidence}
Caveats: {enumerate caveats}

This context is provided for your reference. It is NOT your own work.
Your task is: {current_step.prompt}
</upstream_context>
```

The critical line: **"This context is provided for your reference. It is NOT your own work."** This prevents the agent from treating upstream context as its own output, which would compound errors.

### The Structured Handoff Protocol

For consequential handoffs (e.g., designer → worker), use a structured handoff document:

```json
{
  "handoff_type": "task_assignment",
  "from_role": "designer",
  "to_role": "worker",
  "objective": "What the worker should accomplish",
  "scope": {
    "in_scope": ["specific files", "specific changes"],
    "out_of_scope": ["what NOT to touch"]
  },
  "conventions_to_follow": ["list of relevant convention doc references"],
  "quality_criteria": ["how to know if the output is good"],
  "inputs": {
    "structured_data": {},
    "reference_docs": ["doc IDs to consult"],
    "upstream_findings": ["key findings from previous steps"]
  },
  "constraints": {
    "time_budget": "5 minutes",
    "token_budget": 30000,
    "must_include": ["specific elements required in output"],
    "must_avoid": ["specific anti-patterns"]
  }
}
```

### Preventing Information Loss at Each Hop

| Strategy | How | When |
|----------|-----|------|
| **Structured data over prose** | Pass JSON, not paragraphs | Always — structured data survives summarization |
| **Explicit evidence chains** | Include quotes/data points, not just conclusions | When conclusions matter for downstream decisions |
| **Summary + detail** | Include both a TL;DR and full output | When downstream agents may need different levels of detail |
| **Direct port wiring** | Use json_path extraction to pass specific fields | When only specific data points are needed downstream |
| **Narrative casting** | Re-frame upstream output as third-party context | When downstream agents might confuse upstream work with their own |

---

## 9. Decision Tracing and Accountability

### Why Tracing Matters

> "Agentic AI does not offer human-readable reasoning unless explicitly programmed to log it."
>
> — Adopt.ai, Audit Trails for Agents

In a multi-agent system, debugging "why did the output look like that?" requires tracing the decision chain across agents. Without tracing, failures are black boxes.

### Agent Decision Records (ADRs)

**Source:** [Audit Trails for Agents (Adopt.ai)](https://www.adopt.ai/glossary/audit-trails-for-agents)

Every agent action should produce a Decision Record:

```json
{
  "run_id": "uuid",
  "agent_id": "uuid",
  "step_id": "uuid",
  "timestamp": "2025-01-15T10:30:00Z",
  "action": "Generated API endpoint handler",
  "reasoning": "Following API_CONVENTIONS.md section 2.1 for REST endpoint naming",
  "alternatives_considered": [
    "GraphQL resolver — rejected because project uses REST exclusively",
    "RPC-style endpoint — rejected because conventions specify REST"
  ],
  "confidence": 0.9,
  "inputs_used": [
    "Port input: user_schema from step 1",
    "Convention: API_CONVENTIONS.md section 2.1",
    "Episodic memory: similar task in run xyz succeeded with this pattern"
  ],
  "upstream_decisions": ["step_1_id — provided the data schema"],
  "output_hash": "sha256:abc123"
}
```

### Nexor's Existing Tracing Infrastructure

Nexor already has strong foundations:

| Mechanism | What It Traces | Where |
|-----------|---------------|-------|
| `AgentExecutionRow` | Per-agent inputs, outputs, status | DB |
| `ExecutionMessageRow` | Messages exchanged during execution | DB |
| `ContentVersionRow` | Immutable snapshots of prompts and outputs | DB |
| `WorkflowEventKind` | Step start/complete/fail events | WebSocket |
| `StepExecutionEnvelope` | Structured output with metadata | In-memory + DB |
| `ExecutionMetadata` | Timing, tokens, costs, routing | Envelope |

The gap: **reasoning traces** (why the agent chose what it chose) are not currently captured. Adding a `reasoning` field to the envelope or a separate `DecisionRecord` table would complete the picture.

### Cross-Agent Traceability

**Source:** [Ensure Traceability in a Multi-Agent Ecosystem (Token Security)](https://www.token.security/use-cases/ensure-traceability-in-a-multi-agent-ecosystem)

Track how decisions from one agent influence decisions made by other agents:

```
Step 1 (Designer) decides: "Use cursor-based pagination"
  ↓ influences
Step 2 (Worker A) implements: "GET /users?cursor=abc"
  ↓ influences
Step 3 (Worker B) writes test: "Assert cursor in response headers"
```

Each decision should reference its upstream influences, creating a traceable chain from mission objective to final output.

### Observability Best Practice

**Source:** [AI Observability: Monitoring and Governing AI Agents (Kore.ai)](https://www.kore.ai/blog/what-is-ai-observability)

> "Signals correlated into execution graphs show exactly how an agent perceives context, plans actions, and generates results. Semantic analysis layers detect drift, hallucinations, or guardrail violations."

For nexor, this means the assistant should be able to pull up an execution graph for any run and see:
- What each agent received as input
- What it produced as output
- What reasoning it applied
- How its decisions influenced downstream agents

---

## 10. Multi-Agent Coordination Architectures

### Anthropic's Five Composable Patterns

**Source:** [Anthropic: Building Effective Agents](https://www.anthropic.com/research/building-effective-agents)

> "The most successful agent implementations use simple, composable patterns rather than complex frameworks."

| Pattern | Structure | Best For |
|---------|-----------|----------|
| **Prompt Chaining** | A → B → C (sequential with gates) | Linear workflows with quality checks |
| **Routing** | Classify → dispatch to specialist | Tasks that require different expertise |
| **Parallelization** | Fork → N agents → join | Independent subtasks or diverse perspectives |
| **Orchestrator-Workers** | Dynamic decomposition + delegation | Complex tasks requiring adaptive planning |
| **Evaluator-Optimizer** | Generate → evaluate → refine loop | Tasks with clear quality criteria |

### Nexor's Current Patterns

| Anthropic Pattern | Nexor Implementation |
|-------------------|---------------------|
| Prompt Chaining | DAG edges (topological execution) |
| Routing | Conditional edges + label-based routing |
| Parallelization | For-each steps + chained pipelines |
| Orchestrator-Workers | Designer → Workers via task force |
| Evaluator-Optimizer | Room steps (multi-agent discussion) + debate verification filter |

### The Multi-Agent Research System Results

**Source:** [Anthropic: How We Built Our Multi-Agent Research System](https://www.anthropic.com/engineering/multi-agent-research-system)

| Metric | Result |
|--------|--------|
| Multi-agent vs single-agent | **90.2% outperformance** |
| Parallel subagent spawning | Reduces research time by up to **90%** |
| Self-improvement (prompt optimization) | **40% decrease** in task completion time |
| Token overhead | ~15x standard chat |
| Performance variance explained by token usage | **80%** |

Key architectural decisions:
- Claude Opus as **lead agent** (orchestrator) + Claude Sonnet as **subagents** (workers)
- 3-5 subagents spawned simultaneously for parallel work
- Each subagent gets: specific objective, required output format, tool guidance, clear task boundaries

### Prompt Optimization > Adding Agents

**Source:** [Multi-Agent Prompt Optimization (2025)](https://arxiv.org/html/2502.02533v1)

> "Prompt optimization yields +6% improvement — more cost-effective than adding additional agents."

Before adding more agents to solve a quality problem, first optimize the prompts of existing agents. The research shows:
- Anthropic's own models can diagnose their failures and suggest prompt improvements
- Iterative prompt refinement yielded 40% task time reduction
- Adding agents increases coordination overhead; better prompts do not

---

## 11. Sub-DAG Execution: Nested Workflows

### The Architectural Fit

Nexor's DAG system is well-positioned for sub-DAG execution because:

1. **`execute_workflow_via_engine()` is self-contained** — takes context, creates state, runs loop, returns result
2. **Port routing is database-backed** — data flows cleanly across workflow boundaries
3. **`WorkflowSnapshot` freezes everything** — sub-workflow templates are stable, reproducible references
4. **The envelope system is uniform** — downstream consumers don't care if a step was atomic or a sub-DAG
5. **WebSocket events are already typed** — nested events compose naturally

### The `sub_workflow` Execution Mode

Add a new branch in `run_dag_loop()`, like the existing documenter/task_force/room branches:

```
run_dag_loop() hits step with execution_mode = "sub_workflow"
  ├─ resolve_port_inputs() — same as any other step
  ├─ Load referenced template (frozen snapshot)
  ├─ Map parent port inputs to sub-workflow initial variables
  ├─ Create child WorkflowExecutionRow (linked to parent)
  ├─ Call execute_workflow_via_engine() recursively
  ├─ Capture WorkflowExecutionResult
  ├─ Wrap in StepExecutionEnvelope — same as any other step
  ├─ Broadcast SubWorkflowCompleted
  └─ Downstream steps consume via normal port routing
```

### Data Flow Design

**Inputs (parent → child):**
- Port inputs from the parent step become initial `var_outputs` in the child
- The child workflow's entry steps receive these as their context

**Outputs (child → parent):**
- The child workflow's final step outputs become the parent step's envelope data
- Port routing extracts specific fields via `json_path`

**Events (child → parent → UI):**
- Child events are prefixed/nested: `SubWorkflow.StepStarted`, `SubWorkflow.StepCompleted`
- The UI can render them nested within the parent step's execution view

### When to Use Sub-DAGs

| Scenario | Sub-DAG? | Why |
|----------|----------|-----|
| Reusable workflow patterns | **Yes** | Define once, reference from multiple parent workflows |
| Complex step decomposition | **Yes** | When a single step is too complex for one agent |
| Dynamic task planning | **Yes** | Designer creates a sub-DAG at runtime based on task analysis |
| Simple sequential steps | **No** | Use regular DAG steps; sub-DAG adds unnecessary nesting |
| Parallel independent tasks | **Maybe** | For-each + chained pipeline may be simpler |

### The Hybrid Model for Runtime Decomposition

The designer can request sub-DAG creation at runtime:

```
1. Designer analyzes task → determines it needs decomposition
2. Designer produces a sub-DAG specification (steps, edges, assignments)
3. Assistant reviews the specification
4. User approves (or modifies)
5. System creates a temporary workflow from the specification
6. Parent step executes it as a sub_workflow
7. Results flow back through normal port routing
```

This keeps the user in control while giving the designer the power to decompose complex tasks.

---

## 12. Quantitative Results Summary

| Finding | Impact | Source |
|---------|--------|--------|
| Multi-agent token overhead | 15x standard chat | Anthropic |
| Token usage explains performance variance | 80% | Anthropic |
| Multi-agent vs single-agent quality | 90.2% outperformance | Anthropic |
| Parallel subagent spawning | Up to 90% time reduction | Anthropic |
| Prompt optimization vs adding agents | +6% (more cost-effective) | Academic 2025 |
| Self-improvement prompt optimization | 40% task time decrease | Anthropic |
| Reflexion code generation | 91% vs 80% (GPT-4 baseline) | NeurIPS 2023 |
| Reflexion reasoning improvement | +20% over CoT with ground truth | NeurIPS 2023 |
| Cognitive Workspace memory reuse | 58.6% vs 0% (traditional RAG) | arXiv 2025 |
| Cognitive Workspace efficiency gain | 17-18% net | arXiv 2025 |
| Structured compression vs generic | Higher task-critical retention at same ratio | Factory 2025 |
| Multi-agent debate improvement | +35.5% over baseline | Tool-MAD |
| Multi-agent orchestration (incident response) | 1.7% → 100% actionable | Academic |
| Hallucination reduction (combined approaches) | 96% | Stanford 2024 |

---

## 13. Master Do's and Don'ts

### DO

- **Implement the three-memory model** — working memory (per step), session memory (per run), episodic memory (across runs)
- **Use structured scratchpads** — XML-tagged sections for observations, reasoning, and decisions; not free-form text
- **Require self-assessment after significant actions** — confidence scores, expected vs actual outcomes, revision decisions
- **Store run reflections as episodic memory** — what worked, what didn't, key insights for similar future tasks
- **Use structured progress files** — explicit sections (intent, completed, decisions, blockers, next steps) survive compression
- **Type every inter-agent message** — summary, structured data, evidence, confidence, caveats
- **Apply narrative casting on handoffs** — re-frame upstream output as third-party context, not the receiving agent's own work
- **Log decision records** — reasoning, alternatives considered, upstream influences, confidence
- **Wire data through ports** — use json_path extraction for specific fields, not prose descriptions
- **Optimize prompts before adding agents** — prompt optimization yields +6% at lower cost

### DON'T

- **Don't rely on passive context accumulation** — context rot is real; actively curate what stays in working memory
- **Don't pass prose between agents** — structured data survives compression and prevents telephone-game degradation
- **Don't skip self-assessment on "simple" steps** — compounding errors start small
- **Don't replay past runs** — episodic memory provides heuristics, not playback; adapt insights, don't copy actions
- **Don't use generic summarization for long sessions** — structured summaries retain 2-3x more task-critical information
- **Don't add agents to solve quality problems** — optimize existing prompts first; coordination overhead compounds
- **Don't let sub-DAGs nest more than 2 levels deep** — coordination complexity grows exponentially; flatten if possible
- **Don't treat agent output as ground truth** — validate with post-execution checks, not self-reports
- **Don't ignore token economics** — 15x overhead means every unnecessary message costs real money
- **Don't skip the reasoning trace** — "why did it do that?" is the most common debugging question; make it answerable

---

## Sources

### Memory & Cognition
- [LIGHT Framework: Design Patterns for Long-Term Memory](https://serokell.io/blog/design-patterns-for-long-term-memory-in-llm-powered-architectures)
- [Cognitive Workspace: Active Memory Management for LLMs (2025)](https://arxiv.org/html/2508.13171v1)
- [Position: Episodic Memory is the Missing Piece for Long-Term LLM Agents (2025)](https://arxiv.org/pdf/2502.06975)
- [Amazon Bedrock AgentCore Episodic Memory](https://aws.amazon.com/blogs/machine-learning/build-agents-to-learn-from-experiences-using-amazon-bedrock-agentcore-episodic-memory/)

### Self-Reflection
- [Reflexion: Language Agents with Verbal Reinforcement Learning (NeurIPS 2023)](https://arxiv.org/abs/2303.11366)
- [Self-Reflection Enhances Large Language Models (Nature, 2025)](https://www.nature.com/articles/s44387-025-00045-3)
- [Safeguarding by Progressive Self-Reflection (EMNLP 2025)](https://aclanthology.org/2025.findings-emnlp.503.pdf)

### Agent Patterns
- [ReAct: Synergizing Reasoning and Acting (Google Research)](https://arxiv.org/abs/2210.03629)
- [Anthropic: Building Effective Agents](https://www.anthropic.com/research/building-effective-agents)
- [Anthropic: How We Built Our Multi-Agent Research System](https://www.anthropic.com/engineering/multi-agent-research-system)
- [Anthropic: Effective Context Engineering for AI Agents](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)
- [Anthropic: Effective Harnesses for Long-Running Agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents)

### Context & Compression
- [Evaluating Context Compression for AI Agents (Factory.ai, 2025)](https://factory.ai/news/evaluating-compression)
- [Compressing Context (Factory.ai)](https://factory.ai/news/compressing-context)

### Communication & Coordination
- [A Survey of Agent Interoperability Protocols (2025)](https://arxiv.org/html/2505.02279v1)
- [Structured Inter-Agent Communication](https://www.emergentmind.com/topics/structured-inter-agent-communication)
- [Google ADK: Context-Aware Multi-Agent Framework](https://developers.googleblog.com/architecting-efficient-context-aware-multi-agent-framework-for-production/)

### Traceability & Observability
- [Audit Trails for Agents (Adopt.ai)](https://www.adopt.ai/glossary/audit-trails-for-agents)
- [AI Observability: Monitoring and Governing AI Agents (Kore.ai)](https://www.kore.ai/blog/what-is-ai-observability)
- [Ensure Traceability in a Multi-Agent Ecosystem (Token Security)](https://www.token.security/use-cases/ensure-traceability-in-a-multi-agent-ecosystem)

### Multi-Agent Optimization
- [Multi-Agent Prompt Optimization (2025)](https://arxiv.org/html/2502.02533v1)
- [Multi-Agent Orchestration Patterns (Agyn.io)](https://agyn.io/blog/multi-agent-orchestration-patterns)
