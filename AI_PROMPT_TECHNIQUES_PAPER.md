# AI Prompt Optimization Techniques from Open Source Frameworks

**A Technical Survey for Nexor Workflow Engine Enhancement**

---

## Abstract

This paper surveys prompt optimization and LLM output quality techniques from six major open source AI frameworks: CrewAI, LangChain/LangGraph, DSPy, AutoGen, Semantic Kernel, Haystack, and LlamaIndex. The goal is to identify concrete, implementable techniques that could improve Nexor's multi-step workflow execution engine. We focus exclusively on techniques that operate at the prompt/orchestration layer -- no model fine-tuning, no weight updates. Every technique described here works by changing what text reaches the LLM, how outputs are validated, or how execution is orchestrated.

---

## 1. Human Feedback Distillation (CrewAI)

### What It Is

A human-in-the-loop prompt augmentation pipeline that collects user corrections, synthesizes them into reusable instructions via an LLM, and injects those instructions into future prompts.

### How It Works

1. **Collection phase**: The system runs N iterations of a workflow, forcing human review after every step. For each step, it records a triplet: `(initial_output, human_feedback, improved_output)`.

2. **Distillation phase**: After all iterations, the triplets are sent to an LLM evaluator with the prompt: "Assess the quality of the training data and distill the human feedback patterns into actionable suggestions." The evaluator returns a structured response:

```
{
  "suggestions": ["Always include primary sources with links", ...],
  "quality": 8.5,
  "final_summary": "Step 1: Start with a brief overview..."
}
```

3. **Injection phase**: On subsequent normal runs, the `suggestions` array is appended to every task prompt:

```
\n\nYou MUST follow these instructions:
 - Always include primary sources with links
 - Structure output with headers and bullet points
 - Limit response to 500 words maximum
```

### Key Design Details

- **Feedback accumulates during training**: On iteration 3, the prompt already includes raw feedback from iterations 0, 1, and 2. Each iteration benefits from prior corrections.
- **Role-based persistence**: Final suggestions are keyed by agent role name (e.g., "Research Analyst"), not internal UUID. This survives code changes as long as role names are stable.
- **Two levels of feedback representation**: Raw human text during training, LLM-distilled suggestions for production. The distillation generalizes specific corrections ("add a source for the GDP claim") into reusable rules ("Always include primary sources").
- **Write-only fields**: The evaluator produces `quality` (float) and `final_summary` (step-by-step plan), but only `suggestions` is ever read at runtime. The other fields are dead weight.

### Applicability to Nexor

**High.** Nexor already stores full execution traces in `execution_messages` and structured outputs in `agent_executions`. A feedback system could:
- Store distilled suggestions in an `agent_guidances` table keyed by agent ID + optional workflow step context
- Inject during `compose_prompt()` in `hub/dag/utils/mod.rs`
- Version guidance entries to track effectiveness over time

---

## 2. Automated Prompt Optimization (DSPy)

### What It Is

DSPy treats prompts as programs with optimizable parameters. Instead of hand-writing prompts, you define input/output signatures, and optimizers automatically find the best combination of instructions and few-shot examples.

### Core Concepts

**Signatures** define I/O contracts:
```python
class GenerateAnswer(dspy.Signature):
    """Answer questions with short factoid answers."""
    context = dspy.InputField(desc="may contain relevant facts")
    question = dspy.InputField()
    answer = dspy.OutputField(desc="often between 1 and 5 words")
```

**Predict modules** hold a Signature + a demos list. The two optimizable parameters are:
1. The instruction text (the docstring)
2. The few-shot examples (the demos list)

### Optimization Algorithms

#### BootstrapFewShot -- Automated Example Selection

The core algorithm:

1. Run a "teacher" program on each training example
2. Check if the output passes a user-defined metric (e.g., exact match, F1 score)
3. If it passes, extract the full intermediate trace -- every predict call's inputs and outputs
4. Use those successful traces as few-shot demonstrations for the "student" program

The key insight: **the teacher's chain-of-thought reasoning on successful examples becomes the student's few-shot examples**. This is automated -- no human selects which examples to include.

Multiple rounds with `temperature=1.0` and cache-busting ensure diverse examples. The final student gets up to `max_bootstrapped_demos` teacher-generated examples, padded with raw labeled examples.

#### COPRO -- Iterative Instruction Optimization

Optimizes instruction text through LLM-based proposal and evaluation:

1. **Seed**: Generate N candidate instructions from the current instruction using an LLM
2. **Evaluate**: Score all candidates on the training set
3. **Refine**: Feed the scored candidates (sorted worst-to-best) back to the LLM: "Here are instructions I tried and their scores. Propose a better one."
4. **Repeat** for D depth iterations

