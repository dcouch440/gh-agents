# Agent Passdown & Review Node — Self-Reporting Agents with Quality Gates

## Context

The [Step Activity Stream](./step-activity-stream.md) ticket introduces programmatic milestones — cheap, no-LLM status messages like "Thinking..." and "Running tool: github_search." These tell the user *what's happening* but not *what the agent found* or *how it went*.

This ticket introduces two connected concepts that build on that foundation:

1. **Passdown**: A small follow-up LLM call after an agent completes its main work, where the agent reflects on what it did and produces a 2-3 sentence handoff summary. This serves triple duty: real-time status for the viewer, structured context for downstream agents, and a self-assessment the system can act on.

2. **Review Node**: A new node type that doesn't produce output — it evaluates upstream work. It loads the upstream agent's full message history (not just final output), runs with a review-specialized system prompt, and issues a verdict: pass or retry. On retry, the review node re-invokes the upstream agent with feedback injected, and doesn't mark itself complete until satisfied. No DAG cycles — the review node simply holds until its quality gate is met.

**Key decisions:**
- The passdown is a separate LLM call AFTER the main execution, not baked into the primary prompt. This keeps the main output schema clean and the passdown format consistent regardless of the step's output schema.
- The passdown uses the same agent, same message history, with one additional user message appended: "Provide a brief passdown." This means the agent has full context of what it just did.
- The review node loads `execution_messages` from the upstream agent's execution — it sees every round, every tool call, every response. This is fundamentally richer than reviewing just the final output JSON.
- The review node's retry loop lives INSIDE the node's execution. From the DAG's perspective, the review node is just a step that takes longer to complete. No back-edges, no topological sort changes.
- Passdowns are persisted and broadcast through the same activity stream infrastructure from the milestone ticket.

**Prerequisites:** [Step Activity Stream](./step-activity-stream.md) Parts 1-3 (event type, emitter, frontend store).

---

## Part 1: Passdown Mechanic — Backend

> **Risk:** LOW — Additive. One extra LLM call after step completion. No changes to existing execution flow.
> **Effort:** Medium
> **Dependencies:** Step Activity Stream Part 1 (event type + emitter)

### Problem

After a step completes, the only output is the structured result (JSON conforming to the output schema). There's no human-readable summary of what the agent did, what it found important, or what downstream agents should know. The programmatic milestones from the activity stream ticket help during execution, but at completion the viewer just sees "Completed (1,200 tokens, 3.2s)" — no substance.

### 1A. Passdown execution flow

**File:** `src/server/hub/dag/mod.rs` — after `engine.execute()` returns in `execute_single_step()` (around line 986)

After the main execution completes successfully, make one additional LLM call:

```
execute_single_step() {
    // ... existing execution ...
    let (output, in_tok, out_tok, cost) = result?;

    // === NEW: Passdown call ===
    let passdown = generate_passdown(engine, state, ctx, step, &agent, agent_execution_id).await;

    // ... existing StepCompleted broadcast (now includes passdown) ...
}
```

The passdown call:
1. Loads the `execution_messages` for the just-completed `agent_execution_id` — this is the agent's full message history from the execution that just finished
2. Reconstructs the message list (`Vec<Message>`) from those rows
3. Appends one new user message with the passdown prompt
4. Calls the LLM with the same model, a lightweight system prompt override, and NO tools (pure text response)
5. Returns a `String` — the 2-3 sentence passdown

### 1B. Passdown prompt

The passdown prompt is intentionally minimal. It does NOT use the agent's original system prompt (which may include schema enforcement, tool instructions, etc.). Instead it uses a short, universal passdown system prompt:

```
You are providing a brief passdown summary of the work you just completed.
Summarize in 2-3 sentences: what you did, what you found, and anything
the next person or agent should know. Be specific and concise.
Do not repeat your full output — just the key takeaways.
```

The user message appended to the existing history:

```
Provide your passdown.
```

Because the full message history is loaded, the agent has complete context — it sees its own system prompt, every tool call result, every response it gave. The passdown prompt just asks it to reflect.

### 1C. Passdown generation function

**File:** `src/server/hub/dag/passdown.rs` (new module)

