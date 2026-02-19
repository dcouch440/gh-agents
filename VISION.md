# VISION.md — Context Intelligence System

## The Big Idea

Nexor workflows are not just execution pipelines — they are living workbenches where users design, iterate, and harden AI agent teams through conversation. The board is both the design surface and the runtime. This vision introduces two interlocking concepts that make the board intelligent: **Pinned Nodes** and a **Run Results Summarizer**.

The existing context systems — board overview, chat beliefs, graph context — already handle conversational awareness. What's missing is the assistant's understanding of what nodes actually produce when they run, and the ability to pin nodes that are "done."

---

## Workshop Mode

Today, the node assistant helps users configure steps through conversation. The board is already a workshop — users talk to assistants, configure nodes, run them, review results, iterate. This is the natural loop:

1. User talks to a node's assistant about what they want
2. Assistant configures the node (agents, prompts, tools, ports)
3. User runs the node (or the whole workflow) and sees results
4. User iterates, adjusts, re-runs
5. When satisfied, the user pins the node — freezing its output

The key insight: **the board is not a static blueprint you configure and then run. It's a workshop where you shape agents live, see their results, and progressively lock in the parts that work.**

What's missing to close this loop:

- **Run results awareness** — After a node executes, the assistant receives a summarized view of what happened (via the run results summarizer). Today the assistant is blind after dispatch; it configures the node but never learns what the execution produced.
- **Pin/unpin tools** — The assistant needs tools to pin and unpin nodes on behalf of the user. "That looks good, pin it" should be a natural conversational action.
- **Cross-node data awareness** — Through run results summaries, the assistant understands not just its own node but what neighboring nodes produce. This lets it make informed suggestions: "Based on what the research node outputs, you'll want to adjust the prompt here."

Workshop mode is not a separate UI mode or toggle. It's the natural state of working on the board.

---

## Pinned Nodes

A pinned node is one whose output is frozen. When the workflow runs, a pinned node does not re-execute — it passes through its last execution output as if it had just run.

### What Pinning Is

**Pin is a per-node toggle that means: "skip execution, replay last output."**

That's it. No separate frozen envelope storage, no cascade state management, no origin markers. The pin points at the node's latest execution result and tells the executor to use it instead of re-running.

- **Pin** = toggle on. Node replays its last output during workflow runs.
- **Unpin** = toggle off. Node executes normally on the next run.
- **Re-run a pinned node directly** = the node executes, its output updates, and the pin now points at the new result. The pin stays active.
- **Re-pin** = just re-run the pinned node. The output updates automatically.

### Prerequisites

A node **must have at least one successful execution** before it can be pinned — there's no output to replay without a run. Exception: `context` and `input` nodes, whose output is their configured content and don't need a prior execution.

### Why Pinning Matters

Pinned nodes become **free context sources**. You workshop five different agents, pin them, and now you have a library of pre-computed outputs that flow through the graph at zero execution cost. The only cost is the initial design work. After that, it's just data routing.

The INPUT node becomes the one variable — swap in different inputs and watch how the dynamic nodes process them against all that pinned context.

Pinning makes the input boundary movable. Traditional workflows assume data enters at the top and flows down. With pinned nodes, any node can be frozen, meaning the "live" part of the workflow can start anywhere:

- **Pinned prefix, dynamic suffix** — The first three nodes are workshopped and pinned. The actual function starts at node four with a dynamic input. Upstream pinned nodes are pre-computed context that flows through for free.
- **Pinned research fork** — One parallel path is a pinned research node with curated results. Another path is a live workforce. They merge downstream and the dynamic node gets both the fixed dataset and the fresh output.
- **Pinned agents, dynamic inputs** — The workforce team design is pinned (workshopped and validated), but the task input changes every run. Same team, different problems.
- **Progressive hardening** — The user runs everything dynamically first, reviews results, then starts pinning nodes as they're satisfied. The workflow gradually solidifies from a fully dynamic experiment into a mostly-frozen pipeline with a few dynamic entry points.

### How the DAG Executor Handles Pinned Nodes

During the topological walk, when the executor encounters a pinned node:
1. Skip execution entirely
2. Inject the node's last execution output into `DagExecutionState.envelopes`
3. Mark the step as completed
4. Continue to downstream nodes

Downstream nodes consume the pinned output through normal port resolution — they can't tell the difference between a live result and a pinned one. The data flow is the same.

