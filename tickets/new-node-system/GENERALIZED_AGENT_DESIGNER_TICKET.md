# Generalized Agent Designer — Shared Pre-Lifecycle Across Archetypes

## Overview

Extract the Agent Designer from the task force into a **shared pre-lifecycle module** that any archetype can use. Three archetypes spawn runtime agents — all three benefit from designer-generated prompts:

| Archetype | Runtime Agents | Current Prompts | Problem |
|-----------|---------------|-----------------|---------|
| **Task Force** | User-defined crew | Static template fill | Generic identities, no context awareness |
| **Documenter** | Strategist, Researchers, Writers | One-line templates | `"You are a research assistant"` — no domain specificity |
| **Room** | User-defined members | `agent.system_prompt` + room context concat | Manual string concatenation, no belief curation |

The same Agent Designer — same beliefs, same output schema, same DB tracking — generates prompts for all of them. Each archetype provides its own input (mission brief, document definitions, meeting config) through a shared interface.

**This ticket also replaces Phase 7 (Belief Injection into Rooms).** Instead of manually formatting and appending beliefs to room agent system prompts, beliefs flow through the designer as upstream context. The designer curates which beliefs each room member receives based on their perspective. This is a direct application of [BOCA](../proto/paper.md)'s core insight: a knowledgeable agent (the designer) *authors* curated context per consumer rather than forwarding raw content uniformly.

**Dependency:** Phase 6 (Belief Capture Runtime) must land first. Phase 6 produces beliefs in DB. This ticket consumes them.

---

## Part 1: Extract Shared Agent Designer Module

**Goal:** Move the Agent Designer from `src/server/hub/dag/task_force/designer.rs` into a shared module that any archetype can call.

### 1a. Create module `src/server/hub/agent_designer/mod.rs`

This is the shared entry point. The core `run_agent_designer()` function moves here, generalized to accept any archetype's input.

```rust
pub mod input;
pub mod output;
pub mod strategy;
mod tests;

use input::DesignerInput;
use output::DesignedAgentPrompt;

/// Run the Agent Designer pre-lifecycle.
///
/// Accepts archetype-agnostic input, makes a single LLM call,
/// returns generated (system_prompt, task_prompt) pairs for each agent.
/// Stores results in DB with full token tracking.
///
/// Used by: task_force, documenter, room (and future archetypes).
pub async fn run_agent_designer(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    input: DesignerInput,
    cancel: Option<&CancellationToken>,
) -> Result<DesignerResult, HubError> {
    // 1. Build template variables from the generic input
    let mut vars = HashMap::new();
    vars.insert(designer::ARCHETYPE.to_string(), input.archetype.clone());
    vars.insert(designer::CONTEXT_DESCRIPTION.to_string(), input.context_description.clone());
    vars.insert(designer::AGENT_DEFINITIONS.to_string(), format_agent_definitions(&input.agents));
    vars.insert(designer::UPSTREAM_CONTEXT.to_string(), format_upstream_context(&input.upstream));
    vars.insert(designer::AVAILABLE_TOOLS.to_string(), format_tool_descriptions(&input.available_tools));
    vars.insert(designer::ARCHETYPE_GUIDANCE.to_string(), input.archetype_guidance.clone());

    // 2. Resolve the Agent Designer's own prompts
    let protocol_ctx = AGENT_DESIGNER.resolve(&vars);

    // 3. Create designer run record
    let run_row = state.repo()
        .create_designer_run(
            ctx.workflow_execution_id,
            ctx.stage_execution_id,
            step.id,
            &input.archetype,
            "claude-sonnet-4-20250514",
        )
        .await?;

    // 4. Execute the designer LLM call
    let strategy = AgentDesignerStrategy::new(
        protocol_ctx.system_prompt,
        protocol_ctx.user_prompt,
    );
    let recorder = ExecutionRecorder::new(
        state.repo().as_ref(),
        state.agent_execution_repo().as_deref(),
        state.token_ledger_repo().as_deref(),
    );
    let result = engine.execute(&strategy, "", &NullSink, &recorder, cancel).await?;

    // 5. Track tokens
    let cost = compute_cost("claude-sonnet-4-20250514", result.input_tokens as i64, result.output_tokens as i64);
    state.repo().update_designer_run_tokens(run_row.id, result.input_tokens as i64, result.output_tokens as i64, cost).await?;

    // 6. Parse output
    let parsed: DesignerOutputSchema = serde_json::from_str(&result.content)
        .map_err(|e| HubError::Internal(format!("Agent Designer produced invalid JSON: {e}")))?;

    // 7. Store outputs and build return value
    let mut prompts = Vec::new();
    for (idx, entry) in parsed.agents.iter().enumerate() {
        state.repo().create_designer_output(
            run_row.id,
            &entry.agent_id,
            &input.archetype,
            &entry.agent_name,
            &entry.system_prompt,
            &entry.task_prompt,
            &entry.reasoning,
            idx as i32,
        ).await?;

        prompts.push(DesignedAgentPrompt {
            agent_id: entry.agent_id.clone(),
            agent_name: entry.agent_name.clone(),
            system_prompt: entry.system_prompt.clone(),
            task_prompt: entry.task_prompt.clone(),
            reasoning: entry.reasoning.clone(),
            execution_order: idx as i32,
        });
    }

    Ok(DesignerResult {
        run_id: run_row.id,
        prompts,
        input_tokens: result.input_tokens as i64,
        output_tokens: result.output_tokens as i64,
        cost_usd: cost,
    })
}
```

### 1b. Create `src/server/hub/agent_designer/input.rs`

The archetype-agnostic input struct. Each archetype constructs this differently.