```rust
/// Generate a passdown summary for a completed step execution.
///
/// Loads the agent's execution message history, appends a passdown request,
/// and makes a single LLM call with no tools. Returns None if the call fails
/// (passdown is best-effort, never blocks execution).
pub async fn generate_passdown(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    agent: &AgentRow,
    agent_execution_id: Uuid,
) -> Option<String> {
    // 1. Load execution_messages for this agent_execution_id
    // 2. Reconstruct Vec<Message> from rows (role + content)
    // 3. Append user message: "Provide your passdown."
    // 4. Build LLMRequest with:
    //    - passdown system prompt (not the agent's original)
    //    - same model as the agent
    //    - no tools
    //    - max_tokens: 200 (enforce brevity)
    //    - temperature: 0.3 (focused, not creative)
    // 5. Call provider.send_message() directly (not engine.execute() — no filters, no strategy, no tools)
    // 6. Return Some(content) on success, None on failure
    // 7. Log errors but never propagate — passdown failure must not block the DAG
}
```

**Important:** The passdown call uses `provider.send_message()` directly, NOT `engine.execute()`. This skips filters, strategies, tool execution, and the full engine loop. It's a single stateless LLM call. This keeps it fast and avoids filter side effects (e.g., the reasoning filter would try to wrap the passdown in `{reasoning, result}` — we don't want that).

**Token budget:** `max_tokens: 200` enforces brevity. At ~0.75 words per token, that's ~150 words — plenty for 2-3 sentences. The input tokens are the message history (already paid for) plus the short passdown prompt.

**Cost consideration:** For a step that used 1,200 output tokens, the passdown adds ~200 output tokens and re-sends the message history as input. The input cost is marginal (cached in most providers). The output cost is ~15% of the original step. For steps with heavy tool use (large histories), the input cost is higher — but the passdown is also more valuable because there's more to summarize.

### 1D. Passdown persistence

Two storage locations:

**1. `step_activity_log` table** (from the milestone ticket):
The passdown is persisted as a milestone event with a new milestone type `passdown`:

```rust
emitter.emit(step, agent_name, StepMilestone::Passdown, passdown_text, None).await;
```

This extends the `StepMilestone` enum from the activity stream ticket:

```rust
pub enum StepMilestone {
    Preparing,
    Thinking,
    Acting,
    Decided,
    Passdown,  // NEW — agent's self-reported summary
}
```

**2. `agent_executions.passdown` column** (new column on existing table):

```sql
ALTER TABLE agent_executions ADD COLUMN passdown TEXT;
```

This stores the passdown alongside the execution record so it can be loaded by downstream agents and review nodes without querying the activity log.

### 1E. Passdown in StepCompleted broadcast

Extend the existing `WorkflowEventKind::StepCompleted` variant:

```rust
StepCompleted {
    step_id: Uuid,
    step_name: String,
    agent_id: Option<Uuid>,
    output: Option<String>,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    duration_ms: Option<u64>,
    passdown: Option<String>,  // NEW
}
```

The frontend receives the passdown as part of the completion event. This means the timeline entry can show the passdown inline — the step doesn't just say "Completed (1,200 tokens)" but "Completed — Found 3 authentication vulnerabilities in auth.rs, the most critical is the missing token expiry check."

### 1F. Opt-in per step

Not every step needs a passdown. Entry steps, document steps, and simple transform steps don't benefit from self-reflection. Add a column to `workflow_steps`:

```sql
ALTER TABLE workflow_steps ADD COLUMN passdown_enabled BOOLEAN NOT NULL DEFAULT false;
```

The passdown generation function checks `step.passdown_enabled` before making the call. Steps that don't have it enabled skip the passdown entirely — zero extra cost.

For review nodes (Part 3+), `passdown_enabled` defaults to true on all upstream steps since the review node relies on passdowns for its assessment.

### Tests

- Unit test: `generate_passdown()` constructs correct message list from `ExecutionMessageRow` records
- Unit test: Passdown respects `max_tokens: 200` in the LLM request
- Unit test: Passdown failure (LLM error) returns `None` and does not propagate error
- Unit test: `passdown_enabled = false` skips the LLM call entirely
- Integration test: Full step execution with passdown — verify passdown stored in `agent_executions.passdown` and broadcast via `StepCompleted`
- Unit test: `StepMilestone::Passdown` serializes correctly

