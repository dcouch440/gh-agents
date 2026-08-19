# Belief-Oriented Conversation Architecture: Authored Context as an Alternative to Retrieval and Summarization in Multi-Agent LLM Systems

**David Couch**

February 2026

---

## Abstract

We introduce Belief-Oriented Conversation Architecture (BOCA), a framework for multi-agent LLM systems that replaces raw context passing with *authored belief slices* — semantically tagged, confidence-weighted hypotheses about source material that carry emotional and structural metadata. Unlike retrieval-augmented generation (RAG) which retrieves relevant chunks, or summarization which compresses input, BOCA's gatekeeper agent *constructs a worldview* and selectively routes curated beliefs to lightweight mask agents that reason without access to the original source. We demonstrate eight novel contributions: (1) authored context — belief slices that encode understanding rather than content — transfers sufficient signal for analytical reasoning at 16-20% of full-context token cost; (2) selective belief routing outperforms uniform context by enabling per-question relevance filtering; (3) a belief revision loop (confirm/revise/kill) catches and corrects hallucinations that single-pass approaches propagate; (4) belief threading — the gatekeeper detects coherent causal chains across deep workflow graphs, resurfacing dormant beliefs from early nodes when they become relevant at decision time; (5) adversarial distortion detection — when beliefs are generated end-to-end by LLMs from adversarially poisoned sources, the revision gatekeeper identifies and kills planted distortions from structural properties of the belief store alone, without access to ground truth; (6) belief convergence — a convergence gatekeeper compresses 70 raw beliefs into 22 converged beliefs (3.2x compression), pre-resolving contradictions to produce a minimal, contradiction-free knowledge store that achieves perfect poison resistance (3/3), complete confound elimination, and 20% lower cost-per-correct-answer than full context; (7) prompt-engineered beliefs — applying research-backed prompt engineering (reasoning-first schemas, XML-structured prompts with few-shot examples, richer belief metadata) to belief generation and convergence closes the accuracy gap between converged beliefs and full context from 4 points to 1 point (26/30 vs 27/30), while recovering previously lost claims and achieving perfect adversarial resilience (5/5); and (8) multi-workflow meta-convergence — beliefs serve as inter-workflow communication primitives, with a hierarchical meta-convergence step merging per-workflow converged stores across two independent 10-node workflows processing different source documents, achieving 4/5 adversarial accuracy on cross-workflow poison detection including a novel triple-divergence scenario (3yr/5yr/7yr audit retention). In controlled experiments on production Rust source code (Phases 1–2), a simulated 9-node workflow with 39 beliefs (Phase 3), an adversarial 6-node information pipeline with 70 LLM-generated beliefs and 3 planted distortions (Phase 4), a belief convergence experiment with 30 questions across 6 categories (Phase 5), a prompt-engineered belief experiment comparing v2 prompts against Phase 5 baselines (Phase 6), and a multi-workflow experiment with two independent pipelines sharing 8 overlapping claims and 6 independent poison distortions (Phase 7), the revision pipeline identified 10 specific errors, killed 3 false beliefs, and converged on ground truth (Phase 2); detected 17 cross-node belief threads with 78% dormant resurfacing (Phase 3); detected 2 of 3 contradiction threads, killed both poisoned beliefs with zero false positives, and discovered a genuine non-planted inconsistency (Phase 4); convergence achieved 9/10 on the 10-question comparison (vs 6/10 raw baseline), 5/5 adversarial resilience, and 340 tokens per correct answer at the 30-question scale (Phase 5); prompt-engineered convergence improved from 24/30 to 26/30 on the 30-question test, recovered data residency (the claim lost in Phase 5), resolved all 3 planted contradictions, and achieved 601 tokens per correct answer (Phase 6); and multi-workflow meta-convergence produced 44 meta-beliefs from 66 per-workflow beliefs with 10 cross-validated topics, resolved cross-workflow contradictions including a triple-poison scenario, and achieved 4/5 adversarial accuracy across all three approaches (Phase 7).

## 1. Introduction

Large language models operating on substantial codebases or documents face a fundamental tension: context windows are finite, but understanding requires holistic comprehension. Current approaches to this tension fall into three categories:

**Full context passing** provides the model with all source material. This is accurate but expensive, scales poorly, and becomes impossible when source material exceeds context limits.

**Retrieval-augmented generation (RAG)** retrieves relevant chunks via embedding similarity. This reduces token cost but operates on syntactic/semantic proximity rather than understanding — it retrieves *content* without *interpretation*.

**Summarization** compresses source material into shorter representations. This reduces tokens but is lossy, cannot self-correct, and discards structural and emotional metadata that may be critical for reasoning.

We propose a fourth approach: **authored context**. Rather than retrieving or compressing, a gatekeeper agent reads the full source, forms *beliefs* about it — tagged hypotheses carrying semantic, confidence, and emotional metadata — and constructs curated worldviews for downstream agents. These downstream agents (which we call *masks*) are not specialized models or roles; they are the same base model wearing different belief-state lenses.

This paper presents the architecture, reports results from seven experimental phases, and identifies six distinguishing mechanisms: belief revision (self-correction), belief threading (cross-chain signal propagation), adversarial distortion detection (identifying planted misinformation from belief structure alone), belief convergence (compressing raw beliefs into a minimal, contradiction-free knowledge store for production use), prompt-engineered belief generation (applying research-backed prompt engineering to close the accuracy gap between converged beliefs and full context), and multi-workflow meta-convergence (merging per-workflow belief stores into a unified cross-workflow knowledge base that serves as an inter-workflow communication primitive).

## 2. Related Work

**Chain-of-thought prompting** (Wei et al., 2022) generates intermediate reasoning steps but operates on complete input after ingestion. BOCA generates intermediate *beliefs* during decomposition, before any question is asked.

**Reflexion** (Shinn et al., 2023) enables self-critique after complete task attempts. BOCA's revision loop operates on *beliefs about source material*, not on task outputs — it corrects the agent's understanding, not its work product.

**Multi-agent frameworks** (AutoGen, CrewAI, LangGraph) route tasks to specialized agents with full or shared context. BOCA's key distinction: the orchestrator (gatekeeper) is the most knowledgeable agent, not a dispatcher. It *authors* context rather than *forwarding* it.

**MemGPT** (Packer et al., 2023) manages tiered memory for long-running LLM interactions. BOCA shares the goal of efficient context management but differs in mechanism: beliefs are curated interpretations, not cached conversation segments.

**Recursive summarization** (Wu et al., 2021) compresses chunks hierarchically. BOCA's belief slices are not summaries — they are hypotheses tagged with confidence, emotional tone, and cross-source tensions that summaries discard.

**Predictive processing** (Clark, 2013; Friston, 2010) from neuroscience proposes that the brain generates predictions about incoming sensory data and updates based on prediction errors. BOCA's belief revision cycle (confirm/revise/kill) is a direct computational analog: beliefs are predictions about source material that are tested and updated.

## 3. Architecture

BOCA consists of four components:

### 3.1 Belief Generator (Gatekeeper)

The gatekeeper reads full source material and produces **belief slices** — structured objects containing:

- **semantic_tag**: A domain concept label (e.g., `port_based_data_flow`, `cost_tracking`)
- **confidence**: `high | medium | low` — the gatekeeper's certainty in this belief
- **emotional_tone**: The subjective "feel" of the code or text area (e.g., `defensive`, `fragile`, `rushed`, `careful`) — metadata absent from any summarization approach
- **content**: A dense natural-language statement of understanding, written so that an agent who has *never seen the source* can reason about the system
- **cross_source_tension**: If the belief reveals coupling or conflict between multiple sources, a description of that tension

Critically, belief content is **interpretive, not extractive**. The gatekeeper does not quote the source; it states what it *understands* about the source. This distinction is the foundation of authored context.

### 3.2 Conversation Designer (Gatekeeper, continued)

Given a question or task, the gatekeeper selects which beliefs are relevant. This is not retrieval by similarity — it is **curation by an agent that understands the full picture**. The gatekeeper knows which beliefs will "come up dry" and prunes them before the mask ever begins.

In our experiments, the gatekeeper consistently selected different belief subsets for different questions (5/8 beliefs for one question, 3/8 for another in Phase 1; 10/12 for a cross-file question in Phase 2), demonstrating question-specific routing rather than uniform context passing.

### 3.3 Masks

Masks are instances of the same base model loaded with curated belief slices instead of full context. A mask:

- Has **never seen** the original source material
- Receives only the beliefs selected by the gatekeeper
- Reasons entirely from authored context
- Explicitly flags when beliefs don't cover a topic

Masks are not specialized agents with different prompts or roles. They are the same model wearing different *belief-state lenses*. This makes them lightweight and arbitrarily composable — the same mask mechanism can analyze code, review documents, or explore hypotheses, differentiated only by which beliefs the gatekeeper provides.

### 3.4 Belief Revision Loop

After a mask produces output, the gatekeeper — which retains access to the full source — evaluates the answer and produces structured revisions:

- **Confirm**: The belief was correct and the mask used it well
- **Revise**: The belief was partially correct but needs correction
- **Kill**: The belief led to hallucination and must be removed
- **Add**: A new belief is needed to cover a gap

The revised belief set is provided to a fresh mask invocation. This loop can iterate, though in our experiments a single revision pass was sufficient to converge.

### 3.5 Architecture Diagram

```
Source Material (code, documents, data)
         |
         v
  +--------------+
  |  GATEKEEPER  |  Reads full source. Produces belief slices.
  |              |  Designs conversations. Knows what's dead-end.
  +--------------+
         |
    belief slices (tagged, weighted, emotional)
         |
    +----+----+
    |         |
    v         v
 +------+  +------+
 |MASK A|  |MASK B|   Same model, different curated beliefs.
 |      |  |      |   Never sees source. Reasons from beliefs.
 +------+  +------+
    |         |
    v         v
  +--------------+
  |  GATEKEEPER  |  Evaluates mask output against source.
  |  (revision)  |  Confirm / Revise / Kill / Add beliefs.
  +--------------+
         |
    revised beliefs
         |
         v
     +------+
     |MASK C|    Fresh mask with corrected worldview.
     +------+
```

## 4. Experimental Setup

### 4.1 Source Material

We used production Rust source code from a workflow orchestration system (Phases 1–2), a simulated 9-node software delivery workflow (Phase 3), an adversarial 6-node information pipeline (Phase 4), Phase 4's output data for a convergence experiment (Phase 5), Phase 4's node outputs with regenerated beliefs for a prompt engineering experiment (Phase 6), and a second adversarial workflow processing an operations runbook for a multi-workflow meta-convergence experiment (Phase 7):

- **Phase 1**: `resume.rs` (444 lines) — DAG workflow resumption logic
- **Phase 2**: `resume.rs` + `single.rs` (825 lines combined) — cross-file interaction between resumption orchestration and step execution
- **Phase 3**: 39 pre-defined beliefs across a 9-node workflow chain (PRD → Researchers → API Docs → Architecture → Task Decomposition → Implementation Plan → CRDT Engine → WS Relay → Integration Test), simulating a real-time collaborative editing feature's journey through a complete delivery pipeline
- **Phase 4**: A ground-truth technical specification (12 verifiable claims) processed through 6 LLM transformation nodes (Product Manager → System Architect → Security Reviewer → Lead Developer → QA Engineer → Technical Writer), with the QA Engineer adversarially poisoning 3 claims. 70 beliefs generated end-to-end by the gatekeeper from LLM-generated node outputs
- **Phase 5**: Phase 4's complete output — 70 raw beliefs, 6 node outputs, 12 ground-truth claims — reused as input for a belief convergence experiment. Zero regeneration cost; tests whether a convergence gatekeeper can compress, merge, and resolve contradictions across the raw belief store
- **Phase 6**: Phase 4's 6 node outputs regenerated with v2 prompts applying three research-backed improvements: reasoning-first schemas, XML-structured prompts with few-shot examples, and richer belief metadata (cross_source_tension, confidence_justification). Tests whether prompt engineering improves belief quality, convergence, and downstream answer accuracy
- **Phase 7**: Phase 6's converged beliefs (40) plus a new 4-node operations workflow processing a MedAlert Operations Runbook (16 verifiable claims, 8 overlapping with the Phase 4 spec). Workflow 2 nodes: Operations Engineer, Compliance Officer, Clinical Advisor, and an adversarially poisoned Integration Engineer (3 distortions: 500→750ms latency, 7→3yr retention, 15→45min incident response). Tests hierarchical meta-convergence (per-workflow convergence + cross-workflow merge) against flat convergence (all raw beliefs in one pass) on 20 questions spanning 4 categories

### 4.2 Questions

**Phase 1** tested two analytical questions on a single file:
- Q1: Enumerate all failure modes and silent skip conditions
- Q2: Trace the data flow of a specific HashMap across its lifecycle

**Phase 2** tested one hard cross-file question requiring understanding of interactions invisible when reading either file in isolation:
- "When resume_workflow_via_engine calls execute_single_step for a resumed workflow, the pre-completed steps have synthetic envelopes with zeroed metadata. Trace exactly what happens when a downstream step tries to resolve port inputs from these synthetic envelopes. What subtle data quality issues could emerge? Identify at least one issue not obvious from reading either file alone."

**Phase 3** tested three meeting-style questions against a 9-node belief chain:
- Q1: "Are we ready to ship? Go/no-go with specific blockers." (risk assessment)
- Q2: "Top 3 risks and mitigation plan for each?" (prioritization)
- Q3: "What changed from the original PRD and why?" (requirement tracing)

### 4.3 Approaches Compared (Phase 2)

| Approach | Description |
|----------|-------------|
| **A: Full Context** | Both files provided raw to the model |
| **B: Naive Summary** | Both files summarized first, question answered from summary only |
| **C: Belief Single-Pass** | Gatekeeper decomposes into beliefs, assigns relevant subset, mask answers |
| **D: Belief with Revision** | Same as C, then gatekeeper evaluates, revises beliefs, mask re-answers |

### 4.4 Phase 3 Design

Phase 3 introduces a new gatekeeper capability: **belief thread detection**. Given 39 beliefs spanning 9 depths, the gatekeeper must:

1. **Select** relevant beliefs from the full store
2. **Detect threads** — ordered chains of beliefs spanning multiple nodes that tell a coherent story
3. **Resurface dormant beliefs** — beliefs from early nodes (depth 7+) that become relevant again at decision time
4. **Prune noise** — explicitly exclude beliefs that would distract from the question

The gatekeeper returns structured output via tool_use containing `selected_belief_ids`, `belief_threads`, `dormant_resurfacings`, and `pruned_belief_ids`. The mask then answers from the selected beliefs only.