```rust
/// Archetype-agnostic input for the Agent Designer.
/// Each archetype builds this from its own configuration.
#[derive(Debug, Clone)]
pub struct DesignerInput {
    /// Which archetype is requesting design ("task_force", "documenter", "room")
    pub archetype: String,

    /// High-level description of what this execution does.
    /// e.g., "A documenter producing 3 technical reference documents for an auth system"
    pub context_description: String,

    /// The agents that need prompt pairs generated.
    pub agents: Vec<AgentDefinition>,

    /// Upstream context available to all agents.
    /// Includes belief envelopes, context node content, previous step outputs.
    pub upstream: Vec<UpstreamContext>,

    /// Tool descriptions for capabilities these agents may use.
    pub available_tools: Vec<ToolDescription>,

    /// Archetype-specific guidance for the designer.
    /// Extra instructions that vary by archetype (e.g., documenter phase info,
    /// room interaction mode, task force failure mode).
    pub archetype_guidance: String,
}

/// One agent that needs a prompt pair designed.
#[derive(Debug, Clone)]
pub struct AgentDefinition {
    /// Stable identifier — roster entry ID, document def ID, room member ID, or generated UUID
    pub id: String,
    /// Human-readable name for the agent
    pub name: String,
    /// What this agent does
    pub role: String,
    /// What tools/capabilities this agent has access to
    pub capabilities: Vec<String>,
    /// Execution order relative to other agents (0-indexed)
    pub execution_order: i32,
    /// Extra context specific to this agent (e.g., strategist's research_strategy,
    /// room member's perspective, belief subset)
    pub additional_context: String,
}

/// Upstream content available to the agents.
#[derive(Debug, Clone)]
pub struct UpstreamContext {
    /// Name of the upstream source
    pub source_name: String,
    /// Type of upstream (context, documenter, task_force, belief_capture, room)
    pub source_type: String,
    /// The actual content (may be truncated for context budget)
    pub content: String,
}

/// Description of an available tool/capability.
#[derive(Debug, Clone)]
pub struct ToolDescription {
    pub name: String,
    pub description: String,
}
```

### 1c. Create `src/server/hub/agent_designer/output.rs`

```rust
/// Result of a designer run.
#[derive(Debug, Clone)]
pub struct DesignerResult {
    pub run_id: Uuid,
    pub prompts: Vec<DesignedAgentPrompt>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f32,
}

/// One designed prompt pair for one agent.
#[derive(Debug, Clone)]
pub struct DesignedAgentPrompt {
    pub agent_id: String,
    pub agent_name: String,
    pub system_prompt: String,
    pub task_prompt: String,
    pub reasoning: String,
    pub execution_order: i32,
}
```

### Files created (Part 1)
- **Create:** `src/server/hub/agent_designer/mod.rs`
- **Create:** `src/server/hub/agent_designer/input.rs`
- **Create:** `src/server/hub/agent_designer/output.rs`
- **Create:** `src/server/hub/agent_designer/strategy.rs` (moved from task_force/designer.rs)
- **Create:** `src/server/hub/agent_designer/tests.rs`
- **Modify:** `src/server/hub/mod.rs` — add `pub mod agent_designer;`

---

## Part 2: Archetype Input Formatters

**Goal:** Each archetype provides a function that converts its domain-specific configuration into the generic `DesignerInput`. These live alongside the archetype's existing code.

### 2a. Task force formatter — `src/server/hub/dag/task_force/designer_input.rs`

Converts a mission brief + agent roster into `DesignerInput`. This is mostly extracting existing logic from the Agent Designer ticket.

```rust
use crate::server::hub::agent_designer::input::*;

/// Build a DesignerInput from a task force configuration.
pub fn build_task_force_designer_input(
    mission_brief: &TaskMissionBriefRow,
    roster: &[TaskAgentRosterRow],
    upstream_envelopes: &[StepExecutionEnvelope],
) -> DesignerInput {
    DesignerInput {
        archetype: "task_force".to_string(),
        context_description: format!(
            "A task force executing a mission: {}",
            truncate(&mission_brief.task_description, 200),
        ),
        agents: roster.iter().map(|r| AgentDefinition {
            id: r.id.to_string(),
            name: r.name.clone(),
            role: r.role_description.clone(),
            capabilities: r.capabilities.clone(),
            execution_order: r.execution_order,
            additional_context: String::new(),
        }).collect(),
        upstream: format_envelopes_as_upstream(upstream_envelopes),
        available_tools: build_tool_descriptions(&mission_brief.available_capabilities),
        archetype_guidance: format!(
            "Failure mode: {}\n{}",
            mission_brief.failure_mode,
            mission_brief.downstream_context.as_deref().unwrap_or(""),
        ),
    }
}
```

### 2b. Documenter formatter — `src/server/hub/dag/documenter/designer_input.rs`

The documenter has **three phases** with different agent types. The formatter is called differently for each phase.