---

## Part 2: Passdown in the Frontend

> **Risk:** LOW — Extending existing timeline components with one new field.
> **Effort:** Small
> **Dependencies:** Part 1, Step Activity Stream Part 3

### 2A. Store update

**File:** `frontend/src/stores/workflowExecutionStore.ts`

Extend `StepExecutionState`:

```typescript
passdown: string | null  // Agent's self-reported summary
```

In the `step_completed` handler, extract `passdown` from the event data:

```typescript
case WORKFLOW_EVENT.STEP_COMPLETED: {
    const { step_id, passdown, ...rest } = data
    const step = state.stepStates[step_id]
    if (step) {
        step.status = 'success'
        step.passdown = passdown ?? null
        // ... existing field updates ...
    }
    break
}
```

### 2B. Timeline entry — passdown display

**File:** `frontend/src/components/panels/execution/ExecutionTimelineEntry.tsx`

When a step is in `success` status and `passdown` is non-null, show it as a summary line below the completion metrics:

```
[green check] Code Reviewer  ·  Completed (1,200 tokens, 3.2s)
              Found 3 authentication vulnerabilities in auth.rs.
              The most critical is the missing token expiry check.
              JWT implementation needs revision before merge.
```

Style: normal weight, slightly muted color, wraps naturally. Distinct from the metrics line above it.

### 2C. Activity stream integration

**File:** `frontend/src/stores/workflowExecutionStore.ts`

The `step_activity` event with `milestone: "passdown"` gets appended to `activityStream` like any other milestone. In the full-screen activity stream, passdowns are visually distinguished (slightly bolder, or with a subtle left border) since they carry more substance than "Thinking..."

### Tests

- Store test: `step_completed` with `passdown` field populates `stepStates[id].passdown`
- Store test: `step_completed` without `passdown` field sets `passdown` to `null`
- Component test: Timeline entry renders passdown text when present
- Component test: Timeline entry hides passdown area when null

---

## Part 3: Review Node — Backend Foundation

> **Risk:** MEDIUM — New execution mode. Introduces a new step type with different lifecycle semantics.
> **Effort:** Large
> **Dependencies:** Part 1 (passdown), Step Activity Stream (milestones for status during review)

### Problem

Currently, every step in the DAG produces output and advances. There's no quality gate. If an agent produces poor output, the only feedback loop is a human reviewing the results after the entire workflow completes. A review node would catch quality issues in-flight and request corrections before the rest of the DAG continues.

### 3A. New execution mode: `review`

The review node is a new `execution_mode` value in `workflow_steps`:

```
execution_mode = "review"
```

It sits downstream of one or more steps. Its edges connect it to the steps it reviews (upstream) and to any steps that should wait for review approval (downstream).

**Key properties that distinguish it from other step types:**
- It does NOT have an `output_schema_id` — it doesn't produce structured output for downstream consumption
- It does NOT have an `output_variable_name` — it doesn't contribute to `var_outputs`
- Its agent has a review-specialized system prompt
- It loads upstream agents' full execution history, not just their output
- It can re-invoke upstream agents on failure (retry loop)

### 3B. Review node configuration

New columns on `workflow_steps` for review mode:

```sql
ALTER TABLE workflow_steps ADD COLUMN review_max_retries INT NOT NULL DEFAULT 2;
```

`review_max_retries`: Maximum times the review node will ask an upstream agent to retry before accepting (fail-safe to prevent infinite loops). Default 2 means: original execution + 2 retries = 3 total attempts max.

### 3C. Review node data model — what it receives

When the review node executes, it needs rich context from each upstream step. For each upstream step, load:

1. **Passdown** (`agent_executions.passdown`) — the agent's own summary
2. **Structured output** (`agent_executions.structured_output`) — the actual result
3. **Full message history** (`execution_messages` rows for the `agent_execution_id`) — every round of the agent's execution including tool calls and results

These are assembled into a review context block that becomes the review agent's input.