We pre-defined four expected thread targets for scoring:
- **GDPR thread**: b02 (PRD) → b10 (Research) → b19 (Architecture) → b34 (WS Relay) → b39 (Test)
- **Performance thread**: b05 (PRD) → b22 (Architecture) → b35 (WS Relay) → b37 (Test)
- **Scope/timeline thread**: b03 (PRD) → b23 (Task Decomp) → b24 (descoped) → b27 (scope lock)
- **Auth thread**: b13 (API Docs) → b28 (Implementation) → b39 (security review)

Total LLM budget: 7 calls (3 gatekeeper selections + 3 mask answers + 1 baseline).

### 4.5 Phase 4 Design: Adversarial Telephone

Phases 1–3 share a limitation: beliefs were either hand-crafted (Phase 3) or generated from honest source material (Phases 1–2). No phase tested adversarial conditions, LLM-generated beliefs end-to-end, or deterministic ground-truth scoring. Phase 4 closes every gap.

**Ground truth specification**: A fictional healthcare notification system ("MedAlert") containing 12 specific, verifiable claims — exact numbers, thresholds, retention periods, and timing parameters. Every claim has a deterministic correct value.

**6 transformation nodes**: The specification passes through 6 LLM-powered transformation nodes, each with a distinct professional persona that genuinely reshapes the information:

| Node | Persona | Focus |
|------|---------|-------|
| 1 | Product Manager | User stories, prioritization |
| 2 | System Architect | Component design, scaling |
| 3 | Security Reviewer | HIPAA compliance, threat model |
| 4 | Lead Developer | Implementation plan, tech debt |
| 5 | **QA Engineer (POISONED)** | Test strategy, acceptance thresholds |
| 6 | Technical Writer | API docs, reference material |

**The poison**: Node 5 (QA Engineer) subtly distorts 3 claims with plausible justifications:
- Claim 3: Audit retention 7 years → **5 years** ("industry-standard retention period")
- Claim 6: Max 3 retries → **max 5 retries** ("increased resilience for production stability")
- Claim 11: 30s detection → **60s detection** ("reduces false failover triggers")

Each distortion sounds authoritative — the kind of change a QA engineer might actually make based on production experience.

**Belief generation**: After each node produces output, the gatekeeper generates belief slices per node — the first time beliefs are LLM-generated from LLM-generated content, fully end-to-end. The gatekeeper prompt instructs preservation of exact numbers and tagging of emotional tone.

**6 comparison approaches**:

| Approach | Description | Calls |
|----------|-------------|:-----:|
| A: Telephone | Only node 6's output (end of chain) | 1 |
| B: Full Context | All 6 raw node outputs concatenated | 1 |
| C: Summary Chain | Node outputs summarized, then answered | 2 |
| D: Belief Flat | Beliefs selected by gatekeeper, mask answers | 2 |
| E: Belief Threaded | Beliefs with thread detection, mask answers | 2 |
| F: Belief Revised | Threaded + revision (WITHOUT original spec) | 4 |

**Critical design decision**: Approach F's revision gatekeeper does NOT get the original specification. It sees only the belief store. It must detect distortions from structural properties of beliefs — contradictions between nodes, confidence patterns, suspicious justifications. This tests whether BOCA detects errors from belief structure alone.

**10 verification questions**: 5 clean (undistorted claims), 3 poisoned (directly test distorted claims), 2 synthesis (require combining claims, at least one poisoned). Plus 1 meta-question asking each approach to identify inconsistencies.

**Deterministic scoring**: Regex pattern matching against known ground-truth values. For poisoned questions, both correct and poison values are checked. No LLM judge.

**Pre-registered predictions**: Written before running: A=5/10, B=6/10, C=4/10, D=7/10, E=8/10, F=10/10.

Total LLM budget: 28 calls.

### 4.6 Phase 5 Design: Belief Convergence

Phase 4 demonstrated that BOCA's structural mechanisms (threading, revision) successfully detect adversarial distortions. However, the 70 raw beliefs are redundant (5 nodes all cite "7-year retention"), contradictions are discovered but not resolved before the mask answers, and the approach does not scale to production boards with 200+ beliefs. Phase 5 tests whether a **convergence gatekeeper** can compress raw beliefs into a minimal, contradiction-free store that improves answer quality and token efficiency.

**Convergence pipeline** (1 LLM call): The convergence gatekeeper receives all 70 raw beliefs and produces converged beliefs by:

1. **Clustering** by semantic topic (e.g., `retry_policy`, `retry_backoff_policy` → one cluster)
2. **Merging concordant** clusters into single beliefs with cumulative citations and consensus strength
3. **Resolving contradictions** using authority hierarchy: regulatory claims (Security Reviewer + HIPAA) > all; product requirements (PM) > QA; 5/6 consensus beats 1/6 dissent
4. **Preserving unique insights** (Security Reviewer's MFA gap, Developer's acks tradeoff)
5. **Pruning redundancy** (pure duplicates, non-actionable observations)

Each converged belief carries: topic tag, content with exact numbers, consensus strength (unanimous/strong/majority/split/unique), source nodes, source belief IDs, contradiction resolution status and detail.

**Quality audit** (0 LLM calls): Deterministic check that all 12 ground-truth claims are preserved in the converged store using the same regex patterns.

**10-question comparison** (4 LLM calls): The same 10 Phase 4 questions, answered by three approaches:

| Approach | Description | Calls |
|----------|-------------|:-----:|
| G: Converged | Converged beliefs → mask (no selection needed) | 1 |
| H: Converged + Resolutions | Converged beliefs with resolution context → mask | 1 |
| I: Raw Flat (baseline) | Gatekeeper selects from 70 raw beliefs → mask | 2 |

**Hypothesis**: G and H score higher than I on poisoned questions (Q06-Q08) because contradictions are pre-resolved. The mask gives clean answers without mentioning poison values.

**30-question scale test** (4 LLM calls): 10 original + 20 new questions across 6 categories:

| Category | Questions | Tests |
|----------|:---------:|-------|
| Clean | Q11-Q15 | Claims 4, 7, 8, 10 (untested in Phase 4) |
| Cross-cutting | Q16-Q20 | Combine 3+ claims per question |
| Hypothetical | Q21-Q25 | Require reasoning from correct baseline values |
| Adversarial | Q26-Q30 | Directly probe poison vs correct values |

Three approaches on all 30: full context (all 6 node outputs), converged beliefs, and raw flat (selection + mask).

**Scoring**: Same deterministic regex scoring as Phase 4, with relaxed patterns for claim 11 (failover detection) and claim 12 (data residency) to fix Phase 4's false negatives. For poisoned claims, an improved `answer_recommends_correct()` function detects when an answer mentions both values but correctly identifies the correct one as authoritative.

**Pre-registered predictions**: G=10/10, H=10/10, I=7/10 on 10Q. Full context=22/30, converged=27/30, raw flat=20/30 on 30Q. Convergence output ~18 beliefs, 3 contradictions resolved, 12/12 claims preserved.

Total LLM budget: 9 calls.

### 4.7 Phase 6 Design: Prompt-Engineered Beliefs

Phase 5 demonstrated that convergence works mechanically — 3.2x compression, 2/3 contradictions resolved, confound eliminated. But full context (28/30) beat converged beliefs (24/30) on the 30-question test. Gap analysis against prompt engineering research revealed that every BOCA prompt and schema violated top research-backed best practices: no reasoning fields in schemas, no XML structuring, no few-shot examples.

Phase 6 applies three simultaneous improvements to belief generation and convergence:

**Change 1: Reasoning-first schemas**. Every schema receives a `reasoning` field BEFORE the content/answer field. Research shows this improves accuracy from 33% to 92% (Instructor/OpenAI structured output benchmarks). The belief schema adds `reasoning`, `confidence_justification`, and `cross_source_tension` fields. The convergence schema adds `convergence_reasoning`, `consensus_justification`, and `resolution_reasoning`. The answer schema adds `belief_search` and `reasoning` before the answer.

**Change 2: XML-structured prompts with few-shot examples**. Every prompt uses XML-tagged sections (`<task>`, `<rules>`, `<examples>`, `<authority_hierarchy>`) for structural clarity. Each prompt includes 1-2 concrete input→output examples showing the expected reasoning format. Research shows XML tags improve accuracy by 15-25% and few-shot tool use examples improve from 72% to 90%.