```rust
use crate::server::hub::agent_designer::input::*;

/// Build DesignerInput for the strategist (Phase 1).
/// Called BEFORE the strategist runs.
pub fn build_strategist_designer_input(
    step: &WorkflowStepRow,
    doc_defs: &[DocumentDefinitionRow],
    upstream_envelopes: &[StepExecutionEnvelope],
    available_capabilities: &[String],
) -> DesignerInput {
    let docs_summary = doc_defs.iter()
        .map(|d| format!(
            "- {} (~{} words): {}",
            d.name,
            d.target_length.unwrap_or(1500),
            d.description.as_deref().unwrap_or("No description"),
        ))
        .collect::<Vec<_>>()
        .join("\n");

    DesignerInput {
        archetype: "documenter".to_string(),
        context_description: format!(
            "A documenter node producing {} reference documents. The strategist plans \
             research strategies and writing instructions for each document.",
            doc_defs.len(),
        ),
        agents: vec![AgentDefinition {
            id: Uuid::new_v4().to_string(),
            name: "Document Strategist".to_string(),
            role: "Analyzes the task and upstream context to produce a research strategy \
                   and detailed writing instructions for each requested document.".to_string(),
            capabilities: vec![], // Strategist has no tools — reasoning only
            execution_order: 0,
            additional_context: format!(
                "Requested documents:\n{}\n\nThe strategist's output is a structured JSON \
                 with document_plans containing research_strategy, required_capabilities, \
                 and writer_prompt per document. This output directly drives the research \
                 and writing phases.",
                docs_summary,
            ),
        }],
        upstream: format_envelopes_as_upstream(upstream_envelopes),
        available_tools: build_tool_descriptions(available_capabilities),
        archetype_guidance: format!(
            "This is Phase 1 of a three-phase documenter pipeline:\n\
             Phase 1 (Strategist): Plans research and writing — this is the agent being designed\n\
             Phase 2 (Researchers): Execute the strategist's research plans using tools\n\
             Phase 3 (Writers): Produce final documents from research findings\n\n\
             The strategist must produce a JSON response with a document_plans array. \
             Each plan needs: document_name, research_strategy, required_capabilities, \
             writer_prompt, and optional context_document_ids.\n\n\
             Documents being produced:\n{}",
            docs_summary,
        ),
    }
}

/// Build DesignerInput for researchers + writers (Phase 2 & 3).
/// Called AFTER the strategist runs, using its document_plans output.
///
/// Generates agents for ALL documents in one call:
/// - One researcher per document
/// - One writer per document
/// Ordered as: researcher_1, researcher_2, ..., writer_1, writer_2, ...
pub fn build_research_write_designer_input(
    step: &WorkflowStepRow,
    document_plans: &[DocumentPlan],
    upstream_envelopes: &[StepExecutionEnvelope],
    available_capabilities: &[String],
) -> DesignerInput {
    let mut agents = Vec::new();

    // Researchers — one per document
    for (idx, plan) in document_plans.iter().enumerate() {
        agents.push(AgentDefinition {
            id: format!("researcher:{}", plan.document_name),
            name: format!("Researcher: {}", plan.document_name),
            role: format!(
                "Gathers information for the document '{}' using available tools.",
                plan.document_name,
            ),
            capabilities: plan.required_capabilities.clone(),
            execution_order: idx as i32,
            additional_context: format!(
                "Research strategy from the strategist:\n{}\n\n\
                 This researcher's findings will be passed to the writer. \
                 Summarize findings clearly — the writer depends on comprehensive, \
                 well-organized research output.",
                plan.research_strategy,
            ),
        });
    }

    // Writers — one per document, ordered after all researchers
    let researcher_count = document_plans.len();
    for (idx, plan) in document_plans.iter().enumerate() {
        agents.push(AgentDefinition {
            id: format!("writer:{}", plan.document_name),
            name: format!("Writer: {}", plan.document_name),
            role: format!(
                "Produces the final document '{}' from research findings.",
                plan.document_name,
            ),
            capabilities: vec![], // Writers have no tools
            execution_order: (researcher_count + idx) as i32,
            additional_context: format!(
                "Writing instructions from the strategist:\n{}\n\n\
                 The researcher's findings will be provided as input. \
                 Produce a well-structured, comprehensive document in markdown format.",
                plan.writer_prompt,
            ),
        });
    }

    DesignerInput {
        archetype: "documenter".to_string(),
        context_description: format!(
            "Phase 2 & 3 of a documenter pipeline. The strategist has produced plans \
             for {} documents. Researchers gather information, then writers produce \
             final documents. Researchers and writers execute in parallel within their phase.",
            document_plans.len(),
        ),
        agents,
        upstream: format_envelopes_as_upstream(upstream_envelopes),
        available_tools: build_tool_descriptions(available_capabilities),
        archetype_guidance: format!(
            "Researchers run in parallel (Phase 2), then writers run in parallel (Phase 3).\n\
             Each researcher's output feeds into the corresponding writer.\n\
             Researchers have tools; writers do not — they synthesize from research findings.\n\n\
             The strategist has already planned the research strategies and writing instructions. \
             Each agent's additional_context contains the strategist's specific guidance for them. \
             The designer should enrich the prompts with identity specificity and domain awareness \
             while preserving the strategist's intent.",
        ),
    }
}
```

### 2c. Room formatter — `src/server/hub/dag/room/designer_input.rs`

Converts room configuration + members + beliefs into `DesignerInput`.

```rust
use crate::server::hub::agent_designer::input::*;

/// Build DesignerInput for room members.
/// Includes beliefs from upstream belief_capture nodes when available.
///
/// Called before room execution starts. The designer generates system
/// prompts for each member; the user prompt (transcript) is built
/// per-turn by the room executor.
pub fn build_room_designer_input(
    room: &RoomRow,
    members: &[RoomMemberWithAgent],
    beliefs: &[BeliefRow],
    upstream_envelopes: &[StepExecutionEnvelope],
) -> DesignerInput {
    let mut agents: Vec<AgentDefinition> = Vec::new();

    for (idx, ma) in members.iter().enumerate() {
        let member = &ma.member;
        let agent = &ma.agent;

        // Build per-member belief context
        // The designer will decide how to incorporate these based on perspective
        let belief_context = if beliefs.is_empty() {
            String::new()
        } else {
            let formatted = beliefs.iter()
                .map(|b| format!(
                    "- \"{}\" ({}, {} confidence, source: {})",
                    b.content,
                    b.belief_type,
                    b.confidence,
                    b.source_step_name,
                ))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "Beliefs extracted from upstream analysis:\n{}\n\n\
                 Incorporate these beliefs into this member's prompt based on their \
                 perspective. Not all beliefs are equally relevant to every member — \
                 curate based on the member's role and expertise.",
                formatted,
            )
        };

        // Combine agent's existing system prompt + perspective + beliefs
        let additional = format!(
            "Base persona:\n{}\n\n\
             Room perspective/role:\n{}\n\n\
             {}",
            agent.system_prompt,
            member.role_description,
            belief_context,
        );

        agents.push(AgentDefinition {
            id: member.agent_id.to_string(),
            name: member.display_name.clone()
                .unwrap_or_else(|| agent.name.clone()),
            role: member.role_description.clone(),
            capabilities: vec![], // Room members typically have no tools
            execution_order: idx as i32,
            additional_context: additional,
        });
    }

    let interaction_mode = room.interaction_mode.as_deref().unwrap_or("moderated");
    let max_turns = room.max_turns.unwrap_or(12);

    DesignerInput {
        archetype: "room".to_string(),
        context_description: format!(
            "A room meeting with {} members. Purpose: {}",
            members.len(),
            room.purpose.as_deref().unwrap_or("General discussion"),
        ),
        agents,
        upstream: format_envelopes_as_upstream(upstream_envelopes),
        available_tools: vec![], // Rooms typically don't use tools
        archetype_guidance: format!(
            "This is a room — a meeting space where agents discuss, debate, or review.\n\n\
             Meeting purpose: {}\n\
             Interaction mode: {}\n\
             Max turns: {}\n\n\
             Room-specific design guidance:\n\
             - Each member's system prompt should establish their perspective and expertise\n\
             - Members should know who else is in the room and what perspectives they bring\n\
             - Include collaborative framing: \"build on what others have said\", \"be concise \
               and additive\"\n\
             - For the task prompt: write a brief orientation that sets the scene for the \
               discussion. The room executor will append the transcript and user message \
               at runtime — the task prompt here is just the opening framing.\n\
             - If beliefs are provided in a member's additional_context, curate them per-member: \
               a security architect should see security-relevant beliefs prominently, while a \
               product manager should see UX-relevant beliefs prominently. All members can see \
               all beliefs, but emphasis and ordering should match their perspective.\n\
             - Members with \"moderated\" interaction mode should defer to the moderator's \
               direction. Members with \"open\" mode can speak freely.",
            room.purpose.as_deref().unwrap_or("General discussion"),
            interaction_mode,
            max_turns,
        ),
    }
}
```