### 3D. Review context assembly

**File:** `src/server/hub/dag/review.rs` (new module)

```rust
/// Assembled context for one upstream step being reviewed.
pub struct ReviewTarget {
    pub step_id: Uuid,
    pub step_name: String,
    pub agent_name: String,
    pub passdown: Option<String>,
    pub structured_output: Option<serde_json::Value>,
    pub execution_messages: Vec<ExecutionMessageRow>,
    pub retry_count: u32,           // How many times this step has been retried
    pub prior_feedback: Vec<String>, // Previous review feedback (if retrying)
}
```

The review context is formatted into the review agent's prompt as structured sections:

```
## Agent Under Review: Code Reviewer

### Passdown
Found 3 authentication vulnerabilities in auth.rs. The most critical is
the missing token expiry check. JWT implementation needs revision.

### Output
{structured_output JSON}

### Execution History
[Round 1] System: You are a code reviewer...
[Round 1] User: Review the following PR...
[Round 1] Assistant: Let me search for the relevant files...
[Round 1] Tool Call: github_search({...})
[Round 1] Tool Result: [file list]
[Round 2] Assistant: I found several issues...
...

### Prior Review Feedback (Retry 1 of 2)
The review identified missing coverage of the JWT expiry validation.
Please focus on token lifecycle management.
```

For multi-input review nodes (multiple upstream steps), each upstream step gets its own section.

### 3E. Review agent output schema

The review agent produces a structured verdict:

```json
{
  "verdict": "pass" | "retry",
  "summary": "Brief explanation of the verdict",
  "feedback": "Specific instructions for the upstream agent (only if retry)",
  "targets": ["step_id_1"]  // Which upstream steps need retry (only if retry)
}
```