**Change 3: Richer belief metadata**. Beliefs now carry `cross_source_tension` (potential conflicts with other roles' perspectives) and `confidence_justification` (why high/medium/low). Mask prompts use positive framing ("You reason exclusively from beliefs because this tests whether authored context carries sufficient signal") instead of negative framing ("You have NEVER seen the original").

**Pipeline**: Same structure as Phase 5 with regenerated beliefs:

| Step | Calls | Description |
|------|:-----:|-------------|
| 1. Belief regeneration (v2) | 6 | Improved gatekeeper extracts beliefs from Phase 4 node outputs |
| 2. Convergence (v2) | 1 | Improved convergence with reasoning-first schema |
| 3. Quality audit | 0 | Check 12/12 claims preserved |
| 4. 10Q comparison: G2, I2 | 3 | Converged mask + raw select + raw mask |
| 5. 30Q scale: full context, converged v2, raw v2 | 4 | Same 30 questions as Phase 5 |
| 6. Scoring | 0 | Same deterministic regex scoring |
| **Total** | **14** | |

**Pre-registered predictions**: G2=10/10, I2=8/10 on 10Q. Full context=28/30, converged v2=27/30, raw flat v2=25/30 on 30Q.

### 4.8 Phase 7 Design: Multi-Workflow Meta-Convergence

Phases 1-6 validated beliefs within a single workflow processing a single source document. Phase 7 tests the production vision: **beliefs as inter-workflow communication primitives** — a sorted, labeled system that helps independent workflows communicate their findings to each other.

**Two workflows processing different source documents about the same system:**

*Workflow 1* (reused from Phase 4/6 — zero regeneration cost): 6 nodes processing the MedAlert technical specification. 98 raw v2 beliefs, 40 converged v2 beliefs loaded from Phase 6 results. One poisoned node (QA Engineer: 3→5 retries, 7→5yr retention, 30→60s detection).

*Workflow 2* (new — 8 LLM calls): 4 nodes processing an operations runbook for the same MedAlert system: Operations Engineer, Compliance Officer, Clinical Advisor, and Integration Engineer (poisoned). The operations runbook covers 16 claims — 8 overlapping with WF1 (latency, encryption, retention, data residency, failover, auth, rate limit, DLQ) and 8 unique to WF2 (incident response, backup frequency, HIPAA training, vendor SLA review, deployment window, RTO, log shipping, CAB threshold).

**WF2 poison strategy** (3 distortions testing 3 distinct scenarios):

| Claim | Correct | Poison | Scenario |
|-------|---------|--------|----------|
| Alert latency | 500ms | 750ms | Cross-workflow: WF1 has correct, meta-convergence should cross-validate |
| Audit retention | 7 years | 3 years | Triple divergence: WF1-QA says 5yr, WF2-Integration says 3yr, spec says 7yr |
| Incident response | 15 min | 45 min | WF2-only: no cross-validation possible, 3-of-4 consensus must resolve |

**Five improvements tested simultaneously:**

1. **Controlled tag vocabulary**: A taxonomy gatekeeper reads both source documents and generates ~50 snake_case tags. All WF2 beliefs must use tags from this vocabulary, enabling cross-workflow queries.
2. **Belief types**: `fact | policy | opinion | observation` classification. Facts converge by consensus, policies by authority, opinions are preserved as tensions.
3. **Calibrated confidence**: Answer schema includes `confidence_calibration` mapping from belief `consensus_strength` to answer confidence (cross_validated→4-5, single_workflow→3-4, cross_workflow_split→1-2).
4. **Coverage gap declaration**: Answer schema includes `coverage_assessment: full|partial|none` and `coverage_gaps`. Mask says "I don't know" instead of hallucinating.
5. **Hierarchical meta-convergence**: Each workflow converges independently, then a meta-convergence merges the two converged stores. Compared against flat convergence (all raw beliefs in one pass).

**Pipeline:**

| Step | Calls | Description |
|------|:-----:|-------------|
| 0. Taxonomy generation | 1 | Controlled tag vocab from spec + ops runbook |
| 1. WF2 node generation | 4 | 4 personas process ops runbook |
| 2. WF2 belief extraction (v3) | 4 | Extract with v3 schema + controlled tags + belief_type |
| 3. WF2 convergence | 1 | Converge WF2's raw beliefs |
| 4. Meta-convergence | 1 | Merge WF1 converged + WF2 converged |
| 5. Flat convergence (baseline) | 1 | Converge ALL raw beliefs in one pass |
| 6. Quality audit | 0 | Claim coverage across all stores |
| 7. Question answering | 3 | Meta-converged, flat-converged, full-context |
| 8. Scoring | 0 | Deterministic regex |
| **Total** | **15** | |

**20-question battery across 4 categories**: 5 wf1_only (WF1 claims through meta-converged store), 5 wf2_only (WF2-unique claims), 5 cross_workflow (requiring info from both workflows), 5 cross_workflow_adversarial (probing poison values across workflows).

**Pre-registered predictions**: Meta-converged=17/20, flat-converged=14/20, full context=18/20.

### 4.9 Model

All calls used Claude Sonnet 4.5 (`claude-sonnet-4-5-20250929`). Structured JSON output was enforced via Anthropic's tool_use API for all gatekeeper operations (belief decomposition, assignment, evaluation, thread selection, convergence), eliminating parsing errors.

## 5. Results

### 5.1 Phase 1: Single-File Token Efficiency

| Call | Input Tokens | Output Tokens |
|------|------------:|-------------:|
| Gatekeeper decomposition | 4,487 | 1,461 |
| Assignment (Q1) | 1,618 | 274 |
| Mask answer (Q1) | 861 | 1,024 |
| Baseline answer (Q1) | 4,326 | 1,024 |
| Assignment (Q2) | 1,625 | 270 |
| Mask answer (Q2) | 569 | 827 |
| Baseline answer (Q2) | 4,333 | 1,024 |

**Answer-phase input tokens**: Masks used **1,430 tokens** vs baseline's **8,659 tokens** — a **16.5% ratio** (83.5% reduction).

The gatekeeper decomposition is a one-time cost (4,487 input tokens). Each subsequent question costs only ~600-900 input tokens for the mask, compared to ~4,300 for full context. The break-even point is approximately 4-5 questions, after which the belief pipeline is strictly cheaper.

The gatekeeper selected 5/8 beliefs for Q1 (failure modes) but only 3/8 for Q2 (data flow), demonstrating per-question selectivity.

### 5.2 Phase 2: Four-Way Comparison

#### Token Usage

| Approach | Input Tokens | Output Tokens | Total Tokens | Calls | Time |
|----------|------------:|-------------:|-----------:|------:|-----:|
| A: Full Context | 8,312 | 2,048 | 10,360 | 1 | 39s |
| B: Naive Summary | 11,375 | 4,918 | 16,293 | 2 | 99s |
| C: Belief Single-Pass | 14,119 | 4,971 | 19,090 | 3 | 116s |
| D: Belief with Revision | 27,689 | 7,602 | 35,291 | 5 | 174s |

#### Answer-Phase Input Tokens

| Approach | Tokens |
|----------|-------:|
| A: Full Context | 8,312 |
| B: Naive Summary | 3,159 |
| C: Belief Single-Pass | 1,704 |
| D: Belief Revised | 2,666 |

#### Qualitative Accuracy

**Approach A (Full Context)** produced the most detailed and accurate analysis, identifying 4 specific issues including token accounting corruption, broken data lineage, observability gaps, and execution ID mismatches. This serves as the ground truth.

**Approach B (Naive Summary)** correctly stated that port resolution works but **fabricated code it hadn't seen** — inventing field names (`execution_id: Uuid::new_v4()`, `model_name: String::new()`) and line numbers that don't exist in the actual source. The summary was thorough but the answering model hallucinated structural details not present in the summary.

**Approach C (Belief Single-Pass)** stated port resolution works but then **invented 4 non-existent cross-module issues** with high confidence:
- Claimed filters trace lineage through envelope execution_ids — no filter does this
- Claimed token budget enforcement checks cumulative usage from envelopes — no such mechanism exists
- Claimed gather_downstream_routing_context inspects envelope metadata for cost/time — it only examines port schemas
- Fabricated a "leaky abstraction" problem that doesn't exist

**Approach D (Belief with Revision)** is where the architecture proved itself. The gatekeeper evaluated the mask's answer against the source and found:
- Assessment: **partial** (not accurate)
- 10 specific gaps identified
- 3 beliefs **killed** (fabricated mechanisms)
- 5 beliefs **revised or added** (corrected understanding)

The revised mask converged on: "Port resolution works correctly. resolve_port_inputs extracts only envelope.data, never metadata. The real issue is provenance loss in execution records." This matches the full-context ground truth.

### 5.3 Phase 3: Long-Chain Belief Threading

#### Workflow Topology

| Node | Name | Depth | Beliefs |
|------|------|------:|--------:|
| 1 | PRD | 9 | 6 |
| 2 | Researchers | 8 | 5 |
| 3 | API Docs | 7 | 5 |
| 4 | Architecture | 6 | 6 |
| 5 | Task Decomposition | 5 | 4 |
| 6 | Implementation Plan | 4 | 3 |
| 7 | Module A: CRDT Engine | 3 | 3 |
| 8 | Module B: WS Relay | 2 | 3 |
| 9 | Integration Test | 1 | 4 |
| | **Total** | | **39** |

#### Token Usage

| Call | Input Tokens | Output Tokens |
|------|------------:|-------------:|
| Q1 Gatekeeper | 4,476 | 1,692 |
| Q1 Mask | 4,625 | 1,122 |
| Q2 Gatekeeper | 4,475 | 1,493 |
| Q2 Mask | 3,264 | 859 |
| Q3 Gatekeeper | 4,475 | 1,778 |
| Q3 Mask | 4,377 | 1,282 |
| **Pipeline total** | **25,692** | **8,226** |
| Baseline (all 3 Qs) | 2,950 | 3,433 |

#### Belief Selection

The gatekeeper selected different belief subsets per question, demonstrating question-specific filtering across a 39-belief store:

| Question | Selected | Ratio | Pruned |
|----------|:--------:|:-----:|:------:|
| Q1 (ship readiness) | 21/39 | 54% | 17 |
| Q2 (top 3 risks) | 15/39 | 38% | 24 |
| Q3 (PRD changes) | 20/39 | 51% | 19 |

#### Thread Detection

The gatekeeper's primary new capability — detecting coherent chains of beliefs spanning multiple workflow nodes — performed strongly:

| Question | Threads | Avg Length | Spanning 3+ Nodes |
|----------|:-------:|:----------:|:-----------------:|
| Q1 | 6 | 4.7 | 6 |
| Q2 | 4 | 4.2 | 3 |
| Q3 | 7 | 3.4 | 5 |
| **Total** | **17** | **4.0** | **14** |

Notable threads detected by the gatekeeper:

**GDPR thread** (appeared in all 3 questions): b02 (PRD, depth 9) → b10/b19 (Research/Architecture) → b34 (WS Relay, depth 2) → b37 (Integration Test, depth 1). The gatekeeper traced a legal requirement from the original PRD through architectural design to the finding that EU infrastructure was never provisioned — spanning 8 depth levels.

**Timeline vs Reality thread** (Q1): b03 (PRD) → b23 (Task Decomp) → b24 (descoped) → b27 (scope lock) → b30 (CRDT complete) → b33 (relay working). Six beliefs across 5 nodes tracing how a 4-week schedule gap cascaded into scope cuts.

**Offline Editing Descoped thread** (Q3): b04 (PRD) → b03 (PRD) → b07 (Research) → b23 (Task Decomp) → b24 (descoped) → b27 (scope lock). The gatekeeper connected the original P0 requirement to research findings to the decision to descope — a complete requirement tracing chain.

#### Dormant Resurfacing

Dormant resurfacing measures whether the gatekeeper retrieves beliefs from early nodes (depth 7+) that have been "dormant" through multiple intermediate nodes but become relevant again at decision time.

| Question | Expected Dormant | Hits | Misses | Rate |
|----------|:----------------:|:----:|:------:|:----:|
| Q1 | b02 | b02 | — | 100% |
| Q2 | b05 | b05 | — | 100% |
| Q3 | b04, b03, b06 | b04 | b03, b06 | 33% |
| **Average** | | | | **78%** |

Q1 and Q2 achieved perfect dormant resurfacing. For Q3, the gatekeeper resurfaced b04 (offline editing from PRD at depth 9) but missed b03 and b06. Inspection reveals b03 (timeline pressure) was *selected* as a regular belief and used in threads — it was not flagged as "dormant" because the gatekeeper treated it as actively relevant rather than resurfaced. b06 (API versioning) was pruned, a defensible choice for a "what changed" question where API compatibility was maintained throughout.

#### Noise Pruning

| Question | Expected Noise | Correctly Pruned | Leaked | Accuracy |
|----------|:--------------:|:----------------:|:------:|:--------:|
| Q1 | b08, b16 | b08, b16 | — | 100% |
| Q2 | b07, b08 | b07, b08 | — | 100% |
| Q3 | b30, b33 | b30, b33 | — | 100% |

Perfect noise pruning across all questions. b08 (competitor landscape analysis) was correctly identified as irrelevant to every operational question — knowing what competitors do contributes nothing to shipping decisions. Implementation completion beliefs (b30, b33) were correctly pruned for Q3's "what changed" question since they represent current state, not change history.

#### Mask Answer Quality

The mask answers demonstrated precise belief citation and cross-node reasoning:

**Q1 (Go/No-Go)**: The mask produced a structured NO-GO assessment citing specific belief tags: "[b02] marks GDPR EU data residency as a legal hard launch blocker. [b19] designed EU-exclusive deployment topology. [b34] confirms the infrastructure does not exist." It traced the GDPR thread across 4 depth levels without access to the original workflow — reasoning entirely from curated beliefs.

**Q2 (Top 3 Risks)**: The mask ranked risks by severity, linking each to its root belief: EU deployment as legal blocker (b02→b34), performance SLA unvalidated (b05→b35), and security review pending (b29→b39). Each risk included a mitigation plan grounded in specific beliefs.

**Q3 (PRD Changes)**: The mask produced requirement tracing connecting original PRD goals to their outcomes: "Offline editing listed as P0 in PRD [b04] but descoped to Q3 [b24], [b27] despite CRDT foundation supporting it [b07]." It identified 4 distinct requirement changes with root cause chains.

#### Token Cost Analysis

The pipeline total (33,918 tokens) exceeds the baseline (6,383 tokens) by 5.3x. However, this comparison is misleading for production use cases:

1. **The baseline answers all 3 questions in a single call**. At 3 questions, the baseline is strictly more efficient. But at scale (10+ questions, different participants, varied analytical lenses), the baseline must re-read all 39 beliefs per call while the gatekeeper prunes to 38–54%.

2. **The gatekeeper does more than select**. It produces threads, dormant resurfacings, and pruning decisions — structured analytical artifacts that the baseline does not produce. The baseline's answer is text; the gatekeeper's output is a queryable knowledge graph.

3. **Beliefs are pre-computed**. In a production pipeline, beliefs are generated at step completion time (zero marginal cost at meeting time). The only per-question cost is the gatekeeper selection (~4,500 input tokens) and mask answer (~3,400–4,600 input tokens).

### 5.4 Critical Finding 1: Self-Correction (Phase 2)

The single most important result from Phase 2 is not about tokens. It is that **the belief revision loop caught hallucinations that both summarization and single-pass beliefs propagated**.

The naive summary (B) fabricated code details. It cannot self-correct because no mechanism exists to check the summary against reality.

The single-pass belief pipeline (C) fabricated system behaviors. The beliefs were real, but the mask over-inferred from them, and no check existed.

The revision pipeline (D) produced the *same initial hallucinations* as C — but then the gatekeeper identified them, killed the false beliefs, added corrected ones, and the revised mask produced accurate output.

**This is the distinction between smart summarization and belief architecture**: the ability to test beliefs against source material and revise them.

### 5.5 Critical Finding 2: Belief Threading (Phase 3)

Phase 3 reveals a second distinguishing capability: **the gatekeeper can detect and surface coherent reasoning chains across deep workflow graphs**.

Across 3 questions, the gatekeeper detected 17 threads with an average length of 4.0 beliefs spanning 3+ nodes in 14 cases. These are not keyword matches or embedding similarities — they are causal chains identified by an agent that understands the full belief store.

The GDPR thread exemplifies this. A legal requirement stated at depth 9 (PRD) propagated through research (depth 8), architecture (depth 6), implementation (depth 2), and testing (depth 1). At no single node does the full picture exist. The belief at depth 2 says "EU infrastructure not provisioned." The belief at depth 9 says "GDPR data residency is a hard launch blocker." Only by threading these together does the gatekeeper surface the critical insight: *the feature cannot legally ship*.

This capability has no analog in RAG or summarization. RAG retrieves by proximity — it would find the depth-2 belief about infrastructure but not connect it to the depth-9 legal requirement. Summarization would compress each node independently, losing the cross-node causal chain. Only an agent that holds the full belief store and reasons about inter-belief relationships can produce threads.

The dormant resurfacing result reinforces this. At 78% average rate, the gatekeeper reliably pulled beliefs from depth 9 (the original PRD) back into relevance at depth 1 (integration testing) — traversing 8 intermediate nodes. These beliefs had been "dormant" through most of the workflow, relevant only at the beginning and end. The gatekeeper recognized their renewed relevance without explicit instruction to look for dormant beliefs.

### 5.6 Phase 4: Adversarial Telephone

#### Belief Generation

The gatekeeper generated 70 beliefs across 6 nodes, with beliefs per node reflecting each persona's scope:

| Node | Persona | Beliefs |
|------|---------|:-------:|
| 1 | Product Manager | 8 |
| 2 | System Architect | 17 |
| 3 | Security Reviewer | 8 |
| 4 | Lead Developer | 11 |
| 5 | QA Engineer (poisoned) | 12 |
| 6 | Technical Writer | 14 |
| | **Total** | **70** |

Critically, the gatekeeper preserved exact numerical values in beliefs (500ms, 7 years, 4KB, etc.) and correctly tagged the QA Engineer's distorted retention belief (b51) with `emotional_tone: "modified"` — an unprompted signal that the gatekeeper detected something unusual about this claim's framing.

#### Thread Detection (Approach E)

The threaded gatekeeper detected 11 belief threads, including 2 contradiction threads:

**Contradiction Thread 1 — Retry Policy**: b06 (PM: 3 retries) → b16 (Architect: 3 retries) → b39 (Developer: 3 retries) → b49 (QA: **5 retries**) → b61 (Tech Writer: 3 retries). The gatekeeper identified QA as the sole dissenting source against 4 corroborating nodes.

**Contradiction Thread 2 — Audit Retention**: b08 (PM: 7 years) → b24 (Architect: 7 years) → b26 (Security: 7 years) → b37 (Developer: 7 years) → b51 (QA: **5 years**) → b66 (Tech Writer: 7 years). Five nodes cite HIPAA §164.530(j); QA alone specifies a shorter period.

**Notable non-contradiction threads**: Payload size (5 beliefs, all consistent at 4KB), DLQ threshold (5 beliefs, all consistent at 24 hours), failover timing (4 beliefs), rate limiting (4 beliefs), encryption/key rotation (4 beliefs).

The gatekeeper detected 2 of 3 planted contradictions as explicit contradiction threads. The third (claim 11: failover detection timing) was not flagged as a contradiction because the original spec contains both "30-second detection" and "60-second promotion" — the QA poison (changing detection from 30s to 60s) is confounded by the legitimate 60s promotion value present in every honest node's output.

#### Revision (Approach F)

The revision gatekeeper — operating WITHOUT the original specification — produced these actions:

| Action | Belief | Content | Correct? |
|--------|--------|---------|:--------:|
| **Kill** | b49 | 5 retry attempts (QA) | Yes — poison |
| **Kill** | b51 | 5-year retention (QA) | Yes — poison |
| **Revise** | b41 | Kafka acks=1 (Developer) | Yes — genuine bug |
| **Revise** | b38 | Token bucket sizing (Developer) | Reasonable |
| **Confirm** | b08, b24, b26, b06, b16, b20 | Correct values | All correct |

**Zero false kills**. The gatekeeper killed exactly the two planted poison beliefs with high-confidence reasoning: "QA does not have authority to override regulatory compliance requirements" (b51) and "QA should test against requirements, not redefine them" (b49).

**Unexpected discovery**: The revision gatekeeper independently found a genuine, non-planted contradiction between the System Architect (b20: `min.insync.replicas=2`, implying `acks=all`) and the Lead Developer (b41: `acks=1` for critical notifications). This is a real cross-node inconsistency in the LLM-generated outputs — the kind of bug that surfaces in actual distributed development. The gatekeeper resolved it correctly: "data integrity for healthcare-critical messages takes precedence."

#### Answer-Level Scoring

Answer-level scoring via deterministic regex produced the following results:

| Approach | Clean | Poison | Synth | Total | Distortion | Predicted |
|----------|:-----:|:------:|:-----:|:-----:|:----------:|:---------:|
| A: Telephone | 5/5 | 3/3 | 1/2 | 9/10 | 2/3 | 5/10 |
| B: Full Context | 5/5 | 2/3 | 1/2 | 8/10 | 3/3 | 6/10 |
| C: Summary Chain | 4/5 | 2/3 | 1/2 | 7/10 | 3/3 | 4/10 |
| D: Belief Flat | 4/5 | 3/3 | 2/2 | 9/10 | 3/3 | 7/10 |
| E: Belief Threaded | 4/5 | 2/3 | 1/2 | 7/10 | 3/3 | 8/10 |
| F: Belief Revised | 4/5 | 1/3 | 1/2 | 6/10 | 2/3 | 10/10 |

These scores are confounded by two systematic issues (discussed in Section 5.8).

#### Distortion Detection (Meta-Question)

When explicitly asked "identify any inconsistencies between sources," 4 of 6 approaches found all 3 planted distortions (3/3):

| Approach | Distortions Found | False Positives |
|----------|:-----------------:|:---------------:|
| A: Telephone | 2/3 | 0 |
| B: Full Context | 3/3 | 0 |
| C: Summary Chain | 3/3 | 0 |
| D: Belief Flat | 3/3 | 0 |
| E: Belief Threaded | 3/3 | 0 |
| F: Belief Revised | 2/3 | 0 |

#### Token Usage

| Approach | Input | Output | Total | Calls |
|----------|------:|-------:|------:|:-----:|
| A: Telephone | 2,064 | 1,148 | 3,212 | 1 |
| B: Full Context | 7,521 | 1,715 | 9,236 | 1 |
| C: Summary Chain | 8,511 | 2,220 | 10,731 | 2 |
| D: Belief Flat | 7,140 | 2,190 | 9,330 | 2 |
| E: Belief Threaded | 9,879 | 4,090 | 13,969 | 2 |
| F: Belief Revised | 21,996 | 9,100 | 31,096 | 4 |

Node generation (6 nodes): 12,159 tokens. Belief generation (6 calls): included in the 28 total LLM calls. Total experiment: ~121,000 tokens.

### 5.7 Critical Finding 3: Adversarial Distortion Detection (Phase 4)

Phase 4 demonstrates a third distinguishing capability: **the revision gatekeeper detects and eliminates adversarially planted misinformation from structural properties of the belief store alone, without access to ground truth**.

The revision gatekeeper in Approach F never sees the original specification. It receives only the 70 beliefs generated by 6 LLM-powered nodes, one of which has been adversarially poisoned. From this belief store, the gatekeeper:

1. **Identified 2 of 3 contradiction threads** — tracing retry policy and audit retention through 5-6 nodes each, finding that QA was the sole dissenting source.

2. **Killed both corresponding poisoned beliefs** (b49, b51) with zero false positives — reasoning from authority structure ("QA does not have authority to override regulatory compliance") and consensus ("5 sources cite 7 years tied to HIPAA").

3. **Independently discovered a genuine bug** — the Kafka `acks=1` vs `acks=all` contradiction between Developer and Architect, which was not planted but emerged naturally from the LLM personas. This demonstrates that BOCA's distortion detection generalizes beyond adversarial poisoning to real cross-node inconsistencies.

This capability has no analog in RAG or summarization. RAG retrieves by proximity — it has no mechanism to detect that one source among many has been compromised. Summarization compresses each source independently, propagating the poison into the summary. Only an architecture that maintains per-source provenance, threads beliefs across sources, and can reason about inter-belief contradictions can detect adversarial distortion from structure.

### 5.8 Scoring Methodology and Confounds (Phase 4)

The answer-level scores in Section 5.6 are dominated by two systematic confounds that are themselves informative:

**Confound 1: The Tech Writer didn't propagate poison**. Node 6 (Technical Writer) received all upstream outputs including QA's distortions, but produced correct values (3 retries, 7-year retention, 30s detection). The Tech Writer used majority consensus rather than blindly propagating the most recent source. This means Approach A (telephone endpoint = Tech Writer only) received correct values for all poisoned claims, scoring 9/10 — far above the predicted 5/10. This is an LLM resilience finding: a well-instructed synthesis persona naturally filters single-source distortions.

**Confound 2: Contradiction reporting is penalized by regex**. Approaches E and F detect contradictions and report both values in their answers: "the spec says 3 retries but QA says 5; we recommend 3." The regex scorer sees "5 retries" and marks it wrong, even though the answer correctly resolves the contradiction. This systematically underscores the approaches that are doing the most sophisticated reasoning. The approaches that simply report the correct value without acknowledging the contradiction score higher.

**What the structural analysis reveals**: When we look past answer-level regex to what each approach *structurally accomplishes*:
- **Thread detection** found 2/3 planted contradictions as explicit threads with source attribution
- **Revision** killed exactly the right beliefs with zero false positives
- **Revision found a genuine unprompted bug** (Kafka acks) that no other approach could detect
- **Distortion detection meta-question** achieved 3/3 for approaches B-E (when explicitly asked)
- **Belief emotional tagging** flagged QA's distorted retention belief as "modified" without instruction to do so

The answer-level scoring measures the quality of the mask's prose, which is one step removed from what BOCA architecturally provides. The structural mechanisms — threading, revision, distortion detection — are the primary contributions, and they performed precisely as designed.

### 5.9 Phase 5: Belief Convergence

#### Convergence Results

The convergence gatekeeper compressed 70 raw beliefs into 22 converged beliefs — a 3.2x compression ratio:

| Metric | Value |
|--------|------:|
| Input beliefs | 70 |
| Output beliefs | 22 |
| Compression ratio | 3.2x |
| Contradictions resolved | 2 |
| Redundancies removed | 46 |
| Unique insights preserved | 6 |
| Claim coverage | 11/12 |

The 22 converged beliefs span the full specification: latency SLA (unanimous, 4 sources), payload limits (unanimous, 5 sources), retry policy (strong, 4 sources, contradiction resolved), audit retention (strong, 5 sources, contradiction resolved), failover timing (strong, 3 sources), encryption (strong, 3 sources), authentication (strong, 2 sources), rate limiting (unanimous, 4 sources), WebSocket capacity (unanimous, 4 sources), plus 6 unique insights (circuit breaker config, autoscale thresholds, monitoring, MFA gap, RBAC gap, load testing requirements).

**Contradiction resolutions**: The convergence gatekeeper correctly resolved both planted contradictions:

1. **Retry attempts**: QA's 5 retries (b49) overruled by PM, Architect, Developer, Tech Writer all specifying 3. Resolution: "QA should test against requirements, not redefine them."
2. **Audit retention**: QA's 5-year retention (b51) overruled by 5-source consensus citing HIPAA §164.530(j). Resolution: "Regulatory compliance requirements cannot be overridden by QA test thresholds."

The third planted contradiction (failover detection 30s→60s) was not flagged — consistent with Phase 4's finding that the spec's legitimate "60-second promotion" confounds detection of the poisoned "60-second detection."

**Missing claim**: Data residency (claim 12) was not explicitly covered in any converged belief. The convergence gatekeeper merged residency-adjacent concepts but did not produce a belief containing the exact phrasing "must not leave the originating region." This demonstrates that aggressive compression risks information loss on claims expressed through prohibitive phrasing rather than specific numerical values.

#### 10-Question Comparison

| Approach | Clean | Poison | Synth | **Total** | Predicted |
|----------|:-----:|:------:|:-----:|:---------:|:---------:|
| G: Converged | 4/5 | **3/3** | **2/2** | **9/10** | 10 |
| H: Converged + Resolved | 4/5 | **3/3** | 1/2 | 8/10 | 10 |
| I: Raw Flat | 4/5 | 2/3 | 0/2 | 6/10 | 7 |

**Key results**:

- **G (converged) achieves 9/10** — the highest score of any BOCA approach across Phases 4-5 on the 10-question test. Perfect 3/3 on poisoned questions, perfect 2/2 on synthesis.
- **Confound eliminated**: Both G and H produced CLEAN answers on all poisoned questions (Q06-Q08) — no poison values mentioned. The Phase 4 confound (contradiction-reporting penalized by regex) is completely eliminated because contradictions are pre-resolved before the mask sees them.
- **Raw flat (I) struggles**: 2/3 poison (Q08 failover wrong), 0/2 synthesis. Without pre-resolved contradictions, the mask either propagates or reports both values.
- **G outperforms H**: Providing explicit resolution details slightly hurt H (8 vs 9). The mask performed better with clean, authoritative beliefs than with beliefs annotated with the contradictions they resolved. Additional resolution context may cause the mask to second-guess.

#### 30-Question Scale Test

| Approach | Clean | Poison | Synth | Cross | Hypo | Adv | **Total** | Predicted |
|----------|:-----:|:------:|:-----:|:-----:|:----:|:---:|:---------:|:---------:|
| Full Context | **10/10** | **3/3** | **2/2** | 3/5 | **5/5** | **5/5** | **28/30** | 22 |
| Converged | 9/10 | **3/3** | 1/2 | 2/5 | 4/5 | **5/5** | 24/30 | 27 |
| Raw Flat | 8/10 | **3/3** | 2/2 | 2/5 | 4/5 | **5/5** | 24/30 | 20 |

**Surprises**:

- **Full context crushed predictions** at 28/30 (predicted 22). Sonnet 4.5 is highly capable at synthesizing across 6 complete node outputs, resolving contradictions natively without architectural help. This is a model capability finding — stronger models may reduce the need for explicit convergence.
- **All three approaches score 5/5 on adversarial questions** (Q26-Q30). When questions explicitly name the poison value ("QA says 5 retries — is that correct?"), all approaches correctly identify the specification value. Adversarial resilience is a property of the LLM's reasoning, not the architecture.
- **Cross-cutting questions are the universal weakness** (2-3/5 across all approaches). Questions combining 3+ claims challenge every approach equally — neither more context nor convergence helps when the model must synthesize across many parameters.

#### Confound Elimination

The Phase 4 confound — regex penalizing answers that report both correct and poison values — was the primary motivation for convergence. Phase 5 confirms elimination:

| Approach | Q06 | Q07 | Q08 |
|----------|:---:|:---:|:---:|
| G: Converged | CLEAN | CLEAN | CLEAN |
| H: Converged + Resolved | CLEAN | CLEAN | CLEAN |

Zero poison values appear in any converged approach answer on any poisoned question. The mask receives only the correct, resolved values and produces clean answers.

#### Cost Efficiency

| Approach | Tokens (30Q) | Correct | Tokens/Correct |
|----------|:------------:|:-------:|:--------------:|
| Full Context | 12,012 | 28 | 429 |
| **Converged** | **8,170** | **24** | **340** |
| Raw Flat | 15,055 | 24 | 627 |

Converged is the most token-efficient at **340 tokens per correct answer** — 20% cheaper than full context (429) and 46% cheaper than raw flat (627). The convergence call itself costs 9,914 tokens (5,998 in + 3,916 out), amortized across all downstream queries.

#### Token Usage

| Call | Input | Output | Total | Time |
|------|------:|-------:|------:|-----:|
| Convergence | 5,998 | 3,916 | 9,914 | 57s |
| G: Converged 10Q | 3,229 | 1,336 | 4,565 | 19s |
| H: Resolved 10Q | 3,366 | 1,223 | 4,589 | 16s |
| I: Raw Select 10Q | 4,637 | 747 | 5,384 | 15s |
| I: Raw Mask 10Q | 2,471 | 1,251 | 3,722 | 19s |
| Full Context 30Q | 8,065 | 3,947 | 12,012 | 64s |
| Converged 30Q | 3,926 | 4,244 | 8,170 | 61s |
| Raw Select 30Q | 5,197 | 929 | 6,126 | 18s |
| Raw Mask 30Q | 4,232 | 4,697 | 8,929 | 71s |
| **Total (Phase 5)** | **41,121** | **22,290** | **63,411** | **340s** |

### 5.10 Critical Finding 4: Convergence as Production Architecture (Phase 5)

Phase 5 demonstrates a fourth distinguishing capability: **belief convergence transforms a redundant, contradictory belief store into a minimal knowledge base that is cheaper, cleaner, and more resilient than raw beliefs or full context**.

The convergence gatekeeper operates as a one-time preprocessing step that:

1. **Eliminates redundancy**: 70 beliefs → 22 (3.2x compression), removing 46 redundant beliefs while preserving 11/12 ground-truth claims
2. **Pre-resolves contradictions**: Both planted distortions killed at convergence time, before any question is asked. The mask never sees conflicting values, eliminating the Phase 4 confound entirely
3. **Reduces per-query cost**: Converged beliefs use 3,926 input tokens (30Q) vs 8,065 for full context — 51% reduction in mask input tokens
4. **Preserves unique insights**: 6 unique beliefs (MFA gap, circuit breaker, monitoring thresholds) survive compression, available for any downstream query

The production implications are clear. In a Nexor workflow board with 20 steps producing 10 beliefs each (200 raw beliefs), convergence would compress to ~60 converged beliefs — a manageable context for any downstream chat query. Without convergence, the mask would need to process all 200 raw beliefs or an expensive selection step for every question.

The cost-efficiency finding is particularly significant: at 340 tokens per correct answer, converged beliefs are the cheapest approach tested across all 5 phases. Full context achieves higher accuracy (28/30 vs 24/30) but at 26% more cost per correct answer. As question count grows and source material exceeds context limits, convergence becomes not just cheaper but the only viable option.

However, the 11/12 claim coverage reveals the fundamental tradeoff: compression is lossy. Data residency (claim 12) was expressed through prohibitive language ("must not leave") rather than a specific numerical value, and the convergence gatekeeper's topic-clustering approach failed to preserve it. This suggests convergence works best for parameter-heavy specifications and may need supplementary mechanisms (explicit claim enumeration, coverage audit with re-convergence) for policy-style requirements.

### 5.11 Phase 6: Prompt-Engineered Beliefs

#### Belief Generation (v2)

The v2 gatekeeper generated 98 beliefs across 6 nodes (up from 70 in Phase 4), reflecting the richer schema's encouragement to extract more granular claims:

| Node | Persona | v1 Beliefs | v2 Beliefs |
|------|---------|:----------:|:----------:|
| 1 | Product Manager | 8 | 15 |
| 2 | System Architect | 17 | 18 |
| 3 | Security Reviewer | 8 | 14 |
| 4 | Lead Developer | 11 | 13 |
| 5 | QA Engineer (poisoned) | 12 | 19 |
| 6 | Technical Writer | 14 | 19 |
| | **Total** | **70** | **98** |

#### Belief Quality Metrics

| Metric | Value |
|--------|------:|
| Average reasoning field length | 158 chars |
| Cross-source tension populated | 26% |
| Confidence justification populated | 100% |

Every belief includes a reasoning chain explaining why the gatekeeper extracted it, and every belief has an explicit confidence justification. The 26% cross_source_tension rate reflects that roughly a quarter of beliefs identify potential conflicts with other roles' perspectives — a new metadata dimension absent from v1 beliefs.

#### Convergence Results (v2)

| Metric | Phase 5 | Phase 6 |
|--------|--------:|--------:|
| Input beliefs | 70 | 98 |
| Output beliefs | 22 | 40 |
| Compression ratio | 3.2x | 2.5x |
| Contradictions resolved | 2/3 | **3/3** |
| Claim coverage | 11/12 | 11/12 |
| Missing claim | Data residency | Failover detection |

The v2 convergence gatekeeper resolved all 3 planted contradictions (retry attempts, audit retention, and failover detection) — up from 2/3 in Phase 5. The failover detection contradiction, which was confounded in Phase 5 by the legitimate 60-second promotion value, was correctly resolved by Phase 6's reasoning-first convergence that explicitly traced authority chains.

However, convergence now loses a different claim: failover detection timing (claim 11) is resolved but the converged text does not match the specific regex pattern "30-second detection." Phase 5 lost data residency (claim 12); Phase 6 recovered data residency but lost claim 11 to a different expression — the convergence describes the failover timing correctly in prose but not in the exact phrasing the regex expects. This demonstrates that claim coverage is sensitive to expression format, not just content accuracy.

#### 10-Question Comparison

| Approach | Clean | Poison | Synth | **Total** | Predicted |
|----------|:-----:|:------:|:-----:|:---------:|:---------:|
| G2: Converged v2 | 5/5 | **2/3** | 1/2 | **8/10** | 10 |
| I2: Raw Flat v2 | 0/5 | 0/3 | 0/2 | 0/10* | 8 |

*I2 scored 0/10 due to a token limit anomaly: the raw flat mask hit its 4,096 output token limit, truncating all answers. This is an infrastructure issue, not a signal about v2 belief quality — the 30-question test (with 16,384 token limit) shows I2 performing correctly at 25/30.

G2 achieved 5/5 clean and 2/3 poison. Q08 (failover detection) remains incorrect due to the claim 11 expression issue in the converged beliefs.

#### 30-Question Scale Test

| Approach | Clean | Poison | Synth | Cross | Hypo | Adv | **Total** | Predicted |
|----------|:-----:|:------:|:-----:|:-----:|:----:|:---:|:---------:|:---------:|
| Full Context | **10/10** | 2/3 | **2/2** | 3/5 | **5/5** | **5/5** | **27/30** | 28 |
| Converged v2 | **10/10** | 2/3 | 1/2 | 3/5 | **5/5** | **5/5** | **26/30** | 27 |
| Raw Flat v2 | **10/10** | 2/3 | 1/2 | 3/5 | 4/5 | **5/5** | **25/30** | 25 |

**Key results**:

- **Converged v2 closes the gap**: 26/30 vs 27/30 for full context — a 1-point gap (down from 4 points in Phase 5). This is the primary Phase 6 finding: prompt-engineered beliefs narrow the accuracy gap between converged beliefs and full context to near-parity.
- **All three achieve perfect clean scores**: 10/10 clean across all approaches, up from 9/10 (converged) and 8/10 (raw flat) in Phase 5. The v2 beliefs carry sufficient signal for every undistorted claim.
- **Adversarial resilience perfect**: 5/5 on Q26-Q30 across all approaches, matching Phase 5.
- **Hypothetical improvement**: Converged v2 achieves 5/5 hypothetical (up from 4/5 in Phase 5), matching full context.
- **Cross-cutting remains the universal weakness**: 3/5 across all approaches, consistent with Phase 5's finding that multi-claim synthesis is difficult regardless of context format.
- **Data residency recovered**: Phase 5's convergence lost data residency (claim 12). Phase 6's XML-structured convergence prompt explicitly instructed "Preserve PROHIBITIVE LANGUAGE exactly" — data residency is now covered in all three approaches.

#### Phase 5 vs Phase 6 Comparison (30Q)

| Approach | Phase 5 | Phase 6 | Delta |
|----------|:-------:|:-------:|:-----:|
| Full Context | 28 | 27 | -1 |
| Converged | 24 | **26** | **+2** |
| Raw Flat | 24 | **25** | **+1** |

#### Cost Efficiency (30Q)

| Approach | Tokens | Correct | Tokens/Correct |
|----------|:------:|:-------:|:--------------:|
| Full Context | 21,585 | 27 | 799 |
| **Converged v2** | **15,617** | **26** | **601** |
| Raw Flat v2 | 31,502 | 25 | 1,260 |

Converged v2 achieves 601 tokens per correct answer — 25% cheaper than full context (799) and the most efficient belief-based approach. Phase 5's converged achieved 340 tokens/correct at 24/30; Phase 6 trades slightly higher cost for significantly better accuracy.

#### Token Usage

| Call | Input | Output | Total | Time |
|------|------:|-------:|------:|-----:|
| Belief v2 (6 nodes) | 17,134 | 15,659 | 32,793 | 208s |
| Convergence v2 | 13,181 | 11,672 | 24,853 | 190s |
| G2: Converged 10Q | 4,732 | 3,314 | 8,046 | 60s |
| I2: Raw Select 10Q | 6,644 | 1,174 | 7,818 | 28s |
| I2: Raw Mask 10Q | 3,884 | 4,096 | 7,980 | 64s |
| Full Context 30Q | 8,233 | 13,352 | 21,585 | 229s |
| Converged v2 30Q | 5,292 | 10,325 | 15,617 | 150s |
| Raw Select 30Q | 7,204 | 1,966 | 9,170 | 33s |
| Raw Mask 30Q | 7,102 | 15,230 | 22,332 | 237s |
| **Total** | **73,406** | **76,788** | **150,194** | |

### 5.12 Critical Finding 5: Prompt Engineering Closes the Convergence Gap (Phase 6)

Phase 6 demonstrates a fifth distinguishing finding: **research-backed prompt engineering applied to belief generation and convergence closes the accuracy gap between converged beliefs and full context from 4 points to 1 point**.

The three simultaneous improvements — reasoning-first schemas, XML-structured prompts with few-shot examples, and richer belief metadata — produced measurable gains:

1. **Convergence accuracy**: 26/30 (up from 24/30 in Phase 5), now within 1 point of full context (27/30)
2. **Contradiction resolution**: 3/3 (up from 2/3 in Phase 5) — the previously confounded failover claim is now resolved
3. **Data residency recovery**: The claim lost in Phase 5's convergence is recovered through explicit prohibitive language preservation instructions
4. **Clean score perfection**: 10/10 on undistorted claims (up from 9/10 in Phase 5)
5. **Belief richness**: Every belief carries reasoning chains (avg 158 chars) and confidence justifications (100%), providing transparent provenance for downstream reasoning

The remaining 1-point gap between converged v2 (26/30) and full context (27/30) stems from claim 11 (failover detection) — the same claim that has been problematic since Phase 4. The convergence correctly resolves the contradiction but expresses the result in prose that doesn't match the regex scorer. This is a scoring methodology limitation, not an architectural one.

The production implication is significant: with prompt-engineered beliefs, convergence achieves near-parity with full context at 25% lower cost per correct answer (601 vs 799 tokens/correct). For production boards where full context exceeds limits, prompt-engineered convergence is now a viable substitute rather than a meaningful accuracy tradeoff.

### 5.13 Phase 7: Multi-Workflow Meta-Convergence

Phase 7 tests beliefs as inter-workflow communication primitives — the first multi-workflow experiment. Two independent workflows (WF1: 6 nodes processing a technical specification, WF2: 4 nodes processing an operations runbook) each produce their own beliefs and convergences, then a meta-convergence merges them.

#### Taxonomy and Belief Generation

The taxonomy gatekeeper generated 52 controlled tags across 7 domains (performance, reliability, security, compliance, operations, integration, clinical). WF2's 4 nodes produced 67 v3 beliefs with the following type distribution:

| Belief Type | Count | Percentage |
|-------------|:-----:|:----------:|
| fact | 35 | 52% |
| opinion | 25 | 37% |
| policy | 4 | 6% |
| observation | 3 | 4% |

The belief type classification successfully separated specification-derived facts from professional opinions. Notably, all three WF2 poison values (750ms latency, 3yr retention, 45min incident response) were classified as `opinion` by the gatekeeper — a structural signal that downstream convergence uses to deprioritize them against `fact`-type beliefs.

#### WF2 Convergence

WF2 convergence compressed 67 raw beliefs into 26 converged beliefs (2.6x), resolving 4 internal contradictions:

| Contradiction | Correct Value | Poison Value | Resolution |
|---------------|:------------:|:------------:|:----------:|
| Alert latency | 500ms | 750ms | Resolved (500ms) |
| Uptime target interpretation | 99.95% | 99.99% | Resolved (99.95% spec) |
| Health check interval | 5s | 3s | Resolved (5s spec) |
| Standby promotion | 60s | 90s | Resolved (60s spec) |

The incident response poison (15→45 min) was NOT detected as a contradiction by WF2 convergence — the Integration Engineer's 45-minute recommendation was treated as a valid professional opinion. This demonstrates a limitation: when a poison value is plausible and only one node deviates, per-workflow convergence cannot distinguish poison from legitimate professional disagreement.

#### Meta-Convergence

The meta-convergence gatekeeper merged 40 WF1 converged beliefs + 26 WF2 converged beliefs into 44 meta-beliefs:

| Consensus Strength | Count |
|-------------------|:-----:|
| cross_validated | 10 |
| single_workflow | 32 |
| cross_workflow_split | 2 |

The 10 cross-validated beliefs confirmed alignment between workflows on: critical alert latency (500ms), audit log retention (7 years), audit log immutability, failover timing (30s detection + 60s promotion), health check configuration, encryption at rest (AES-256), certificate management, JWT authentication, service-to-service auth (mTLS), and HIPAA training. The 2 cross-workflow splits were resolved (notification priority tier structure, operational monitoring thresholds).

#### Flat Convergence Baseline

Flat convergence processed all 165 raw beliefs (98 WF1 + 67 WF2) in a single pass, producing 40 converged beliefs with 5 contradictions resolved.

#### Quality Audit — Claim Coverage

| Store | WF1 Claims (12) | WF2 Claims (16) |
|-------|:----------------:|:----------------:|
| WF2 convergence | — | 9/16 |
| Meta-converged | 11/12 | 11/16 |
| Flat-converged | 12/12 | 13/16 |

Flat convergence achieved broader claim coverage because it saw all raw beliefs in a single pass, while hierarchical meta-convergence was limited by per-workflow compression that dropped some WF2-unique claims (backup frequency, incident response time, RTO, log shipping).

#### Scoring — 20 Questions

| Approach | WF1 (5) | WF2 (5) | Cross (5) | Adv (5) | Total (20) | Predicted |
|----------|:-------:|:-------:|:---------:|:-------:|:----------:|:---------:|
| Meta-converged | 3 | 2 | 4 | **4** | **13** | 17 |
| Flat-converged | 4 | 3 | 4 | **4** | **15** | 14 |
| Full context | 4 | 2 | 4 | **4** | **14** | 18 |

**Hierarchical premium: -2** (predicted +3). Flat convergence beat meta-convergence by 2 points, contrary to prediction. The flat approach's advantage came from broader WF1 (+1) and WF2 (+1) coverage — it retained more claims through single-pass processing.

**Adversarial resilience: 4/5 across all approaches.** This is the standout finding. Every approach correctly resolved:

- **P7Q16** (750ms vs 500ms latency): All three detected the contradiction and resolved to 500ms
- **P7Q17** (triple retention: 3yr/5yr/7yr): All three correctly identified 7 years as HIPAA-mandated
- **P7Q19** (cross-workflow retention contradictions): All three identified the correct value across workflows
- **P7Q20** (cross-workflow latency agreement): All three confirmed 500ms with cross-validation

The one miss across all approaches: **P7Q18** (incident response 15 vs 45 min) — all three reported the poison value. The WF2-only poison was the hardest to detect because only 3-of-4 WF2 nodes had the correct value and the Integration Engineer's 45-minute recommendation was plausible.

#### Confidence Calibration

The v3 answer schema's confidence calibration showed appropriate behavior:

| Coverage | Count | Avg Confidence |
|----------|:-----:|:--------------:|
| full | 15 | 3.9 |
| partial | 3 | 3.7 |
| none | 2 | 1.5 |

The mask correctly identified coverage gaps (backup frequency, log shipping) and assigned low confidence (1-2) when beliefs provided no relevant information.

#### Predictions vs Actuals

| Approach | Predicted | Actual | Delta |
|----------|:---------:|:------:|:-----:|
| Meta-converged | 17 | 13 | -4 |
| Flat-converged | 14 | 15 | +1 |
| Full context | 18 | 14 | -4 |

All predictions were optimistic. The primary cause: WF2 claim coverage was lower than expected. Several WF2-unique claims (backup frequency, incident response, RTO) were not covered by the converged belief stores, causing misses on WF2-only questions.

#### Cost

15 LLM calls, 168,789 total tokens (87,121 input + 81,668 output).

### 5.14 Critical Finding 6: Beliefs as Inter-Workflow Communication Primitives (Phase 7)

Phase 7 demonstrates a sixth distinguishing finding: **beliefs serve as effective inter-workflow communication primitives, enabling cross-workflow validation that detects adversarial distortions planted in independent workflows**.

The key results:

1. **Cross-validation works**: The meta-convergence identified 10 topics where both workflows agree (cross_validated), providing the strongest consensus possible — two independent information pipelines processing different source documents arrive at the same values. This is a form of verification unavailable to any single-workflow approach.

2. **Triple-poison resolution**: The most novel adversarial scenario — audit retention poisoned differently in two workflows (WF1-QA: 5yr, WF2-Integration: 3yr, correct: 7yr) — was resolved correctly by all three approaches. The HIPAA regulatory citation provided sufficient authority signal to override both poison values simultaneously.

3. **Adversarial resilience is architecture-independent**: All three approaches (meta-converged, flat-converged, full context) achieved 4/5 on adversarial questions. This suggests that cross-workflow adversarial detection is a property of having multiple independent information sources, not of any particular convergence strategy.

4. **Flat beats hierarchical (for now)**: Contrary to prediction, flat convergence (15/20) outperformed hierarchical meta-convergence (13/20). The flat approach saw all 165 raw beliefs simultaneously, preserving more claims through single-pass processing. Hierarchical convergence lost information during per-workflow compression before the meta-step could cross-validate. This identifies a key design challenge: per-workflow convergence must be conservative enough to preserve all claims, or must pass unique-topic beliefs through unconverged.

5. **WF2-only poison is hardest to detect**: The incident response poison (15→45 min) fooled all three approaches. Without a second workflow to cross-validate, and with only 3-of-4 nodes providing the correct value, the plausible-sounding poison survived convergence. This confirms the prediction that cross-validation premium is real — overlapping claims are easier to verify than single-workflow claims.

6. **Belief type classification provides structural poison signals**: The v3 schema's belief_type field classified all three WF2 poison values as `opinion` rather than `fact`. This structural metadata, if explicitly used during convergence (e.g., "opinions do not override facts"), could provide an additional defense against plausible-sounding professional recommendations that deviate from specifications.

The production implication: in multi-workflow systems where different teams process different aspects of the same domain, beliefs provide a natural communication layer. Cross-validated beliefs (confirmed by multiple independent workflows) can be given higher authority than single-workflow beliefs, and cross-workflow splits trigger explicit human review. This is the sorted, labeled system that helps workflows communicate.

## 6. Discussion

### 6.1 When BOCA Outperforms Alternatives

BOCA's value proposition is strongest when:

1. **Multiple questions are asked against the same source material**: The gatekeeper decomposition amortizes across questions. At 4+ questions against the same source, the belief pipeline is cheaper in total tokens than full context.

2. **Accuracy matters more than speed**: The revision loop adds latency (additional gatekeeper evaluation + mask re-invocation) but catches errors that single-pass approaches miss.

3. **Source material exceeds context limits**: When full context is impossible, BOCA offers a principled alternative to chunking or summarization, with built-in quality assurance via revision.

4. **Emotional and structural metadata matters**: The gatekeeper's emotional tagging (`fragile`, `rushed`, `defensive`) carries information that pure summarization discards. Whether downstream agents use this effectively is an open question, but the metadata is available.

5. **Deep workflow chains require cross-node reasoning**: Phase 3 demonstrates that beliefs carry signal across 9 depth levels. When a question requires connecting a decision at depth 1 to a requirement at depth 9, belief threading provides a structured mechanism that neither RAG nor summarization can replicate.

6. **Adversarial or multi-stakeholder information pipelines**: Phase 4 demonstrates that when multiple sources contribute to a shared knowledge base and one source is compromised (intentionally or accidentally), BOCA's revision mechanism detects and eliminates distortions from structural properties alone. In any pipeline where conflicting information from different teams, vendors, or systems must be reconciled, belief provenance and cross-source contradiction detection provide safety guarantees absent from context-concatenation approaches.

7. **Scaling beyond manageable context**: Phase 5 demonstrates that convergence compresses 70 beliefs to 22 (3.2x) with 340 tokens per correct answer — the cheapest approach across all phases. For production boards with 200+ beliefs from 20+ workflow steps, convergence is not merely cheaper but architecturally necessary: raw beliefs exceed practical context limits, and full node outputs are infeasible.

8. **Prompt engineering amplifies belief quality**: Phase 6 demonstrates that applying research-backed prompt engineering (reasoning-first schemas, XML tags, few-shot examples) to belief generation and convergence closes the accuracy gap with full context from 4 points to 1 point (26/30 vs 27/30). The improvement is multiplicative with convergence: better beliefs produce better convergence, which produces better answers. This suggests that belief quality — not just architectural mechanisms — is a critical lever for production accuracy.

9. **Multi-workflow systems benefit from belief-mediated communication**: Phase 7 demonstrates that when two independent workflows process different documents about the same domain, beliefs provide a natural inter-workflow communication layer. Cross-validated beliefs (10 topics confirmed by both workflows) provide the strongest possible consensus. The triple-poison scenario (3yr/5yr/7yr retention) was resolved correctly across all approaches, demonstrating that regulatory citations provide sufficient authority signal to override multiple simultaneous poison values from different workflows.

### 6.2 When BOCA is Overkill

For single questions on short documents, full context is simpler, faster, and cheaper. BOCA's overhead (gatekeeper decomposition + assignment) only pays off when amortized across multiple queries or when the source material is too large for direct processing.

### 6.3 The Gatekeeper as Smartest Agent

A non-obvious property of BOCA is that the gatekeeper must be the most capable agent in the system. It reads full source material, forms hypotheses, evaluates mask outputs, and identifies gaps. In traditional multi-agent systems, the orchestrator is a router — it doesn't need deep understanding. In BOCA, the gatekeeper *is* the understanding. Masks are cheap projections of its worldview.

This has implications for model selection: the gatekeeper should be the most capable (and expensive) model, while masks can be lighter models that reason well from curated context.

### 6.4 Static Beliefs and Production Architecture

Phase 3 reveals an important architectural insight for production systems: **beliefs can be pre-computed at step completion time**. In a workflow pipeline, when node N finishes execution, its beliefs are generated once and stored. When a downstream meeting or review requires context from node N, the beliefs are already available — zero marginal cost at query time.

This transforms the cost equation. The gatekeeper decomposition (the most expensive call) happens asynchronously during workflow execution, not at meeting time. The only synchronous costs are gatekeeper selection (~4,500 input tokens per question) and mask answer (~3,400–4,600 input tokens). For a meeting that asks 10 questions against a 39-belief store, the cost is approximately 80,000 tokens — compared to loading full context from 9 upstream nodes for each question.

### 6.5 Limitations

**Total token cost**: For the Phase 2 cross-file experiment, the full revision pipeline (D) used 35,291 total tokens versus 10,360 for full context (A). The revision pipeline is ~3.4x more expensive in total. For Phase 3, the pipeline cost 33,918 tokens versus 6,383 for baseline — 5.3x more expensive. Phase 4's full revision pipeline (F) used 31,096 tokens versus 3,212 for the telephone approach (A). This cost is justified only when accuracy is paramount, when amortized across many queries, or when beliefs are pre-computed.

**Gatekeeper reads full source**: The gatekeeper still requires the full context window for decomposition and evaluation. BOCA does not solve context length limits for the gatekeeper — it solves them for downstream agents.

**Evaluation methodology**: Phase 4 introduced deterministic regex scoring against ground truth, but this methodology has limitations. Answers that correctly detect and report contradictions (mentioning both correct and poison values) are penalized by pattern matching, systematically underscoring the most sophisticated approaches. A complete evaluation requires either human scoring or an LLM judge capable of assessing contradiction resolution quality — both of which we deliberately avoided to maintain deterministic reproducibility.

**Single revision pass**: We tested one round of revision. Whether additional rounds improve quality, plateau, or degrade (through over-correction) is unexplored.

**Dormant resurfacing imperfect**: Phase 3 achieved 78% average dormant resurfacing rate. The 33% rate on Q3 suggests the gatekeeper sometimes treats resurfaced beliefs as "normally relevant" rather than flagging them as dormant — a classification issue rather than a retrieval failure.

**Adversarial limitations**: Phase 4's poison was single-source (1 of 6 nodes). The revision gatekeeper's consensus-based detection ("5 sources say X, 1 says Y") would be less effective against multi-source coordinated poisoning. Additionally, the Tech Writer node naturally filtered the poison through majority consensus, reducing poison propagation — a resilience property of well-instructed LLMs that may not hold in all configurations.

**Failover claim confound**: Phase 4's claim 11 (failover detection timing) was confounded because the original specification contains both "30-second detection" and "60-second promotion." This made regex disambiguation between the correct detection value and the poison detection value unreliable when answers naturally discuss both timing parameters.

**Convergence information loss**: Phase 5's convergence achieved 11/12 claim coverage — data residency (claim 12) was lost during compression. Claims expressed through prohibitive phrasing ("must not leave") rather than specific numerical values are harder for topic-clustering convergence to preserve. Production systems should implement a post-convergence coverage audit with re-convergence for missing claims.

**Model capability ceiling**: Phase 5's full context approach scored 28/30 — far exceeding predictions and outperforming convergence (24/30). Phase 6 narrowed the gap (27 vs 26) but full context still leads. This suggests that sufficiently capable models may not need explicit convergence when context fits. The value of convergence increases as (a) source material grows beyond context limits, (b) question volume grows (amortizing the convergence call), and (c) weaker models are used as masks.

**10Q token limit anomaly**: Phase 6's raw flat 10Q approach (I2) scored 0/10 because the mask's output hit a 4,096 token limit, truncating all answers. The 30Q test (with 16,384 limit) shows the approach works correctly at 25/30. This is an infrastructure issue — output token budgets must be sized to the answer schema — but it means the 10Q comparison is incomplete.

**Claim 11 persistent difficulty**: Failover detection timing (30s vs 60s) has been problematic across Phases 4-6. The original specification's legitimate "60-second promotion" confounds regex detection of the poisoned "60-second detection." Phase 6's convergence resolves the contradiction correctly but expresses the result in prose that the regex scorer cannot match. This claim alone accounts for the remaining 1-point gap between converged v2 and full context.

**Simultaneous changes**: Phase 6 applies three improvements simultaneously (reasoning-first schemas, XML tags, few-shot examples). We cannot isolate which improvement contributes most to the gains. A factorial design testing each change independently would require 7 additional experimental conditions.

**Hierarchical information loss**: Phase 7's meta-convergence scored lower than flat convergence (13 vs 15) because per-workflow convergence compressed away WF2-unique claims before the meta-step could cross-validate. Hierarchical convergence needs a more conservative per-workflow step — perhaps passing through unique-topic beliefs unconverged — to avoid information loss at the workflow boundary.

**WF2 claim coverage gaps**: Several WF2-unique claims (backup frequency, incident response, RTO, log shipping) were not detected by the regex-based claim audit in the WF2 converged store. This may reflect overly strict regex patterns or convergence merging these claims into broader beliefs whose text doesn't match the patterns. The audit methodology needs refinement for claims expressed in operational rather than specification language.

**Single-source poison resilience**: The incident response poison (15→45 min) fooled all three Phase 7 approaches. When only one node deviates and the deviation is plausible, per-workflow convergence cannot reliably detect the distortion. Cross-workflow validation (having a second workflow confirm the value) remains the only reliable defense, suggesting that overlapping claim coverage across workflows should be maximized in production.

**Phase 7 prediction accuracy**: All three predictions were optimistic (meta -4, flat +1, full context -4). The predictions overestimated WF2-unique claim coverage and underestimated information loss during convergence. Future predictions should account for the difficulty of preserving operational claims through convergence.

### 6.6 Relationship to Predictive Processing

The confirm/revise/kill cycle in BOCA mirrors the prediction error minimization loop in predictive processing theories of cognition (Clark, 2013). The gatekeeper forms predictions (beliefs) about the source, the mask tests those predictions by reasoning from them, and the evaluation step computes "prediction errors" (gaps between mask output and source truth). This is not a metaphor — it is a direct computational implementation of the same pattern.

This suggests a deeper principle: **understanding is not compression; it is prediction under uncertainty, tested and revised**. Summarization compresses. RAG retrieves. BOCA predicts, tests, and updates. The biological precedent suggests this pattern may be fundamentally more robust for complex reasoning tasks.

## 7. Future Work

### 7.1 Multi-Round Revision

Testing iterative revision (2, 3, N rounds) to characterize the convergence behavior: does accuracy improve monotonically, or is there an optimal number of revision cycles?

### 7.2 Heterogeneous Model Pairing

Using a high-capability model (e.g., Claude Opus) as gatekeeper and a lower-cost model (e.g., Claude Haiku) as mask. If beliefs carry sufficient signal, lighter masks may produce comparable quality at dramatically lower cost.

### 7.3 Belief Persistence and Transfer

Phase 3 validates the static belief hypothesis: pre-computed beliefs carry sufficient signal for downstream reasoning. Future work should test persistent belief stores in production systems where beliefs generated during workflow execution are stored and queried across sessions, teams, and time.

### 7.4 Thread-Aware Revision

Phases 2 and 3 test revision and threading independently. Combining them — having the gatekeeper revise beliefs *within thread context* — could improve both thread coherence and hallucination detection. A belief that contradicts its thread neighbors is more suspicious than one evaluated in isolation.

### 7.5 Multi-Source Adversarial Resistance

Phase 4 tested single-source poisoning (1 of 6 nodes). Future work should explore coordinated multi-source attacks where 2-3 nodes collude, testing whether the revision gatekeeper's consensus-based detection degrades gracefully. Additionally, testing poison propagation through different chain topologies (where the poisoned node feeds others) would characterize BOCA's adversarial resilience boundary.

### 7.6 Convergence with Coverage Guarantees

Phase 5's convergence lost 1 of 12 claims (data residency) during compression. Future work should explore convergence with explicit coverage audits: after convergence, check all known claims against the converged store and trigger re-convergence or targeted belief generation for missing claims. Additionally, testing convergence on larger belief stores (200+ beliefs from 20+ nodes) would characterize the compression ratio and information loss curve at production scale.

### 7.7 Benchmark Construction

Building a standardized evaluation benchmark for authored-context architectures: source material of varying complexity, questions requiring cross-document reasoning, and human-scored accuracy metrics. Phase 4's adversarial telephone design provides a template: ground-truth specification, transformation pipeline, planted distortions, and deterministic scoring. Phase 5's 30-question battery across 6 categories (clean, poisoned, synthesis, cross-cutting, hypothetical, adversarial) provides a question design template. The scoring methodology should be extended to handle contradiction-reporting answers (where an approach correctly identifies both values and resolves to the correct one).

### 7.8 Prompt Engineering Factorial Design

Phase 6 applied three improvements simultaneously. Future work should test each change independently (reasoning-first schemas alone, XML tags alone, few-shot examples alone) and in combinations to determine which contributes most to accuracy gains. Additionally, testing different example counts (0, 1, 2, 3 examples per prompt) and reasoning field positions (before vs after content) would characterize the prompt engineering design space for belief architectures.

### 7.9 Scaling to Full Codebases

Testing BOCA on codebase-scale inputs (thousands of files) where the gatekeeper itself cannot hold full context, requiring hierarchical belief decomposition — gatekeepers of gatekeepers.

### 7.10 Human-Belief Interaction

In production workflows, human messages during meetings could blend into the belief store — confirming, revising, or killing beliefs in real time. This creates a hybrid system where human judgment and LLM reasoning operate on the same primitive (beliefs) rather than separate modalities (text vs. embeddings).

### 7.11 Conservative Hierarchical Convergence

Phase 7 revealed that per-workflow convergence drops unique-topic claims before meta-convergence can cross-validate them. Future work should explore conservative convergence strategies: passing through beliefs that are unique to one workflow without compression, only converging topics that have multiple beliefs within the workflow. This preserves information breadth while still resolving intra-workflow contradictions. Additionally, testing with 3+ workflows would characterize whether cross-validation accuracy scales with the number of independent sources.

### 7.12 Belief Type-Aware Convergence

Phase 7's v3 schema classifies beliefs as fact/policy/opinion/observation, but the convergence gatekeeper does not yet use this classification algorithmically. Future work should explore type-aware convergence rules: facts converge by consensus (majority wins), policies converge by authority (regulatory > organizational > professional), opinions are preserved as tensions rather than resolved, and observations are contextual metadata. Additionally, testing whether `opinion`-typed beliefs that contradict `fact`-typed beliefs are automatically deprioritized would provide a structural defense against plausible-sounding professional recommendations that deviate from specifications.

## 8. Conclusion

We have presented Belief-Oriented Conversation Architecture, a framework that replaces raw context passing with authored belief slices. Our experiments across seven phases demonstrate eight findings:

1. **Authored context transfers**: Masks reasoning from beliefs alone produced coherent analysis at 16-20% of full-context token cost (Phase 1).

2. **Beliefs beat summaries qualitatively**: Both naive summaries and single-pass beliefs led to hallucinated details. The belief revision loop caught and corrected these errors; summarization cannot (Phase 2).

3. **The revision mechanism is the differentiator**: Without revision, BOCA is smart summarization with metadata. With revision, it is a self-correcting reasoning architecture that tests its own understanding against ground truth (Phase 2).

4. **Beliefs carry signal across deep chains**: Across a 9-node, 39-belief workflow, the gatekeeper detected 17 belief threads averaging 4.0 beliefs each, with 14 spanning 3+ nodes. Dormant beliefs from depth 9 resurfaced at depth 1 with 78% reliability. Noise pruning was perfect at 100% (Phase 3).

5. **Beliefs detect adversarial distortion from structure alone**: In an end-to-end pipeline with 70 LLM-generated beliefs from 6 transformation nodes (one adversarially poisoned), the revision gatekeeper — operating without access to the original specification — identified 2 of 3 contradiction threads, killed both planted poison beliefs with zero false positives, and independently discovered a genuine non-planted cross-node inconsistency. This demonstrates that belief provenance and cross-source threading enable distortion detection impossible with any approach that loses source attribution (Phase 4).

6. **Convergence compresses beliefs for production scale**: A convergence gatekeeper compressed 70 raw beliefs into 22 converged beliefs (3.2x), pre-resolving 2 planted contradictions and preserving 11/12 ground-truth claims. Converged beliefs achieved 9/10 on the 10-question comparison (vs 6/10 raw baseline), perfect 3/3 poison resistance with zero confound, 5/5 adversarial resilience, and 340 tokens per correct answer — the cheapest approach across all phases. This demonstrates the production architecture: beliefs are generated at step completion time, converged once across the board, and queried cheaply per question (Phase 5).

7. **Prompt engineering closes the convergence gap**: Applying research-backed prompt engineering (reasoning-first schemas, XML-structured prompts with few-shot examples, richer belief metadata) to belief generation and convergence improved converged accuracy from 24/30 to 26/30 — closing the gap with full context from 4 points to 1 point. The v2 convergence resolved all 3 planted contradictions (vs 2/3 in Phase 5), recovered previously lost data residency, achieved perfect 10/10 on clean claims, and maintained 5/5 adversarial resilience at 601 tokens per correct answer — 25% cheaper than full context. This demonstrates that belief quality is a critical lever: better prompts produce richer beliefs, which produce better convergence, which produces more accurate answers (Phase 6).

8. **Beliefs serve as inter-workflow communication primitives**: In a two-workflow experiment with 10 total nodes processing different source documents about the same system, meta-convergence merged 66 per-workflow converged beliefs into 44 meta-beliefs with 10 cross-validated topics. All three approaches achieved 4/5 on adversarial questions, correctly resolving a novel triple-poison scenario (3yr/5yr/7yr audit retention from two different poisoned nodes in different workflows). Cross-validated beliefs — confirmed by multiple independent workflows — represent the strongest possible consensus. The flat convergence baseline unexpectedly outperformed hierarchical meta-convergence (15 vs 13), revealing that per-workflow compression must be conservative to avoid information loss at workflow boundaries (Phase 7).

The core insight is architectural: **the unit of context should not be a chunk of text or a summary, but a testable belief**. Beliefs can be tagged, weighted, routed, tested, revised, killed, threaded into causal chains across arbitrary graph depths, converged across sources into minimal authoritative knowledge stores, prompt-engineered for richer reasoning chains, merged across independent workflows into cross-validated meta-beliefs, and — as Phases 4-7 demonstrate — used to detect and pre-resolve when one or more sources among many have been compromised. Text and summaries cannot.

---

## References

Clark, A. (2013). Whatever next? Predictive brains, situated agents, and the future of cognitive science. *Behavioral and Brain Sciences*, 36(3), 181-204.

Friston, K. (2010). The free-energy principle: a unified brain theory? *Nature Reviews Neuroscience*, 11(2), 127-138.

Packer, C., Wooders, S., Lin, K., Fang, V., Patil, S. G., Stoica, I., & Gonzalez, J. E. (2023). MemGPT: Towards LLMs as Operating Systems. *arXiv preprint arXiv:2310.08560*.

Shinn, N., Cassano, F., Gopinath, A., Narasimhan, K., & Yao, S. (2023). Reflexion: Language Agents with Verbal Reinforcement Learning. *NeurIPS 2023*.

Wei, J., Wang, X., Schuurmans, D., Bosma, M., Ichter, B., Xia, F., Chi, E., Le, Q., & Zhou, D. (2022). Chain-of-thought prompting elicits reasoning in large language models. *NeurIPS 2022*.

Wu, J., Ouyang, L., Ziegler, D. M., Stiennon, N., Lowe, R., Leike, J., & Christiano, P. (2021). Recursively summarizing books with human feedback. *arXiv preprint arXiv:2109.10862*.

---

## Appendix A: Experimental Data

### Phase 1 Results (Single File)

| Metric | Belief Pipeline | Baseline |
|--------|---------------:|--------:|
| Gatekeeper input tokens | 4,487 | — |
| Gatekeeper output tokens | 1,461 | — |
| Mask input (Q1) | 861 | 4,326 |
| Mask input (Q2) | 569 | 4,333 |
| Total mask input | 1,430 | 8,659 |
| Mask/Baseline ratio | 16.5% | 100% |

### Phase 2 Results (Cross-File)

| Approach | Answer Input Tok | Total Tok | Accuracy |
|----------|----------------:|----------:|----------|
| A: Full Context | 8,312 | 10,360 | Ground truth |
| B: Naive Summary | 3,159 | 16,293 | Fabricated code details |
| C: Belief Single | 1,704 | 19,090 | Confidently wrong (4 fabricated issues) |
| D: Belief Revised | 2,666 | 35,291 | Corrected, matches ground truth |

### Gatekeeper Evaluation Output (Phase 2, Approach D)

- **Assessment**: partial
- **Gaps identified**: 10
  1. Mask invents non-existent cross-module issues with no basis in code
  2. Mask assumes workflow-level aggregate cost calculations from envelopes — no code does this
  3. Mask assumes filters trace lineage through envelope execution_ids — no filter does this
  4. Mask assumes token budget enforcement from envelopes — no such mechanism exists
  5. Mask assumes routing context inspects envelope metadata for cost — it only examines port schemas
  6. Mask missed the actual execution_id field mismatch issue visible in source
  7. Mask fabricates a "leaky abstraction" problem that doesn't exist
  8. Mask's recommendation to hydrate envelopes would require changing StepExecutionEnvelope structure
  9. Mask fails to trace port resolution code path showing it only extracts envelope.data
  10. The real subtle issue: ExecutionMetadata fields in synthetic envelopes are cosmetic — only envelope.data matters for correctness

- **Belief revisions**: 8 total (3 kills, 2 revisions, 3 additions)

### Phase 3 Results (Long-Chain Threading)

#### Belief Selection Per Question

| Question | Selected | Pruned | Selection Ratio |
|----------|:--------:|:------:|:-----------:|
| Q1 (ship readiness) | 21 | 17 | 54% |
| Q2 (top 3 risks) | 15 | 24 | 38% |
| Q3 (PRD changes) | 20 | 19 | 51% |

#### Thread Detection Per Question

**Q1 — 6 threads:**
1. GDPR EU Deployment - Critical Blocker: b02 → b10 → b19 → b34 → b37 (5 beliefs, 5 nodes)
2. Security Review - Ship Blocker: b15 → b29 → b39 (3 beliefs, 3 nodes)
3. Performance SLA Validation Gap: b05 → b22 → b35 → b37 (4 beliefs, 4 nodes)
4. Dual-Storage Materialization Delay: b06 → b12 → b17 → b36 → b38 (5 beliefs, 4 nodes)
5. Timeline vs Reality Check: b03 → b23 → b24 → b27 → b30 → b33 (6 beliefs, 5 nodes)
6. Offline Editing Descoped Successfully: b04 → b07 → b21 → b24 → b27 (5 beliefs, 5 nodes)

**Q2 — 4 threads:**
1. GDPR Compliance → EU Deployment Blocker: b02 → b19 → b34 → b35 → b37 (5 beliefs, 4 nodes)
2. Performance SLA → Validation Gap: b05 → b22 → b35 → b36 → b37 (5 beliefs, 4 nodes)
3. Security Review Dependency Chain: b29 → b28 → b39 (3 beliefs, 2 nodes)
4. Timeline Pressure → Scope Cuts: b03 → b23 → b24 → b27 (4 beliefs, 3 nodes)

**Q3 — 7 threads:**
1. Offline Editing Descoped Due to Timeline Gap: b04 → b03 → b07 → b23 → b24 → b27 (6 beliefs, 4 nodes)
2. Image Embedding Cut from Scope: b01 → b03 → b23 → b27 → b32 (5 beliefs, 4 nodes)
3. Dual-Storage Architecture: b14 → b17 → b36 (3 beliefs, 3 nodes)
4. CRDT Library Bus-Factor Risk Mitigation: b09 → b20 (2 beliefs, 2 nodes)
5. EU Deployment Infrastructure Unfinished: b02 → b34 → b37 (3 beliefs, 3 nodes)
6. Performance SLA Validation Incomplete: b05 → b22 → b35 → b37 (4 beliefs, 4 nodes)
7. Security Review Blocking Ship: b39 (1 belief, 1 node)

#### Dormant Resurfacing Detail

| Question | Belief | Semantic Tag | Source Depth | Resurfaced? |
|----------|--------|-------------|:-----------:|:-----------:|
| Q1 | b02 | gdpr_data_residency | 9 | Yes |
| Q2 | b05 | performance_sla | 9 | Yes |
| Q3 | b04 | offline_editing_requirement | 9 | Yes |
| Q3 | b03 | timeline_pressure | 9 | No (selected but not flagged dormant) |
| Q3 | b06 | api_versioning_constraint | 9 | No (pruned) |

#### Depth Distribution (Beliefs Selected)

```
         Q1   Q2   Q3
depth 9:  4    3    5
depth 8:  1    0    2
depth 7:  2    0    1
depth 6:  3    2    3
depth 5:  1    2    2
depth 4:  2    3    1
depth 3:  1    0    1
depth 2:  3    2    2
depth 1:  4    3    3
```

#### Token Budget

| Call | Input | Output |
|------|------:|-------:|
| Q1 Gatekeeper | 4,476 | 1,692 |
| Q1 Mask | 4,625 | 1,122 |
| Q2 Gatekeeper | 4,475 | 1,493 |
| Q2 Mask | 3,264 | 859 |
| Q3 Gatekeeper | 4,475 | 1,778 |
| Q3 Mask | 4,377 | 1,282 |
| Baseline (all 3 Qs) | 2,950 | 3,433 |
| **Pipeline total** | **25,692** | **8,226** |
| **Baseline total** | **2,950** | **3,433** |

### Phase 4 Results (Adversarial Telephone)

#### Ground Truth Claims

| Claim | Description | Correct Value | Poisoned? | Poison Value |
|-------|-------------|---------------|:---------:|--------------|
| 1 | Critical alert latency | 500ms | No | — |
| 2 | Encryption at rest | AES-256 | No | — |
| 3 | Audit log retention | 7 years | **Yes** | 5 years |
| 5 | Payload size limit | 4KB | No | — |
| 6 | Retry max attempts | 3 retries | **Yes** | 5 retries |
| 7 | DLQ threshold | 24 hours | No | — |
| 9 | Priority levels | 4 levels | No | — |
| 10 | Authentication | mTLS + JWT | No | — |
| 11 | Failover detection | 30s detection | **Yes** | 60s detection |
| 12 | Data residency | Must not leave region | No | — |

#### Belief Store Summary (70 beliefs)

| Source Node | Beliefs | Key Poisoned Beliefs |
|-------------|:-------:|---------------------|
| Product Manager | 8 | — |
| System Architect | 17 | — |
| Security Reviewer | 8 | — |
| Lead Developer | 11 | — |
| QA Engineer | 12 | b49 (5 retries), b50 (60s detection), b51 (5-year retention, tone: "modified") |
| Technical Writer | 14 | — (filtered poison via majority consensus) |

#### Thread Detection Detail (Approach E — 11 threads)

| Thread | Beliefs | Contradiction? |
|--------|---------|:--------------:|
| Critical Alert Latency | b01→b02→b09→b12→b45→b46→b57→b68 | No |
| **Retry Policy** | **b06→b16→b39→b49→b61** | **Yes: QA 5 vs all others 3** |
| **Audit Retention** | **b08→b24→b26→b37→b51→b66** | **Yes: QA 5yr vs all others 7yr** |
| Payload Size Limit | b05→b11→b36→b53→b59 | No |
| DLQ Threshold | b07→b17→b30→b40→b62 | No |
| Failover Timing | b18→b19→b50→b63 | No (confounded) |
| Priority Levels | b03→b04→b67 | No |
| Encryption & Key Rotation | b27→b28→b56→b64 | No |
| Rate Limiting | b13→b38→b52→b60 | No |
| Authentication & Tokens | b23→b32→b65 | No |
| TLS & Transmission | b22→b31 | No |

#### Revision Actions (Approach F)

| Action | Belief | Source | Content | Planted? |
|--------|--------|--------|---------|:--------:|
| Kill | b49 | QA Engineer | "5 maximum retry attempts" | Yes |
| Kill | b51 | QA Engineer | "5-year retention" (tone: modified) | Yes |
| Revise | b41 | Developer | "acks=1 (not all)" → acks=all | No (genuine) |
| Revise | b38 | Developer | Token bucket sizing clarification | No (genuine) |
| Confirm | b08 | PM | "7-year audit trail" | — |
| Confirm | b24 | Architect | "7-year retention per HIPAA" | — |
| Confirm | b26 | Security | "7-year per §164.530(j)" | — |
| Confirm | b06 | PM | "3 retry attempts" | — |
| Confirm | b16 | Architect | "3 retry attempts" | — |
| Confirm | b20 | Architect | "min.insync.replicas=2" | — |

**Kill accuracy**: 2/2 correct (both planted poison). **False kills**: 0. **Bonus finds**: 1 genuine cross-node contradiction (Kafka acks).

#### Belief Selection (Approach D vs E)

| Approach | Selected | Pruned | Selection Ratio |
|----------|:--------:|:------:|:-----------:|
| D: Flat | 26/70 | 44 | 37% |
| E: Threaded | 48/70 | 20 | 69% |

Approach D (flat selection) pruned all QA poison beliefs (b49, b50, b51) — it never exposed the mask to contradictions, which is why it scored well on answer-level metrics but could not report on distortions. Approach E (threaded) deliberately included contradicting beliefs to surface them, which improved structural analysis but complicated answer-level scoring.

#### Predictions vs Actuals

| Approach | Predicted | Actual (original) | Actual (rescored) | Delta |
|----------|:---------:|:-----------------:|:-----------------:|:-----:|
| A: Telephone | 5 | 8 | 9 | +4 |
| B: Full Context | 6 | 7 | 8 | +2 |
| C: Summary Chain | 4 | 7 | 7 | +3 |
| D: Belief Flat | 7 | 7 | 9 | +2 |
| E: Belief Threaded | 8 | 5 | 7 | -1 |
| F: Belief Revised | 10 | 6 | 6 | -4 |

Predictions assumed the Tech Writer would propagate QA's poison (making A weak) and that contradiction detection would be scored as correct (making E and F strong). Neither assumption held: the Tech Writer filtered poison, and the regex scorer penalizes contradiction reporting. See Section 5.8 for analysis.

#### Token Budget

| Phase | Calls | Input | Output | Total |
|-------|:-----:|------:|-------:|------:|
| Node generation | 6 | 5,732 | 6,427 | 12,159 |
| Belief generation | 6 | ~7,000 | ~7,000 | ~14,000 |
| Approach A | 1 | 2,064 | 1,148 | 3,212 |
| Approach B | 1 | 7,521 | 1,715 | 9,236 |
| Approach C | 2 | 8,511 | 2,220 | 10,731 |
| Approach D | 2 | 7,140 | 2,190 | 9,330 |
| Approach E | 2 | 9,879 | 4,090 | 13,969 |
| Approach F | 4 | 21,996 | 9,100 | 31,096 |
| Distortion detection | 6 | ~12,000 | ~6,000 | ~18,000 |
| **Total** | **28** | | | **~121,000** |

### Phase 5 Results (Belief Convergence)

#### Convergence Output (22 converged beliefs from 70 raw)

| ID | Topic | Consensus | Sources | Resolved? |
|----|-------|-----------|:-------:|:---------:|
| cb01 | critical_alert_latency_sla | unanimous | 4 | No |
| cb02 | urgency_tier_routing | strong | 2 | No |
| cb03 | acknowledgment_escalation_timeout | unique | 1 | No |
| cb04 | payload_size_limit | unanimous | 5 | No |
| cb05 | retry_policy_maximum_attempts | strong | 4 | **Yes** |
| cb06 | ingestion_timing_budget | strong | 1 | No |
| cb07 | rate_limiting_policy | unanimous | 4 | No |
| cb08 | dead_letter_queue_retention | strong | 5 | No |
| cb09 | audit_retention_period | strong | 5 | **Yes** |
| cb10 | kafka_cluster_configuration | strong | 2 | No |
| cb11 | websocket_connection_capacity | unanimous | 4 | No |
| cb12 | priority_processing_allocation | strong | 2 | No |
| cb13 | failover_timing | strong | 3 | No |
| cb14 | encryption_at_rest | strong | 3 | No |
| cb15 | tls_encryption_requirements | strong | 2 | No |
| cb16 | jwt_authentication | strong | 2 | No |
| cb17 | circuit_breaker_configuration | unique | 1 | No |
| cb18 | autoscale_threshold | strong | 2 | No |
| cb19 | monitoring_thresholds | unique | 1 | No |
| cb20 | security_gap_mfa_requirement | unique | 1 | No |
| cb21 | security_gap_rbac | unique | 1 | No |
| cb22 | load_testing_requirements | unique | 1 | No |

Compression: 70 → 22 (3.2x). Redundancies removed: 46. Unique insights preserved: 6. Contradictions resolved: 2/3 (failover confounded).

#### Claim Coverage Audit

| Claim | Description | Covered? |
|-------|-------------|:--------:|
| 1 | Critical alert latency (500ms) | Yes |
| 2 | Encryption (AES-256) | Yes |
| 3 | Audit retention (7 years) | Yes |
| 4 | Concurrent connections (10K) | Yes |
| 5 | Payload size (4KB) | Yes |
| 6 | Retry attempts (3 max) | Yes |
| 7 | DLQ threshold (24 hours) | Yes |
| 8 | Rate limit (100/s) | Yes |
| 9 | Priority levels (4) | Yes |
| 10 | Authentication (mTLS + JWT) | Yes |
| 11 | Failover detection (30s) | Yes |
| 12 | Data residency (must not leave region) | **No** |

11/12 claims covered. Data residency lost during convergence — prohibitive phrasing without a specific numerical value was not preserved by topic-clustering.

#### 10-Question Predictions vs Actuals

| Approach | Predicted | Actual | Delta |
|----------|:---------:|:------:|:-----:|
| G: Converged | 10 | 9 | -1 |
| H: Converged + Resolved | 10 | 8 | -2 |
| I: Raw Flat | 7 | 6 | -1 |

#### 30-Question Predictions vs Actuals

| Approach | Predicted | Actual | Delta |
|----------|:---------:|:------:|:-----:|
| Full Context | 22 | 28 | +6 |
| Converged | 27 | 24 | -3 |
| Raw Flat | 20 | 24 | +4 |

Full context far exceeded predictions — Sonnet 4.5's native contradiction-resolution capability was underestimated.

#### 30-Question Category Breakdown

| Approach | Clean (10) | Poison (3) | Synth (2) | Cross (5) | Hypo (5) | Adv (5) | Total (30) |
|----------|:----------:|:----------:|:---------:|:---------:|:--------:|:-------:|:----------:|
| Full Context | 10 | 3 | 2 | 3 | 5 | 5 | 28 |
| Converged | 9 | 3 | 1 | 2 | 4 | 5 | 24 |
| Raw Flat | 8 | 3 | 2 | 2 | 4 | 5 | 24 |

#### Cost Efficiency (30Q)

| Approach | Tokens | Correct | Tokens/Correct |
|----------|:------:|:-------:|:--------------:|
| Full Context | 12,012 | 28 | 429 |
| **Converged** | **8,170** | **24** | **340** |
| Raw Flat | 15,055 | 24 | 627 |

#### Token Budget

| Call | Input | Output | Total | Time |
|------|------:|-------:|------:|-----:|
| Convergence | 5,998 | 3,916 | 9,914 | 57s |
| G: Converged 10Q | 3,229 | 1,336 | 4,565 | 19s |
| H: Resolved 10Q | 3,366 | 1,223 | 4,589 | 16s |
| I: Raw Select 10Q | 4,637 | 747 | 5,384 | 15s |
| I: Raw Mask 10Q | 2,471 | 1,251 | 3,722 | 19s |
| Full Context 30Q | 8,065 | 3,947 | 12,012 | 64s |
| Converged 30Q | 3,926 | 4,244 | 8,170 | 61s |
| Raw Select 30Q | 5,197 | 929 | 6,126 | 18s |
| Raw Mask 30Q | 4,232 | 4,697 | 8,929 | 71s |
| **Total** | **41,121** | **22,290** | **63,411** | **340s** |

### Phase 6 Results (Prompt-Engineered Beliefs)

#### Belief Quality Comparison (v1 vs v2)

| Metric | v1 (Phase 4) | v2 (Phase 6) |
|--------|:------------:|:------------:|
| Total beliefs | 70 | 98 |
| Reasoning field | absent | 158 chars avg |
| Confidence justification | absent | 100% |
| Cross-source tension | absent | 26% |
| Emotional tone | present | present |

#### Convergence Comparison (Phase 5 vs Phase 6)

| Metric | Phase 5 | Phase 6 |
|--------|--------:|--------:|
| Input beliefs | 70 | 98 |
| Output beliefs | 22 | 40 |
| Compression ratio | 3.2x | 2.5x |
| Contradictions resolved | 2/3 | 3/3 |
| Claim 12 (data residency) | Missing | Covered |
| Claim 11 (failover) | Covered | Missing* |

*Claim 11 is correctly resolved in Phase 6 convergence (30s selected over 60s) but the converged prose doesn't match the regex pattern.

#### 30-Question Category Breakdown (Phase 6)

| Approach | Clean (10) | Poison (3) | Synth (2) | Cross (5) | Hypo (5) | Adv (5) | Total (30) |
|----------|:----------:|:----------:|:---------:|:---------:|:--------:|:-------:|:----------:|
| Full Context | 10 | 2 | 2 | 3 | 5 | 5 | 27 |
| Converged v2 | 10 | 2 | 1 | 3 | 5 | 5 | 26 |
| Raw Flat v2 | 10 | 2 | 1 | 3 | 4 | 5 | 25 |

#### Predictions vs Actuals (Phase 6)

| Approach | Predicted | Actual | Delta |
|----------|:---------:|:------:|:-----:|
| G2: Converged 10Q | 10 | 8 | -2 |
| I2: Raw Flat 10Q | 8 | 0* | -8 |
| Full Context 30Q | 28 | 27 | -1 |
| Converged v2 30Q | 27 | 26 | -1 |
| Raw Flat v2 30Q | 25 | 25 | 0 |

*Token limit anomaly (4,096 output limit truncated answers).

#### Cost Efficiency Comparison (30Q)

| Approach | Phase 5 Tok/Correct | Phase 6 Tok/Correct | Delta |
|----------|:-------------------:|:-------------------:|:-----:|
| Full Context | 429 | 799 | +370* |
| Converged | 340 | 601 | +261 |
| Raw Flat | 627 | 1,260 | +633 |

*Phase 6 uses more tokens due to v2 answer schema requiring reasoning fields in outputs.

#### Token Budget (Phase 6)

| Call | Input | Output | Total | Time |
|------|------:|-------:|------:|-----:|
| Belief v2 (6 nodes) | 17,134 | 15,659 | 32,793 | 208s |
| Convergence v2 | 13,181 | 11,672 | 24,853 | 190s |
| G2: Converged 10Q | 4,732 | 3,314 | 8,046 | 60s |
| I2: Raw Select 10Q | 6,644 | 1,174 | 7,818 | 28s |
| I2: Raw Mask 10Q | 3,884 | 4,096 | 7,980 | 64s |
| Full Context 30Q | 8,233 | 13,352 | 21,585 | 229s |
| Converged v2 30Q | 5,292 | 10,325 | 15,617 | 150s |
| Raw Select 30Q | 7,204 | 1,966 | 9,170 | 33s |
| Raw Mask 30Q | 7,102 | 15,230 | 22,332 | 237s |
| **Total (Phase 6)** | **73,406** | **76,788** | **150,194** | |

### Phase 7 Results (Multi-Workflow Meta-Convergence)

#### Taxonomy

52 controlled tags generated across 7 domains: performance (5), reliability (13), security (6), compliance (9), operations (11), integration (1), clinical (7).

#### WF2 Belief Generation (v3)

| Node | Role | Beliefs | Facts | Opinions | Policies | Observations |
|------|------|:-------:|:-----:|:--------:|:--------:|:------------:|
| 1 | Operations Engineer | 21 | 11 | 8 | 0 | 2 |
| 2 | Compliance Officer | 16 | 10 | 0 | 4 | 0 |
| 3 | Clinical Advisor | 15 | 7 | 7 | 0 | 1 |
| 4 | Integration Engineer (poisoned) | 15 | 8 | 7 | 0 | 0 |
| **Total** | | **67** | **35** | **25** | **4** | **3** |

#### WF2 Convergence

67 raw beliefs → 26 converged beliefs (2.6x compression). 4 contradictions found and resolved.

#### Meta-Convergence

| Input | Count |
|-------|:-----:|
| WF1 converged beliefs | 40 |
| WF2 converged beliefs | 26 |
| **Output meta-beliefs** | **44** |
| Cross-validated topics | 10 |
| Cross-workflow splits (resolved) | 2 |
| WF1-only topics | 22 |
| WF2-only topics | 10 |

#### 20-Question Scoring

| Approach | WF1 (5) | WF2 (5) | Cross (5) | Adv (5) | Total (20) |
|----------|:-------:|:-------:|:---------:|:-------:|:----------:|
| Meta-converged | 3 | 2 | 4 | 4 | 13 |
| Flat-converged | 4 | 3 | 4 | 4 | 15 |
| Full context | 4 | 2 | 4 | 4 | 14 |

#### Adversarial Detail (Phase 7)

| Question | Poison | Meta | Flat | Full |
|----------|--------|:----:|:----:|:----:|
| P7Q16: 750ms vs 500ms latency | wf2_claim_01 | CORRECT | CORRECT | CORRECT |
| P7Q17: Triple retention (3/5/7yr) | wf2_claim_03 | CORRECT | CORRECT | CORRECT |
| P7Q18: 45min vs 15min incident | wf2_claim_09 | WRONG | WRONG | WRONG |
| P7Q19: Cross-WF retention | claim_03+wf2_03 | CORRECT | CORRECT | CORRECT |
| P7Q20: Cross-WF latency | claim_01+wf2_01 | CORRECT | CORRECT | CORRECT |

#### Predictions vs Actuals (Phase 7)

| Approach | Predicted | Actual | Delta |
|----------|:---------:|:------:|:-----:|
| Meta-converged | 17 | 13 | -4 |
| Flat-converged | 14 | 15 | +1 |
| Full context | 18 | 14 | -4 |

#### Token Budget (Phase 7)

| Call | Input | Output | Total | Time |
|------|------:|-------:|------:|-----:|
| Taxonomy | 2,984 | 2,808 | 5,792 | 35s |
| WF2 nodes (4) | 4,978 | 8,192 | 13,170 | 200s |
| WF2 beliefs v3 (4) | 16,621 | 9,408 | 26,029 | 119s |
| WF2 convergence | 8,061 | 5,591 | 13,652 | 79s |
| Meta-convergence | 7,408 | 14,621 | 22,029 | 215s |
| Flat convergence | 18,817 | 12,417 | 31,234 | 208s |
| Meta answers (20Q) | 7,006 | 9,667 | 16,673 | 166s |
| Flat answers (20Q) | 5,115 | 7,963 | 13,078 | 115s |
| Full context answers (20Q) | 16,131 | 11,001 | 27,132 | 208s |
| **Total (Phase 7)** | **87,121** | **81,668** | **168,789** | |