### Dead-Path Elimination

When a pinned node makes upstream execution pointless, the executor skips those nodes automatically. This is a **runtime optimization**, not user-facing state.

If A → B → C and C is pinned, then A and B's output would go nowhere — C replays its frozen result regardless. The executor detects this during the topological walk and skips A and B. No cascade flags, no auto-pin propagation, no origin markers. The executor simply asks: "does this node have any unpinned downstream consumers?" If not, skip it.

```
A → B → C (pinned)
         ↓
Executor skips A and B at runtime (their output has no consumer)

A → C (pinned)
A → D (dynamic)
         ↓
Executor runs A (D needs it). C replays frozen output.
A's output flows to D. The edge A → C is effectively dead.
```

On the board, during execution, skipped nodes can be visually dimmed — "not needed this run" — but they are NOT pinned. Only nodes the user explicitly pinned show the pin indicator.

### User Scenarios

| Scenario | What happens |
|---|---|
| **Happy path** | Workshop node, run it, like the result → pin it. Move on. |
| **Iterate on a pinned node** | Re-run the node directly. New output replaces old. Pin stays active with new result. |
| **Upstream change** | Edit upstream node A. Pinned downstream C keeps its frozen output. Re-run C if you want it updated. |
| **Parallel path** | A → C (pinned), A → D (dynamic). A runs for D. C replays frozen output. Clean. |
| **Test the rest of the graph** | Pin first 3 nodes, run workflow. They pass through (free), rest executes fresh. |
| **Swap inputs** | Pin all agent nodes. Change INPUT. Re-run. All pinned outputs route through for free, only dynamic nodes execute against the new input. |

### Pin-Eligible Node Types

Not all execution modes support pinning in v1. The complexity of freezing output varies significantly by node type:

**v1 — Pin-eligible:**
- **`single`** — One envelope, straightforward. This is the primary use case.
- **`context`** — Static content nodes. These are already conceptually frozen (they just emit their `prompt_template`). Pinnable without a prior execution.
- **`input`** — See INPUT node section below. Pinnable without a prior execution.

**Future — Requires design work:**
- **`workforce`** — Freezing a workforce means capturing the designer phase output + all agent execution results. The frozen output would be the final workforce result JSON. Feasible but the envelope is complex (multiple agents, child workflows).
- **`for_each`** — Would need to freeze N iteration envelopes (one per item). Storage and injection into `DagExecutionState` is straightforward but the frozen output size scales with iteration count.
- **`sub_workflow`** — Freezing a sub-workflow means capturing the entire child execution's output map. Similar to workforce — feasible, but the output is a composite of multiple child steps.
- **`for_each_chain`** — Same as for_each but across chained stages. Most complex case.

**Not pin-eligible:**
- **`room`** — Rooms are inherently interactive and non-deterministic. Freezing a room's output defeats its purpose.
- **`belief_capture`** — Belief extraction is a side-effect operation, not a data producer in the pipeline sense.

### The INPUT Node

The INPUT node (`execution_mode = "input"`) is the declared dynamic entry point on the board. It provides the initial input to the workflow and is where external callers inject data when running the workflow as a function.

Pinned nodes and INPUT nodes work together: pinned nodes provide frozen context, the INPUT node provides fresh data, and dynamic nodes downstream combine both.

**Special cases:**

- **Pinned INPUT node** — An INPUT node can be pinned. This means the workflow runs with a fixed input every time, which is useful for testing or for workflows where the input is a known dataset. When a pinned INPUT is present, external callers don't need to provide input — the frozen value is used. If an external caller provides input anyway, it overrides the pin (the caller's intent takes priority).
- **All nodes pinned** — If every node on the board is pinned, the workflow instantly emits all frozen outputs with no execution. This is a valid state — it represents a fully hardened pipeline where the user has validated every step. It's also useful for testing downstream consumers: "here's exactly what this workflow produces."
- **No INPUT node** — A workflow with all pinned nodes and no INPUT doesn't need an entry point. It's a frozen artifact.

### Pin Storage

Pin state lives on the step row:
- A `pinned` boolean flag on the step
- The pin references the node's latest execution output — no separate frozen envelope storage. The executor reads from the existing execution history.
- Run results summary is stored on the step row (new column), generated post-execution regardless of pin state

No pin history is maintained. Re-running a pinned node overwrites the referenced output.

