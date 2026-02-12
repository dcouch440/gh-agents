# Belief-Oriented Conversation Architecture: Authored Context as an Alternative to Retrieval and Summarization in Multi-Agent LLM Systems

**David Couch**

February 2026

---

## Abstract

We introduce Belief-Oriented Conversation Architecture (BOCA), a framework for multi-agent LLM systems that replaces raw context passing with *authored belief slices* — semantically tagged, confidence-weighted hypotheses about source material that carry emotional and structural metadata. Unlike retrieval-augmented generation (RAG) which retrieves relevant chunks, or summarization which compresses input, BOCA's gatekeeper agent *constructs a worldview* and selectively routes curated beliefs to lightweight mask agents that reason without access to the original source. We demonstrate three novel contributions: (1) authored context — belief slices that encode understanding rather than content — transfers sufficient signal for analytical reasoning at 16-20% of full-context token cost; (2) selective belief routing outperforms uniform context by enabling per-question relevance filtering; and (3) a belief revision loop (confirm/revise/kill) catches and corrects hallucinations that single-pass approaches propagate. In a controlled experiment on production Rust source code, the single-pass belief pipeline produced confident but fabricated analysis, while the revision pipeline identified 10 specific errors, killed 3 false beliefs, and converged on answers that matched full-context ground truth — a self-correction capability absent from both naive summarization and standard multi-agent delegation.

## 1. Introduction

Large language models operating on substantial codebases or documents face a fundamental tension: context windows are finite, but understanding requires holistic comprehension. Current approaches to this tension fall into three categories:

**Full context passing** provides the model with all source material. This is accurate but expensive, scales poorly, and becomes impossible when source material exceeds context limits.

**Retrieval-augmented generation (RAG)** retrieves relevant chunks via embedding similarity. This reduces token cost but operates on syntactic/semantic proximity rather than understanding — it retrieves *content* without *interpretation*.

**Summarization** compresses source material into shorter representations. This reduces tokens but is lossy, cannot self-correct, and discards structural and emotional metadata that may be critical for reasoning.

We propose a fourth approach: **authored context**. Rather than retrieving or compressing, a gatekeeper agent reads the full source, forms *beliefs* about it — tagged hypotheses carrying semantic, confidence, and emotional metadata — and constructs curated worldviews for downstream agents. These downstream agents (which we call *masks*) are not specialized models or roles; they are the same base model wearing different belief-state lenses.

This paper presents the architecture, reports results from two experimental phases, and identifies the specific mechanism — belief revision — that distinguishes BOCA from intelligent summarization.

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

We used production Rust source code from a workflow orchestration system:

- **Phase 1**: `resume.rs` (444 lines) — DAG workflow resumption logic
- **Phase 2**: `resume.rs` + `single.rs` (825 lines combined) — cross-file interaction between resumption orchestration and step execution

### 4.2 Questions

**Phase 1** tested two analytical questions on a single file:
- Q1: Enumerate all failure modes and silent skip conditions
- Q2: Trace the data flow of a specific HashMap across its lifecycle

**Phase 2** tested one hard cross-file question requiring understanding of interactions invisible when reading either file in isolation:
- "When resume_workflow_via_engine calls execute_single_step for a resumed workflow, the pre-completed steps have synthetic envelopes with zeroed metadata. Trace exactly what happens when a downstream step tries to resolve port inputs from these synthetic envelopes. What subtle data quality issues could emerge? Identify at least one issue not obvious from reading either file alone."

### 4.3 Approaches Compared (Phase 2)

| Approach | Description |
|----------|-------------|
| **A: Full Context** | Both files provided raw to the model |
| **B: Naive Summary** | Both files summarized first, question answered from summary only |
| **C: Belief Single-Pass** | Gatekeeper decomposes into beliefs, assigns relevant subset, mask answers |
| **D: Belief with Revision** | Same as C, then gatekeeper evaluates, revises beliefs, mask re-answers |

### 4.4 Model

All calls used Claude Sonnet 4.5 (`claude-sonnet-4-5-20250929`). Structured JSON output was enforced via Anthropic's tool_use API for all gatekeeper operations (belief decomposition, assignment, evaluation), eliminating parsing errors.

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

### 5.3 The Critical Finding: Self-Correction

The single most important result is not about tokens. It is that **the belief revision loop caught hallucinations that both summarization and single-pass beliefs propagated**.

The naive summary (B) fabricated code details. It cannot self-correct because no mechanism exists to check the summary against reality.

The single-pass belief pipeline (C) fabricated system behaviors. The beliefs were real, but the mask over-inferred from them, and no check existed.

The revision pipeline (D) produced the *same initial hallucinations* as C — but then the gatekeeper identified them, killed the false beliefs, added corrected ones, and the revised mask produced accurate output.

**This is the distinction between smart summarization and belief architecture**: the ability to test beliefs against source material and revise them.

## 6. Discussion

### 6.1 When BOCA Outperforms Alternatives

