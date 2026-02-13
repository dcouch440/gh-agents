# Embedded User-Context Pattern — Design-Time Chat

## Overview

Restructure the node assistant's design-time chat so that **project state lives in the user message, not the system prompt**. The system prompt stays behavioral (identity, tools, guidelines). The user message carries the current state of what the user is building, rendered as if the user typed it, with their actual input appended at the bottom.

**Why this matters:**
- LLMs treat user-provided context as ground truth with higher attention weight
- Putting queries/instructions at the end of the user message improves quality by up to 30% ([Anthropic Long Context Tips](https://platform.claude.com/docs/en/docs/build-with-claude/prompt-engineering/long-context-tips))
- The system prompt becomes **stable and cacheable** — identity + tools + guidelines rarely change within a conversation, so prompt caching kicks in ([Anthropic Prompt Caching](https://platform.claude.com/docs/en/build-with-claude/prompt-caching))
- Context engineering research shows that providing structured, bespoke context at each step produces more reliable agent behavior ([Neo4j: Context Engineering](https://neo4j.com/blog/genai/context-engineering-vs-prompt-engineering/))

**Current architecture (everything in system prompt):**
```
SYSTEM: identity + graph_context + archetypes + archetype_block + current_config + guidelines
USER:   raw user input
```

**New architecture (split behavioral from contextual):**
```
SYSTEM: identity + archetypes + archetype_block + tool_instructions + guidelines
USER:   project_state + graph_context + user_input (at bottom)
```

The user message gets re-rendered each turn with the latest project state, so the LLM always sees the freshest config even after tool calls modify it mid-conversation.

---

## Part 1: User Context Templates

**Goal:** Create per-archetype user context templates that render the project state as a natural briefing, with the user's actual input at the bottom.

### 1a. Create `config/protocols/node_assistant/base/user_context.md`

This is the base user context wrapper used when **no archetype is selected yet** (blank node).

```markdown
Here's the current state of this node:

<graph>
{{.Context.graph}}
</graph>

<node>
{{.Context.node_state}}
</node>

{{.User.input}}
```

### 1b. Create `config/protocols/node_assistant/documenter/user_context.md`

This is the documenter-specific user context. Renders documents, incoming context, and step metadata as a project briefing.

```markdown
Here's where this documenter node is at:

<node>
Name: {{.Context.step_name}}
Description: {{.Context.step_description}}
</node>

<documents>
{{.Context.documents}}
</documents>

<incoming_context>
{{.Context.incoming}}
</incoming_context>

<graph>
{{.Context.graph}}
</graph>

{{.User.input}}
```

**What each section renders to (example):**

```markdown
Here's where this documenter node is at:

<node>
Name: Product Research
Description: Analyze the PRD and generate reference documents for the engineering team
</node>

<documents>
- API Reference — Authentication Endpoints
  Description: Comprehensive reference for all auth-related REST endpoints
  Target length: ~3000 words

- Data Model Overview
  Description: Entity relationships and field definitions for the user system
  Target length: ~1500 words
</documents>

<incoming_context>
- Product Requirements (context, populated, ~2400 words)
  Preview: "The authentication system should support OAuth 2.0 with PKCE flow
  for mobile clients and session-based auth for the web dashboard..."

- Technical Constraints (context, empty)
  No content yet.
</incoming_context>

<graph>
Workflow nodes:
  - Product Requirements (context)
  - Technical Constraints (context)
  - Product Research (documenter) [SELECTED]
  - API Designer (task_force)

Connections:
  Product Requirements -> Product Research
  Technical Constraints -> Product Research
  Product Research -> API Designer
</graph>

I want to add a third document that covers the error handling patterns. What do you think about "Error Response Catalog"?
```

### 1c. Create `config/protocols/node_assistant/task_force/user_context.md`

```markdown
Here's where this task force node is at:

<node>
Name: {{.Context.step_name}}
Description: {{.Context.step_description}}
</node>

<mission>
{{.Context.mission}}
</mission>

<agents>
{{.Context.agent_roster}}
</agents>

<incoming_context>
{{.Context.incoming}}
</incoming_context>

<graph>
{{.Context.graph}}
</graph>

{{.User.input}}
```

### 1d. Create `config/protocols/node_assistant/belief_capture/user_context.md`

```markdown
Here's where this belief capture node is at:

<node>
Name: {{.Context.step_name}}
Description: {{.Context.step_description}}
</node>

<extraction_plan>
{{.Context.extraction_plan}}
</extraction_plan>

<incoming_context>
{{.Context.incoming}}
</incoming_context>

<graph>
{{.Context.graph}}
</graph>

{{.User.input}}
```

### 1e. Create `config/protocols/node_assistant/room/user_context.md`

```markdown
Here's where this room node is at:

<node>
Name: {{.Context.step_name}}
Description: {{.Context.step_description}}
</node>

<room>
{{.Context.room_config}}
</room>

<incoming_context>
{{.Context.incoming}}
</incoming_context>

<graph>
{{.Context.graph}}
</graph>

{{.User.input}}
```

### Files created (Part 1)
- **Create:** `config/protocols/node_assistant/base/user_context.md`
- **Create:** `config/protocols/node_assistant/documenter/user_context.md`
- **Create:** `config/protocols/node_assistant/task_force/user_context.md`
- **Create:** `config/protocols/node_assistant/belief_capture/user_context.md`
- **Create:** `config/protocols/node_assistant/room/user_context.md`

---

## Part 2: Modify System Prompt Template

**Goal:** Remove dynamic project state from the system prompt. Keep only behavioral content.

### 2a. Modify `config/protocols/node_assistant/base/system.md`

**Remove:** `{{.System.graph_context}}` and `{{.System.current_config}}` injection points.

**Before:**
```markdown
<identity>
You are the workflow configuration assistant. Users drop blank nodes onto
a canvas and talk to you to define what each node does. You evaluate the
user's intent and configure nodes through tool calls.
</identity>

<graph_context>
{{.System.graph_context}}
</graph_context>

<archetypes>
...
</archetypes>

{{.System.archetype_block}}

{{.System.current_config}}

<guidelines>
...
</guidelines>
```

**After:**
```markdown
<identity>
You are the workflow configuration assistant. Users drop blank nodes onto
a canvas and talk to you to define what each node does. You evaluate the
user's intent and configure nodes through tool calls.

The user provides the current state of their node and workflow at the
start of each message. Use that state to understand what's configured
and what still needs work.
</identity>

<archetypes>
When the user describes what they need, determine which archetype fits:

- documenter: A research-and-write pipeline that produces structured
  documents. Use when the user wants comprehensive written output
  organized into sections or documents.

- task_force: A team of agents that executes a multi-step mission.
  Use when the user describes work that requires planning, execution,
  and deliverables.

- belief_capture: A context summarizer that extracts structured knowledge
  from upstream results. Use when the user wants to distill findings
  for downstream consumption.

- room: A meeting space where agents discuss, debate, or review.
  Use when the user wants collaborative deliberation on a topic.

Call set_node_archetype once the intent is clear. If the user changes
direction, call it again — archetype switching is expected.
</archetypes>

{{.System.archetype_block}}

<guidelines>
- Evaluate the user's intent before selecting an archetype. Ask a
  clarifying question if two archetypes could fit equally well.
- Configure through tool calls, not prose. Each tool call updates
  the node's visual representation in real-time.
- Keep responses concise. The user sees the node update live —
  you don't need to repeat what the tools just did.
- Reference the project state the user provides at the start of
  their message. It reflects the latest configuration.
</guidelines>
```

**Key changes:**
- Removed `<graph_context>` section entirely (moves to user message)
- Removed `{{.System.current_config}}` (moves to user message)
- Added note in `<identity>` that the user provides state in their messages
- Added guideline to reference the user-provided state
- `{{.System.archetype_block}}` stays — it's behavioral (what this mode is, how to think about it)

### 2b. Modify archetype blocks — no changes needed

The archetype blocks (`documenter/block.md`, `task_force/block.md`, etc.) stay in the system prompt. They describe **how to think about the archetype** — design principles, available capabilities, pipeline structure. This is behavioral guidance, not project state.

### Files modified (Part 2)
- **Modify:** `config/protocols/node_assistant/base/system.md`

---

## Part 3: Rust — Split Prompt Building

**Goal:** `build_step_system_prompt()` currently returns one string (system prompt with everything). Split it so it returns a system prompt AND a user context builder that can render project state per-turn.

### 3a. Register user context templates in `src/config/protocols.rs`

Add static template strings alongside existing ones:

```rust
// User context templates (injected into user message, not system prompt)
pub static NODE_ASSISTANT_USER_CONTEXT_BASE: &str =
    include_str!("../../config/protocols/node_assistant/base/user_context.md");

pub static NODE_ASSISTANT_USER_CONTEXT_DOCUMENTER: &str =
    include_str!("../../config/protocols/node_assistant/documenter/user_context.md");

pub static NODE_ASSISTANT_USER_CONTEXT_TASK_FORCE: &str =
    include_str!("../../config/protocols/node_assistant/task_force/user_context.md");

pub static NODE_ASSISTANT_USER_CONTEXT_BELIEF_CAPTURE: &str =
    include_str!("../../config/protocols/node_assistant/belief_capture/user_context.md");

pub static NODE_ASSISTANT_USER_CONTEXT_ROOM: &str =
    include_str!("../../config/protocols/node_assistant/room/user_context.md");

// Template variable keys for user context
pub mod context {
    pub const GRAPH: &str = "Context.graph";
    pub const NODE_STATE: &str = "Context.node_state";
    pub const STEP_NAME: &str = "Context.step_name";
    pub const STEP_DESCRIPTION: &str = "Context.step_description";
    pub const DOCUMENTS: &str = "Context.documents";
    pub const INCOMING: &str = "Context.incoming";
    pub const MISSION: &str = "Context.mission";
    pub const AGENT_ROSTER: &str = "Context.agent_roster";
    pub const EXTRACTION_PLAN: &str = "Context.extraction_plan";
    pub const ROOM_CONFIG: &str = "Context.room_config";
}
```

### 3b. Modify `build_step_system_prompt()` in `src/server/hub/mod.rs`

Rename to `build_step_prompts()` and return both system prompt and user context template.

```rust
/// Result of building step prompts — system prompt is stable,
/// user context gets rendered per-turn with fresh state.
pub struct StepPrompts {
    pub system_prompt: String,
    pub user_context_template: String,
}

/// Build both the system prompt and user context template for a step chat.
///
/// System prompt: identity + archetypes + archetype block + guidelines (stable)
/// User context template: project state template with {{.Context.*}} variables
async fn build_step_prompts(
    state: &AppState,
    workflow_id: Uuid,
    step_id: Uuid,
    execution_mode: &str,
) -> Result<StepPrompts, HubError> {
    // 1. Load archetype block (stays in system prompt)
    let archetype_block = match execution_mode {
        "documenter" => roles::NODE_ASSISTANT_DOCUMENTER_BLOCK.to_string(),
        "task_force" => roles::NODE_ASSISTANT_TASK_FORCE_BLOCK.to_string(),
        "belief_capture" => roles::NODE_ASSISTANT_BELIEF_CAPTURE_BLOCK.to_string(),
        "room" => roles::NODE_ASSISTANT_ROOM_BLOCK.to_string(),
        _ => String::new(),
    };

    // 2. Resolve system prompt (no graph_context or current_config — those go in user message)
    let mut system_vars = HashMap::new();
    system_vars.insert(
        system::ARCHETYPE_BLOCK.to_string(),
        archetype_block,
    );
    let system_prompt = roles::NODE_ASSISTANT_BASE.resolve(&system_vars).system_prompt;

    // 3. Select user context template
    let user_context_template = match execution_mode {
        "documenter" => roles::NODE_ASSISTANT_USER_CONTEXT_DOCUMENTER.to_string(),
        "task_force" => roles::NODE_ASSISTANT_USER_CONTEXT_TASK_FORCE.to_string(),
        "belief_capture" => roles::NODE_ASSISTANT_USER_CONTEXT_BELIEF_CAPTURE.to_string(),
        "room" => roles::NODE_ASSISTANT_USER_CONTEXT_ROOM.to_string(),
        _ => roles::NODE_ASSISTANT_USER_CONTEXT_BASE.to_string(),
    };

    Ok(StepPrompts {
        system_prompt,
        user_context_template,
    })
}
```

### 3c. Create `build_user_context()` in `src/server/hub/mod.rs`

This function renders the user context template with fresh project state each turn. Called before every LLM request.

```rust
/// Render the user context template with fresh project state + user input.
///
/// Called each turn to ensure the LLM sees the latest configuration
/// (tool calls between turns may have changed the state).
async fn build_user_context(
    state: &AppState,
    workflow_id: Uuid,
    step_id: Uuid,
    execution_mode: &str,
    user_context_template: &str,
    user_input: &str,
) -> Result<String, HubError> {
    let mut vars = HashMap::new();

    // Graph context (shared across all archetypes)
    let graph = graph_context::build_graph_context(state, workflow_id, step_id).await?;
    vars.insert(context::GRAPH.to_string(), graph);

    // Step metadata (shared)
    let step = state.repo().get_workflow_step(step_id).await?;
    vars.insert(
        context::STEP_NAME.to_string(),
        step.name.clone().unwrap_or_else(|| "Unnamed".to_string()),
    );
    vars.insert(
        context::STEP_DESCRIPTION.to_string(),
        step.description.clone().unwrap_or_default(),
    );

    // Archetype-specific state
    match execution_mode {
        "documenter" => {
            let (documents, incoming) =
                build_documenter_user_context(state, workflow_id, step_id).await?;
            vars.insert(context::DOCUMENTS.to_string(), documents);
            vars.insert(context::INCOMING.to_string(), incoming);
        }
        "task_force" => {
            let (mission, roster, incoming) =
                build_task_force_user_context(state, workflow_id, step_id).await?;
            vars.insert(context::MISSION.to_string(), mission);
            vars.insert(context::AGENT_ROSTER.to_string(), roster);
            vars.insert(context::INCOMING.to_string(), incoming);
        }
        "belief_capture" => {
            let (extraction_plan, incoming) =
                build_belief_capture_user_context(state, workflow_id, step_id).await?;
            vars.insert(context::EXTRACTION_PLAN.to_string(), extraction_plan);
            vars.insert(context::INCOMING.to_string(), incoming);
        }
        "room" => {
            let (room_config, incoming) =
                build_room_user_context(state, workflow_id, step_id).await?;
            vars.insert(context::ROOM_CONFIG.to_string(), room_config);
            vars.insert(context::INCOMING.to_string(), incoming);
        }
        _ => {
            let node_state = format!(
                "Archetype: not selected yet\nExecution mode: {}",
                execution_mode,
            );
            vars.insert(context::NODE_STATE.to_string(), node_state);
        }
    }

    // User input — always last
    vars.insert("User.input".to_string(), user_input.to_string());

    // Resolve template
    let rendered = resolve_template(user_context_template, &vars);
    Ok(rendered)
}
```

### 3d. Documenter-specific user context builder

Extract from the existing `build_config_snapshot()` in `src/server/tools/documenter/mod.rs`. Same data, different formatting — rendered as a user-facing project briefing instead of a system prompt injection.

```rust
/// Build the documenter sections for the user context message.
/// Returns (documents_section, incoming_section).
async fn build_documenter_user_context(
    state: &AppState,
    workflow_id: Uuid,
    step_id: Uuid,
) -> Result<(String, String), HubError> {
    // Load document definitions for this step
    let doc_defs = state.repo().get_doc_definitions_by_step(step_id).await?;

    let documents = if doc_defs.is_empty() {
        "No documents defined yet.".to_string()
    } else {
        let mut out = String::new();
        for doc in &doc_defs {
            out.push_str(&format!(
                "- {}\n  Description: {}\n  Target length: ~{} words\n\n",
                doc.name,
                doc.description.as_deref().unwrap_or("No description"),
                doc.target_length.unwrap_or(1500),
            ));
        }
        out
    };

    // Load incoming context (upstream nodes)
    let incoming = build_incoming_context(state, workflow_id, step_id).await?;

    Ok((documents, incoming))
}

/// Build the incoming context section — shared across archetypes.
/// Shows upstream nodes with their content status and previews.
async fn build_incoming_context(
    state: &AppState,
    workflow_id: Uuid,
    step_id: Uuid,
) -> Result<String, HubError> {
    let edges = state.repo().get_workflow_edges(workflow_id).await?;
    let upstream_step_ids: Vec<Uuid> = edges
        .iter()
        .filter(|e| e.target_step_id == step_id)
        .map(|e| e.source_step_id)
        .collect();

    if upstream_step_ids.is_empty() {
        return Ok("No incoming connections.".to_string());
    }

    let mut out = String::new();
    for upstream_id in &upstream_step_ids {
        let upstream_step = state.repo().get_workflow_step(*upstream_id).await?;
        let name = upstream_step.name.clone().unwrap_or_else(|| "Unnamed".to_string());
        let mode = &upstream_step.execution_mode;

        let status = classify_content_status(&upstream_step);
        let preview = build_content_preview(&upstream_step);

        out.push_str(&format!("- {} ({}, {})\n", name, mode, status));
        if let Some(desc) = &upstream_step.description {
            if !desc.is_empty() {
                out.push_str(&format!("  Description: {}\n", desc));
            }
        }
        if let Some(preview) = preview {
            out.push_str(&format!("  Preview: \"{}\"\n", preview));
        }
        out.push('\n');
    }
    Ok(out)
}
```

### Files modified (Part 3)
- **Modify:** `src/config/protocols.rs` — add user context template statics + variable keys
- **Modify:** `src/server/hub/mod.rs` — rename `build_step_system_prompt()` to `build_step_prompts()`, add `build_user_context()`, add per-archetype user context builders

---

## Part 4: ChatStrategy Integration

**Goal:** The ChatStrategy needs to use the rendered user context instead of raw user input. The user context is rendered each turn with fresh state.

### 4a. Add user context to `ChatConfig`

```rust
pub struct ChatConfig {
    pub system_prompt: String,
    pub user_context_template: Option<String>,  // NEW — if set, wraps user input
    pub tool_names: Vec<String>,
    pub model_id: String,
    pub temperature: f32,
    pub max_history: usize,
    // ... existing fields
}
```

### 4b. Modify `run_step_chat()` in `src/server/hub/mod.rs`

```rust
pub async fn run_step_chat(
    state: &AppState,
    workflow_id: Uuid,
    step_id: Uuid,
    message: &str,
    user_id: Uuid,
    session_id: Uuid,
    message_id: Uuid,
) -> Result<(), HubError> {
    let step = state.repo().get_workflow_step(step_id).await?;
    let execution_mode = &step.execution_mode;

    // Build both prompts
    let step_prompts = build_step_prompts(state, workflow_id, step_id, execution_mode).await?;

    // Render user context with fresh state + user's input
    let rendered_user_message = build_user_context(
        state,
        workflow_id,
        step_id,
        execution_mode,
        &step_prompts.user_context_template,
        message,
    ).await?;

    let chat_config = ChatConfig {
        system_prompt: step_prompts.system_prompt,
        user_context_template: Some(step_prompts.user_context_template),
        // ... other fields unchanged
    };

    // Pass rendered_user_message instead of raw message
    // The ChatStrategy receives the rendered message as input
    let strategy = ChatStrategy::with_step_context(
        chat_config,
        state.clone(),
        user_id,
        Some(session_id),
        message_id,
        step_context,
    );

    let engine = ExecutionEngine::new(provider);
    let recorder = ExecutionRecorder::new(/* ... */);
    let sink = ChatStreamSink::new(/* ... */);

    // Execute with rendered user context as input
    engine.execute(&strategy, &rendered_user_message, &sink, &recorder, cancel).await?;

    Ok(())
}
```

### 4c. Chat message storage consideration

**Important:** The raw user message (`message`) should be stored in `chat_messages` — not the rendered user context. The rendered context is ephemeral and reconstructed each turn. The chat history should contain what the user actually typed.

When `build_messages()` loads chat history, previous turns contain:
- Previous user messages (raw, what the user typed)
- Previous assistant messages (raw responses)

Only the **current turn's** user message gets the rendered context wrapper. Previous turns don't need the state re-injected — the chat history provides continuity, and the current turn's context provides the latest state.

**In `build_messages()`:**
```rust
async fn build_messages(&self, input: &str) -> Result<Vec<Message>, HubError> {
    let mut messages = Vec::new();

    // Load previous messages from chat history (raw, as stored)
    let history = self.load_session_history().await?;
    messages.extend(history);

    // Current turn uses the rendered user context (input is already rendered)
    messages.push(Message::user(input));

    Ok(messages)
}
```

The `input` parameter already contains the fully rendered user context because `run_step_chat()` passes `rendered_user_message` as the input.

### Files modified (Part 4)
- **Modify:** `src/server/hub/strategies/chat/mod.rs` — add `user_context_template` to `ChatConfig`
- **Modify:** `src/server/hub/mod.rs` — update `run_step_chat()` to render user context before passing to strategy

---

## Part 5: Testing

### 5a. Template tests

```rust
#[test]
fn test_documenter_user_context_template_has_variables() {
    let template = roles::NODE_ASSISTANT_USER_CONTEXT_DOCUMENTER;
    assert!(template.contains("{{.Context.step_name}}"));
    assert!(template.contains("{{.Context.documents}}"));
    assert!(template.contains("{{.Context.incoming}}"));
    assert!(template.contains("{{.Context.graph}}"));
    assert!(template.contains("{{.User.input}}"));
}

#[test]
fn test_base_user_context_template_has_variables() {
    let template = roles::NODE_ASSISTANT_USER_CONTEXT_BASE;
    assert!(template.contains("{{.Context.graph}}"));
    assert!(template.contains("{{.Context.node_state}}"));
    assert!(template.contains("{{.User.input}}"));
}
```

### 5b. User context rendering tests

```rust
#[test]
fn test_documenter_user_context_renders() {
    let template = roles::NODE_ASSISTANT_USER_CONTEXT_DOCUMENTER;
    let mut vars = HashMap::new();
    vars.insert("Context.step_name".into(), "Product Research".into());
    vars.insert("Context.step_description".into(), "Analyze the PRD".into());
    vars.insert("Context.documents".into(),
        "- API Reference\n  Description: Auth endpoints\n  Target length: ~3000 words\n".into());
    vars.insert("Context.incoming".into(),
        "- Product Requirements (context, populated, ~2400 words)\n".into());
    vars.insert("Context.graph".into(),
        "Workflow nodes:\n  - Product Research (documenter) [SELECTED]\n".into());
    vars.insert("User.input".into(),
        "I want to add a third document for error handling patterns.".into());

    let rendered = resolve_template(template, &vars);

    // User input is at the end
    assert!(rendered.ends_with(
        "I want to add a third document for error handling patterns."
    ));
    // Project state comes before user input
    let input_pos = rendered.find("I want to add").unwrap();
    let docs_pos = rendered.find("API Reference").unwrap();
    assert!(docs_pos < input_pos, "documents should appear before user input");
}
```

### 5c. System prompt no longer contains project state

```rust
#[test]
fn test_system_prompt_excludes_dynamic_state() {
    let template = &roles::NODE_ASSISTANT_BASE.system_template;

    // These should NOT be in the system prompt anymore
    assert!(!template.contains("{{.System.graph_context}}"));
    assert!(!template.contains("{{.System.current_config}}"));

    // Archetype block stays in system prompt
    assert!(template.contains("{{.System.archetype_block}}"));
}
```

### 5d. Integration test — full round-trip

```rust
#[tokio::test]
async fn test_step_chat_renders_user_context() {
    // Given: a documenter step with 2 document definitions and 1 upstream context node
    // When: user sends "Add a third document for error handling"
    // Then: the rendered user message contains:
    //   - Step name and description in <node>
    //   - Both document definitions in <documents>
    //   - Upstream context with status in <incoming_context>
    //   - Graph visualization in <graph>
    //   - User input at the very end
    // And: the system prompt does NOT contain graph_context or current_config
}

#[tokio::test]
async fn test_user_context_updates_after_tool_call() {
    // Given: a documenter step with 1 document
    // When: user says "Add another doc called Error Catalog"
    // And: assistant calls add_document tool
    // And: user sends a follow-up message
    // Then: the re-rendered user context for the follow-up shows 2 documents
    //   (confirming fresh state is loaded each turn)
}
```

### Files created/modified (Part 5)
- **Create:** tests in `src/server/hub/tests.rs` or appropriate test file
- **Create:** tests in `src/config/protocols/tests.rs`

---

## Appendix A: Before/After Comparison

### Before (current — everything in system prompt)

**System prompt (~800-1200 tokens, rebuilt each turn):**
```
<identity>You are the workflow configuration assistant...</identity>

<graph_context>
Workflow nodes:
  - Product Requirements (context)
  - Product Research (documenter) [SELECTED]
  - API Designer (task_force)
Connections:
  Product Requirements -> Product Research
  Product Research -> API Designer
</graph_context>

<archetypes>...</archetypes>

<archetype_context type="documenter">
The documenter runs a three-phase pipeline...
</archetype_context>
<archetype_guidelines>...</archetype_guidelines>

Name: Product Research
Description: Analyze the PRD
Documents:
  - API Reference (target: ~3000 words) — Auth endpoints
Incoming Context:
  - Product Requirements (context, populated) — Preview: "The auth system..."

<guidelines>...</guidelines>
```

**User message:**
```
Add a third document for error handling patterns
```

### After (new — state in user message)

**System prompt (~500-600 tokens, stable across conversation):**
```
<identity>
You are the workflow configuration assistant...
The user provides the current state of their node and workflow at the
start of each message.
</identity>

<archetypes>...</archetypes>

<archetype_context type="documenter">
The documenter runs a three-phase pipeline...
</archetype_context>
<archetype_guidelines>...</archetype_guidelines>

<guidelines>
...
- Reference the project state the user provides at the start of
  their message. It reflects the latest configuration.
</guidelines>
```

**User message (rendered — user sees only what they typed, LLM sees the full context):**
```
Here's where this documenter node is at:

<node>
Name: Product Research
Description: Analyze the PRD and generate reference documents
</node>

<documents>
- API Reference — Authentication Endpoints
  Description: Comprehensive reference for all auth-related REST endpoints
  Target length: ~3000 words

- Data Model Overview
  Description: Entity relationships and field definitions
  Target length: ~1500 words
</documents>

<incoming_context>
- Product Requirements (context, populated, ~2400 words)
  Preview: "The authentication system should support OAuth 2.0 with PKCE flow..."

- Technical Constraints (context, empty)
  No content yet.
</incoming_context>

<graph>
Workflow nodes:
  - Product Requirements (context)
  - Technical Constraints (context)
  - Product Research (documenter) [SELECTED]
  - API Designer (task_force)
Connections:
  Product Requirements -> Product Research
  Technical Constraints -> Product Research
  Product Research -> API Designer
</graph>

Add a third document for error handling patterns
```

### Why this is better

| Property | Before | After |
|----------|--------|-------|
| System prompt size | ~800-1200 tokens (varies per turn) | ~500-600 tokens (stable) |
| Prompt caching | Poor — system prompt changes every turn | Strong — system prompt is cacheable |
| Context attention | Model may deprioritize system-injected state | Model treats user-provided state as ground truth |
| Query position | User input is a separate short message | User input is at the END of a rich context block (+30% quality) |
| State freshness | Rebuilt in system prompt each turn | Rebuilt in user message each turn (same freshness) |

---

## Appendix B: Prompt Caching Impact

With the new pattern, the system prompt becomes **stable across the entire conversation**. It only changes when:
- The archetype switches (user calls `set_node_archetype`)
- Never within a single archetype session

This means Anthropic's prompt caching can cache the system prompt at cache read price (0.1x base). For a typical 10-turn design conversation:

| | Before (system changes each turn) | After (stable system prompt) |
|---|---|---|
| System prompt cache hits | 0 out of 10 | 9 out of 10 |
| Cache savings per conversation | None | ~4,500 cached input tokens × 0.9 discount |

The dynamic state moves to the user message, which is always fresh and not cached — but that's fine because it's the part that SHOULD change each turn.