---

## Run Results Summarizer

The one new summarizer. After a node completes execution, Haiku summarizes what the node produced. This summary is injected into connected nodes' assistant system prompts so they understand the shape of data flowing through the graph.

### What It Does

- **Scope:** Per-node, injected into the node's own assistant AND directly connected downstream nodes (one hop). Upstream nodes do not receive downstream run context — data flows forward, not backward.
- **Purpose:** "Here's what this node produces when it runs" — a forward-looking reference for what the data looks like coming through
- **Trigger:** Fires after a node completes execution (success or failure)
- **Output:** Compact summary of the node's output — shape, content, key data points. Stored on the step row (new column).
- **Injection:** The node's own assistant system prompt (so it can discuss its own results with the user) AND connected downstream nodes' assistant system prompts, both as read-only reference context via `<run_context>`
- **Pattern:** Same `tokio::spawn` fire-and-forget background task pattern as existing board overview and chat beliefs summarizers

### Critical Distinction

The run results summary is NOT something the assistant writes to its notes. It is injected as read-only reference context. The assistant sees it and understands what its own node and upstream nodes produce — the shape of the data, the content, the constraints — without having been told by the user. It uses this as a reference for what is GOING to happen when the workflow runs. It's forward-looking, not retrospective.

A node's own assistant receives its own run results summary so it can participate in post-run conversations. When the user says "that run looked good but the output was missing severity scores," the assistant already knows what the output looked like — it doesn't need the user to explain.

For downstream nodes, this means when a user is workshopping node B, the assistant can say: "Based on what node A produces, you'll want your synthesis agent to expect an array of findings with severity fields" — because node A's run results summary is in B's system prompt.

When the run results change (user tweaks node A, re-runs it), the summary is regenerated and node B's assistant automatically has the updated picture next time the user chats with it.

### What the Summarizer Receives

The run results Haiku call is given:
- The step's `assistant_notes` (if any) — this provides interpretive framing. The notes say what the node is *supposed* to do; the output shows what it *actually did*.
- The step's `execution_mode` — so the summarizer knows what kind of output to expect (single step structured output vs workforce multi-agent result vs for-each array).
- The `StepExecutionEnvelope.data` field — the actual output payload. For large outputs, this is truncated to a token budget before being sent to Haiku.
- The `StepExecutionEnvelope.structured_output` (if present) — the parsed structured output, which is often more useful than raw data for summarization.
- The step's output port definitions — so the summarizer can describe the output in terms of the ports downstream nodes will consume.

The summarizer produces a compact description: what the output contains, what shape it takes, key values or patterns, and anything notable (errors, empty results, unexpected structure). If the node is pinned, the summary includes that fact — so the consuming assistant knows the data is frozen and guaranteed stable, not a live result that might change on the next run.

### Failed Executions

When a node fails, the summarizer still runs. The summary captures the failure: what went wrong, the error message, and any partial output. This is injected as run context so connected node assistants know the upstream is broken rather than showing stale success data. The summary is clearly marked as a failure so the assistant can communicate it: "The research node failed on its last run — it hit a rate limit. You may want to re-run it before iterating here."

If the node has never been run successfully, there's no run context to inject — the connected assistant simply doesn't have a `<run_context>` block for that node.

### Re-summarization Triggers

Run results only need to be re-summarized when:
- The node is re-executed (new results replace old summary)
- The `assistant_notes` change on the source node (the notes provide the interpretive lens for the results — new notes may reframe the summary)

The summary is stable between runs. No scheduled re-summarization.

### Debounce via Cancellation

If a node is re-run while a summarization is already in flight, the in-flight request is cancelled and a new one starts with the latest results. Only the most recent summarization completes and writes to the DB.

The implementation is a per-node cancellation token. Each summarizer checks its token before writing results — if cancelled, it exits silently. The new invocation replaces the token before starting. No wasted writes, no stale results overwriting fresh ones.

### Summarization Status Flags

The assistant needs to know when a summarization is currently in progress. A lightweight in-memory map on `AppState` keyed by `(step_id, "run_results")` with values of `{ status: running | complete, started_at }`. Set to `running` when the summarizer spawns, cleared to `complete` when it writes results.

The system prompt builder checks this map when assembling context. If a connected node's summarization is in flight, a note is injected:

