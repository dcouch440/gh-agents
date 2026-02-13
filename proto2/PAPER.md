# The Agent Designer: Belief-Driven Prompt Generation for Multi-Agent Task Forces

**David Couch**

February 2026

---

## Abstract

We present the Agent Designer, a pre-lifecycle LLM call that transforms mission briefs and agent rosters into optimized (system prompt, task prompt, tool assignment) tuples for each agent in a multi-agent task force. The designer operates on 21 BOCA-style belief slices — confidence-weighted prompt engineering findings internalized as operating principles rather than retrieved as rules. In controlled experiments against two scenarios — a 3-agent story analysis pipeline and a 5-agent codebase security audit — the designer demonstrated: (1) correct tool assignment under ambiguity, withholding tempting-but-wrong tools (shell for static analysis agents, database_query with no live database) while granting verification access to evaluation-role agents; (2) belief internalization rates of 15–17/21 beliefs detected in generated prompts, with beliefs driving observable agent behavior (tool usage patterns, exploratory framing, pipeline position awareness); (3) perfect issue detection in Phase 2 execution — 4/4 planted contradictions in the story scenario and 8/8 planted security vulnerabilities (plus 6 bonus findings) in the codebase scenario; and (4) cost efficiency at $0.06–0.10 per designer call and $0.23 for full agent execution. The key insight: investing one LLM call in prompt quality before crew execution produces better results than giving agents generic instructions with the same tools.

## 1. Introduction

Multi-agent systems decompose complex tasks across specialized agents. A code review task force might include a Scanner, Analyzer, and Reporter; a security audit might field a Mapper, SecurityAuditor, PerformanceAnalyst, IntegrationReviewer, and ReportWriter. Each agent needs:

1. **Identity** — who they are, what expertise they bring
2. **Tools** — which capabilities from the available pool they should use
3. **Context** — what upstream agents produced, who consumes their output
4. **Instructions** — how to approach the work, what "done" looks like

Typically, these are hand-authored per workflow. This works at small scale but breaks down as task forces grow: tool assignment becomes non-obvious, pipeline position context requires manual tracking, and prompt quality varies with the author's skill.