The prompt to the LLM includes the full history of attempts with their scores, enabling it to learn from what worked and what didn't.

#### MIPROv2 -- Joint Bayesian Optimization

The most sophisticated optimizer. Jointly optimizes both instructions AND examples using Optuna's TPE (Tree-structured Parzen Estimator):

1. **Generate candidates**: Create multiple demo sets (via bootstrap) AND multiple instruction candidates (via a "grounded proposer" that uses dataset summaries, program source code, and random "tips" like "be creative" or "high stakes")
2. **Define search space**: For each predictor in the program, two categorical variables: which instruction candidate and which demo set
3. **Bayesian search**: Optuna's TPE sampler explores the joint space, evaluating each combination on minibatches
4. **Full evaluation**: Periodically, the best-performing combination gets a full validation set evaluation

The grounded proposer is notable: it feeds the LLM the actual Python source code of the program, a summary of the dataset, descriptions of each module's role, and randomly selected "tips" to encourage diverse proposals.

### Quality Enforcement at Runtime

#### BestOfN -- Rejection Sampling
Run the module N times with `temperature=1.0`, score each with a reward function, keep the best. Short-circuit if any score exceeds a threshold.

#### Refine -- LLM-Generated Feedback Loop
1. Run the module, compute reward
2. If below threshold, use an `OfferFeedback` signature to have the LLM analyze the full program trajectory and assign blame to each module
3. Inject the blame/advice as `hint_` fields into each module's next attempt
4. Repeat up to N times

### Applicability to Nexor

**Medium-High.** The full Bayesian optimization requires a training dataset with metrics, which may not always exist for Nexor workflows. However:
- **BootstrapFewShot** could be adapted: store successful execution traces and inject them as examples in future prompts for similar steps
- **COPRO-style instruction refinement** could optimize agent system prompts based on execution success rates
- **BestOfN** could be applied to critical workflow steps: generate multiple outputs, score them, pick the best
- **The Refine pattern** (blame assignment across modules) maps directly to multi-step DAG workflows -- if a downstream step fails, trace blame back to which upstream step produced bad output

---

## 3. Self-Critique and Revision (LangChain Constitutional AI)

### What It Is

An iterative self-improvement loop where the LLM critiques its own output against a set of principles, then revises if needed.

### How It Works

```
Initial Output → For each Principle:
  → Critique: "Does this output violate the principle?"
  → If "No critique needed" → Skip
  → If critique found → Revise output
  → Revised output feeds into next principle
```

### Key Design Details

1. **Short-circuit on "no critique needed"**: Not every output needs revision. The critique prompt includes few-shot examples where the correct answer is "No critique needed," teaching the LLM that passing is acceptable. This prevents over-correction.

2. **Principle library**: ~20 built-in principles covering harmfulness, accuracy, sensitivity, truthfulness calibration. The truthfulness calibration principle is notable -- it teaches the LLM to express appropriate uncertainty about specific technical claims.

