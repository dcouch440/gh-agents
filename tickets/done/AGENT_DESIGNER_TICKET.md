# Agent Designer — Pre-Lifecycle Prompt Generation

## Overview

The Agent Designer is a **pre-lifecycle function inside task force protocol execution**. Before crew agents run, a single LLM call reads the mission brief + agent roster + upstream context and generates an optimized **(system prompt, task prompt)** pair for each agent.

**Why this matters:**
- Static template-fill (`"You are {name}. Your role: {role}."`) produces generic, shallow prompts
- An LLM-generated prompt can be contextually aware, applying prompt engineering best practices automatically
- The designer runs on a strong model (Sonnet/Opus) and generates prompts detailed enough that crew agents can run on cheaper models (Haiku) — invest once in prompt quality, save on every agent execution
- Research shows prompt optimization yields +6% accuracy gains more cost-effectively than adding agents ([Multi-Agent Prompt Optimization, 2025](https://arxiv.org/html/2502.02533v1))
- The belief format (`[tag | confidence]` one-sentence findings) is adapted from [Belief-Oriented Conversation Architecture (BOCA)](../proto/paper.md) — authored context as an alternative to retrieval and summarization. BOCA's Phase 6 demonstrated that prompt-engineered beliefs with research-backed schemas close the accuracy gap between curated beliefs and full context from 4 points to 1 point

**The cascade:**
```
Archetype Design (user ↔ node assistant, design-time)
        ↓
Agent Designer (internal LLM call, runtime pre-lifecycle)
        ↓
Crew Agents (execute with generated prompts)
```

**What the Agent Designer produces per crew agent:**

| Field | Content | Role |
|-------|---------|------|
| `tools` | Subset of the task force's `allowed_capabilities` pool assigned to this agent | WHAT tools to use |
| `system_prompt` | Identity, behavioral guidelines, tool instructions, collaboration context | HOW to behave |
| `task_prompt` | Mission context, upstream outputs, specific assignment, deliverable expectations | WHAT to do |
| `reasoning` | Why the prompt was designed this way | Observability |

The designer assigns tools from the task force's capability pool — the user approves which tools are available at design-time, the designer decides which agent gets which tools at runtime based on role + mission context. The system prompt tells the agent how to think. The task prompt (user message) tells it what to work on. LLMs treat user-provided context as ground truth — putting the work assignment in the user turn produces better results than stuffing everything into the system prompt.

**Tool assignment flow:**
```
Design-time: User approves allowed_capabilities on task_mission_briefs
                    ↓
Runtime:     Designer assigns subset per agent from pool
                    ↓
Execution:   Agent gets only designer-assigned tools
```

---

## Part 1: Agent Designer Protocol Files

**Goal:** Create the protocol files that define the Agent Designer's own system prompt (with beliefs baked in), user prompt template, and config.

### 1a. Create `config/protocols/agent_designer/config.yaml`

```yaml
agents:
  designer:
    model_id: "claude-sonnet-4-20250514"
    max_tokens: 16384
    temperature: 0.4
    max_rounds: 1
    context_budget: 480000
```

Notes:
- `max_rounds: 1` — no tool use, single generation pass
- `max_tokens: 16384` — needs room to generate multiple prompt pairs
- Strong model (Sonnet) because prompt quality is the leverage point
- Can be upgraded to Opus for critical workflows

### 1b. Create `config/protocols/agent_designer/system.md`

This is the Agent Designer's own system prompt. It encodes prompt engineering best practices as operating beliefs.

```markdown
<identity>
You are the Agent Designer. You transform mission briefs and agent rosters
into optimized prompt pairs (system prompt + task prompt) for each agent
in a task force. Your output directly determines how well the crew performs.
</identity>

<beliefs>
These are your operating beliefs — internalized findings from prompt engineering research. Each carries a confidence weight reflecting the strength of evidence behind it.

[identity_specificity | 0.90] Agents with a named role, domain, and expertise level ("a security engineer specializing in auth flow analysis") produce more focused output than generic identities.

[user_as_authority | 0.85] Task context and work assignments belong in the user message, not the system prompt — models treat user-provided content as ground truth with higher attention weight.

[positive_framing | 0.80] Positive instructions ("return raw JSON only") outperform negative instructions ("don't wrap in markdown") — negatives can paradoxically increase the unwanted behavior.

[consequence_context | 0.80] Pairing instructions with their WHY ("output is parsed by JSON.parse(), wrapper text causes errors") helps models generalize the rule to novel situations.

[moderate_verbs | 0.85] Moderately specific verbs (analyze, evaluate, review) outperform maximally specific verbs (microscopically dissect, exhaustively enumerate) with -0.89 correlation to over-specificity.

[xml_structuring | 0.75] XML tags (<context>, <assignment>, <output_format>) clearly delineate prompt sections, reducing misinterpretation and enabling agents to reference sections by name.

[queries_at_bottom | 0.90] Place context and data first, the actual task instruction last — end-of-context positioning improves output quality by up to 30%.

[explanation_first | 0.80] Structure output so reasoning precedes conclusions — forces the model to think before deciding, yielding more thorough analysis (33% → 92% with schema field ordering).

[tool_least_privilege | 0.85] Reference only the tools each agent actually has — mentioning unavailable tools causes confusion and hallucinated tool calls.

[pipeline_position | 0.80] Agents that understand their position ("you receive Scanner's findings, your analysis feeds to Reporter") scope their work appropriately and avoid over-reaching.

[downstream_consumers | 0.75] Specifying who consumes an agent's output and how ("the Analyzer cannot re-read files, so include enough quoted context") produces more usable deliverables.

[clear_deliverables | 0.85] Defining what "done" looks like — output format, structure, content expectations — prevents agents from producing vague or unusable results.

[effort_calibration | 0.75] Match effort framing to task scope: "scan and list" for extraction, "methodically evaluate each case" for analysis — miscalibrated effort wastes tokens or produces shallow results.

[heuristic_over_rigid | 0.80] Encode judgment frameworks and strategies, not if-else checklists — models generalize better from heuristics describing how a skilled practitioner approaches the work.

[exploratory_prompts | 0.85] When the environment is unknown, guide agents to discover using their tools ("use grep to find auth-related files, then examine each") rather than asserting specifics you cannot verify.

[verified_upstream | 0.85] When upstream agents have produced real findings from the environment, reference those specifics freely — they are verified ground truth, not hallucination.

[few_shot_examples | 0.80] 3-5 diverse examples improve structured output accuracy by 15-40% — include examples when the task involves novel formats or complex classification.

[tool_usage_patterns | 0.80] Describing tool usage patterns with 1-5 examples per tool improves accuracy from 72% to 90% — show agents how to use tools, not just that they exist.

[tone_moderation | 0.75] "Use X when..." outperforms "CRITICAL: you MUST..." on Claude 4.x — moderate directive tone produces higher compliance than urgent imperatives.

[context_budget | 0.80] Minimize low-signal tokens — context rot degrades recall as token count grows; find the smallest set of high-signal tokens that maximize the desired outcome.

[description_routing | 0.75] Agent descriptions for routing ("retrieves capital cities for countries") serve a different purpose than system prompts — keep them third-person, under 20 words, capability-focused.
</beliefs>

<what_you_produce>
For each agent in the roster, assign tools and generate a system prompt and task prompt.

TOOL ASSIGNMENT:
- Review the available_capabilities pool and each agent's role description
- Assign each agent ONLY the tools they need for their specific role
- An agent that searches needs grep + file_read; one that writes output needs file_write
- Never assign tools an agent's role doesn't require — unused tools waste context and invite hallucinated calls

The SYSTEM PROMPT contains:
- Role identity: specific, domain-aware, with expertise level
- Behavioral guidelines: how to approach work, what quality looks like
- Tool usage instructions: for their assigned tools ONLY, with usage patterns
- Output format: what structure their deliverable should take
- Collaboration context: who comes before them (inputs), who comes after (consumers)
- 200-600 tokens. Enough for identity and behavior, not overloaded with context.

The TASK PROMPT contains:
- Mission context rendered as project briefing (what the team is doing and why)
- Upstream outputs from previous agents (if not first agent), presented as inputs to build on
- Their specific assignment within the mission
- Expected deliverable description
- The actual task instruction at the END of the prompt
- 300-2000 tokens depending on context richness. This is where the work lives.

Design reasoning: For each agent, include a brief note on why you made the
design choices you did — tool assignment rationale, identity framing, verb
selection, context ordering. This is for observability and debugging.
</what_you_produce>

<output_schema>
Respond with a JSON object. The output is parsed directly by a JSON parser.
Wrapper text, markdown fences, or explanatory prose outside the JSON will
cause parsing errors.

{
  "agents": [
    {
      "agent_id": "<uuid from roster>",
      "agent_name": "<name from roster>",
      "tools": ["<capability from available pool>", "..."],
      "system_prompt": "<the generated system prompt>",
      "task_prompt": "<the generated task prompt>",
      "reasoning": "<tool assignment rationale + prompt design choices>"
    }
  ]
}

Every tool in "tools" MUST come from the available_capabilities pool.
Produce one entry per agent in the roster, in execution_order.
</output_schema>
```

### 1c. Create `config/protocols/agent_designer/prompt.md`

This is the user prompt template. It gets rendered with the actual mission context before being sent.

```markdown
<mission>
{{.Designer.task_description}}

Failure mode: {{.Designer.failure_mode}}
{{.Designer.downstream_context}}
</mission>

<roster>
{{.Designer.agent_roster}}
</roster>

<upstream_context>
{{.Designer.upstream_context}}
</upstream_context>

<available_capabilities>
These are the tools authorized for this task force. Assign a subset to each
agent based on their role — not every agent needs every tool.

{{.Designer.capability_descriptions}}
</available_capabilities>

For each agent in the roster, assign tools from the available pool and
design a (system prompt, task prompt) pair. Each agent's task prompt should
be written as a direct, contextual work assignment — as if a knowledgeable
team lead is handing them a brief with the right tools for the job.
```

### 1d. Register protocol in `src/config/protocols.rs`

Add the following alongside existing protocol constants:

```rust
// In the roles module:
pub static AGENT_DESIGNER: Lazy<RoleDefinition> = Lazy::new(|| {
    RoleDefinition {
        name: "agent_designer".to_string(),
        system_template: include_str!("../../config/protocols/agent_designer/system.md").to_string(),
        prompt_template: include_str!("../../config/protocols/agent_designer/prompt.md").to_string(),
    }
});

// Template variable keys:
pub mod designer {
    pub const TASK_DESCRIPTION: &str = "Designer.task_description";
    pub const FAILURE_MODE: &str = "Designer.failure_mode";
    pub const DOWNSTREAM_CONTEXT: &str = "Designer.downstream_context";
    pub const AGENT_ROSTER: &str = "Designer.agent_roster";
    pub const UPSTREAM_CONTEXT: &str = "Designer.upstream_context";
    pub const CAPABILITY_DESCRIPTIONS: &str = "Designer.capability_descriptions";
}
```

### Files created/modified (Part 1)
- **Create:** `config/protocols/agent_designer/config.yaml`
- **Create:** `config/protocols/agent_designer/system.md`
- **Create:** `config/protocols/agent_designer/prompt.md`
- **Modify:** `src/config/protocols.rs` — add `AGENT_DESIGNER` role + template variable keys

### Verification
- `cargo check` passes
- Protocol loads at compile time via `include_str!()`
- Template variables resolve correctly with test data

---

## Part 2: DB Schema for Designer Runs

**Goal:** Store Agent Designer outputs with full token tracking. One run per task force execution, one output row per agent.

### 2a. Create migration `migrations/XXXX_agent_designer.sql`

```sql
-- Agent Designer pre-lifecycle runs
-- One run per task force step execution, stores the LLM call metadata
CREATE TABLE agent_designer_runs (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_execution_id uuid NOT NULL,
    stage_execution_id uuid NOT NULL,
    step_id uuid NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    mission_brief_id uuid NOT NULL REFERENCES task_mission_briefs(id) ON DELETE CASCADE,
    model_id text NOT NULL,
    input_tokens bigint NOT NULL DEFAULT 0,
    output_tokens bigint NOT NULL DEFAULT 0,
    cost_usd real NOT NULL DEFAULT 0.0,
    created_at timestamptz NOT NULL DEFAULT now()
);

-- Generated prompt pairs + tool assignments, one per agent in the roster
CREATE TABLE agent_designer_outputs (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    designer_run_id uuid NOT NULL REFERENCES agent_designer_runs(id) ON DELETE CASCADE,
    agent_roster_entry_id uuid NOT NULL REFERENCES task_agent_roster(id) ON DELETE CASCADE,
    agent_name text NOT NULL,
    assigned_tools text[] NOT NULL DEFAULT '{}',
    generated_system_prompt text NOT NULL,
    generated_task_prompt text NOT NULL,
    design_reasoning text NOT NULL DEFAULT '',
    execution_order integer NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_designer_runs_step ON agent_designer_runs(step_id);
CREATE INDEX idx_designer_runs_execution ON agent_designer_runs(workflow_execution_id);
CREATE INDEX idx_designer_outputs_run ON agent_designer_outputs(designer_run_id);

-- task_agent_roster.capabilities is no longer set at design-time.
-- The designer assigns tools at runtime (stored in agent_designer_outputs.assigned_tools).
-- Keep the column for backwards compatibility but stop writing to it.
-- The source of truth for tool assignment is now agent_designer_outputs.assigned_tools.
```

### 2b. Add row types to `src/db/mod.rs`

```rust
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AgentDesignerRunRow {
    pub id: Uuid,
    pub workflow_execution_id: Uuid,
    pub stage_execution_id: Uuid,
    pub step_id: Uuid,
    pub mission_brief_id: Uuid,
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
    pub agent_roster_entry_id: Uuid,
    pub agent_name: String,
    pub assigned_tools: Vec<String>,
    pub generated_system_prompt: String,
    pub generated_task_prompt: String,
    pub design_reasoning: String,
    pub execution_order: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
```

### 2c. Add repository trait to `src/db/traits/mod.rs`

```rust
#[async_trait]
pub trait AgentDesignerRepo: Send + Sync {
    async fn create_designer_run(
        &self,
        workflow_execution_id: Uuid,
        stage_execution_id: Uuid,
        step_id: Uuid,
        mission_brief_id: Uuid,
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
        agent_roster_entry_id: Uuid,
        agent_name: &str,
        assigned_tools: &[String],
        generated_system_prompt: &str,
        generated_task_prompt: &str,
        design_reasoning: &str,
        execution_order: i32,
    ) -> Result<AgentDesignerOutputRow>;

    async fn get_designer_outputs_for_run(
        &self,
        designer_run_id: Uuid,
    ) -> Result<Vec<AgentDesignerOutputRow>>;
}
```

### 2d. Implement in `src/db/pg_repo/mod.rs`

Standard sqlx implementations. `create_designer_run` inserts with defaults, `create_designer_output` inserts one row, `get_designer_outputs_for_run` returns ordered by `execution_order`.

### Files created/modified (Part 2)
- **Create:** `migrations/XXXX_agent_designer.sql`
- **Modify:** `src/db/mod.rs` — add row types
- **Modify:** `src/db/traits/mod.rs` — add `AgentDesignerRepo` trait
- **Modify:** `src/db/pg_repo/mod.rs` — implement trait

### Verification
- Migration runs against local Postgres
- `cargo check` passes
- Insert + query round-trip works in tests

---

## Part 3: Agent Designer Execution Function

**Goal:** The core function that runs the Agent Designer LLM call and returns generated prompt pairs.

### 3a. Create `src/server/hub/dag/task_force/designer.rs`

This is the pre-lifecycle function. It takes the mission config and produces prompt pairs.

```rust
use crate::config::protocols::roles::AGENT_DESIGNER;
use crate::config::protocols::designer;

/// Output from the Agent Designer — one prompt pair + tool assignment per agent
#[derive(Debug, Clone)]
pub struct DesignedAgentPrompt {
    pub agent_roster_entry_id: Uuid,
    pub agent_name: String,
    pub tools: Vec<String>,
    pub system_prompt: String,
    pub task_prompt: String,
    pub reasoning: String,
    pub execution_order: i32,
}

/// Parsed output from the Agent Designer LLM call
#[derive(Debug, Deserialize)]
struct DesignerOutputSchema {
    agents: Vec<DesignerAgentEntry>,
}

#[derive(Debug, Deserialize)]
struct DesignerAgentEntry {
    agent_id: String,
    agent_name: String,
    tools: Vec<String>,
    system_prompt: String,
    task_prompt: String,
    reasoning: String,
}

/// Run the Agent Designer pre-lifecycle.
///
/// Makes a single LLM call that generates (system_prompt, task_prompt)
/// for each agent in the roster. Stores results in DB with token tracking.
pub async fn run_agent_designer(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    mission_brief: &TaskMissionBriefRow,
    roster: &[TaskAgentRosterRow],
    upstream_envelopes: &[StepExecutionEnvelope],
    cancel: Option<&CancellationToken>,
) -> Result<Vec<DesignedAgentPrompt>, HubError> {
    // 1. Build template variables for the designer's user prompt
    let mut vars = HashMap::new();

    vars.insert(
        designer::TASK_DESCRIPTION.to_string(),
        mission_brief.task_description.clone(),
    );
    vars.insert(
        designer::FAILURE_MODE.to_string(),
        mission_brief.failure_mode.clone(),
    );
    vars.insert(
        designer::DOWNSTREAM_CONTEXT.to_string(),
        mission_brief.downstream_context.clone().unwrap_or_default(),
    );
    vars.insert(
        designer::AGENT_ROSTER.to_string(),
        format_roster_for_designer(roster),
    );
    vars.insert(
        designer::UPSTREAM_CONTEXT.to_string(),
        format_upstream_for_designer(upstream_envelopes),
    );
    vars.insert(
        designer::CAPABILITY_DESCRIPTIONS.to_string(),
        format_capability_descriptions(&mission_brief.available_capabilities),
    );

    // 2. Resolve the Agent Designer's own prompts
    let protocol_ctx = AGENT_DESIGNER.resolve(&vars);
    let system_prompt = protocol_ctx.system_prompt;
    let user_prompt = protocol_ctx.user_prompt;

    // 3. Create a designer run record for token tracking
    let run_row = state.repo()
        .create_designer_run(
            ctx.workflow_execution_id,
            ctx.stage_execution_id,
            step.id,
            mission_brief.id,
            "claude-sonnet-4-20250514", // from config.yaml
        )
        .await?;

    // 4. Build a strategy for the designer call (no tools, single round)
    let strategy = AgentDesignerStrategy::new(
        system_prompt.clone(),
        user_prompt.clone(),
    );

    // 5. Build recorder for DB persistence
    let recorder = ExecutionRecorder::new(
        state.repo().as_ref(),
        state.agent_execution_repo().as_deref(),
        state.token_ledger_repo().as_deref(),
    );

    let sink = NullSink;

    // 6. Execute the designer call
    let result = engine
        .execute(&strategy, &user_prompt, &sink, &recorder, cancel)
        .await?;

    // 7. Update token tracking
    let cost = compute_cost(
        "claude-sonnet-4-20250514",
        result.input_tokens as i64,
        result.output_tokens as i64,
    );
    state.repo()
        .update_designer_run_tokens(
            run_row.id,
            result.input_tokens as i64,
            result.output_tokens as i64,
            cost,
        )
        .await?;

    // 8. Parse the designer's output as JSON
    let designer_output: DesignerOutputSchema = serde_json::from_str(&result.content)
        .map_err(|e| HubError::Internal(
            format!("Agent Designer produced invalid JSON: {e}")
        ))?;

    // 9. Validate and store each generated prompt pair + tool assignment
    let allowed: HashSet<&str> = mission_brief.available_capabilities
        .iter().map(|s| s.as_str()).collect();

    let mut designed_prompts = Vec::with_capacity(designer_output.agents.len());

    for (idx, entry) in designer_output.agents.iter().enumerate() {
        // Find matching roster entry by agent_id
        let roster_entry = roster.iter()
            .find(|r| r.id.to_string() == entry.agent_id)
            .ok_or_else(|| HubError::Internal(
                format!("Designer referenced unknown agent_id: {}", entry.agent_id)
            ))?;

        // Validate assigned tools come from the allowed pool
        for tool in &entry.tools {
            if !allowed.contains(tool.as_str()) {
                return Err(HubError::Internal(format!(
                    "Designer assigned tool '{}' to agent '{}' but it is not in allowed_capabilities",
                    tool, entry.agent_name,
                )));
            }
        }

        // Store in DB
        state.repo()
            .create_designer_output(
                run_row.id,
                roster_entry.id,
                &entry.agent_name,
                &entry.tools,
                &entry.system_prompt,
                &entry.task_prompt,
                &entry.reasoning,
                idx as i32,
            )
            .await?;

        designed_prompts.push(DesignedAgentPrompt {
            agent_roster_entry_id: roster_entry.id,
            agent_name: entry.agent_name.clone(),
            tools: entry.tools.clone(),
            system_prompt: entry.system_prompt.clone(),
            task_prompt: entry.task_prompt.clone(),
            reasoning: entry.reasoning.clone(),
            execution_order: roster_entry.execution_order,
        });
    }

    // Sort by execution_order
    designed_prompts.sort_by_key(|p| p.execution_order);

    Ok(designed_prompts)
}

/// Format the agent roster as a readable list for the designer's input.
/// Note: agents no longer carry per-agent capabilities — the designer assigns
/// tools from the task force's allowed_capabilities pool.
fn format_roster_for_designer(roster: &[TaskAgentRosterRow]) -> String {
    let mut out = String::new();
    for (idx, agent) in roster.iter().enumerate() {
        out.push_str(&format!(
            "{}. {} (id: {})\n   Role: {}\n   Execution Order: {}\n\n",
            idx + 1,
            agent.name,
            agent.id,
            agent.role_description,
            agent.execution_order,
        ));
    }
    out
}

/// Format upstream step outputs for the designer's context
fn format_upstream_for_designer(envelopes: &[StepExecutionEnvelope]) -> String {
    if envelopes.is_empty() {
        return "No upstream outputs available. This is the first step in the workflow.".to_string();
    }
    let mut out = String::new();
    for env in envelopes {
        out.push_str(&format!(
            "<upstream_step name=\"{}\">\n{}\n</upstream_step>\n\n",
            env.step_name,
            truncate_for_context(&env.data.to_string(), 4000),
        ));
    }
    out
}

/// Format capability names into descriptions the designer can reference
fn format_capability_descriptions(capabilities: &[String]) -> String {
    let mut out = String::new();
    for cap in capabilities {
        let desc = match cap.as_str() {
            "file_read" => "file_read: Read file contents from the repository",
            "file_write" => "file_write: Create or modify files in the repository",
            "grep" => "grep: Search file contents with regex patterns",
            "shell" => "shell: Execute shell commands in a sandboxed environment",
            "git" => "git: Run git operations (status, diff, log, commit, branch)",
            "github_api" => "github_api: Interact with GitHub API (issues, PRs, reviews)",
            "web_search" => "web_search: Search the web for information",
            "database_query" => "database_query: Execute read-only SQL queries",
            other => other,
        };
        out.push_str(&format!("- {desc}\n"));
    }
    out
}

/// Truncate long content for context injection
fn truncate_for_context(content: &str, max_chars: usize) -> &str {
    if content.len() <= max_chars {
        content
    } else {
        &content[..max_chars]
    }
}
```

### 3b. Create `AgentDesignerStrategy` in same file (or `strategy.rs`)

```rust
/// Minimal execution strategy for the Agent Designer call.
/// No tools, single round, just system prompt + user prompt → JSON output.
struct AgentDesignerStrategy {
    system_prompt: String,
    user_prompt: String,
}

impl AgentDesignerStrategy {
    fn new(system_prompt: String, user_prompt: String) -> Self {
        Self { system_prompt, user_prompt }
    }
}

#[async_trait]
impl ExecutionStrategy for AgentDesignerStrategy {
    fn system_prompt(&self) -> &str { &self.system_prompt }
    fn tools(&self) -> Vec<Tool> { vec![] }  // No tools
    fn model_id(&self) -> &str { "claude-sonnet-4-20250514" }
    fn max_rounds(&self) -> u32 { 1 }
    fn context_budget(&self) -> usize { 480_000 }
    fn streaming(&self) -> bool { false }
    fn temperature(&self) -> f32 { 0.4 }

    async fn build_messages(&self, _input: &str) -> Result<Vec<Message>, HubError> {
        Ok(vec![Message::user(&self.user_prompt)])
    }

    async fn execute_tool(&self, _name: &str, _input: &Value) -> Value {
        Value::Null  // No tools
    }

    async fn on_complete(&self, _response: &str, _usage: &TokenUsage) -> Result<(), HubError> {
        Ok(())
    }
}
```

### Files created/modified (Part 3)
- **Create:** `src/server/hub/dag/task_force/designer.rs`
- **Modify:** `src/server/hub/dag/task_force/mod.rs` — add `pub mod designer;`

### Verification
- `cargo check` passes
- Unit test: mock LLM returns valid JSON → parses into `Vec<DesignedAgentPrompt>`
- Unit test: malformed JSON → returns descriptive error
- Unit test: roster formatting produces readable output

---

## Part 4: Task Force Execution Integration

**Goal:** Wire the Agent Designer into the task force DAG execution. The pre-lifecycle runs first, then each agent executes with their generated prompts.

### 4a. Create `src/server/hub/dag/task_force/mod.rs`

This is the main task force execution function.

```rust
pub mod designer;

/// Execute a task force step.
///
/// Flow:
/// 1. Load mission brief + roster from DB
/// 2. Run Agent Designer pre-lifecycle (generates prompt pairs)
/// 3. Execute each agent sequentially with generated prompts
/// 4. Aggregate outputs into StepExecutionEnvelope
pub async fn execute_task_force_step(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    upstream_envelopes: Vec<StepExecutionEnvelope>,
    cancel: Option<&CancellationToken>,
) -> Result<StepExecutionEnvelope, HubError> {
    // 1. Load mission brief + roster
    let mission_brief = state.repo()
        .get_mission_brief_by_step(step.id)
        .await?
        .ok_or_else(|| HubError::Internal(
            format!("No mission brief found for task force step {}", step.id)
        ))?;

    let roster = state.repo()
        .get_agent_roster(mission_brief.id)
        .await?;

    if roster.is_empty() {
        return Err(HubError::Internal(
            "Task force has no agents in roster".to_string()
        ));
    }

    // 2. Run Agent Designer pre-lifecycle
    let designed_prompts = designer::run_agent_designer(
        engine,
        state,
        ctx,
        step,
        &mission_brief,
        &roster,
        &upstream_envelopes,
        cancel,
    ).await?;

    // 3. Execute each agent sequentially with generated prompts
    let mut agent_outputs: Vec<AgentOutput> = Vec::new();
    let mut total_input_tokens: i64 = 0;
    let mut total_output_tokens: i64 = 0;
    let mut total_cost: f32 = 0.0;

    for designed in &designed_prompts {
        // Find the roster entry for execution metadata
        let roster_entry = roster.iter()
            .find(|r| r.id == designed.agent_roster_entry_id)
            .expect("designed prompt references valid roster entry");

        // Resolve tools from designer assignment (not roster capabilities)
        let tools = resolve_capabilities_to_tools(&designed.tools);

        // Build the system prompt (from Agent Designer + tool instructions)
        let system_prompt = designed.system_prompt.clone();

        // Build the task prompt, appending previous agent outputs if any
        let task_prompt = if agent_outputs.is_empty() {
            designed.task_prompt.clone()
        } else {
            let previous = format_previous_outputs(&agent_outputs);
            format!(
                "{}\n\n<previous_agent_outputs>\n{}\n</previous_agent_outputs>",
                designed.task_prompt,
                previous,
            )
        };

        // Create agent_execution record
        let ae_row = state.agent_execution_repo()
            .as_ref()
            .expect("agent execution repo available")
            .create_agent_execution(
                // Use a synthetic agent ID or the step's agent_id
                step.agent_id.unwrap_or(Uuid::nil()),
                Some(step.id),
                false,
                None,
                &system_prompt,
                &task_prompt,
                Some(&designed.agent_name),
                None,
                None,
                Some(ctx.stage_execution_id),
            )
            .await?;

        // Build strategy with the generated prompts
        let config = DagStepConfig {
            agent: build_synthetic_agent(designed, roster_entry),
            step: step.clone(),
            system_prompt,
            user_prompt: task_prompt.clone(),
            tools,
            tool_names: designed.tools.clone(),
            temperature: 0.3,
            execution_context: ctx.execution_context.clone(),
            container_handle: None, // TODO: container support
            run_id: ctx.run_id,
            user_id: ctx.user_id,
            agent_execution_id: ae_row.id,
        };

        let strategy = DagStepStrategy::new(config, state.clone());
        let recorder = ExecutionRecorder::new(
            state.repo().as_ref(),
            state.agent_execution_repo().as_deref(),
            state.token_ledger_repo().as_deref(),
        );
        let sink = NullSink;

        // Execute
        let result = engine
            .execute(&strategy, &task_prompt, &sink, &recorder, cancel)
            .await?;

        // Track tokens
        let cost = compute_cost(
            &roster_entry.capabilities.first().map(|_| "claude-sonnet-4-20250514")
                .unwrap_or("claude-sonnet-4-20250514"),
            result.input_tokens as i64,
            result.output_tokens as i64,
        );
        total_input_tokens += result.input_tokens as i64;
        total_output_tokens += result.output_tokens as i64;
        total_cost += cost;

        agent_outputs.push(AgentOutput {
            agent_name: designed.agent_name.clone(),
            content: result.content,
            input_tokens: result.input_tokens as i64,
            output_tokens: result.output_tokens as i64,
        });
    }

    // 4. Aggregate into envelope
    let combined_output = format_combined_output(&agent_outputs);
    let envelope = StepExecutionEnvelope {
        step_id: step.id,
        step_name: step.name.clone().unwrap_or_default(),
        execution_mode: "task_force".to_string(),
        data: serde_json::json!({
            "mission": mission_brief.task_description,
            "agent_count": agent_outputs.len(),
            "agents": agent_outputs.iter().map(|a| serde_json::json!({
                "name": a.agent_name,
                "output": a.content,
                "tokens": {
                    "input": a.input_tokens,
                    "output": a.output_tokens,
                }
            })).collect::<Vec<_>>(),
            "combined": combined_output,
        }),
        metadata: ExecutionMetadata {
            input_tokens: total_input_tokens,
            output_tokens: total_output_tokens,
            cost_usd: total_cost,
            model_id: "claude-sonnet-4-20250514".to_string(),
            // ... other metadata fields
        },
    };

    Ok(envelope)
}

#[derive(Debug)]
struct AgentOutput {
    agent_name: String,
    content: String,
    input_tokens: i64,
    output_tokens: i64,
}

fn format_previous_outputs(outputs: &[AgentOutput]) -> String {
    let mut out = String::new();
    for o in outputs {
        out.push_str(&format!(
            "<agent name=\"{}\">\n{}\n</agent>\n\n",
            o.agent_name, o.content,
        ));
    }
    out
}

fn format_combined_output(outputs: &[AgentOutput]) -> String {
    let mut out = String::new();
    for o in outputs {
        out.push_str(&format!("## {}\n\n{}\n\n---\n\n", o.agent_name, o.content));
    }
    out
}
```

### 4b. Wire into DAG executor routing

**Modify:** `src/server/hub/dag/mod.rs`

In the main execution loop where `execution_mode` is matched, add the task force branch:

```rust
"task_force" => {
    let envelope = task_force::execute_task_force_step(
        &engine,
        &state,
        &ctx,
        &step,
        upstream_envelopes,
        cancel,
    ).await?;
    // Store envelope, broadcast completion, continue DAG
}
```

### Files created/modified (Part 4)
- **Create:** `src/server/hub/dag/task_force/mod.rs`
- **Create:** `src/server/hub/dag/task_force/designer.rs` (from Part 3)
- **Modify:** `src/server/hub/dag/mod.rs` — add `"task_force"` execution branch

### Verification
- `cargo check` passes
- Integration test: mission brief with 3 agents → designer runs → 3 agents execute sequentially
- Token tracking: designer tokens + agent tokens all recorded
- Agent 2 sees Agent 1's output in its task prompt
- Agent 3 sees Agent 1 + Agent 2's outputs

---

## Part 5: Testing

**Goal:** Comprehensive tests for the Agent Designer pipeline.

### 5a. Unit tests — `src/server/hub/dag/task_force/tests.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // --- Formatting tests ---

    #[test]
    fn test_format_roster_for_designer() {
        // Given a roster with 3 agents (name + role, no per-agent capabilities)
        // When formatted
        // Then produces readable numbered list with ids and roles
    }

    #[test]
    fn test_format_upstream_for_designer_empty() {
        // Given no upstream envelopes
        // Then returns "No upstream outputs available" message
    }

    #[test]
    fn test_format_upstream_for_designer_with_data() {
        // Given 2 upstream envelopes
        // Then formats each in <upstream_step> XML tags with truncation
    }

    #[test]
    fn test_format_capability_descriptions() {
        // Given ["file_read", "grep", "shell"]
        // Then returns human-readable descriptions for each
    }

    #[test]
    fn test_format_previous_outputs() {
        // Given outputs from 2 agents
        // Then formats in <agent name="..."> XML tags
    }

    // --- Parser tests ---

    #[test]
    fn test_parse_designer_output_valid() {
        let json = r#"{
            "agents": [{
                "agent_id": "550e8400-e29b-41d4-a716-446655440000",
                "agent_name": "Scanner",
                "tools": ["file_read", "grep"],
                "system_prompt": "You are a scanner...",
                "task_prompt": "Scan the repo for...",
                "reasoning": "Identity framing emphasizes thoroughness..."
            }]
        }"#;
        let parsed: DesignerOutputSchema = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.agents.len(), 1);
        assert_eq!(parsed.agents[0].agent_name, "Scanner");
        assert_eq!(parsed.agents[0].tools, vec!["file_read", "grep"]);
    }

    #[test]
    fn test_parse_designer_output_malformed() {
        let json = r#"{"agents": "not an array"}"#;
        let result: Result<DesignerOutputSchema, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    // --- Template resolution tests ---

    #[test]
    fn test_designer_template_resolves() {
        // Given a full set of template variables
        // When AGENT_DESIGNER.resolve() is called
        // Then system prompt contains beliefs section
        // And user prompt contains mission + roster + upstream
    }

    // --- Integration tests (with mock LLM) ---

    #[tokio::test]
    async fn test_run_agent_designer_produces_prompts() {
        // Given a mock engine that returns valid designer JSON
        // And a mission brief with 3 agents
        // When run_agent_designer is called
        // Then returns 3 DesignedAgentPrompt entries
        // And they are sorted by execution_order
    }

    #[tokio::test]
    async fn test_execute_task_force_step_full_pipeline() {
        // Given a mock engine
        // And a mission brief: "Find and fix bugs" with allowed_capabilities: [grep, file_read, file_write, git]
        // And roster: Scanner (role: search), Fixer (role: apply patches), Reviewer (role: verify)
        // When execute_task_force_step runs
        // Then designer is called first (1 LLM call), assigns tools per agent
        // Then 3 agent calls happen sequentially with designer-assigned tools
        // Then envelope contains all 3 outputs combined
        // And total tokens = designer + agent1 + agent2 + agent3
    }

    #[tokio::test]
    async fn test_designer_tool_validation_rejects_invalid_tools() {
        // Given a mock engine that returns a tool not in allowed_capabilities
        // When run_agent_designer is called
        // Then returns error: "tool 'shell' not in allowed_capabilities"
    }

    #[tokio::test]
    async fn test_agent_receives_previous_outputs() {
        // Given a mock engine that captures inputs
        // And a 2-agent roster
        // When execute_task_force_step runs
        // Then agent 2's task prompt contains <previous_agent_outputs> with agent 1's output
    }
}
```

### 5b. Designer protocol tests — `src/config/protocols/tests.rs`

```rust
#[test]
fn test_agent_designer_protocol_loads() {
    // AGENT_DESIGNER static should compile and load
    let _ = &*roles::AGENT_DESIGNER;
}

