# Assistant's Notes + Context Injection Simplification

## Summary

Replace selective context routing with universal injection, and introduce a persistent "Assistant's Notes" system where the node assistant maintains its own scratchpad of revelations, direction changes, and special requirements across the conversation.

**Two types of injected knowledge:**
- **User Notes** (context nodes) — user-provided reference material (API specs, docker info, requirements). Injected into ALL agents at execution time.
- **Agent Notes** (assistant's notes) — accumulated by the node assistant during conversation. Injected into the DESIGNER ONLY at execution time.

---

## Motivation

Currently, context nodes are injected into agents through the same edge-based routing as other step outputs. This means the designer must reason about context delivery alongside agent-to-agent output routing — unnecessary complexity. Context nodes are user-provided reference material (API specs, requirements, docker info) that every agent should see.

The designer's `receives_from` routing is still valuable for **agent-to-agent outputs** — a Reporter may only need the Analyzer's findings, not the Scanner's raw dump. But context nodes should bypass this entirely.

Additionally, there's no mechanism for the node assistant to carry forward accumulated knowledge. When the assistant discovers something important (a direction change, an API constraint, a special requirement), that insight lives only in the conversation history and may be lost to context window compression. The assistant needs a persistent, structured place to record these insights so they inform future workflow executions.

---

## Part 1: Separate Context Routing from Agent Output Routing

The designer's `receives_from` **stays** for agent-to-agent output routing. What changes is that context nodes ("User Notes") are no longer part of the routing system — they're injected universally.

### 1A. Designer System Prompt — Clarify Routing Scope

**File:** `config/protocols/agent_designer/designer/system.md`

Update the `OUTPUT ROUTING (receives_from)` section (lines 65-80) to clarify it only controls agent-to-agent output flow, not context nodes:

```
OUTPUT ROUTING (receives_from):
- For each agent, specify which upstream agents' outputs it should receive
- This controls agent-to-agent output routing only — User Notes (context nodes)
  are injected into all agents automatically and are not affected by receives_from
- Use receives_from with an array of agent names from the roster
- Route selectively: an agent that evaluates upstream findings needs only those
  findings, not every prior agent's raw output
- When an agent genuinely needs all prior agent output (e.g., a final ReportWriter
  synthesizing the full pipeline), use an empty array [] to receive everything
- The first agent in execution order always has receives_from: []
- Use agent names in receives_from exactly as they appear in the roster
```

Update the output schema section to clarify:
```
The "receives_from" array controls which previous agents' outputs are injected
at runtime. This only affects agent-to-agent output routing. User Notes
(context nodes) are always available to all agents regardless of receives_from.
```

Update the `[pipeline_position | 0.80]` belief to mention that User Notes are always available.

### 1B. Node Assistant Task Force Block — Update Routing Language

**File:** `config/protocols/node_assistant/task_force/block.md`

Update the `<archetype_designer>` section (lines 19-27). Currently says:
```
The designer decides which agent's output flows to which downstream agent —
agents only see upstream output relevant to their task, not everything.
```

Replace with:
```
Before execution, an Agent Designer reads your roster and generates tailored
prompts, tool assignments, and output routing for each agent. The designer
decides which agent's output flows to which downstream agent via receives_from
routing. All agents automatically receive User Notes (context nodes) regardless
of routing. Use consistent, clear agent names — the designer uses these for
routing. Think about data flow when designing the roster: which agent produces
output that another agent needs? Order and name agents to make these
dependencies obvious.
```

---

## Part 2: Universal Context Node Injection

### 2A. Task Force Execution — Inject Context Nodes

**File:** `src/server/hub/dag/task_force/mod.rs`

The documenter already collects upstream context via `collect_upstream_context_data()`. The task force does not. Add context node injection:

1. Import `collect_upstream_context_data` from `super::utils`
2. After resolving port inputs (line 106), collect upstream context:
   ```rust
   let upstream_context = collect_upstream_context_data(
       step.id, edges, steps, &dag_state.completed_envelopes
   );
   ```
3. Build context documents and a context block using `build_context_block()`
4. Inject the context block into each agent's task prompt (alongside previous outputs)

The context block should wrap in `<user_notes>` tags to distinguish from agent outputs:
```xml
<user_notes>
<document_550e8400 title="API Spec">
{content}
</document_550e8400>
</user_notes>
```

Note: `execute_task_force_step` currently takes `_steps` (unused). This will now be used for context collection.

### 2B. Designer Input — Include Context Nodes

**File:** `src/server/hub/dag/designer_input/task_force.rs`

Currently, `build_task_force_designer_input()` formats envelopes as upstream context. Context nodes are mixed in with other step outputs. Make context nodes distinguishable:

- Add a `source_type: "context"` to upstream entries that come from context-mode steps
- This lets the designer know which upstream content is user-provided reference material vs. computed outputs

### 2C. Room Execution — Inject Context Nodes

**File:** `src/server/hub/dag/room/mod.rs` (if not already doing this)

Ensure room execution also injects all context node content into each room member's system prompt. Same pattern as task force.

---

## Part 3: Assistant's Notes — Database

### 3A. Migration

Create migration: `XXXX_add_assistant_notes.sql`

```sql
CREATE TABLE assistant_notes (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    step_id     UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    content     TEXT NOT NULL DEFAULT '',
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_assistant_notes_step_id ON assistant_notes(step_id);
```

**Scoped per step** (not per agent table row). Each workflow step's node assistant has its own notes. This is correct because:
- The node assistant is instantiated per-step (each step on the board has its own assistant)
- Notes accumulate as the user configures that specific step
- Notes travel with the step if the workflow is cloned or templated

### 3B. Repository

**File:** `src/db/workflows/` (or new file `src/db/assistant_notes/`)

```rust
pub async fn get_assistant_notes(&self, step_id: Uuid) -> Result<Option<String>>;
pub async fn upsert_assistant_notes(&self, step_id: Uuid, content: &str) -> Result<()>;
```

`upsert` because the assistant replaces the full content on each update (not append-only — the assistant can reorganize and prune).

---

## Part 4: Assistant's Notes — Tool

### 4A. Tool Definition

**File:** `src/tools/registry/mod.rs` (add to universal node assistant tools)

Tool name: `update_notes`

Schema:
```json
{
  "name": "update_notes",
  "description": "Update your personal notes. These notes persist across conversations and are injected into the workflow designer at execution time. Use this to record important discoveries, direction changes, special requirements, API details, or infrastructure notes. You can reorganize, prune, or rewrite your notes at any time — the full content is replaced on each call.",
  "input_schema": {
    "type": "object",
    "properties": {
      "content": {
        "type": "string",
        "description": "The complete updated notes content (replaces all previous notes). Use markdown formatting. Keep notes concise and actionable."
      }
    },
    "required": ["content"]
  }
}
```

### 4B. Tool Handler

**File:** `src/server/tools/assistant_notes.rs` (new)

```rust
pub async fn handle_update_notes(
    state: &AppState,
    step_id: Uuid,
    content: &str,
) -> Result<String, ToolError> {
    state.repos().workflows
        .upsert_assistant_notes(step_id, content)
        .await?;
    Ok("Notes updated.".to_string())
}
```

### 4C. Tool Dispatch

**File:** `src/server/hub/strategies/chat/mod.rs` → `dispatch_step_tool()`

Add `update_notes` to the universal step tools (not archetype-specific). Route to the handler.

---

## Part 5: Assistant's Notes — Injection

### 5A. Node Assistant System Prompt

**File:** `config/protocols/node_assistant/base/system.md`

Add a new template variable section between `<board_context>` and the archetype block:

```xml
<board_context>
{{.System.board_context}}
</board_context>

<your_notes>
{{.System.assistant_notes}}
</your_notes>

{{.System.archetype_block}}
```

When notes are empty, the section renders as:
```xml
<your_notes>
No notes yet. Use update_notes to record important discoveries.
</your_notes>
```

### 5B. System Prompt Assembly

**File:** `src/server/hub/mod.rs` → `build_step_system_prompt()`

After loading beliefs and before resolving the template:

```rust
// Load assistant notes for this step
let notes = state.repos().workflows
    .get_assistant_notes(step_id)
    .await
    .unwrap_or_default()
    .unwrap_or_else(|| "No notes yet. Use update_notes to record important discoveries.".to_string());

vars_map.insert(vars::system::ASSISTANT_NOTES.to_string(), notes);
```

### 5C. Template Variable

**File:** `src/config/protocols.rs` → `vars::system`

Add:
```rust
pub const ASSISTANT_NOTES: &str = "System.assistant_notes";
```

### 5D. Designer Injection (Execution Time)

**File:** `src/server/hub/dag/task_force/mod.rs` (or `designer_input/task_force.rs`)

When building the designer input, load the step's assistant notes and include them as a dedicated upstream context entry:

```rust
let notes = state.repos().workflows
    .get_assistant_notes(step.id)
    .await
    .unwrap_or_default();

if let Some(notes_content) = notes {
    if !notes_content.is_empty() {
        // Add as a special upstream context for the designer
        designer_input.upstream.push(UpstreamContext {
            source_name: "Assistant's Notes".to_string(),
            source_type: "agent_notes".to_string(),
            content: notes_content,
        });
    }
}
```

The designer will see these notes as part of its upstream context and can factor them into the prompts it generates (direction changes, special API requirements, infrastructure constraints, etc.).

Similarly for documenter and room archetypes — their designer input builders should also include assistant notes.

---

## Part 6: Prompting Strategy

This is the critical design piece. Based on the research in `AGENT_PERSONALITY.md`, here's the prompting strategy for note-taking.

### 6A. Why Tool-Based Write + System Prompt Read

Per the personality research:
- **Structural separation** (Section 5) — notes are a distinct concern from conversation. Mixing them into responses creates noise. A tool call keeps note-taking as a side-effect, not conversational content.
- **Anti-pattern inoculation** (Section 9, Pattern 4) — we don't want the assistant narrating "Let me update my notes now..." The tool call is silent from the user's perspective.
- **Register shifts** (Section 6) — notes should be in the "working register" regardless of the conversation's current register. The assistant may be in relaxed brainstorming mode but its notes should remain precise and factual.

### 6B. Note-Taking Guidance in System Prompt

Add to the `<identity>` or create a new `<notes_guidance>` section in the base system prompt:

```xml
<notes_guidance>
You have a persistent notepad (update_notes tool). These notes survive across
conversations and feed into the workflow designer at execution time.

Record when:
- The user changes direction or clarifies intent
- You discover a constraint or requirement that affects execution
- Technical details surface (API specs, container config, credentials setup)
- The user makes a decision that narrows the solution space

Keep notes:
- Factual and concise — bullet points over prose
- Organized by topic, not chronologically
- Pruned — remove outdated items when direction changes
- Written for another AI to consume, not for the user to read

Do not:
- Narrate that you're taking notes — just call the tool
- Record every conversation detail — only record what changes execution
- Duplicate information already in the step config
</notes_guidance>
```

### 6C. Note Format Convention

The assistant should maintain notes in a consistent structure. Provide a lightweight template in the guidance or as the default empty-state:

```markdown
## Direction
- [What we're building and why]

## Requirements
- [Hard constraints, special requirements]

## Technical Details
- [API specs, container info, infrastructure notes]

## Decisions
- [Key choices made and their reasoning]
```

This structure helps the designer quickly parse relevant sections when generating prompts.

### 6D. Designer's Consumption of Notes

The designer system prompt should acknowledge that assistant notes exist. Add a brief note to the `<what_you_produce>` section:

```
ASSISTANT'S NOTES:
When present in upstream context with source_type "agent_notes", these are
accumulated observations from the step's configuration assistant. They contain
direction changes, special requirements, and technical details discovered
during user conversations. Factor these into your prompt design — they
represent verified project-specific knowledge that should inform agent
behavior and task framing.
```

### 6E. Personality Considerations (from AGENT_PERSONALITY.md)

The note-taking behavior should align with the agent's personality profile:
- **High conscientiousness** (Section 4) — notes should be thorough and reliable, not speculative
- **Working register** (Section 6) — notes stay in the working register regardless of conversation tone
- **Anti-sycophancy** (Section 7) — notes record what IS, not what the user wants to hear. If the user's plan has a gap, the notes should reflect that
- **Personality-task separation** (Section 5) — note-taking is task behavior, not personality expression. The tool call is functional, not performative

---

## Part 7: Cleanup

### 7A. Verify Routing Code Still Works

`receives_from` stays for agent-to-agent routing, so `filter_outputs_for_agent()`, `build_filtered_outputs_block()`, and `normalize_agent_name()` all remain. Verify they still work correctly after context nodes are separated out (context nodes should never appear in `agent_outputs`, only in the separately-injected `<user_notes>` block).

Remove the `[description_routing | 0.75]` belief from the designer system prompt — it conflated context routing with agent descriptions.

### 7B. Update Documenter Assistant Prompt

**File:** `config/protocols/documenter/assistant/system.md`

The "Understanding incoming context" section (lines 26-38) describes context sources as things connected via edges. Update to reflect that User Notes (context nodes) flow to all agents universally, not just through explicit connections.

### 7C. Update Room Block

**File:** `config/protocols/node_assistant/room/block.md`

Line 12 says "If upstream belief capture nodes are connected, each agent's system prompt is enriched with relevant beliefs." This is about beliefs (staying). But verify there's no context routing language to clean up.

---

## Part 8: Board Overview Summary (Haiku Distiller)

### Background: The Existing Belief System

The codebase has an existing pattern for background AI-powered extraction. After every assistant response in a step-scoped chat, `spawn_chat_belief_extraction()` fires a background tokio task that:

1. Loads the conversation from `chat_messages`
2. Loads existing beliefs from connected nodes (board awareness)
3. Calls Haiku (`claude-3-5-haiku-20241022`) with a protocol template
4. Parses structured JSON output into `BeliefRow` structs
5. Replaces (delete + re-insert) beliefs for the step in the `beliefs` table

**Key code locations:**
- Spawn helper: `src/server/hub/chat_beliefs/mod.rs:36-48`
- Extraction logic: `src/server/hub/chat_beliefs/mod.rs:53-173`
- Trigger point: `src/server/hub/strategies/chat/mod.rs:456-462` (inside `on_complete()`)
- Haiku utility functions: `src/server/tools/mod.rs:526-603`
- Protocol templates: `config/protocols/chat_belief_extraction/`

**Beliefs** capture what the USER said — goals, requirements, decisions, constraints. They are per-step, scoped to connected nodes, and structured (type, confidence, semantic tags).

### What Part 8 Adds: Board Overview Summary

A **second haiku distiller** that runs alongside the belief extractor but produces something different: a single-paragraph summary of what ALL steps on the board are doing, derived from ALL assistant notes across the workflow.

**Separation of concerns:**

| | Belief Extraction (existing) | Board Overview Summary (new) |
|---|---|---|
| **Input** | One step's conversation messages | All steps' assistant notes |
| **Output** | Structured atomic beliefs (JSON array) | One plain-text paragraph |
| **Scope** | Per-step → injected into connected nodes | Per-workflow → injected into ALL assistants |
| **Purpose** | "What has the user decided at this node?" | "What's happening across the whole board?" |
| **Trigger** | After every assistant response | After any `update_notes` call |
| **Storage** | `beliefs` table (many rows per step) | `workflow_board_summary` column (one string per workflow) |

### 8A. Database

Add a column to the existing `workflows` table:

**Migration:** `XXXX_add_board_overview_summary.sql`

```sql
ALTER TABLE workflows ADD COLUMN board_overview_summary TEXT NOT NULL DEFAULT '';
```

No new table needed. One summary per workflow, updated in-place.

**Repository methods:**

```rust
/// Get the cached board overview summary for a workflow.
pub async fn get_board_overview_summary(&self, workflow_id: Uuid) -> Result<String>;

/// Update the board overview summary for a workflow.
pub async fn update_board_overview_summary(&self, workflow_id: Uuid, summary: &str) -> Result<()>;
```

### 8B. Haiku Summarizer Function

**File:** `src/server/hub/board_overview/mod.rs` (new module)

This follows the exact same pattern as `chat_beliefs/mod.rs`. Here's the full design:

```rust
//! Board overview summary — haiku distiller for cross-board awareness.
//!
//! After any assistant updates its notes, Haiku summarizes ALL assistant
//! notes across the workflow into a single paragraph. This summary is
//! injected into every assistant's system prompt so each node has ambient
//! awareness of the full board.

use tracing::{info, warn};
use uuid::Uuid;

use crate::llm::{AnthropicClient, AnthropicConfig, LLMProvider, LLMRequest, Message as LlmMessage};
use crate::server::state::AppState;

/// Max tokens for the board overview response.
/// One paragraph = ~100-150 tokens. Allow headroom.
const MAX_TOKENS_BOARD_OVERVIEW: u32 = 512;

/// Spawn a background board overview summarization.
/// Non-blocking — fires and forgets. Errors are logged, not propagated.
///
/// Called after any `update_notes` tool call completes.
pub fn spawn_board_overview_update(state: AppState, workflow_id: Uuid) {
    tokio::spawn(async move {
        if let Err(e) = regenerate_board_overview(&state, workflow_id).await {
            tracing::error!("Board overview update failed for workflow {workflow_id}: {e}");
        }
    });
}
```

**The core function:**

```rust
/// Load all assistant notes across the workflow, summarize via Haiku,
/// store the result on the workflow row.
async fn regenerate_board_overview(
    state: &AppState,
    workflow_id: Uuid,
) -> Result<(), anyhow::Error> {
    // 1. Load all steps for this workflow
    let steps = state.repos().workflows.list_steps(workflow_id).await?;

    // 2. Load assistant notes for every step that has them
    let mut notes_by_step: Vec<(String, String)> = Vec::new();
    for step in &steps {
        if let Some(notes) = state.repos().workflows
            .get_assistant_notes(step.id)
            .await?
        {
            if !notes.is_empty() {
                let step_name = step.name.as_deref().unwrap_or("(unnamed)");
                let step_label = format!("{} ({})", step_name, step.execution_mode);
                notes_by_step.push((step_label, notes));
            }
        }
    }

    // 3. If no notes exist anywhere, clear the summary
    if notes_by_step.is_empty() {
        state.repos().workflows
            .update_board_overview_summary(workflow_id, "")
            .await?;
        return Ok(());
    }

    // 4. Format all notes as input for Haiku
    let formatted_input = format_notes_for_summarization(&notes_by_step);

    // 5. Call Haiku
    let config = AnthropicConfig::from_env()?;
    let client = AnthropicClient::new(config)?;

    let request = LLMRequest::new(
        crate::constants::MODEL_HAIKU,
        vec![LlmMessage::user(formatted_input)],
    )
    .with_system(BOARD_OVERVIEW_SYSTEM_PROMPT)
    .with_max_tokens(MAX_TOKENS_BOARD_OVERVIEW);

    let response = client.send_message(request).await?;
    let summary = response.content.trim().to_string();

    info!(
        workflow_id = %workflow_id,
        steps_with_notes = notes_by_step.len(),
        summary_len = summary.len(),
        "Board overview summary updated"
    );

    // 6. Store on workflow
    state.repos().workflows
        .update_board_overview_summary(workflow_id, &summary)
        .await?;

    Ok(())
}
```

### 8C. Input Formatting

The function that formats all assistant notes into a single input for Haiku:

```rust
/// Format all assistant notes across the board into Haiku input.
///
/// Example output:
/// ```text
/// [Security Scanner (task_force)]
/// ## Direction
/// - Scanning GitHub repos for OWASP top 10 vulnerabilities
/// ## Requirements
/// - Must support Python and JavaScript repos
/// - Docker container with semgrep pre-installed
///
/// [API Reference (documenter)]
/// ## Direction
/// - Generate OpenAPI spec from source code
/// ## Technical Details
/// - REST API uses Express.js with Zod validation
/// ```
fn format_notes_for_summarization(notes_by_step: &[(String, String)]) -> String {
    let mut out = String::new();
    for (i, (step_label, notes)) in notes_by_step.iter().enumerate() {
        if i > 0 {
            out.push_str("\n\n");
        }
        out.push_str(&format!("[{}]\n{}", step_label, notes));
    }
    out
}
```

### 8D. System Prompt for the Haiku Summarizer

This is the system prompt that tells Haiku how to summarize. It should be a constant in the module (not a protocol template, since this is simple enough to inline):

```rust
const BOARD_OVERVIEW_SYSTEM_PROMPT: &str = r#"You summarize what a workflow board is doing.

You receive notes from multiple workflow steps. Each step is a different part of the pipeline (task forces, documenters, rooms, etc.). The notes were written by each step's configuration assistant during conversations with the user.

Produce ONE paragraph (3-5 sentences max) that describes:
- What the overall workflow does (the big picture)
- What each step contributes to the pipeline
- Any key constraints or technical details that affect multiple steps

Write for an AI assistant that needs ambient awareness of the full board. Be specific — name the actual technologies, domains, and deliverables mentioned in the notes. Do not be vague ("the team is working on a project") — be concrete ("the pipeline scans Python repos for auth vulnerabilities, generates a remediation guide, then a review panel debates priority").

If only one step has notes, still summarize it — the other steps haven't been configured yet.

Return ONLY the paragraph. No headers, no bullet points, no preamble."#;
```

### 8E. Trigger Point

**File:** `src/server/tools/assistant_notes.rs` (the handler from Part 4B)

After successfully upserting notes, spawn the board overview update:

```rust
pub async fn handle_update_notes(
    state: &AppState,
    workflow_id: Uuid,  // Need to pass this through from step context
    step_id: Uuid,
    content: &str,
) -> Result<String, ToolError> {
    state.repos().workflows
        .upsert_assistant_notes(step_id, content)
        .await?;

    // Regenerate board overview in background (non-blocking)
    crate::server::hub::board_overview::spawn_board_overview_update(
        state.clone(),
        workflow_id,
    );

    Ok("Notes updated.".to_string())
}
```

Note: The tool handler needs `workflow_id` in addition to `step_id`. This is available from `StepChatContext` in the chat strategy. Pass it through during tool dispatch.

### 8F. Injection into Assistant System Prompt

**File:** `config/protocols/node_assistant/base/system.md`

Add a `<board_overview>` section. This goes BEFORE `<board_context>` because it's higher-level context:

```xml
<board_overview>
{{.System.board_overview}}
</board_overview>

<board_context>
{{.System.board_context}}
</board_context>

<your_notes>
{{.System.assistant_notes}}
</your_notes>

{{.System.archetype_block}}

{{.System.current_config}}
```

**When the summary is empty** (no notes on any step yet), render:

```xml
<board_overview>
No steps have been configured yet.
</board_overview>
```

### 8G. System Prompt Assembly

**File:** `src/server/hub/mod.rs` → `build_step_system_prompt()`

Load the board overview and inject it as a template variable:

```rust
// Load board overview summary
let board_overview = state.repos().workflows
    .get_board_overview_summary(workflow_id)
    .await
    .unwrap_or_default();

let board_overview_text = if board_overview.is_empty() {
    "No steps have been configured yet.".to_string()
} else {
    board_overview
};

vars_map.insert(vars::system::BOARD_OVERVIEW.to_string(), board_overview_text);
```

### 8H. Template Variable

**File:** `src/config/protocols.rs` → `vars::system`

```rust
pub const BOARD_OVERVIEW: &str = "System.board_overview";
```

### 8I. Example: What This Looks Like End-to-End

**Scenario:** A workflow with three steps — a task force, a documenter, and a room.

**Step 1: User configures the task force.** The assistant takes notes:

```markdown
## Direction
- Scanning GitHub repos for OWASP top 10 vulnerabilities
- Focus on Python and JavaScript codebases

## Requirements
- Docker container with semgrep and bandit pre-installed
- Must output findings in SARIF format

## Decisions
- Using semgrep for pattern matching, bandit for Python-specific checks
- Fail-fast mode: stop if scanner agent crashes
```

The `update_notes` call triggers `spawn_board_overview_update()`. Haiku produces:

> *"This workflow scans GitHub repositories for OWASP top 10 vulnerabilities in Python and JavaScript codebases using semgrep and bandit in a Docker container, with findings output in SARIF format."*

(Only one step has notes so far, so the summary is just about that step.)

**Step 2: User configures the documenter.** The assistant takes notes:

```markdown
## Direction
- Generate a remediation guide from the scanner's SARIF findings

## Technical Details
- Each finding should map to a specific CWE
- Guide should include code examples for fixes
- Target audience: junior developers
```

The `update_notes` call triggers another `spawn_board_overview_update()`. Haiku now sees both steps' notes and produces:

> *"This pipeline scans Python and JavaScript GitHub repos for OWASP top 10 vulnerabilities using semgrep and bandit in Docker, outputting SARIF findings. A documenter then generates a CWE-mapped remediation guide with fix code examples targeted at junior developers. A review room is connected but not yet configured."*

**Step 3: User configures the room.** The assistant takes notes:

```markdown
## Direction
- Security lead and tech lead debate remediation priority

## Requirements
- Must consider business impact alongside severity
- Final output: prioritized remediation backlog
```

Final board overview after all three steps:

> *"This pipeline scans Python and JavaScript GitHub repos for OWASP top 10 vulnerabilities using semgrep and bandit, outputs SARIF findings, then generates a CWE-mapped remediation guide with code examples for junior developers. A review room with security and tech leads debates remediation priority considering business impact, producing a prioritized remediation backlog."*

**What each assistant sees in their system prompt:**

The task force assistant sees:
```xml
<board_overview>
This pipeline scans Python and JavaScript GitHub repos for OWASP top 10
vulnerabilities using semgrep and bandit, outputs SARIF findings, then
generates a CWE-mapped remediation guide with code examples for junior
developers. A review room with security and tech leads debates remediation
priority considering business impact, producing a prioritized remediation backlog.
</board_overview>

<board_context>
Documenter:
- Generate remediation guide from SARIF findings [goal]
- Guide targets junior developers [requirement]
Review Room:
- Security and tech leads debate priority [goal]
</board_context>

<your_notes>
## Direction
- Scanning GitHub repos for OWASP top 10 vulnerabilities
...
</your_notes>
```

Three layers of awareness:
1. **Board overview** — one paragraph, the big picture
2. **Board context** — granular beliefs from connected nodes (existing system)
3. **Your notes** — this assistant's own accumulated knowledge

### 8J. Staleness and Update Frequency

The board overview updates ONLY when `update_notes` is called. It is slightly stale between updates. This is acceptable because:

- The overview is ambient context, not precision data
- Beliefs (board_context) provide real-time granular updates between overview refreshes
- Haiku is fast (~200ms) but we don't want to call it on every chat message

If a step has no notes yet, it won't appear in the overview. As the user configures more steps, the overview grows richer. This is the intended progressive disclosure.

### 8K. Testing

- **Unit test:** `format_notes_for_summarization` — format with 0, 1, and 3 steps
- **Unit test:** `regenerate_board_overview` with mocked haiku — verify empty notes clears summary, non-empty notes produces summary
- **Unit test:** `build_step_system_prompt` includes board overview when present, shows default when absent
- **Integration test:** Update notes on two steps, verify board overview is regenerated and contains content from both
- **Integration test:** Verify board overview appears in assistant system prompt for a step that did NOT trigger the update (cross-board injection)

---

## Implementation Order

1. **Migration + Repository** (Part 3 + Part 8A) — `assistant_notes` table + `board_overview_summary` column
2. **Tool** (Part 4) — `update_notes` tool definition, handler, dispatch
3. **Board overview summarizer** (Part 8B-E) — haiku module, trigger from `update_notes`
4. **Injection into node assistant** (Part 5A-C + Part 8F-H) — template variables, system prompt assembly for notes + board overview
5. **Universal context injection** (Part 2) — inject context nodes into all agents, separate from agent output routing
6. **Update designer prompt** (Part 1A-B) — clarify routing scope, add User Notes awareness
7. **Designer injection of notes** (Part 5D) — feed assistant notes into designer input
8. **Prompting strategy** (Part 6) — add note-taking guidance to system prompt
9. **Cleanup** (Part 7) — prompt updates, verify routing still works

Steps 1-4 can ship as one feature (Assistant's Notes + Board Overview). Steps 5-6 can ship together (context injection changes). Steps 7-9 follow.

---

## Testing

### Assistant's Notes (Parts 3-4)
- **Unit test:** `update_notes` tool handler — upsert, retrieve, empty state
- **Unit test:** `build_step_system_prompt` includes notes when present, shows default when absent
- **Unit test:** Note persistence — create notes, verify they survive across chat turns

### Board Overview Summary (Part 8)
- **Unit test:** `format_notes_for_summarization` — format with 0, 1, and 3 steps
- **Unit test:** `regenerate_board_overview` with mocked haiku — verify empty notes clears summary, non-empty produces summary
- **Unit test:** `build_step_system_prompt` includes board overview when present, shows default when absent
- **Integration test:** Update notes on two steps, verify board overview regenerated with both steps' content
- **Integration test:** Board overview appears in system prompt for steps that did NOT trigger the update

### Context Injection (Parts 1-2)
- **Unit test:** Task force execution injects all context nodes universally (separate from `receives_from` agent routing)
- **Unit test:** Designer input includes assistant notes as `agent_notes` source type
- **Integration test:** Full task force execution with context nodes — verify all agents receive context

### Designer Integration (Parts 5D, 6)
- **Unit test:** Designer input includes assistant notes as `agent_notes` upstream context
- **Unit test:** Designer prompt mentions User Notes are always available