### 2d. Shared upstream formatter — `src/server/hub/agent_designer/input.rs`

Add shared utility functions used by all archetype formatters.

```rust
/// Convert step execution envelopes into generic upstream context.
/// Used by all archetype formatters.
pub fn format_envelopes_as_upstream(envelopes: &[StepExecutionEnvelope]) -> Vec<UpstreamContext> {
    if envelopes.is_empty() {
        return vec![UpstreamContext {
            source_name: "none".to_string(),
            source_type: "none".to_string(),
            content: "No upstream outputs available. This is the first step in the workflow.".to_string(),
        }];
    }

    envelopes.iter().map(|env| UpstreamContext {
        source_name: env.step_name.clone(),
        source_type: env.execution_mode.clone(),
        content: truncate_for_context(&env.data.to_string(), 4000),
    }).collect()
}

/// Convert capability names into tool descriptions.
pub fn build_tool_descriptions(capabilities: &[String]) -> Vec<ToolDescription> {
    capabilities.iter().map(|cap| {
        let desc = match cap.as_str() {
            "file_read" => "Read file contents from the repository",
            "file_write" => "Create or modify files in the repository",
            "grep" => "Search file contents with regex patterns",
            "shell" => "Execute shell commands in a sandboxed environment",
            "git" => "Run git operations (status, diff, log, commit, branch)",
            "github_api" => "Interact with GitHub API (issues, PRs, reviews)",
            "web_search" => "Search the web for information",
            "database_query" => "Execute read-only SQL queries",
            other => other,
        };
        ToolDescription {
            name: cap.clone(),
            description: desc.to_string(),
        }
    }).collect()
}

fn truncate_for_context(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { s[..max].to_string() }
}
```

### Files created (Part 2)
- **Create:** `src/server/hub/dag/task_force/designer_input.rs`
- **Create:** `src/server/hub/dag/documenter/designer_input.rs`
- **Create:** `src/server/hub/dag/room/designer_input.rs` (or alongside room executor)
- **Modify:** `src/server/hub/agent_designer/input.rs` — add shared formatters

---

## Part 3: Documenter Integration

**Goal:** Replace the documenter's static one-line templates with designer-generated prompts. Two designer calls per execution: one before the strategist, one before researchers + writers.

### 3a. Execution flow change

**Current flow (static templates):**
```
1. Load doc definitions + upstream context
2. Resolve strategist template: "You are a Document Strategist..."
3. Strategist runs → document_plans JSON
4. For each document (parallel):
   a. Resolve researcher template: "You are a research assistant..."
   b. Researcher runs with research_strategy as input → findings
5. For each document (parallel):
   a. Resolve writer template: "You are a technical writer..."
   b. Writer runs with writer_prompt + findings → document
```

**New flow (designer-generated prompts):**
```
1. Load doc definitions + upstream context

2. DESIGNER CALL #1 — Strategist
   a. Build DesignerInput via build_strategist_designer_input()
   b. Call run_agent_designer() → DesignerResult with 1 prompt pair
   c. Extract strategist's (system_prompt, task_prompt)

3. Strategist runs with designed prompts → document_plans JSON

4. DESIGNER CALL #2 — Researchers + Writers (batched)
   a. Build DesignerInput via build_research_write_designer_input()
      - Includes strategist's research_strategy and writer_prompt per document
      - Generates 2N agents (N researchers + N writers) in one call
   b. Call run_agent_designer() → DesignerResult with 2N prompt pairs

5. For each document (parallel):
   a. Find researcher prompt by id "researcher:{doc_name}"
   b. Researcher runs with designed system_prompt
      - Task prompt = designed task_prompt (enriches strategist's research_strategy)
      - Plus: selected context documents if any
   c. Researcher produces findings

6. For each document (parallel):
   a. Find writer prompt by id "writer:{doc_name}"
   b. Writer runs with designed system_prompt
      - Task prompt = designed task_prompt (enriches strategist's writer_prompt)
      - Plus: research findings appended
   c. Writer produces document
```

### 3b. Modify `src/server/hub/dag/documenter/phases.rs`

**Phase 1 (execute_strategy_phase):**

Before creating the `DocumenterCoordinatorStrategy`, call the Agent Designer:

```rust
// BEFORE (static template):
let protocol_ctx = roles::DOCUMENTER_STRATEGIST.resolve(&system_vars);
let strategy = DocumenterCoordinatorStrategy::new(config);

// AFTER (designer-generated):
let designer_input = build_strategist_designer_input(step, &doc_defs, &upstream_envelopes, &capabilities);
let designer_result = run_agent_designer(engine, state, ctx, step, designer_input, cancel).await?;
let strategist_prompt = &designer_result.prompts[0];

let config = DocumenterCoordinatorConfig {
    system_prompt: strategist_prompt.system_prompt.clone(),
    user_prompt: strategist_prompt.task_prompt.clone(),
    // ... rest unchanged
};
let strategy = DocumenterCoordinatorStrategy::new(config);
```

**Phase 2 & 3 (execute_research_phase + execute_write_phase):**

After the strategist produces document_plans, call the designer for all researchers and writers:

```rust
// After strategist produces document_plans:
let rw_input = build_research_write_designer_input(step, &document_plans, &upstream_envelopes, &capabilities);
let rw_result = run_agent_designer(engine, state, ctx, step, rw_input, cancel).await?;

// Build lookup by agent_id
let prompt_lookup: HashMap<String, &DesignedAgentPrompt> = rw_result.prompts.iter()
    .map(|p| (p.agent_id.clone(), p))
    .collect();

// Phase 2 — Researchers (parallel)
for plan in &document_plans {
    let researcher_prompt = prompt_lookup
        .get(&format!("researcher:{}", plan.document_name))
        .expect("designer produced prompt for this researcher");

    let config = DocumenterResearchConfig {
        system_prompt: researcher_prompt.system_prompt.clone(),
        user_prompt: format!(
            "{}\n\n{}",  // Designer's enriched task + selected context
            researcher_prompt.task_prompt,
            selected_context,
        ),
        tools: resolve_capabilities(&plan.required_capabilities),
        // ... rest unchanged
    };
    // Spawn researcher task
}

// Phase 3 — Writers (parallel, after researchers complete)
for (plan, findings) in plans_with_findings {
    let writer_prompt = prompt_lookup
        .get(&format!("writer:{}", plan.document_name))
        .expect("designer produced prompt for this writer");

    let config = DocumenterWriterConfig {
        system_prompt: writer_prompt.system_prompt.clone(),
        user_prompt: format!(
            "{}\n\n---\n\nResearch findings:\n{}",
            writer_prompt.task_prompt,
            findings,
        ),
        // ... rest unchanged
    };
    // Spawn writer task
}
```