BOCA's value proposition is strongest when:

1. **Multiple questions are asked against the same source material**: The gatekeeper decomposition amortizes across questions. At 4+ questions against the same source, the belief pipeline is cheaper in total tokens than full context.

2. **Accuracy matters more than speed**: The revision loop adds latency (additional gatekeeper evaluation + mask re-invocation) but catches errors that single-pass approaches miss.

3. **Source material exceeds context limits**: When full context is impossible, BOCA offers a principled alternative to chunking or summarization, with built-in quality assurance via revision.

4. **Emotional and structural metadata matters**: The gatekeeper's emotional tagging (`fragile`, `rushed`, `defensive`) carries information that pure summarization discards. Whether downstream agents use this effectively is an open question, but the metadata is available.

### 6.2 When BOCA is Overkill

For single questions on short documents, full context is simpler, faster, and cheaper. BOCA's overhead (gatekeeper decomposition + assignment) only pays off when amortized across multiple queries or when the source material is too large for direct processing.

### 6.3 The Gatekeeper as Smartest Agent

A non-obvious property of BOCA is that the gatekeeper must be the most capable agent in the system. It reads full source material, forms hypotheses, evaluates mask outputs, and identifies gaps. In traditional multi-agent systems, the orchestrator is a router — it doesn't need deep understanding. In BOCA, the gatekeeper *is* the understanding. Masks are cheap projections of its worldview.

This has implications for model selection: the gatekeeper should be the most capable (and expensive) model, while masks can be lighter models that reason well from curated context.

### 6.4 Limitations

**Total token cost**: For the Phase 2 cross-file experiment, the full revision pipeline (D) used 35,291 total tokens versus 10,360 for full context (A). The revision pipeline is ~3.4x more expensive in total. This cost is justified only when accuracy is paramount or when amortized across many queries.

**Gatekeeper reads full source**: The gatekeeper still requires the full context window for decomposition and evaluation. BOCA does not solve context length limits for the gatekeeper — it solves them for downstream agents.

**Evaluation is qualitative**: We assessed accuracy by comparing answers against full-context ground truth. A rigorous evaluation would require human scoring on a diverse benchmark. Our results demonstrate the mechanism works but do not quantify accuracy on a standardized scale.

**Single revision pass**: We tested one round of revision. Whether additional rounds improve quality, plateau, or degrade (through over-correction) is unexplored.

### 6.5 Relationship to Predictive Processing

The confirm/revise/kill cycle in BOCA mirrors the prediction error minimization loop in predictive processing theories of cognition (Clark, 2013). The gatekeeper forms predictions (beliefs) about the source, the mask tests those predictions by reasoning from them, and the evaluation step computes "prediction errors" (gaps between mask output and source truth). This is not a metaphor — it is a direct computational implementation of the same pattern.

This suggests a deeper principle: **understanding is not compression; it is prediction under uncertainty, tested and revised**. Summarization compresses. RAG retrieves. BOCA predicts, tests, and updates. The biological precedent suggests this pattern may be fundamentally more robust for complex reasoning tasks.

## 7. Future Work

### 7.1 Multi-Round Revision

Testing iterative revision (2, 3, N rounds) to characterize the convergence behavior: does accuracy improve monotonically, or is there an optimal number of revision cycles?

### 7.2 Heterogeneous Model Pairing

Using a high-capability model (e.g., Claude Opus) as gatekeeper and a lower-cost model (e.g., Claude Haiku) as mask. If beliefs carry sufficient signal, lighter masks may produce comparable quality at dramatically lower cost.

### 7.3 Belief Persistence and Transfer

Beliefs generated for one task may be reusable for related tasks. A persistent belief store could amortize decomposition cost across an entire session or project.

### 7.4 Benchmark Construction

Building a standardized evaluation benchmark for authored-context architectures: source material of varying complexity, questions requiring cross-document reasoning, and human-scored accuracy metrics.

### 7.5 Scaling to Full Codebases

Testing BOCA on codebase-scale inputs (thousands of files) where the gatekeeper itself cannot hold full context, requiring hierarchical belief decomposition — gatekeepers of gatekeepers.

## 8. Conclusion

We have presented Belief-Oriented Conversation Architecture, a framework that replaces raw context passing with authored belief slices. Our experiments demonstrate three findings:

1. **Authored context transfers**: Masks reasoning from beliefs alone produced coherent analysis at 16-20% of full-context token cost.

2. **Beliefs beat summaries qualitatively**: Both naive summaries and single-pass beliefs led to hallucinated details. The belief revision loop caught and corrected these errors; summarization cannot.

3. **The revision mechanism is the differentiator**: Without revision, BOCA is smart summarization with metadata. With revision, it is a self-correcting reasoning architecture that tests its own understanding against ground truth.

The core insight is architectural: **the unit of context should not be a chunk of text or a summary, but a testable belief**. Beliefs can be tagged, weighted, routed, tested, revised, and killed. Text and summaries cannot.

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