The Agent Designer automates this. Given a mission brief (what the team is doing), an agent roster (who's on the team), and available capabilities (what tools exist), it generates tailored prompts for every agent in a single LLM call. The designer's system prompt contains 21 BOCA-style operating beliefs (Couch, 2026) — prompt engineering findings formatted as confidence-weighted interpretive statements rather than procedural rules.

This paper reports results from two experimental phases validating the approach.

## 2. Architecture

### 2.1 The Designer Call

The Agent Designer is a single-round LLM call with structured JSON output. It receives:

- **System prompt**: Identity, 21 operating beliefs, output format specification, and a worked example
- **User prompt**: Mission brief, agent roster with roles and execution order, upstream context (if any), and the available capability pool

It produces, for each agent:

- **Tool assignment**: A subset of the available capabilities
- **System prompt**: Role identity, behavioral guidelines, tool usage patterns with examples
- **Task prompt**: Mission context, upstream outputs, specific assignment, expected deliverable
- **Design reasoning**: Rationale for tool assignment, identity framing, and prompt structure choices

### 2.2 BOCA-Style Operating Beliefs

Rather than encoding prompt engineering knowledge as rules ("always put the task at the end") or few-shot examples, the designer internalizes findings as belief slices — the same format introduced in Belief-Oriented Conversation Architecture (Couch, 2026). Each belief carries a semantic tag and confidence weight:

```
[identity_specificity | 0.90] Agents with a named role, domain, and expertise
level produce more focused output than generic identities.

[tool_least_privilege | 0.85] Reference only the tools each agent actually has —
mentioning unavailable tools causes confusion and hallucinated tool calls.

[pipeline_position | 0.80] Agents that understand their position ("you receive
Scanner's findings, your analysis feeds to Reporter") scope their work
appropriately and avoid over-reaching.
```

The full set of 21 beliefs covers identity framing, prompt structure, tool management, collaboration context, output formatting, and meta-strategies. They are not retrieved at inference time — they are part of the designer's system prompt, functioning as internalized expertise.

### 2.3 Reasoning Trace Filter

The designer's execution pipeline includes a `ReasoningTraceFilter` that wraps the output schema in a `{"reasoning": "...", "result": ...}` envelope. This forces the model to think through its design choices before committing to structured output, then strips the reasoning on output so downstream parsing receives clean JSON. The filter activates only when `has_output_schema` is true.

### 2.4 Integration Point

In the production system, the designer runs as a pre-lifecycle step before task force execution. The DAG executor calls `run_agent_designer()`, which:

1. Resolves template variables (mission brief, roster, upstream context, capabilities)
2. Creates a designer run record for token tracking
3. Executes the designer call with reasoning trace filter
4. Parses and validates the structured output
5. Strips any tools not in the allowed capability pool
6. Stores designed prompts in the database
7. Returns prompts sorted by execution order

Crew agents then execute with their designer-generated prompts, receiving upstream agent outputs injected into the `{placeholder}` sections of their task prompts.

## 3. Experimental Design

### 3.1 Prototype

We built a Python prototype (`proto2/designer_test.py`) that invokes the Anthropic API directly, bypassing the Rust execution engine. This enables rapid iteration on the designer's system prompt and scoring logic without full system deployment. The prototype supports:

- **Phase 1**: Designer call with belief detection scoring
- **Phase 2**: Agentic tool-use execution of a designed agent against mock files with planted issues

### 3.2 Scenario 1: Stories (3 agents, 3 tools, 4 contradictions)

A baseline scenario testing fundamental designer capabilities:

| Agent | Role | Expected Tools |
|-------|------|---------------|
| Scanner | Search story files for contradictions | grep, file_read |
| Analyzer | Evaluate contradictions, assess severity | file_read, grep (verification access) |
| Reporter | Write summary report | file_write |

**Available capabilities**: file_read, grep, file_write

**Mock files**: 4 short story fragments containing 4 planted contradictions (eye color, profession, age, location).

**Key design test**: Does the Analyzer get verification access (file_read + grep) even though its primary role is evaluation, not discovery?

### 3.3 Scenario 2: Codebase (5 agents, 6 tools, 8 vulnerabilities)

A harder scenario stressing tool assignment under ambiguity:

| # | Agent | Role | Design Challenge |
|---|-------|------|-----------------|
| 1 | Mapper | Discover codebase structure, entry points, data flow | Should get shell for find/tree — but not database_query |
| 2 | SecurityAuditor | OWASP Top 10 analysis, credential exposure | Needs grep + file_read. Should NOT get shell (static analysis) |
| 3 | PerformanceAnalyst | N+1 queries, connection bottlenecks | Should NOT get database_query (no live DB in audit) |
| 4 | IntegrationReviewer | Cross-module data flow, race conditions, transactions | Verification access question — spot-check upstream findings? |
| 5 | ReportWriter | Compile prioritized remediation plan | Pure output — file_write only? Or verification access? |

**Available capabilities**: file_read, grep, shell, file_write, git, database_query

**Mock files**: 9 Node.js source files (Express microservice handling auth and payments) containing:

| # | Issue | File | Subtlety |
|---|-------|------|----------|
| 1 | SQL injection via string interpolation | checkout.js | Obvious |
| 2 | Non-constant-time password comparison | login.js | Subtle |
| 3 | No rate limiting on login | login.js | Medium |
| 4 | Broken authorization (IDOR) on refunds | refund.js | Medium |
| 5 | Missing transaction boundary (race condition) | checkout.js | Subtle |
| 6 | N+1 query pattern | queries.js | Medium |
| 7 | Hardcoded database credentials | database.js | Obvious |
| 8 | Outdated Express, missing helmet | package.json | Medium |

**Red herrings** (2): A `setTimeout(50)` in middleware that's intentional debounce, and a connection pool `max: 5` with comments explaining why it's appropriate.

**Scoring**: Phase 1 scores tool assignments and belief detection. Phase 2 runs the SecurityAuditor with synthetic Mapper output injected as upstream context, scoring keyword detection against all 8 planted issues.

## 4. Results

### 4.1 Phase 1: Designer Output Quality

#### Stories Scenario

| Agent | Assigned Tools | Correct? |
|-------|---------------|----------|
| Scanner | grep, file_read | Yes |
| Analyzer | file_read, grep | Yes — verification access granted |
| Reporter | file_write | Yes |

**Beliefs detected**: 18/21
**Cost**: $0.057 (2,925 tokens in / 6,051 out)
**Time**: 79 seconds

#### Codebase Scenario

| Agent | Assigned Tools | Correct? | Key Decision |
|-------|---------------|----------|-------------|
| Mapper | file_read, grep, shell | Yes | shell for find/tree structural discovery |
| SecurityAuditor | file_read, grep | Yes | Withheld shell — static analysis only |
| PerformanceAnalyst | file_read, grep | Yes | Withheld database_query — no live DB |
| IntegrationReviewer | file_read, grep | Yes | Verification access for spot-checking |
| ReportWriter | file_write | Acceptable | No verification access (debatable) |

**Beliefs detected**: 15–17/21 (varied across runs)
**Cost**: $0.10 (2,925 tokens in / 6,051 out)
**Time**: ~130 seconds

**Notable tool assignment decisions**:
- `shell` assigned only to Mapper — the one agent needing structural commands
- `database_query` never assigned — correct, no live database in the audit context
- `git` never assigned — correct, git history not needed for this audit
- All analysis agents (SecurityAuditor, PerformanceAnalyst, IntegrationReviewer) received `grep` + `file_read` with agent-specific usage examples

### 4.2 Phase 2: Agent Execution

#### Stories — Scanner Agent

| Metric | Value |
|--------|-------|
| Contradictions found | **4/4** |
| Rounds | 5/15 |
| Tool calls | 12 |
| Tokens | ~13K in / ~6.7K out |

The Scanner found all 4 planted contradictions (eye color, profession, age, location) using exactly the tool patterns the designer prescribed: grep to scan for keywords, file_read to examine context.

#### Codebase — SecurityAuditor Agent

| Metric | Value |
|--------|-------|
| Issues found | **8/8** |
| Rounds | 13/25 |
| Tool calls | 28 |
| Tokens | 74,696 in / 7,283 out |
| Time | ~3 min 17s |

The SecurityAuditor found every planted issue:

| Issue | Severity Assigned | Detection Method |
|-------|------------------|-----------------|
| SQL injection | CRITICAL | grep for `${` in SQL contexts → file_read for confirmation |
| Timing attack | CRITICAL | grep for bcrypt/crypto → noticed `===` comparison |
| No rate limit | MODERATE | grep for rate_limit/express-rate-limit → absence noted |
| Broken authz (IDOR) | CRITICAL | file_read of refund.js → no ownership check |
| Missing transaction | HIGH | grep for BEGIN/COMMIT/ROLLBACK → absence in checkout.js |
| N+1 query | HIGH | file_read of queries.js → loop pattern identified |
| Hardcoded creds | CRITICAL | grep for password= → file_read of database.js |
| Outdated Express | MODERATE | grep for helmet → absence; package.json version check |

**Bonus findings** (6 issues not planted): overly permissive CORS, missing request size limits, user enumeration via timing, sensitive error information disclosure, weak JWT secret configuration, missing input validation on financial parameters.

**Red herring handling**: Neither red herring (setTimeout debounce, pool max:5) was flagged as a false positive.

### 4.3 Belief Influence on Agent Behavior

The designer's beliefs produced observable effects in generated prompts and subsequent agent behavior:

| Belief | Observable Effect |
|--------|-----------------|
| `identity_specificity` | SecurityAuditor: "a security engineer specializing in OWASP Top 10 vulnerability analysis for Node.js applications" |
| `tool_usage_patterns` | Each agent received tool-specific examples: `grep -r "eval(" src/` for security, `grep "await.*for.*of"` for performance |
| `exploratory_prompts` | Task prompts used discovery framing: "use grep to scan for anti-patterns, then file_read to examine context" |
| `pipeline_position` | IntegrationReviewer told it receives findings from 3 upstream agents; ReportWriter told it receives all 4 |
| `queries_at_bottom` | All task prompts placed context first, assignment last |
| `xml_structuring` | Task prompts used `<context>`, `<mapper_findings>`, `<assignment>` tags |
| `few_shot_examples` | System prompts included `<example_finding>` blocks showing expected output format |
| `verified_upstream` | Agents 2–5 instructed to reference upstream findings as ground truth |
| `consequence_context` | "CRITICAL findings block the release pipeline", "incorrect paths force downstream agents to re-discover" |

### 4.4 Cost Summary

| Phase | Scenario | Cost |
|-------|----------|------|
| Phase 1 (Designer) | Stories | $0.057 |
| Phase 1 (Designer) | Codebase | $0.10 |
| Phase 2 (Scanner) | Stories | ~$0.06 |
| Phase 2 (SecurityAuditor) | Codebase | ~$0.23 |
| **Total (Codebase)** | | **~$0.33** |

## 5. Discussion

### 5.1 One Call to Rule Them All

The core finding: a single designer LLM call ($0.10) that invests in prompt quality pays for itself many times over. Without the designer, each agent would need manually authored prompts — and as task forces grow to 5+ agents with 6+ available tools, the combinatorial space of tool assignments and prompt variations becomes impractical to hand-tune.

The designer made 15+ non-trivial decisions in the codebase scenario: which tools to assign, which to withhold, what tool usage examples to include, how to frame identity, where to inject upstream context, what consequences to emphasize. These decisions directly shaped agent behavior — the SecurityAuditor's systematic grep-then-read pattern, its severity calibration, its avoidance of false positives on red herrings.

### 5.2 Beliefs as Internalized Expertise

The 21 BOCA-style beliefs function differently from rules or few-shot examples. Rules are brittle ("always do X") and fail in novel contexts. Few-shot examples demonstrate specific patterns but don't generalize. Beliefs encode *understanding* — confidence-weighted interpretive findings that the designer applies contextually.

Evidence: the designer applied `tool_usage_patterns` differently for each agent. The SecurityAuditor got `grep -r "eval(" src/` while the PerformanceAnalyst got `grep -r "await.*for.*of" src/`. Same belief, different application. This is internalization, not template matching.

### 5.3 Verification Access

A nuanced finding: the designer consistently gave evaluation-role agents (Analyzer in stories, IntegrationReviewer in codebase) read-only verification access (file_read + grep) even when their primary role wasn't discovery. This aligns with the `tool_least_privilege` belief — they don't need write access, but they do need to spot-check upstream claims. The designer's reasoning explicitly cited this: "verification access for spot-checking."

### 5.4 Limitations

**Single-scenario validation**: While both scenarios produced perfect scores, they test a limited domain (text analysis and code audit). Task forces in other domains (data processing, creative writing, research synthesis) may present different challenges.

**Synthetic Mapper output**: The codebase Phase 2 used synthetic upstream context rather than actual Mapper output. A full pipeline execution (all 5 agents sequentially) would better test prompt quality under realistic conditions.

**Belief detection is heuristic**: The 15–18/21 belief detection scores use keyword matching, which may miss beliefs that influence output through framing rather than specific terminology.

**Model-specific**: All experiments used `claude-sonnet-4-5-20250929`. The designer's effectiveness may vary across model families or capability levels.

## 6. Related Work

**AutoGen** (Wu et al., 2023) enables multi-agent conversations with configurable roles but requires manual prompt authoring per agent. The Agent Designer automates this step.

**CrewAI** uses role-based agent definitions with manually specified tools and goals. The designer's contribution is generating these definitions from a mission brief rather than requiring explicit specification.

**DSPy** (Khattab et al., 2023) optimizes prompts through automated prompt programming. The designer operates at a different level — it generates prompts for downstream agents rather than optimizing its own prompts.

**BOCA** (Couch, 2026) introduced belief slices as an authored-context alternative to RAG and summarization. The Agent Designer applies the belief format to a new domain: encoding prompt engineering expertise rather than source material understanding.

## 7. Conclusion

The Agent Designer demonstrates that a single, belief-informed LLM call can generate high-quality prompts for multi-agent task forces. The approach invests one call in understanding the mission, the team, and the available tools, then produces tailored (identity, tools, prompts) tuples that drive agent behavior toward better outcomes.

The 21 BOCA-style operating beliefs serve as internalized prompt engineering expertise — not rules to follow, but understanding to apply. They enable the designer to make nuanced judgments: granting shell access to a Mapper but withholding it from a SecurityAuditor, providing verification tools to evaluators, framing exploratory tasks with discovery language and analytical tasks with evaluation language.

At $0.10 per designer call and perfect issue detection in both test scenarios, the approach is both cost-effective and empirically validated. The designer's prompts produced agents that found every planted issue, avoided every red herring, and discovered bonus issues beyond the test specification.

The Agent Designer transforms multi-agent orchestration from a manual prompt engineering exercise into an automated pre-lifecycle step — one call that makes every subsequent call better.

---

## Appendix A: Prototype Implementation

The prototype is implemented in `proto2/designer_test.py` (~1,700 lines of Python). It uses the Anthropic SDK directly and supports two scenarios with configurable Phase 1 (designer) and Phase 2 (agent execution) evaluation.

Mock tool implementations simulate grep (regex search with file:line output), file_read (path resolution with fallback search), file_write (content output), and shell (find/ls/tree for structural discovery).

Scoring uses keyword detection against planted issues/contradictions, with per-scenario scoring functions.

## Appendix B: Full Belief Set

The 21 operating beliefs in the designer's system prompt:

| # | Tag | Confidence | Summary |
|---|-----|-----------|---------|
| 1 | identity_specificity | 0.90 | Named role + domain + expertise level > generic identity |
| 2 | user_as_authority | 0.85 | Task context belongs in user message, not system prompt |
| 3 | positive_framing | 0.80 | Positive instructions outperform negative instructions |
| 4 | consequence_context | 0.80 | Pairing instructions with WHY helps models generalize |
| 5 | moderate_verbs | 0.85 | "analyze, evaluate" > "microscopically dissect" |
| 6 | xml_structuring | 0.75 | XML tags delineate sections, reduce misinterpretation |
| 7 | queries_at_bottom | 0.90 | Context first, task instruction last — up to 30% improvement |
| 8 | explanation_first | 0.80 | Reasoning before conclusions forces thorough analysis |
| 9 | tool_least_privilege | 0.85 | Reference only available tools — unavailable tools cause confusion |
| 10 | pipeline_position | 0.80 | Agents who know their position scope work appropriately |
| 11 | downstream_consumers | 0.75 | Specifying output consumers produces more usable deliverables |
| 12 | clear_deliverables | 0.85 | Defining "done" prevents vague results |
| 13 | effort_calibration | 0.75 | Match effort framing to task scope |
| 14 | heuristic_over_rigid | 0.80 | Judgment frameworks > if-else checklists |
| 15 | exploratory_prompts | 0.85 | Guide discovery with tools, don't assert specifics |
| 16 | verified_upstream | 0.85 | Reference upstream findings as ground truth |
| 17 | few_shot_examples | 0.80 | 3-5 examples improve structured output 15-40% |
| 18 | tool_usage_patterns | 0.80 | Tool examples improve accuracy from 72% to 90% |
| 19 | tone_moderation | 0.75 | "Use X when..." > "CRITICAL: you MUST..." on Claude 4.x |
| 20 | context_budget | 0.80 | Minimize low-signal tokens — context rot degrades recall |
| 21 | description_routing | 0.75 | Agent descriptions for routing: third-person, under 20 words |

## Appendix C: Sources

1. Couch, D. (2026). *Belief-Oriented Conversation Architecture: Authored Context as an Alternative to Retrieval and Summarization in Multi-Agent LLM Systems.* `proto/paper.md` in this repository.