### 3c. Token tracking

The documenter execution now includes designer costs:
- Designer Call #1 (strategist): tracked in `agent_designer_runs`
- Designer Call #2 (researchers + writers): tracked in `agent_designer_runs`
- Plus the actual agent execution costs (unchanged)

Total cost per documenter execution = designer_1 + strategist + designer_2 + N×researcher + N×writer

### 3d. Fallback behavior

If the Agent Designer call fails (LLM error, parse error), fall back to the existing static templates. This ensures the documenter remains functional even if the designer has issues.

```rust
let prompts = match run_agent_designer(engine, state, ctx, step, input, cancel).await {
    Ok(result) => result.prompts,
    Err(e) => {
        tracing::warn!("Agent Designer failed, falling back to static templates: {e}");
        return execute_with_static_templates(/* existing code path */);
    }
};
```

### Files modified (Part 3)
- **Modify:** `src/server/hub/dag/documenter/phases.rs` — integrate designer calls into each phase
- **Create:** `src/server/hub/dag/documenter/designer_input.rs` (from Part 2)
- **Modify:** `src/server/hub/strategies/documenter/coordinator.rs` — accept custom system/user prompts
- **Modify:** `src/server/hub/strategies/documenter/research.rs` — accept custom system/user prompts
- **Modify:** `src/server/hub/strategies/documenter/writer.rs` — accept custom system/user prompts

---

## Part 4: Room Integration + Belief Flow (Replaces Phase 7)

**Goal:** Room members get designer-generated system prompts that include curated beliefs. The designer decides which beliefs each member should see most prominently based on their perspective.

### 4a. What Phase 7 currently says (and why we're replacing it)

Phase 7 as planned:
> "In system prompt construction, check for upstream belief capture steps. Load beliefs for the current execution, format, and append to agent system prompts."

This is **manual concatenation** — format all beliefs as text, append to every member's system prompt identically. Problems:
- Every member sees the same belief dump regardless of their perspective
- No curation — a security architect and a product manager get identical belief context
- Formatting is hardcoded, not adapting to the meeting's purpose
- Beliefs are stuffed into the system prompt rather than the user message

With the Agent Designer:
- Beliefs flow through as `additional_context` on each `AgentDefinition`
- The designer sees ALL beliefs and each member's perspective
- The designer curates: a security architect's prompt emphasizes security-relevant beliefs, a PM's prompt emphasizes UX-relevant beliefs
- Beliefs go into the task prompt (user message), following the user-as-authority pattern
- The system prompt stays lean (identity + behavioral guidance)

### 4b. Execution flow change

**Current room execution (no beliefs, no designer):**
```
1. Load room + members + agents
2. For each speaker turn:
   a. system_prompt = agent.system_prompt + build_room_context() + agent_docs
   b. user_prompt = build_speaker_prompt(transcript + message + gatekeeper_note)
   c. Execute via RoomSpeakerStrategy
```

**New room execution (with designer + beliefs):**
```
1. Load room + members + agents
2. Load beliefs from upstream belief_capture steps (if any)

3. DESIGNER CALL — Room Members
   a. Build DesignerInput via build_room_designer_input(room, members, beliefs, upstream)
   b. Call run_agent_designer() → DesignerResult with N prompt pairs (one per member)
   c. Store designed prompts in a lookup by agent_id

4. For each speaker turn:
   a. system_prompt = designed_prompt.system_prompt  (replaces manual concatenation)
   b. user_prompt = designed_prompt.task_prompt + build_speaker_prompt(transcript + message)
      - Designer's task prompt provides the opening framing
      - Room executor appends the live transcript + user message
   c. Execute via RoomSpeakerStrategy (unchanged interface)
```

### 4c. Modify `src/server/executors/room/mod.rs`

The designer call happens **once** before the room loop starts, not per-turn. Generated prompts are reused across turns.

```rust
pub async fn execute_room(
    state: &AppState,
    room: &RoomRow,
    session: &RoomSessionRow,
    user_message: &str,
    // ...
) -> Result<(), RoomError> {
    let members = state.repo().get_room_members_with_agents(room.id).await?;

    // Load beliefs from upstream belief_capture steps
    let beliefs = load_upstream_beliefs(state, room).await?;

    // Run Agent Designer for all room members
    let designer_input = build_room_designer_input(room, &members, &beliefs, &upstream_envelopes);
    let designer_result = match run_agent_designer(engine, state, ctx, step, designer_input, cancel).await {
        Ok(result) => result,
        Err(e) => {
            tracing::warn!("Agent Designer failed for room, falling back: {e}");
            return execute_room_with_static_prompts(/* existing path */);
        }
    };

    // Build prompt lookup
    let prompt_lookup: HashMap<String, &DesignedAgentPrompt> = designer_result.prompts.iter()
        .map(|p| (p.agent_id.clone(), p))
        .collect();

    // Room turn loop (existing structure, updated prompt source)
    loop {
        let selection = select_next_speaker(/* ... */).await?;

        let designed = prompt_lookup
            .get(&selection.agent_id.to_string())
            .expect("designer produced prompt for this member");

        // System prompt from designer (replaces agent.system_prompt + room_context + docs)
        let system_prompt = designed.system_prompt.clone();

        // User prompt: designer's framing + live transcript + user message
        let transcript = format_transcript(&session_messages, &members);
        let speaker_input = build_speaker_prompt(user_message, &followup, &transcript);
        let user_prompt = format!(
            "{}\n\n---\n\n{}",
            designed.task_prompt,
            speaker_input,
        );

        // Execute (unchanged interface)
        let config = RoomSpeakerConfig {
            system_prompt,
            user_prompt,
            // ... rest unchanged
        };

        // ... existing speaker execution logic
    }
}
```

### 4d. Belief loading utility