3. **Negative examples in few-shot**: The critique prompt shows examples with varied outcomes (some need revision, some don't), creating a balanced evaluator.

4. **Sequential principle application**: Each principle's revision feeds into the next. Order matters -- applying "conciseness" before "completeness" produces different results than the reverse.

### LangGraph Evolution

The newer LangGraph version improves on this by:
- Using `with_structured_output(Critique)` to get a typed boolean `critique_needed` field
- Eliminating brittle string matching ("no critique needed" text parsing)
- Implementing it as a state machine graph with clear node transitions

### Applicability to Nexor

**High.** This maps directly to workflow steps. A "critique step" could be inserted after any step that produces user-facing output:
- Define principles per workflow or per step
- Run the critique as a lightweight LLM call with the step's output
- Only trigger revision if the critique finds issues
- Store critique results for analysis (which steps consistently need revision?)

---

## 4. Stall Detection and Automatic Replanning (AutoGen Magentic-One)

### What It Is

A dual-loop orchestration system that monitors execution progress and automatically replans when stuck.

### How It Works

**Outer Loop -- Task Ledger**: Before execution, the LLM decomposes the task into structured categories:
- GIVEN/VERIFIED FACTS
- FACTS TO LOOK UP
- FACTS TO DERIVE
- EDUCATED GUESSES

This structured decomposition prevents hallucination by making the LLM explicitly declare what it knows vs. what it needs to find.

**Inner Loop -- Progress Ledger**: After each agent action, a structured JSON evaluation:

```json
{
  "is_request_satisfied": {"reason": "...", "answer": false},
  "is_in_loop": {"reason": "...", "answer": false},
  "is_progress_being_made": {"reason": "...", "answer": true},
  "next_speaker": {"reason": "...", "answer": "coder"},
  "instruction_or_question": {"reason": "...", "answer": "..."}
}
```

**Stall detection**: If progress stalls or a loop is detected, a counter increments. When it exceeds a threshold:
1. The fact sheet is updated: "What went wrong? What did we learn?"
2. A new plan is generated that explicitly avoids prior mistakes
3. Execution restarts with the updated context

### Critical Design Pattern: Reason-Before-Answer

Every structured output field requires `{"reason": str, "answer": bool/str}`. The reason field forces chain-of-thought before the decision, improving decision quality. This is a general technique applicable to any structured LLM output.

### Applicability to Nexor

**High.** Nexor's DAG executor already tracks step execution status. Adding:
- A progress evaluation after each step (lightweight LLM call)
- Stall detection across multi-step workflows
- Automatic replanning when workflows get stuck (re-compose prompts with updated context about what failed)

This is especially valuable for long-running workflows where early failures cascade.

---

## 5. Output Parsing and Recovery (LangChain + LlamaIndex)

### Partial JSON Recovery

LangChain's `parse_partial_json` recovers from truncated LLM output by:
1. Scanning character by character, tracking bracket/brace nesting via a stack
2. When the string ends prematurely, auto-closing all open structures
3. If inside a string literal, closing the quote first
4. Progressively removing trailing characters until valid JSON is obtained

This handles the common case where the LLM runs out of tokens mid-JSON.

### Negative Examples in Format Instructions

LangChain's Pydantic output parser includes both positive AND negative examples in format instructions:

```
The object {"foo": ["bar", "baz"]} is a well-formatted instance.
The object {"properties": {"foo": ["bar", "baz"]}} is NOT well-formatted.
```

This addresses a specific failure mode where LLMs echo the schema structure instead of conforming to it.

### Repeated Verdict Extraction

LangChain's evaluation system asks the LLM to output its verdict (Y/N) twice -- once at the end of reasoning and once on its own line. The parser tries multiple extraction strategies: end of text, beginning of text, last line. This redundancy significantly improves parse reliability.

### Validation-Retry with Error Context

Haystack implements a pipeline pattern where:
1. LLM generates output
2. `JsonSchemaValidator` checks against schema
3. On failure, the validator produces error messages
4. A `BranchJoiner` routes errors back to the generator with the error context appended
5. `max_runs_per_component` caps retries

The error messages include what was wrong, giving the LLM precise signal for self-correction.

### Structured Answer Filtering (LlamaIndex)

LlamaIndex's structured answer refine mode asks the LLM to output a `query_satisfied` boolean alongside the answer. If the LLM reports that the query is NOT satisfied by the available context, the answer is discarded. This is a simple self-assessment mechanism that prevents low-confidence answers from propagating.

### Applicability to Nexor

**Very High.** These are low-hanging fruit:
- Partial JSON recovery in the execution engine when parsing `structured_output`
- Negative examples in output schema instructions (currently Nexor injects schemas into system prompts)
- Validation-retry loop for steps with `output_schema_id` -- retry with error context when schema validation fails
- Self-assessment boolean on critical steps

---

## 6. Memory and Context Management

### Progressive Summarization (LangChain)

For long conversations, maintain a running summary. When token count exceeds a limit:
1. Pop oldest messages into a "pruned" list
2. Feed pruned messages + existing summary to a summarizer LLM
3. Replace pruned messages with the updated summary as a SystemMessage
4. Keep recent messages verbatim

This gives full detail on recent context while preserving compressed older context.

### Entity Memory (LangChain)

Per-turn, two LLM calls:
1. **Entity extraction**: Identify proper nouns/entities from the latest message (with coreference resolution via conversation history)
2. **Entity summarization**: For each entity, update a per-entity summary

Entities are stored as key-value pairs and injected into context when relevant.

### Semantic Vector Memory (AutoGen + Semantic Kernel)

Both frameworks implement memory as vector-search-backed retrieval:
1. Store execution outputs/facts with embeddings
2. On each new turn, embed the current input
3. Retrieve top-k similar memories by cosine similarity (with score thresholding)
4. Inject as SystemMessage before the LLM call

### Context Window Budgeting (LangGraph)

`RemainingSteps` managed value tells each node how many steps are left in its budget. Nodes can check `is_last_step` and choose to output a partial answer rather than requesting more tool calls. This forces convergence and prevents infinite loops.

### Applicability to Nexor

**Medium.** Most relevant for:
- Workflows that process large documents or long conversations (progressive summarization of prior step outputs)
- Cross-workflow memory: storing successful execution patterns and retrieving them for similar future workflows
- Step budgeting: capping the execution engine's `max_rounds` dynamically based on remaining token budget

---

## 7. Multi-Agent Deliberation (AutoGen)

### Society of Mind Pattern

Wrap an entire team of agents as a single agent:
1. Run an inner team (e.g., writer + critic in round-robin) to completion
2. Collect all inner messages (the "deliberation")
3. Make a final LLM call that synthesizes the inner conversation into a clean response
4. The user/downstream step only sees the final synthesis, not the messy deliberation

### Structured Retry Decisions

When code execution fails, instead of blindly retrying, ask the LLM for a structured decision:

```json
{"reason": "The import error suggests missing dependency", "retry": true}
```

Using Pydantic models with `json_output` forces the LLM into a structured evaluation. The `reason` field serves double duty: it improves decision quality (chain-of-thought) and provides debuggable audit trails.

### Post-Execution Reflection

After any tool/code execution loop, always run a final "reflection" inference that synthesizes the entire execution history into a coherent response. This prevents the common failure mode where the final tool output is returned raw without contextualization.

### Applicability to Nexor

**Medium.** The Society of Mind pattern could be implemented as a special workflow step type that internally runs a sub-workflow (critique loop) before producing output. The structured retry decision pattern is directly applicable to the execution engine's tool-use loop.

---

## 8. Filter and Middleware Systems (Semantic Kernel)

### Three-Layer Filter Pipeline

Semantic Kernel intercepts at three points:
1. **Prompt rendering**: Before the template becomes text
2. **Function invocation**: Around any function/tool call
3. **Auto function invocation**: Around LLM-initiated tool calls

Each layer supports:
- **Modification**: Change the prompt, arguments, or result
- **Short-circuiting**: Return a cached result without calling the LLM
- **Observation**: Log, trace, or score without modifying

### Executable Functions in Templates

SK's template engine can execute real functions during rendering:
```
Related context: {{memory.recall $input}}
Current date: {{time.now}}
```

`{{memory.recall $input}}` actually calls a kernel function that performs vector search, and the result is injected into the prompt before it reaches the LLM. This declarative RAG-in-template approach is powerful.

### Content Trust Boundaries

All template variable inputs are HTML-escaped by default. Function outputs are also escaped. This prevents prompt injection from untrusted user inputs. Opting out requires explicit `allow_dangerously_set_content = true`.

### Applicability to Nexor

**High.** Nexor's `compose_prompt()` function already does variable resolution and port input resolution. Adding:
- A filter/middleware pipeline around prompt composition and LLM calls
- Prompt-level caching (short-circuit identical prompts)
- Content sanitization for user-provided inputs in workflow variables
- Executable functions in prompt templates (e.g., `{memory.recall input}` or `{db.query "SELECT ..."}`)

---

## 9. Query Transformation (LlamaIndex)

### Multi-Step Query Decomposition

Break a complex query into sub-queries, execute each against the most appropriate data source, then synthesize:

```
"Compare Q3 revenue across all divisions"
→ Sub-query 1: "What was Q3 revenue for Division A?" → Table index
→ Sub-query 2: "What was Q3 revenue for Division B?" → Table index
→ Sub-query 3: "What was Q3 revenue for Division C?" → Table index
→ Synthesis: Combine all sub-answers
```

### HyDE -- Hypothetical Document Embeddings

Instead of searching with the raw query, first generate a hypothetical answer, then search using that answer's embedding. This often retrieves better documents because the hypothetical answer is in the same "semantic space" as the actual documents.

### Feedback Query Transformation

When a retry is needed, append the previous bad answer AND the evaluator's feedback to the query:
```
Original query: "What is the capital of France?"
Previous answer: "Lyon is the capital of France."
Feedback: "The answer is factually incorrect."
→ Retry query includes all three, giving the LLM maximum signal for self-correction.
```

### Applicability to Nexor

**Medium.** Most relevant for:
- Workflow steps that involve search/retrieval (sub-query decomposition)
- The mode resolver (HyDE-style: generate a hypothetical mode description, then match)
- Retry logic (feedback query transformation when steps fail)

---

## 10. Evaluation and Scoring (LangChain + LlamaIndex)

### LLM-as-Judge with Debiasing (LlamaIndex)

**Pairwise comparison with position debiasing**: Run the comparison twice with outputs A and B swapped. Use a voting system:
- If both orderings agree → clear winner
- If they disagree → tie

This eliminates the well-documented positional bias in LLM evaluators (tendency to prefer the first or second option).

### Criteria-Based Evaluation (LangChain)

14 built-in criteria (conciseness, relevance, correctness, coherence, etc.) with step-by-step reasoning prompts. The prompt explicitly instructs: "First, write out in a step by step manner your reasoning about each criterion to be sure that your conclusion is correct. Avoid simply stating the correct answers at the outset."

### Faithfulness Evaluation (LlamaIndex)

Decompose the answer into individual claims, then verify each claim against the source context:
```
Answer: "Paris is the capital of France and has 2.1M people"
→ Claim 1: "Paris is the capital of France" → Supported ✓
→ Claim 2: "Paris has 2.1M people" → Not found in context ✗
→ Faithfulness score: 0.5
```

### Applicability to Nexor

**High.** Automated evaluation could:
- Score step outputs after execution (is the output faithful to the input context?)
- Compare outputs across workflow runs (pairwise with debiasing)
- Feed scores into the feedback distillation system (close the loop)
- Identify consistently low-scoring agents/steps for human review

---

## Synthesis: Recommended Techniques for Nexor

Ranked by impact-to-effort ratio:

### Tier 1 -- Low Effort, High Impact

| Technique | Source | Implementation |
|-----------|--------|----------------|
| Partial JSON recovery | LangChain | Add to execution engine's output parser |
| Negative examples in schema instructions | LangChain | Modify schema injection in system prompt builder |
| Reason-before-answer in structured outputs | AutoGen | Update output schema patterns to include `reasoning` field |
| Validation-retry with error context | Haystack | Add retry loop to steps with `output_schema_id` |
| Post-execution reflection | AutoGen | Add synthesis step after multi-round tool use |

### Tier 2 -- Medium Effort, High Impact

| Technique | Source | Implementation |
|-----------|--------|----------------|
| Human feedback distillation | CrewAI | New `agent_guidances` table, inject in `compose_prompt()` |
| Self-critique step type | LangChain | New workflow step mode that runs critique-revision loop |
| Stall detection in DAG execution | AutoGen | Progress evaluation after each step, replan on stall |
| Automated quality scoring | LangChain/LlamaIndex | LLM-as-judge after step execution, store scores |
| Few-shot from successful traces | DSPy | Store successful execution traces, inject as examples |

### Tier 3 -- High Effort, Transformative Impact

| Technique | Source | Implementation |
|-----------|--------|----------------|
| Bayesian prompt optimization (MIPROv2) | DSPy | Requires training dataset + metrics, Optuna integration |
| Filter/middleware pipeline | Semantic Kernel | Refactor prompt composition into interceptable pipeline |
| Society of Mind step type | AutoGen | Sub-workflow execution within a single step |
| Iterative instruction optimization (COPRO) | DSPy | LLM-based system prompt refinement with scoring |
| Cross-step blame attribution (Refine) | DSPy | Trace failures back through DAG to identify root cause step |

---

## Conclusion

The open source AI framework ecosystem has converged on a common set of techniques for improving LLM output quality. These techniques fall into five categories:

1. **Prompt augmentation**: Injecting distilled feedback, few-shot examples, or context into prompts (CrewAI, DSPy, AutoGen)
2. **Output validation and retry**: Parsing recovery, schema validation with error-context retries, structured self-assessment (LangChain, Haystack, LlamaIndex)
3. **Self-critique loops**: Having the LLM evaluate and revise its own output against principles or metrics (LangChain Constitutional AI, DSPy Refine)
4. **Orchestration intelligence**: Stall detection, progress monitoring, automatic replanning, blame attribution (AutoGen Magentic-One, DSPy)
5. **Automated optimization**: Treating prompts as searchable parameter spaces, using metrics to guide the search (DSPy MIPROv2, COPRO)

None of these techniques require model fine-tuning. They all operate at the prompt and orchestration layer, making them implementable in any LLM application framework -- including Nexor.

The most important insight across all frameworks: **the difference between a mediocre AI system and a good one is rarely the model -- it's the scaffolding around the model.** Better prompts, smarter retries, self-critique loops, and feedback accumulation compound to produce dramatically better outputs from the same underlying LLM.

---

*Research conducted February 2026. Source frameworks: CrewAI v0.x, LangChain v0.3.x, LangGraph v0.2.x, DSPy v2.6.x, AutoGen v0.4.x, Semantic Kernel v1.x, Haystack v2.x, LlamaIndex v0.12.x.*