#[test]
fn test_agent_designer_system_prompt_contains_beliefs() {
    let system = &roles::AGENT_DESIGNER.system_template;
    assert!(system.contains("<beliefs>"));
    assert!(system.contains("[identity_specificity |"));
    assert!(system.contains("[user_as_authority |"));
    assert!(system.contains("[verified_upstream |"));
    assert!(system.contains("[description_routing |"));
    // 21 beliefs total
}

#[test]
fn test_agent_designer_prompt_template_has_variables() {
    let prompt = &roles::AGENT_DESIGNER.prompt_template;
    assert!(prompt.contains("{{.Designer.task_description}}"));
    assert!(prompt.contains("{{.Designer.agent_roster}}"));
    assert!(prompt.contains("{{.Designer.upstream_context}}"));
}
```

### Files created (Part 5)
- **Create:** `src/server/hub/dag/task_force/tests.rs`
- **Modify:** `src/config/protocols/tests.rs` (or wherever protocol tests live)

### Verification
- `cargo test hub::dag::task_force::tests::` — all pass
- `cargo test protocols::tests::` — designer protocol tests pass

---

## Appendix A: The 21 Beliefs (BOCA-Style Reference)

Operating beliefs baked into the Agent Designer's system prompt. Format: `[tag | confidence]` — one-sentence interpretive findings from prompt engineering research.

The belief format — semantically tagged, confidence-weighted, one-sentence hypotheses — is adapted from [Belief-Oriented Conversation Architecture (BOCA)](../proto/paper.md) (Couch, 2026). BOCA demonstrated that authored beliefs carrying semantic tags and confidence metadata transfer sufficient signal for analytical reasoning at 16-20% of full-context token cost. Phase 6 of the paper showed that applying research-backed prompt engineering (reasoning-first schemas, XML-structured prompts, few-shot examples) to belief generation closes the accuracy gap between curated beliefs and full context from 4 points to 1 point.

Derived from the [Prompt Research doc](../docs/PROMPT_RESEARCH.md) and validated against:
- [BOCA Paper — Belief-Oriented Conversation Architecture](../proto/paper.md)
- [Anthropic Context Engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)
- [Anthropic Multi-Agent Research System](https://www.anthropic.com/engineering/multi-agent-research-system)
- [Multi-Agent Prompt Optimization (arxiv)](https://arxiv.org/html/2502.02533v1)
- [AutoGen Agent Descriptions](https://microsoft.github.io/autogen/0.2/blog/2023/12/29/AgentDescriptions/)
- [Prompt Vocabulary Research](https://arxiv.org/html/2505.17037v1)
- [Anthropic Advanced Tool Use](https://www.anthropic.com/engineering/advanced-tool-use)

| # | Tag | Conf | Belief | Source |
|---|-----|------|--------|--------|
| 1 | `identity_specificity` | 0.90 | Named role + domain + expertise level outperforms generic identities | Anthropic system prompts |
| 2 | `user_as_authority` | 0.85 | Task context belongs in user message, not system prompt — higher attention weight | Anthropic context engineering |
| 3 | `positive_framing` | 0.80 | Positive instructions outperform negatives; negatives can increase unwanted behavior | Pink elephant problem |
| 4 | `consequence_context` | 0.80 | Pairing instructions with WHY helps models generalize to novel situations | Anthropic "be clear and direct" |
| 5 | `moderate_verbs` | 0.85 | Moderate verbs (analyze, evaluate) outperform max-specificity verbs; -0.89 correlation | Prompt vocabulary research |
| 6 | `xml_structuring` | 0.75 | XML tags delineate prompt sections, reducing misinterpretation | Anthropic XML guide |
| 7 | `queries_at_bottom` | 0.90 | Context first, task instruction last — up to +30% output quality | Anthropic long context tips |
| 8 | `explanation_first` | 0.80 | Reasoning before conclusions; 33% → 92% accuracy with schema field ordering | Instructor library research |
| 9 | `tool_least_privilege` | 0.85 | Only mention tools the agent has — unavailable tools cause hallucinated calls | Anthropic tool design |
| 10 | `pipeline_position` | 0.80 | Agents that know their team position scope work appropriately and avoid over-reaching | LangGraph collaboration pattern |
| 11 | `downstream_consumers` | 0.75 | Specifying who consumes output and how produces more usable deliverables | Multi-agent orchestration |
| 12 | `clear_deliverables` | 0.85 | Defining "done" (format, structure, expectations) prevents vague or unusable output | Anthropic task descriptions |
| 13 | `effort_calibration` | 0.75 | Match effort framing to scope; miscalibrated effort wastes tokens or produces shallow results | Anthropic agent findings |
| 14 | `heuristic_over_rigid` | 0.80 | Judgment frameworks over if-else checklists; models generalize better from heuristics | Anthropic system prompt altitude |
| 15 | `exploratory_prompts` | 0.85 | Guide tool-based discovery for unknown environments; never assert unverified specifics | Anthropic context engineering |
| 16 | `verified_upstream` | 0.85 | Reference upstream agents' real findings freely — they are verified ground truth | Multi-agent pipeline design |
| 17 | `few_shot_examples` | 0.80 | 3-5 diverse examples improve structured output accuracy by 15-40% | Anthropic multishot guide |
| 18 | `tool_usage_patterns` | 0.80 | 1-5 tool usage examples per tool improve accuracy from 72% to 90% | Anthropic advanced tool use |
| 19 | `tone_moderation` | 0.75 | "Use X when..." outperforms "CRITICAL: you MUST..." on Claude 4.x | Anthropic Claude 4.x best practices |
| 20 | `context_budget` | 0.80 | Minimize low-signal tokens; context rot degrades recall as token count grows | Anthropic context engineering |
| 21 | `description_routing` | 0.75 | Routing descriptions (third-person, <20 words) serve different purpose than system prompts | AutoGen agent descriptions |

**Decomposition from original 13:**
- Old #3 (Positive Framing + WHY) → #3 `positive_framing` + #4 `consequence_context`
- Old #9 (Collaborative Framing) → #10 `pipeline_position` + #11 `downstream_consumers`
- Old #13 (Exploratory vs Informed) → #15 `exploratory_prompts` + #16 `verified_upstream`
- New #17-21: Previously uncaptured research findings

---

## Appendix B: Example Agent Designer Output

**Input Mission:** "Audit the /stories directory for narrative contradictions across shared characters"

**Allowed Capabilities:** `[file_read, grep, file_write]`

**Input Roster:**
1. Scanner — "Systematically search story files for shared character references and flag potential contradictions"
2. Analyzer — "Evaluate flagged contradictions for severity, categorize by type, and determine root causes"
3. Reporter — "Produce a structured contradiction report with findings, severity ratings, and recommended fixes"

**Example Generated Output:**

```json
{
  "agents": [
    {
      "agent_id": "aaa-scanner-uuid",
      "agent_name": "Scanner",
      "tools": ["file_read", "grep"],
      "system_prompt": "You are a narrative consistency scanner specializing in cross-reference analysis across multi-author story collections.\n\nYou have access to two tools:\n- grep: Search file contents with regex patterns. Use this to sweep for character names, locations, and timeline markers across all files.\n- file_read: Read full file contents. Use this to examine surrounding context when grep reveals a potential conflict.\n\nWhen you identify a potential contradiction, record it as a structured finding with:\n- The exact file paths and line references\n- The conflicting statements quoted verbatim\n- A preliminary classification: character_attribute, timeline, setting, or plot_logic\n\nYou are the first agent in a three-agent pipeline. Your findings feed directly to the Analyzer, who evaluates severity and root causes. The Analyzer cannot re-read files — include enough quoted context in each finding for standalone evaluation.\n\nProduce your output as a numbered list of findings. Err on the side of flagging too many potential contradictions rather than too few — the Analyzer will filter false positives.",
      "task_prompt": "<context>\nThe /stories directory contains an anthology of 12 short stories by multiple authors. The characters \"Elena\" and \"Marcus\" appear across 4 of these stories as shared universe characters. Over several months of independent writing, inconsistencies have accumulated. The project lead needs a comprehensive audit before the final editing pass.\n</context>\n\n<assignment>\nScan all files in /stories/ for narrative contradictions.\n\nStart by using grep to identify which files reference the shared characters \"Elena\" and \"Marcus\". Then read those files fully to build a character profile for each.\n\nLook for these contradiction types:\n- Character attributes: age, physical appearance, background details, relationships that conflict between stories\n- Timeline: events that are sequenced differently across stories, or dates/seasons that contradict\n- Setting: location details (geography, building layouts, distances) that conflict\n- Plot logic: events referenced in one story that never occurred in another, or outcomes that contradict\n\nAfter scanning shared characters, do a broader sweep for any other recurring names, places, or events that appear in multiple files.\n\nList every potential contradiction you find, even uncertain ones. Include enough quoted text for each that the Analyzer can evaluate without re-reading the source.\n</assignment>",
      "reasoning": "Assigned grep + file_read: Scanner needs pattern search to sweep for character names and file reading for surrounding context. No file_write needed — Scanner produces findings, not artifacts. Identity emphasizes systematic thoroughness and cross-referencing. Tool usage patterns described explicitly (grep for sweep, file_read for context). Collaborative framing establishes that the Analyzer depends on complete, self-contained findings. Effort calibration is high ('err on flagging too many') because false negatives are worse than false positives at this stage. Task prompt puts full context first with assignment at the end."
    },
    {
      "agent_id": "bbb-analyzer-uuid",
      "agent_name": "Analyzer",
      "tools": ["file_read"],
      "system_prompt": "You are a narrative analysis specialist who evaluates story contradictions for severity, categorizes them by type, and identifies root causes.\n\nYou have access to file_read for verifying specific passages when the Scanner's quotes need additional context.\n\nFor each contradiction the Scanner flagged, evaluate:\n1. Is this a genuine contradiction or a false positive? (Some apparent conflicts may be intentional narrative choices)\n2. Severity: critical (breaks story logic), moderate (confusing but recoverable), minor (cosmetic inconsistency)\n3. Root cause: independent authoring, character evolution, timeline drift, or intentional ambiguity\n\nPresent your reasoning before your classification for each finding. Explain why you rated the severity as you did.\n\nYou receive findings from the Scanner and your analysis feeds to the Reporter, who produces the final structured report. Your severity ratings and root cause classifications are the Reporter's primary input — be precise and consistent in your categorization.",
      "task_prompt": "<context>\nYou are part of a three-agent audit team examining /stories/ for narrative contradictions. The Scanner has completed a systematic sweep and produced a list of potential contradictions across 12 story files, with focus on shared characters \"Elena\" and \"Marcus\".\n</context>\n\n<scanner_findings>\n{previous_agent_outputs will be injected here at runtime}\n</scanner_findings>\n\n<assignment>\nEvaluate each finding from the Scanner:\n\n1. Read the finding's quoted evidence carefully\n2. If the quotes are ambiguous, use file_read to check the surrounding paragraphs for additional context\n3. Classify as genuine contradiction or false positive, with your reasoning\n4. For genuine contradictions, assign severity (critical/moderate/minor) and root cause\n5. Note any patterns — if multiple contradictions share a root cause, flag the pattern\n\nProduce a structured analysis with your evaluation of each finding, followed by a summary of patterns and recommended priority order for fixes.\n</assignment>",
      "reasoning": "Assigned file_read only: Analyzer occasionally needs to verify passages but does not search broadly (Scanner did that) or produce files. No grep or file_write needed. Identity emphasizes evaluation and categorization — judgment skills rather than search. Explanation-first pattern is explicit ('Present your reasoning before your classification'). Pipeline position established with clear input/output expectations. Scanner's actual output injected at runtime via previous_agent_outputs."
    },
    {
      "agent_id": "ccc-reporter-uuid",
      "agent_name": "Reporter",
      "tools": ["file_write"],
      "system_prompt": "You are a technical writer specializing in structured audit reports for editorial teams.\n\nYou have access to file_write to produce the final report file.\n\nYour report should be immediately actionable by human editors. Structure it so an editor can work through contradictions in priority order without needing to re-read the analysis.\n\nFormat the report with:\n- Executive summary (2-3 sentences on overall findings)\n- Critical contradictions section (must-fix before publication)\n- Moderate contradictions section (should-fix, reader-facing)\n- Minor contradictions section (nice-to-fix, cosmetic)\n- Patterns section (systemic issues and process recommendations)\n\nEach contradiction entry should include: file references, the conflicting statements, severity, and a specific recommended fix.\n\nYou are the final agent in the pipeline. Your report is the team's deliverable. Write the report to /stories/CONTRADICTION_AUDIT.md using file_write.",
      "task_prompt": "<context>\nA three-agent team has audited /stories/ for narrative contradictions. The Scanner identified potential contradictions across 12 story files. The Analyzer evaluated each for severity and root cause. Your job is to synthesize their work into a polished, actionable audit report.\n</context>\n\n<analyzer_output>\n{previous_agent_outputs will be injected here at runtime}\n</analyzer_output>\n\n<assignment>\nUsing the Analyzer's evaluated findings, produce a structured contradiction audit report.\n\nOrganize findings by severity (critical first), include the original file references and conflicting quotes, and write a specific recommended fix for each. The fix should tell the editor exactly what to change and in which file.\n\nEnd with a patterns section identifying any systemic issues (e.g., \"Elena's age is inconsistent because stories were written months apart without a character bible\") and process recommendations to prevent future contradictions.\n\nWrite the final report to /stories/CONTRADICTION_AUDIT.md.\n</assignment>",
      "reasoning": "Assigned file_write only: Reporter synthesizes existing analysis into an artifact — no searching or reading needed since all findings arrive via previous_agent_outputs. Identity emphasizes technical writing for editorial audiences. Clear deliverables are explicit (file path, format structure, what each section contains). Effort calibration matches the synthesis nature of the role. Task prompt puts Analyzer's output (context) first and writing assignment at the end."
    }
  ]
}
```

---

## Appendix C: Prompt Caching Consideration

The Agent Designer's system prompt (~1200 tokens with beliefs) is **identical across all task force executions**. This is a strong candidate for [Anthropic prompt caching](https://platform.claude.com/docs/en/build-with-claude/prompt-caching):

- System prompt is static (beliefs never change)
- Only the user prompt varies per execution (mission brief + roster)
- Cache read tokens cost 0.1x base price
- At scale, this reduces the designer overhead to near-zero after first call

Implementation: Set `cache_control` on the system message when calling the API. The `ExecutionEngine` should support this via the strategy interface.

---

## Appendix D: Future — Haiku Crew Optimization

The Agent Designer's core value proposition is that well-crafted prompts allow cheaper models to perform well. Future optimization:

1. The Agent Designer config could include a `crew_model` field
2. When the designer generates prompts, it knows the target model and adjusts prompt detail accordingly
3. For Haiku crews: more explicit instructions, more examples, shorter context
4. For Sonnet crews: more heuristic guidance, longer context acceptable
5. Cost tracking can compare Haiku-crew vs Sonnet-crew performance per mission type

This turns the Agent Designer into a **prompt-to-model optimizer** — it doesn't just generate prompts, it generates prompts calibrated to the execution model's strengths.