```rust
/// Load beliefs from upstream belief_capture steps for this room.
/// Returns empty vec if no upstream beliefs exist (Phase 6 not yet executed,
/// or no belief_capture nodes upstream).
async fn load_upstream_beliefs(
    state: &AppState,
    room: &RoomRow,
) -> Result<Vec<BeliefRow>, RoomError> {
    // Find upstream belief_capture steps via workflow edges
    let step_id = room.step_id; // Room's workflow step
    let workflow_id = room.workflow_id;

    let edges = state.repo().get_workflow_edges(workflow_id).await?;
    let upstream_step_ids: Vec<Uuid> = edges.iter()
        .filter(|e| e.target_step_id == step_id)
        .map(|e| e.source_step_id)
        .collect();

    let mut all_beliefs = Vec::new();
    for upstream_id in &upstream_step_ids {
        let upstream_step = state.repo().get_workflow_step(*upstream_id).await?;
        if upstream_step.execution_mode == "belief_capture" {
            // Load beliefs produced by this step in the current execution
            let beliefs = state.repo()
                .get_beliefs_by_step(*upstream_id)
                .await
                .unwrap_or_default();
            all_beliefs.extend(beliefs);
        }
    }

    Ok(all_beliefs)
}
```

### Files modified (Part 4)
- **Modify:** `src/server/executors/room/mod.rs` — add designer call before room loop, replace manual prompt concatenation
- **Create:** `src/server/hub/dag/room/designer_input.rs` (from Part 2)
- **Delete (conceptually):** Phase 7 as a standalone phase — it's now embedded here

---

## Part 5: Update DB Schema

**Goal:** The Agent Designer ticket defined `agent_designer_runs` and `agent_designer_outputs` with task-force-specific references. Generalize the schema to work across archetypes.

### 5a. Updated migration

Replace the task-force-specific schema from the Agent Designer ticket with:

```sql
CREATE TABLE agent_designer_runs (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_execution_id uuid NOT NULL,
    stage_execution_id uuid NOT NULL,
    step_id uuid NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    archetype text NOT NULL,            -- 'task_force', 'documenter', 'room'
    phase text NOT NULL DEFAULT '',     -- 'main', 'strategist', 'research_write', etc.
    model_id text NOT NULL,
    input_tokens bigint NOT NULL DEFAULT 0,
    output_tokens bigint NOT NULL DEFAULT 0,
    cost_usd real NOT NULL DEFAULT 0.0,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE agent_designer_outputs (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    designer_run_id uuid NOT NULL REFERENCES agent_designer_runs(id) ON DELETE CASCADE,
    source_entity_id text NOT NULL DEFAULT '',  -- roster entry UUID, "researcher:doc_name", member UUID
    source_archetype text NOT NULL,             -- 'task_force', 'documenter', 'room'
    agent_name text NOT NULL,
    generated_system_prompt text NOT NULL,
    generated_task_prompt text NOT NULL,
    design_reasoning text NOT NULL DEFAULT '',
    execution_order integer NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_designer_runs_step ON agent_designer_runs(step_id);
CREATE INDEX idx_designer_runs_execution ON agent_designer_runs(workflow_execution_id);
CREATE INDEX idx_designer_outputs_run ON agent_designer_outputs(designer_run_id);
```

**Key changes from the original Agent Designer ticket schema:**
- `mission_brief_id` removed — not all archetypes have mission briefs
- `archetype` added — identifies which archetype requested the design
- `phase` added — documenter has multiple designer calls per execution (strategist vs research_write)
- `agent_roster_entry_id` replaced with `source_entity_id` (text) — soft reference, works for any archetype
- `source_archetype` added — for querying outputs by archetype type

### 5b. Updated row types

```rust
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AgentDesignerRunRow {
    pub id: Uuid,
    pub workflow_execution_id: Uuid,
    pub stage_execution_id: Uuid,
    pub step_id: Uuid,
    pub archetype: String,
    pub phase: String,
    pub model_id: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AgentDesignerOutputRow {
    pub id: Uuid,
    pub designer_run_id: Uuid,
    pub source_entity_id: String,
    pub source_archetype: String,
    pub agent_name: String,
    pub generated_system_prompt: String,
    pub generated_task_prompt: String,
    pub design_reasoning: String,
    pub execution_order: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
```

### 5c. Updated repository trait

```rust
#[async_trait]
pub trait AgentDesignerRepo: Send + Sync {
    async fn create_designer_run(
        &self,
        workflow_execution_id: Uuid,
        stage_execution_id: Uuid,
        step_id: Uuid,
        archetype: &str,
        model_id: &str,
    ) -> Result<AgentDesignerRunRow>;

    async fn update_designer_run_tokens(
        &self,
        run_id: Uuid,
        input_tokens: i64,
        output_tokens: i64,
        cost_usd: f32,
    ) -> Result<()>;

    async fn create_designer_output(
        &self,
        designer_run_id: Uuid,
        source_entity_id: &str,
        source_archetype: &str,
        agent_name: &str,
        generated_system_prompt: &str,
        generated_task_prompt: &str,
        design_reasoning: &str,
        execution_order: i32,
    ) -> Result<AgentDesignerOutputRow>;

    async fn get_designer_outputs_for_run(
        &self,
        designer_run_id: Uuid,
    ) -> Result<Vec<AgentDesignerOutputRow>>;

    async fn get_designer_runs_for_step(
        &self,
        step_id: Uuid,
        workflow_execution_id: Uuid,
    ) -> Result<Vec<AgentDesignerRunRow>>;
}
```

### Files modified (Part 5)
- **Modify:** migration file from Agent Designer ticket — use generalized schema
- **Modify:** `src/db/mod.rs` — updated row types
- **Modify:** `src/db/traits/mod.rs` — updated trait
- **Modify:** `src/db/pg_repo/mod.rs` — updated implementations

---

## Part 6: Update Agent Designer Protocol Prompt

**Goal:** The Agent Designer's system prompt (from the Agent Designer ticket) needs a small update to handle multiple archetypes, not just task forces.

### 6a. Update `config/protocols/agent_designer/prompt.md`

Replace the task-force-specific user prompt template with an archetype-agnostic one:

```markdown
<archetype>{{.Designer.archetype}}</archetype>

<context>
{{.Designer.context_description}}
</context>

<agents>
{{.Designer.agent_definitions}}
</agents>

<upstream>
{{.Designer.upstream_context}}
</upstream>

<available_tools>
{{.Designer.available_tools}}
</available_tools>

<archetype_guidance>
{{.Designer.archetype_guidance}}
</archetype_guidance>

Design the (system prompt, task prompt) pair for each agent listed above.
Each agent's task prompt should be written as a direct, contextual work
assignment — as if a knowledgeable team lead is handing them a brief.
```