This is enforced via the standard output schema mechanism — the review node has an `output_schema_id` pointing to a schema with this shape. (Correction to 3A: the review node DOES have an output schema, but it's the review verdict schema, not a domain output schema.)

### 3F. Review execution flow

**File:** `src/server/hub/dag/mod.rs` — new branch in the step execution dispatch

```rust
} else if step.execution_mode == "review" {
    execute_review_step(
        effective_engine,
        state,
        ctx,
        step,
        &agent,
        steps,
        edges,
        &mut var_outputs,
        &mut completed,
        &mut completed_envelopes,
        &port_meta,
        &mut total_input_tokens,
        &mut total_output_tokens,
        &mut total_cost_usd,
        cancel,
    )
    .await
}
```

### 3G. `execute_review_step()` function

**File:** `src/server/hub/dag/review.rs`

```rust
pub async fn execute_review_step(...) -> Result<(), HubError> {
    let max_retries = step.review_max_retries.unwrap_or(2);

    // 1. Identify upstream steps (parents in the DAG)
    let upstream_step_ids = get_parent_steps(step.id, edges);

    // 2. Load review targets from completed executions
    let mut targets = load_review_targets(state, &upstream_step_ids, completed, completed_envelopes).await?;

    // 3. Emit milestone: preparing
    emitter.emit(step, agent_name, Preparing, format!(
        "Reviewing {} upstream agent(s): {}",
        targets.len(),
        target_names.join(", ")
    )).await;

    for retry_round in 0..=max_retries {
        // 4. Assemble review context into prompt
        let review_prompt = assemble_review_prompt(&targets);

        // 5. Emit milestone: thinking
        emitter.emit(step, agent_name, Thinking,
            if retry_round == 0 { "Evaluating upstream work...".into() }
            else { format!("Re-evaluating after retry {} of {}...", retry_round, max_retries) }
        ).await;

        // 6. Execute review agent via engine (standard execution with schema)
        let result = run_review_via_engine(engine, state, ctx, step, agent, &review_prompt, cancel).await?;

        // 7. Parse verdict
        let verdict: ReviewVerdict = parse_verdict(&result)?;

        if verdict.verdict == "pass" {
            // 8a. PASS — emit milestone + mark complete
            emitter.emit(step, agent_name, Decided, format!(
                "Approved: {}", verdict.summary
            )).await;
            // Store review result, mark step complete
            break;
        }

        if retry_round == max_retries {
            // 8b. MAX RETRIES — accept with warning
            emitter.emit(step, agent_name, Decided, format!(
                "Accepted after {} retries (max reached): {}", max_retries, verdict.summary
            )).await;
            break;
        }

        // 8c. RETRY — re-invoke upstream agent(s)
        for target_step_id in &verdict.targets {
            emitter.emit(step, agent_name, Decided, format!(
                "Requesting retry for {}: {}", target_step_name, verdict.feedback
            )).await;

            // 9. Re-execute the upstream step with feedback
            let retry_result = retry_upstream_step(
                engine, state, ctx,
                target_step_id,
                &verdict.feedback,
                steps, edges, &mut var_outputs,
                &mut completed, &mut completed_envelopes,
                &port_meta,
                total_input_tokens, total_output_tokens, total_cost_usd,
                cancel,
            ).await?;

            // 10. Update the review target with new execution data
            update_review_target(&mut targets, target_step_id, &retry_result);
        }
    }

    Ok(())
}
```

### 3H. Upstream agent re-invocation

**File:** `src/server/hub/dag/review.rs`

```rust
/// Re-execute an upstream step with review feedback injected.
///
/// Loads the upstream agent's original execution context (system prompt,
/// resolved prompt, tools) and re-runs it with the review feedback appended
/// as additional context in the user prompt.
async fn retry_upstream_step(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    target_step_id: &Uuid,
    feedback: &str,
    // ... standard DAG context ...
) -> Result<StepOutput, HubError> {
    // 1. Load the original step + agent
    // 2. Re-resolve port inputs (upstream data hasn't changed)
    // 3. Re-compose the prompt
    // 4. APPEND review feedback to the prompt:
    //    "\n\n<review_feedback>\nA reviewer has requested changes:\n{feedback}\n
    //     Please address this feedback in your revised response.\n</review_feedback>"
    // 5. Create new agent_execution record (linked to same step, new execution)
    // 6. Execute via engine (full execution with tools, filters, etc.)
    // 7. Generate passdown for the retry
    // 8. Update completed/completed_envelopes with new output (replacing old)
    // 9. Return new StepOutput
}
```

**Critical:** The retry replaces the old output in `completed` and `completed_envelopes`. The old output is not lost — it's still in the `agent_executions` table with its own ID. But downstream steps (after the review node) will see only the final approved output.

**The DAG doesn't know retries happened.** From the main loop's perspective, the review step was executed and completed. The fact that it internally re-ran upstream agents is an implementation detail. The `completed` map has the latest outputs, and the DAG continues.

### Tests

- Unit test: `assemble_review_prompt()` produces correct sections for single and multi-target reviews
- Unit test: Verdict parsing handles `pass` and `retry` verdicts
- Unit test: Verdict with `retry` targeting non-existent step returns error
- Unit test: `review_max_retries` is respected — loop exits after max
- Integration test: Review node with `pass` verdict — upstream step not re-executed
- Integration test: Review node with `retry` → `pass` — upstream step re-executed once, review runs twice
- Integration test: Review node with repeated `retry` hitting max_retries — accepts and continues
- Integration test: Retry replaces output in `completed` and `completed_envelopes`
- Integration test: Passdown generated for retry execution

---

## Part 4: Review Node — Frontend

> **Risk:** LOW — New node type in canvas + timeline display for review events.
> **Effort:** Medium
> **Dependencies:** Part 3, Step Activity Stream Parts 3-4

### 4A. Canvas node type

**File:** `frontend/src/components/canvas/nodes/` — new `ReviewNode.tsx`

A visually distinct node type on the workflow canvas:
- Different shape or color to indicate it's a quality gate (e.g., shield icon, amber/gold accent)
- Shows configuration: max retries, which upstream steps it reviews
- Connected to upstream steps via standard edges

### 4B. Execution timeline for review steps

The review node's timeline entry shows its verdict and retry history:

```
[amber shield] Code Review Gate
               Reviewing: Code Analyzer, Planner
               Evaluating upstream work...
```

On retry:
```
[amber shield] Code Review Gate
               Requesting retry for Code Analyzer: Missing JWT expiry validation
```

After upstream agent re-executes:
```
[amber shield] Code Review Gate
               Re-evaluating after retry 1 of 2...
```

On pass:
```
[green shield] Code Review Gate
               Approved: All authentication concerns addressed
```

### 4C. Upstream step retry indication

When the review node triggers a retry, the upstream step's timeline entry should reflect this:

```
[blue pulse] Code Analyzer  ·  Retry 1 — Addressing review feedback
```

This means the `StepExecutionState` needs to track retry state:

```typescript
// Add to StepExecutionState:
retryCount: number      // How many times this step has been retried
retryReason: string | null  // Latest review feedback triggering retry
```

New event variant or extension to handle this:

```typescript
// Extend StepStarted data to include:
is_retry: boolean
retry_count: number
retry_reason: string | null
```

### 4D. Activity stream entries for review

Review milestones appear in the activity stream alongside normal step milestones:

```
12:00:08  Code Analyzer    · Completed — Analyzed 3 files, found 2 issues
12:00:09  Review Gate      · Reviewing Code Analyzer
12:00:10  Review Gate      · Requesting retry: Missing JWT expiry validation
12:00:10  Code Analyzer    · Retry 1 — Addressing review feedback
12:00:11  Code Analyzer    · Thinking...
12:00:14  Code Analyzer    · Completed — Added JWT expiry check, revised auth flow
12:00:15  Review Gate      · Re-evaluating after retry 1 of 2...
12:00:17  Review Gate      · Approved: All authentication concerns addressed
12:00:17  Summarizer       · Resolving inputs from Code Analyzer, Planner
```

The user sees the full review conversation play out in real time.

### Tests

- Component test: ReviewNode renders with shield icon and correct connections
- Component test: Timeline entry shows review verdict states
- Store test: Retry events update `retryCount` and `retryReason` on upstream step
- Integration test: Full review flow in activity stream — correct chronological order

---

## Part 5: Multi-Input Review

> **Risk:** LOW — Extension of single-input review to handle multiple upstream steps.
> **Effort:** Small (most infrastructure built in Part 3)
> **Dependencies:** Part 3

### 5A. Multiple upstream steps

A review node can have edges from multiple upstream steps. The review agent receives context from ALL of them in a single prompt:

```
## Agent Under Review: Code Analyzer
### Passdown
Found 2 critical bugs in the authentication module...
### Output
{...}

## Agent Under Review: Test Writer
### Passdown
Generated 15 unit tests covering the auth flow...
### Output
{...}
```

The review agent evaluates holistically: "The Code Analyzer found 2 auth bugs, but the Test Writer's tests don't cover the JWT expiry case. The test coverage is insufficient."

### 5B. Selective retry

The review verdict's `targets` array specifies WHICH upstream steps need retry. The review might pass one agent and retry another:

```json
{
  "verdict": "retry",
  "summary": "Code analysis is thorough but tests are incomplete",
  "feedback": "Add test coverage for the JWT token expiry validation identified by Code Analyzer",
  "targets": ["test_writer_step_id"]
}
```

Only the Test Writer gets re-invoked. The Code Analyzer's output is preserved.

### 5C. Cross-agent context in retry

When the Test Writer is retried, its prompt includes:
1. Original prompt (re-composed)
2. Review feedback
3. **Passdowns from sibling agents** — so the Test Writer knows what the Code Analyzer found

This is assembled by `retry_upstream_step()`:

```
<review_feedback>
A reviewer has requested changes:
Add test coverage for the JWT token expiry validation identified by Code Analyzer.

Context from sibling agents:
- Code Analyzer: Found 2 critical bugs in the authentication module. The JWT expiry
  check is missing entirely, and the signing key rotation is not thread-safe.
</review_feedback>
```

### Tests

- Integration test: Multi-input review with 2 upstream steps — both contexts in review prompt
- Integration test: Selective retry — retry one target, other's output preserved
- Integration test: Cross-agent passdown included in retry prompt

---

## Edge Cases

| Case | Handling |
|------|----------|
| **Passdown LLM call fails** | Returns `None`. Passdown is best-effort. Step still completes successfully. Review node works without passdown (just has less context). |
| **Passdown on non-LLM steps** | Entry steps and document steps skip passdown entirely (no agent, no message history). `passdown_enabled` defaults to false. |
| **Review node with no upstream passdowns** | Review still works — it has structured output and full message history. Passdown is bonus context, not required. |
| **Review retry on a for-each step** | The entire for-each re-executes. This is expensive but correct — the review applies to the aggregate output. Future optimization: selective item retry. |
| **Review retry on a room step** | The room re-executes with feedback injected into the room's system context. All speakers re-debate. |
| **Review retry on a step that has its own downstream steps** | Only the reviewed step re-executes. Its downstream steps (other than the review node) are NOT re-triggered. The review node holds the DAG — downstream steps beyond the review node haven't started yet. |
| **Max retries exhausted** | Review node accepts the latest output with a warning logged. The DAG continues. Activity stream shows "Accepted after 2 retries (max reached)". |
| **Chained review nodes** | Review Node A reviews Step 1, Review Node B reviews Review Node A. Technically possible but unusual. Works because the review node's verdict IS its output. |
| **Review node as only downstream step** | Valid. The review node is a terminal quality gate. It completes, the DAG completes. |
| **Concurrent steps feeding into one review node** | Both must complete before the review node's dependencies are met. Standard DAG behavior — review waits for all upstream. |
| **Cancelled workflow during retry** | Cancellation token is passed through to retry execution. Retry respects cancellation. |
| **Large message histories** | For agents with 10+ tool rounds, the message history can be large. The review prompt may need truncation or summarization for very long histories. Part 3's initial implementation passes full history; optimization can truncate tool results to first N characters if context budget is exceeded. |
| **Passdown token cost** | ~200 output tokens per passdown. For a 10-step workflow with passdowns enabled on all steps, that's ~2,000 extra output tokens total. Input tokens are cached/reused. Cost is marginal. |

---

## Implementation Order

| Part | Ships Independently? | What You Get |
|------|---------------------|--------------|
| **Part 1** | Yes | Agents produce passdown summaries. Stored + broadcast. No review yet. |
| **Part 2** | Yes (with Part 1) | Passdowns visible in timeline and activity stream. Immediate UX improvement. |
| **Part 3** | Yes (with Parts 1-2) | Review nodes work. Single upstream step review with retry loop. Core feature complete. |
| **Part 4** | Yes (with Part 3) | Review nodes visible in canvas + timeline. Full user-facing feature. |
| **Part 5** | Yes (with Parts 3-4) | Multi-input review with selective retry and cross-agent context. Advanced feature. |

After Part 2, every agent step has a rich completion summary. After Part 3, workflows have quality gates. Parts 4-5 are polish and power features.

---

## Files Changed (Summary)

**Backend — New:**
- `migrations/0021_passdown_and_review.sql` (passdown column, review_max_retries column)
- `src/server/hub/dag/passdown.rs` (passdown generation)
- `src/server/hub/dag/passdown/tests.rs`
- `src/server/hub/dag/review.rs` (review node execution, retry logic, context assembly)
- `src/server/hub/dag/review/tests.rs`

**Backend — Modified:**
- `src/server/ws/events.rs` (StepMilestone::Passdown, passdown field on StepCompleted)
- `src/db/mod.rs` (AgentExecutionRow.passdown, WorkflowStepRow.review_max_retries + passdown_enabled)
- `src/db/traits/mod.rs` (new repo methods for review context loading)
- `src/db/pg_repo/mod.rs` (implementations)
- `src/server/hub/dag/mod.rs` (passdown call in execute_single_step, review mode dispatch in main loop)

**Frontend — New:**
- `frontend/src/components/canvas/nodes/ReviewNode.tsx`

**Frontend — Modified:**
- `frontend/src/types/ws.ts` (StepMilestone.passdown, passdown on StepCompleted, retry fields)
- `frontend/src/stores/workflowExecutionStore.ts` (passdown + retry state)
- `frontend/src/components/panels/execution/ExecutionTimelineEntry.tsx` (passdown display, retry indication)
- `frontend/src/components/panels/execution/ActivityStream.tsx` (passdown + review styling)