```xml
<summarization_status>
Node "Research Agent" run results summary is being updated (started 3s ago).
Context below may not reflect the latest execution.
</summarization_status>
```

This prevents the assistant from making confident claims about upstream data while a summarization is still in flight.

---

## Context Injection

With the run results summarizer added, the assistant's system prompt now has four context layers (three existing + one new). Here's how they fit together:

### Injection Order

Ordered from most stable to most volatile:

```xml
<!-- 1. Structural — almost never changes during a session -->
<graph_context>
  Workflow nodes:
    - Research Agent (single) [SELECTED]
    - Synthesis (workforce)
  Connections:
    Research Agent -> Synthesis
</graph_context>

<!-- 2. Big picture — changes when notes are updated -->
<board_overview>
  The pipeline scans Python repos for auth vulnerabilities...
</board_overview>

<!-- 3. Neighbor awareness — changes after each chat on connected nodes -->
<board_context>
  Research Agent:
  - The node investigates CVE databases for recent findings [fact]
  - Output should include severity scores [goal]
</board_context>

<!-- 4. Data shape — changes after each run on upstream nodes (NEW) -->
<run_context>
  Research Agent (last run — success, pinned):
  - Produces a JSON array of 12 vulnerability objects
  - Each object has: cve_id (string), severity (high/medium/low), description (string), affected_versions (array)
  - Total output size: ~4KB
  - Note: This node is pinned. Output is frozen and guaranteed stable across runs.
</run_context>

<!-- 5. Status — ephemeral, only present when something is in flight -->
<summarization_status>
  Node "Research Agent" run results summary is being updated.
</summarization_status>
```

Each layer provides a different lens:
- **Graph context** = "how are the nodes wired together"
- **Board overview** = "what is this workflow about"
- **Board context (beliefs)** = "what has been discussed and decided on nearby nodes"
- **Run context (new)** = "what does the data actually look like coming through"

### Truncation Priority

When approaching the context token budget (default: ~4000 tokens across all layers), truncation follows this priority (first cut → last cut):

1. `<run_context>` — compressed to shorter summary
2. `<board_context>` beliefs with low confidence — dropped
3. `<board_overview>` — always included, it's small
4. `<graph_context>` — always included, it's small

---

## Board UX

### Pinned Nodes

Pinned nodes should be visually distinct from dynamic nodes:

- **Pin icon** — A clear pin indicator on the node. Only appears on nodes the user explicitly pinned.
- **Frozen output preview** — A compact badge showing that the node has a pinned output (e.g., "Pinned" or a small snippet of the run results summary).
- **Pin/unpin control** — A toggle or action button on the node to pin/unpin. Also available as a conversational action through the assistant ("pin this").
- **Skipped-at-runtime indicator** — During execution, nodes that are skipped by dead-path elimination (upstream of pinned nodes with no other consumers) are visually dimmed. They are NOT marked as pinned — just "not needed this run."

### Run Results Indicators

- **Summary available badge** — A small indicator on nodes that have a run results summary. Clicking it shows the summary text.
- **In-flight indicator** — A subtle activity indicator (spinner or pulse) on nodes that are currently being summarized. Appears briefly after runs complete.
- **Failure indicator** — When a node's last run failed, the run results badge reflects this (e.g., red tint or error icon) so the user can see at a glance that upstream data may be broken.

---

## How It All Fits Together

The workflow lifecycle becomes:

1. **Design** — User opens the board, starts talking to node assistants. Chat beliefs flow between nodes. Board overview builds ambient awareness.

2. **Run** — User executes nodes or the full workflow. Run results summaries are generated in the background. Connected node assistants now understand what upstream nodes produce.

3. **Iterate** — User reviews results, talks to assistants about changes. Assistants have full context: what the board does (overview), what was discussed (beliefs), what the data looks like (run results), and how things are wired (graph context).

4. **Pin** — User pins nodes they're happy with. Those outputs become free context sources — routed through the graph at zero execution cost on future runs.

5. **Function** — The workflow becomes callable. Pinned nodes pass through, dynamic nodes execute. The INPUT node is the entry point for external data. The whole thing runs like a function with pre-computed context baked in.

6. **Maintain** — User modifies the board, re-runs pinned nodes to update their output, unpins nodes to iterate further. The cycle continues.

This transforms the board from a static workflow builder into a **living AI workbench** — part design tool, part runtime, part version control for agent configurations and their outputs.