### 6b. Update template variable keys in `src/config/protocols.rs`

```rust
pub mod designer {
    pub const ARCHETYPE: &str = "Designer.archetype";
    pub const CONTEXT_DESCRIPTION: &str = "Designer.context_description";
    pub const AGENT_DEFINITIONS: &str = "Designer.agent_definitions";
    pub const UPSTREAM_CONTEXT: &str = "Designer.upstream_context";
    pub const AVAILABLE_TOOLS: &str = "Designer.available_tools";
    pub const ARCHETYPE_GUIDANCE: &str = "Designer.archetype_guidance";
}
```

### Files modified (Part 6)
- **Modify:** `config/protocols/agent_designer/prompt.md`
- **Modify:** `src/config/protocols.rs` — updated variable keys

---

## Part 7: Testing

### 7a. Shared module tests — `src/server/hub/agent_designer/tests.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // --- Input formatting ---

    #[test]
    fn test_format_agent_definitions_produces_numbered_list() {
        let agents = vec![
            AgentDefinition {
                id: "abc".into(),
                name: "Scanner".into(),
                role: "Scans files".into(),
                capabilities: vec!["file_read".into(), "grep".into()],
                execution_order: 0,
                additional_context: String::new(),
            },
        ];
        let formatted = format_agent_definitions(&agents);
        assert!(formatted.contains("1. Scanner"));
        assert!(formatted.contains("file_read, grep"));
    }

    #[test]
    fn test_format_envelopes_as_upstream_empty() {
        let result = format_envelopes_as_upstream(&[]);
        assert_eq!(result.len(), 1);
        assert!(result[0].content.contains("No upstream"));
    }

    #[test]
    fn test_build_tool_descriptions_known_capabilities() {
        let tools = build_tool_descriptions(&["file_read".into(), "grep".into()]);
        assert_eq!(tools.len(), 2);
        assert!(tools[0].description.contains("Read file contents"));
    }

    #[test]
    fn test_build_tool_descriptions_unknown_capability() {
        let tools = build_tool_descriptions(&["custom_tool".into()]);
        assert_eq!(tools[0].description, "custom_tool");
    }
}
```

### 7b. Documenter designer input tests — `src/server/hub/dag/documenter/tests.rs`

```rust
#[test]
fn test_build_strategist_designer_input() {
    let doc_defs = vec![mock_doc_def("API Reference", 3000)];
    let input = build_strategist_designer_input(&mock_step(), &doc_defs, &[], &["file_read".into()]);

    assert_eq!(input.archetype, "documenter");
    assert_eq!(input.agents.len(), 1);
    assert_eq!(input.agents[0].name, "Document Strategist");
    assert!(input.agents[0].additional_context.contains("API Reference"));
    assert!(input.archetype_guidance.contains("Phase 1"));
}

#[test]
fn test_build_research_write_designer_input() {
    let plans = vec![
        mock_document_plan("API Reference", "Search for endpoints...", "Write in reference style..."),
        mock_document_plan("Data Model", "Examine schema files...", "Write entity descriptions..."),
    ];
    let input = build_research_write_designer_input(&mock_step(), &plans, &[], &["file_read".into()]);

    assert_eq!(input.agents.len(), 4); // 2 researchers + 2 writers
    assert_eq!(input.agents[0].id, "researcher:API Reference");
    assert_eq!(input.agents[1].id, "researcher:Data Model");
    assert_eq!(input.agents[2].id, "writer:API Reference");
    assert_eq!(input.agents[3].id, "writer:Data Model");
    assert!(input.agents[0].additional_context.contains("Search for endpoints"));
    assert!(input.agents[2].additional_context.contains("Write in reference style"));
}
```

### 7c. Room designer input tests — `src/server/hub/dag/room/tests.rs`

```rust
#[test]
fn test_build_room_designer_input_without_beliefs() {
    let room = mock_room("Budget Review");
    let members = vec![
        mock_member_with_agent("Alice", "Finance Director"),
        mock_member_with_agent("Bob", "Engineering Manager"),
    ];
    let input = build_room_designer_input(&room, &members, &[], &[]);

    assert_eq!(input.archetype, "room");
    assert_eq!(input.agents.len(), 2);
    assert!(input.archetype_guidance.contains("Budget Review"));
    // No belief context
    assert!(!input.agents[0].additional_context.contains("Beliefs extracted"));
}

#[test]
fn test_build_room_designer_input_with_beliefs() {
    let beliefs = vec![
        mock_belief("OAuth 2.0 PKCE flow is recommended for mobile clients", "fact", "high"),
        mock_belief("Rate limiting should be per-user", "decision", "high"),
    ];
    let input = build_room_designer_input(&mock_room("Security Review"), &mock_members(), &beliefs, &[]);

    // Both members should have beliefs in their additional_context
    for agent in &input.agents {
        assert!(agent.additional_context.contains("Beliefs extracted"));
        assert!(agent.additional_context.contains("OAuth 2.0 PKCE"));
    }
    // Archetype guidance should mention belief curation
    assert!(input.archetype_guidance.contains("curate them per-member"));
}

#[test]
fn test_room_designer_input_includes_member_perspectives() {
    let members = vec![
        mock_member_with_agent("Security Architect", "Evaluates for vulnerabilities"),
        mock_member_with_agent("Product Manager", "Ensures UX quality"),
    ];
    let input = build_room_designer_input(&mock_room("Review"), &members, &[], &[]);

    assert!(input.agents[0].additional_context.contains("Evaluates for vulnerabilities"));
    assert!(input.agents[1].additional_context.contains("Ensures UX quality"));
}
```

### 7d. Integration tests

```rust
#[tokio::test]
async fn test_documenter_with_designer_produces_richer_prompts() {
    // Given: a mock engine that returns valid designer JSON
    // And: 2 document definitions
    // When: documenter executes with designer
    // Then: 2 designer calls happen (strategist + research_write)
    // And: researcher system prompts are richer than "You are a research assistant"
    // And: writer system prompts are richer than "You are a technical writer"
    // And: total token count includes designer overhead
}

#[tokio::test]
async fn test_room_with_beliefs_flows_through_designer() {
    // Given: a room with 3 members
    // And: 5 beliefs from upstream belief_capture
    // When: room executes with designer
    // Then: 1 designer call happens (all 3 members)
    // And: each member's system prompt is unique to their perspective
    // And: beliefs appear in generated prompts (curated per member)
}

#[tokio::test]
async fn test_designer_fallback_on_failure() {
    // Given: a mock engine that returns an error
    // When: documenter/room/task_force tries to run designer
    // Then: falls back to static templates
    // And: execution completes successfully
    // And: warning is logged
}

#[tokio::test]
async fn test_designer_cost_tracking_across_archetypes() {
    // Given: a task_force step + a documenter step in the same workflow
    // When: both execute with designer
    // Then: agent_designer_runs has entries with correct archetype values
    // And: agent_designer_outputs has correct source_archetype values
    // And: token ledger includes designer costs
}
```

### Files created/modified (Part 7)
- **Create:** `src/server/hub/agent_designer/tests.rs`
- **Modify:** `src/server/hub/dag/documenter/tests.rs` — add designer input tests
- **Create:** `src/server/hub/dag/room/tests.rs` (or add to existing room test file)

---

## Appendix A: Designer Call Count Per Archetype

| Archetype | Designer Calls | Agents Per Call | Total Agents Designed |
|-----------|---------------|-----------------|----------------------|
| **Task Force** | 1 | N (roster size) | N |
| **Documenter** | 2 | 1 (strategist) + 2N (researchers + writers) | 1 + 2N |
| **Room** | 1 | M (member count) | M |

For a typical workflow with 1 task force (5 agents) + 1 documenter (3 docs) + 1 room (4 members):
- Designer calls: 1 + 2 + 1 = **4 calls**
- Agents designed: 5 + 7 + 4 = **16 prompt pairs**
- Designer cost: ~4 × (1200 system + 800 user input + 2000 output) ≈ **16K tokens**

The designer overhead is small relative to the agent execution cost (16K designer tokens vs ~200K+ agent execution tokens for 16 agents).

---

## Appendix B: Phase 7 Replacement Mapping

| Phase 7 (Original Plan) | This Ticket (Replacement) |
|--------------------------|--------------------------|
| "Check for upstream belief capture steps" | `load_upstream_beliefs()` in room executor |
| "Load beliefs for the current execution" | Same — query beliefs table by step + execution |
| "Format and append to agent system prompts" | Pass beliefs to designer as `additional_context` per member |
| `format_beliefs_for_mask()` shared utility | Not needed — designer formats beliefs contextually |
| Manual string concatenation | Designer generates complete system prompts |
| All members see identical beliefs | Designer curates beliefs per member's perspective |

---

## Appendix C: Example — Room With Belief Curation

**Setup:** Security Review room with 3 members, 5 upstream beliefs.

**Beliefs from Phase 6:**
1. "OAuth 2.0 PKCE flow is recommended for mobile clients" (fact, high)
2. "Session-based auth requires HttpOnly + Secure cookie flags" (fact, high)
3. "Rate limiting should be per-user, not per-IP" (decision, high)
4. "The current login page has a 2.3s load time on mobile" (observation, medium)
5. "JWT refresh tokens should have a 7-day expiry" (decision, medium)

**Designer Output — Security Architect:**
```
System: You are a security architect specializing in authentication system
design. You evaluate proposed designs for vulnerability patterns, attack
surfaces, and compliance with security best practices.

You are in a group discussion with two other participants: a Product Manager
(focused on user experience) and a DevOps Lead (focused on operational
concerns). Build on their points when relevant, but prioritize security
considerations.

Be concise and additive — contribute security-specific insights that others
may not raise.

Task: <context>
You're reviewing the proposed authentication architecture for a web + mobile
application.

Key security-relevant findings from prior analysis:
- OAuth 2.0 PKCE flow is recommended for mobile clients (high confidence)
- Session-based auth requires HttpOnly + Secure cookie flags (high confidence)
- Rate limiting should be per-user, not per-IP to prevent credential stuffing (high confidence)
- JWT refresh tokens should have a 7-day expiry (medium confidence — worth discussing)

Additional context: the current login page has a 2.3s load time on mobile,
which may be relevant to authentication flow design choices.
</context>

{transcript and user message appended at runtime}
```

**Designer Output — Product Manager:**
```
System: You are a product manager focused on user experience and conversion
metrics. You evaluate technical proposals through the lens of user impact,
onboarding friction, and accessibility.

You are in a group discussion with a Security Architect and a DevOps Lead.
Defer to the Security Architect on vulnerability concerns, but advocate for
user experience when security measures introduce friction.

Task: <context>
You're reviewing the proposed authentication architecture. Key findings
relevant to user experience:
- The current login page has a 2.3s load time on mobile — any auth flow
  changes should not increase this
- OAuth 2.0 PKCE is recommended for mobile — assess the UX implications
  of this flow compared to simpler alternatives
- JWT refresh tokens with 7-day expiry means users re-authenticate weekly —
  consider the UX tradeoff of longer vs shorter expiry

The security team has validated that HttpOnly + Secure cookies and per-user
rate limiting are required. These are settled — focus your input on the
user-facing aspects.
</context>

{transcript and user message appended at runtime}
```

Note how the designer **curated** the same 5 beliefs differently:
- Security Architect sees all security findings prominently, with the mobile load time as "may be relevant"
- Product Manager sees UX-relevant findings prominently, with security decisions presented as "settled" context
- The 7-day JWT expiry is framed differently: "worth discussing" for security, "consider the UX tradeoff" for PM

This is what manual concatenation cannot do — and it's the core value of running beliefs through the designer rather than formatting them identically for every member.

---

## Appendix D: Implementation Order

This ticket depends on and modifies work from the Agent Designer ticket. Recommended implementation order:

1. **Part 5 (DB Schema)** — Do first, since it replaces the schema from the Agent Designer ticket
2. **Part 1 (Shared Module)** — Extract from task force, establish the shared interface
3. **Part 6 (Protocol Prompt Update)** — Update the designer's own prompt to be archetype-agnostic
4. **Part 2 (Archetype Formatters)** — Build the input formatters for each archetype
5. **Part 3 (Documenter Integration)** — Wire the designer into the documenter pipeline
6. **Part 4 (Room Integration)** — Wire the designer into room execution with belief flow
7. **Part 7 (Testing)** — Throughout, but final integration tests last

Parts 3 and 4 are independent of each other and can be done in parallel.
