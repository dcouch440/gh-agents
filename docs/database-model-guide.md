# Nexor Database Model Guide

21 tables across 3 layers:

- **Definition** — what the user builds (agents, schemas, workflows, pipelines, documents, prompt templates)
- **Wiring** — how things connect (steps, edges, stage members, step documents)
- **Execution** — what happened at runtime (runs, stage executions, agent executions, messages, token ledger)

---

## Flow Diagram

```
DEFINITION LAYER (user creates and reuses)
===========================================

  agents              output_schemas        prompt_templates       documents        tools
  (who)               (output shape)        (reusable prompts)     (attachable context) (callable actions)
    │                                                                                    │
    └────────────────────────── agent_tools (N tools per agent) ─────────────────────────┘
    │                      │                      │                      │
    │                      │                      │                      │
    ▼                      ▼                      ▼                      ▼
    └──────────────────────┴──────────────────────┴──────────────────────┘
                                    │
                    ┌───────────────┼───────────────┐
                    ▼                               ▼
               workflows                       pipelines
               (execution DAGs)                (stage sequences)
                    │                               │
           ┌───────┴───────┐                  pipeline_stages
     workflow_steps   workflow_step_edges           │
      (DAG nodes)      (DAG edges)          pipeline_stage_members
           │                                (N workflows per stage)
      step_documents
      (attached docs)


EXECUTION LAYER (runtime records)
==================================

                        pipeline_runs
                             │
                      stage_executions
                             │
                      agent_executions
                      /       |       \
              exec_messages  results  token_ledger
```

---

## Schema

> **21 tables** after migration 046 added `tools` and `agent_tools`.

---

### 1. users

```sql
users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT,
    github_id BIGINT UNIQUE,
    github_login TEXT,
    github_token_encrypted TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
-- idx_users_email(email)
-- idx_users_github_id(github_id) WHERE github_id IS NOT NULL
```

Root of the whole system. Every major entity references `user_id`. Multi-tenant — users only see their own data.

| Column | Purpose |
|--------|---------|
| `id` | Primary key. All foreign keys across the system reference this. |
| `email` | Login identifier. Unique constraint prevents duplicate accounts. |
| `password_hash` | Bcrypt/argon2 hash for email+password auth. Nullable because GitHub OAuth users may not have a password. |
| `github_id` | GitHub's numeric user ID. Used to match OAuth callbacks to existing accounts. |
| `github_login` | GitHub username. Display only — not used for auth since usernames can change. |
| `github_token_encrypted` | Encrypted GitHub OAuth token for API calls (repo access, PR creation). Encrypted at rest, decrypted only when making GitHub API calls. |
| `created_at` | Account creation timestamp. |
| `updated_at` | Last profile update. Used for cache invalidation. |

---

### 2. sessions

```sql
sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    token_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    last_active TIMESTAMPTZ NOT NULL
)
-- idx_sessions_user(user_id)
-- idx_sessions_expires(expires_at)
```

Server-side session store. The client holds a token, the server stores the hash. Stateless per-request auth — hash the incoming token, look it up.

| Column | Purpose |
|--------|---------|
| `id` | Session identifier. |
| `user_id` | Which user this session belongs to. Index supports "list all sessions for user" (device management). |
| `token_hash` | SHA-256 hash of the bearer token. The raw token is never stored. |
| `expires_at` | Hard expiry. Index supports cleanup job that deletes expired rows. |
| `last_active` | Updated on each authenticated request. Used for idle timeout and "last seen" display. |

---

### 3. agents

```sql
agents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    system_prompt TEXT NOT NULL DEFAULT '',
    model_provider TEXT NOT NULL DEFAULT 'anthropic',
    model_id TEXT NOT NULL,
    model_max_tokens INTEGER NOT NULL DEFAULT 4096,
    model_temperature REAL NOT NULL DEFAULT 0.7,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
-- idx_agents_user(user_id)
```

A reusable agent template. This is **who the agent is**, not what it's doing right now. No runtime state — no `status`, no `current_task`. The same agent definition can be used in multiple workflows and pipelines simultaneously. Runtime state lives in `agent_executions`.

| Column | Purpose |
|--------|---------|
| `id` | Referenced by workflow_steps, pipeline_stage_members, and agent_executions. |
| `user_id` | Owner. Agents are private to the user who created them. |
| `name` | Display name shown in the graph UI and execution tree (e.g. "Dave", "Code Reviewer", "Ticket Writer"). |
| `system_prompt` | The agent's identity and instructions. This is the only thing that makes one agent different from another. Injected as the system message in every LLM call. |
| `model_provider` | LLM provider identifier (e.g. `'anthropic'`, `'openai'`). Determines which API client to use at runtime. |
| `model_id` | Specific model (e.g. `'claude-sonnet-4-20250514'`, `'gpt-4o'`). Paired with provider to route the LLM call. |
| `model_max_tokens` | Max output tokens per LLM call. Controls response length and cost. |
| `model_temperature` | Sampling temperature. Lower = more deterministic, higher = more creative. |

---

### 4. output_schemas

```sql
output_schemas (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    schema JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, name)
)
-- idx_output_schemas_user(user_id)
```

Reusable structured output definitions. When assigned to a workflow step, the agent is instructed to return data matching this shape. This is how the system enforces single-in, single-out with predictable types at every node.

The passdown concept lives here — it's just a field in the schema like any other:

```json
{
    "name": { "type": "string", "description": "Feature name" },
    "content": { "type": "array", "description": "List of sub-features" },
    "passdown": { "type": "string", "description": "Summarize what you did and any issues found" }
}
```

| Column | Purpose |
|--------|---------|
| `id` | Referenced by workflow_steps and pipeline_stage_members. |
| `user_id` | Owner. |
| `name` | Human-readable identifier (e.g. `'feature_list'`, `'ticket'`, `'code_review'`). Unique per user so schemas can be referenced by name in the UI. |
| `schema` | JSONB object defining the expected output shape. Field names, types, and descriptions. Used both to instruct the LLM and to validate/parse the response. |

---

### 5. prompt_templates

```sql
prompt_templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, name)
)
-- idx_prompt_templates_user(user_id)
```

Reusable prompt text. Instead of rewriting the same task instructions on every workflow step, save it once and reference it. Supports `{variable}` placeholders that get resolved at runtime from prior step outputs.

| Column | Purpose |
|--------|---------|
| `id` | Referenced by workflow_steps via `prompt_template_id`. |
| `user_id` | Owner. |
| `name` | Human-readable identifier (e.g. `'decompose_features'`, `'write_ticket'`). Unique per user. |
| `content` | The prompt text with `{variable}` placeholders. Example: `"Review {conventions} and create a component for {features.0.name}"`. |

---

### 6. documents

```sql
documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    content TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
-- idx_documents_user(user_id)
```

User-created documents that can be attached to workflow steps as additional context. PRDs, specs, coding conventions, architecture docs — anything the agent should read before executing.

| Column | Purpose |
|--------|---------|
| `id` | Referenced by step_documents. |
| `user_id` | Owner. |
| `name` | Display name (e.g. `'Project Requirements'`, `'TypeScript Conventions'`). |
| `content` | The full document text. Appended to the agent's prompt at runtime when attached via step_documents. |

---

### 7. tools

```sql
tools (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    display_name TEXT NOT NULL,
    description TEXT NOT NULL,
    parameters JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, name)
)
-- idx_tools_user(user_id)
```

Metadata for hardcoded tool implementations. The `name` field is the machine key used to match against the `execute_execution_tool` dispatch table. The `description` and `parameters` are sent to the LLM in the `tools` array. The `display_name` is shown in the UI.

Tools are optional on agents. If an agent has no tools assigned, the LLM is called once with no tool definitions. If tools are assigned, the DAG executor runs a react loop (up to 15 rounds) — the LLM can call tools and receive results until it produces a final answer.

| Column | Purpose |
|--------|---------|
| `id` | Referenced by agent_tools. |
| `user_id` | Owner. Tools are per-user so descriptions can be customized. |
| `name` | Machine key matching the hardcoded dispatch (`read_file`, `write_file`, etc.). Unique per user. Immutable in practice — changing it breaks the dispatch. |
| `display_name` | Human-readable label for the UI (e.g. "Read File", "Git Status"). Editable. |
| `description` | Sent to the LLM to explain what the tool does. Editable — tweak how the LLM understands the tool without recompiling. |
| `parameters` | JSON Schema describing the tool's input parameters. Sent to the LLM. |

---

### 8. agent_tools

```sql
agent_tools (
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    tool_id UUID NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
    PRIMARY KEY (agent_id, tool_id)
)
-- idx_agent_tools_tool(tool_id)
```

Join table linking agents to their available tools. At execution time, the DAG executor queries this to build the `tools` array for the LLM request.

| Column | Purpose |
|--------|---------|
| `agent_id` | Which agent has access to this tool. |
| `tool_id` | Which tool is available. |

---

### 9. workflows

```sql
workflows (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
-- idx_workflows_user(user_id)
```

A reusable execution graph (DAG) of agent steps. This is the core orchestration unit — every pipeline stage runs one or more workflows. Even a single agent is wrapped in a one-step workflow. This keeps the system uniform: one model for execution, one model for the graph UI, one model for context resolution.

| Column | Purpose |
|--------|---------|
| `id` | Referenced by pipeline_stage_members and the graph UI. |
| `user_id` | Owner. |
| `name` | Display name (e.g. `'Feature Decomposer'`, `'Code Implementation'`). |
| `description` | Optional notes about what this workflow does. |

---

### 8. workflow_steps

```sql
workflow_steps (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_id UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    agent_id UUID NOT NULL REFERENCES agents(id),
    execution_mode TEXT NOT NULL DEFAULT 'single',
    for_each_ref TEXT,
    prompt_template_id UUID REFERENCES prompt_templates(id),
    prompt_template TEXT NOT NULL DEFAULT '',
    output_schema_id UUID REFERENCES output_schemas(id),
    output_variable_name TEXT,
    interactive_agent_id UUID REFERENCES agents(id),
    for_each_label_field TEXT,
    display_order INTEGER NOT NULL DEFAULT 0
)
-- idx_workflow_steps_workflow(workflow_id)
-- idx_workflow_steps_agent(agent_id)
```

Each node in the workflow DAG. This is where all the configuration lives — what agent runs, what it's told to do, what shape its output takes, and how it relates to other steps.

| Column | Purpose |
|--------|---------|
| `id` | Referenced by workflow_step_edges, step_documents, agent_executions. |
| `workflow_id` | Which workflow this step belongs to. CASCADE delete — removing a workflow removes all its steps. |
| `agent_id` | Which agent template to use for this step. The agent's system_prompt becomes the LLM system message. |
| `execution_mode` | `'single'` — run once. `'for_each'` — run once per element in the array referenced by `for_each_ref`. |
| `for_each_ref` | Path into a prior step's output array (e.g. `'features.content'`). Only used when `execution_mode = 'for_each'`. The runtime iterates over the array and creates one agent_execution per element. |
| `prompt_template_id` | Reference to a saved prompt_template. If set, the saved template's content is used. If null, falls back to the inline `prompt_template` field. |
| `prompt_template` | Inline prompt text with `{variable}` placeholders. Used when the user writes a one-off prompt instead of selecting a saved template. Ignored if `prompt_template_id` is set. |
| `output_schema_id` | The expected output shape. The agent is instructed to return structured data matching this schema. If null, the agent returns freeform text. |
| `output_variable_name` | Names this step's output so other steps can reference it via `{variable_name}` in their prompt templates. Example: `'features'`, `'tickets'`, `'review_notes'`. |
| `interactive_agent_id` | References an agent template that acts as the reviewer. When not null, the step pauses after the main agent completes. The interactive agent receives the main agent's output, responds with its review/feedback (driven by its own system prompt), and the user chats with it to refine the result. On approval, the interactive agent's final output replaces the step output. When null, the step completes with no pause. |
| `for_each_label_field` | Which field from each array element to use as the display label in the execution tree UI (e.g. `'name'`, `'title'`). Only used when `execution_mode = 'for_each'`. At runtime, `element[for_each_label_field]` populates `for_each_label` in the tree response. If null, falls back to the element index. The workflow editor populates this via a dropdown of the referenced array's element schema fields. |
| `display_order` | Rendering order within the tree UI. Steps are displayed in ascending order. The DAG edges define execution order; this only controls visual layout. |

---

### 9. workflow_step_edges

```sql
workflow_step_edges (
    from_step_id UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    to_step_id UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    PRIMARY KEY (from_step_id, to_step_id)
)
-- idx_workflow_step_edges_from(from_step_id)
-- idx_workflow_step_edges_to(to_step_id)
```

Directed edges that define execution order and parallelism within a workflow. This is what makes workflows a DAG rather than a linear chain.

| Column | Purpose |
|--------|---------|
| `from_step_id` | The step that must complete before `to_step_id` can start. |
| `to_step_id` | The step that waits for `from_step_id` to finish. |

**Execution rules:**
- Steps with **no incoming edges** are entry points — they start immediately when the workflow begins.
- Steps with **multiple incoming edges** wait for **all** parent steps to complete before starting.
- Steps with **no outgoing edges** are terminal nodes — their outputs become the workflow's final output.
- Steps with **multiple outgoing edges** fan out — all children start in parallel once the parent completes.

**Example — 4 parallel sub-agents:**
```
Step A (entry)
  ├──→ Step B (parallel)
  ├──→ Step C (parallel)
  ├──→ Step D (parallel)
  └──→ Step E (parallel)
         all ──→ Step F (merge, waits for B+C+D+E)
```

---

### 10. step_documents

```sql
step_documents (
    step_id UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    PRIMARY KEY (step_id, document_id)
)
-- idx_step_documents_step(step_id)
```

Attaches documents to a workflow step. At runtime, all attached documents' content is appended to the agent's prompt before execution. This is how static context (PRDs, specs, conventions) gets fed to agents.

| Column | Purpose |
|--------|---------|
| `step_id` | The workflow step that receives this document as context. |
| `document_id` | The document to attach. |

---

### 11. pipelines

```sql
pipelines (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
-- idx_pipelines_user(user_id)
```

The top-level orchestration unit. A pipeline is a sequence of stages that execute in order. Each stage can contain multiple workflows running in parallel. Pipelines are reusable — run the same pipeline with different initial input.

| Column | Purpose |
|--------|---------|
| `id` | Referenced by pipeline_stages and pipeline_runs. |
| `user_id` | Owner. |
| `name` | Display name (e.g. `'Feature Implementation Pipeline'`, `'Code Review Pipeline'`). |
| `description` | Optional notes about the pipeline's purpose. |

---

### 12. pipeline_stages

```sql
pipeline_stages (
    pipeline_id UUID NOT NULL REFERENCES pipelines(id) ON DELETE CASCADE,
    stage_number INTEGER NOT NULL,
    stage_name TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (pipeline_id, stage_number)
)
```

Sequential stages within a pipeline. Stage 1 must complete entirely before stage 2 begins. The stage itself is just a container — the actual work is defined by its members (workflows).

| Column | Purpose |
|--------|---------|
| `pipeline_id` | Which pipeline this stage belongs to. |
| `stage_number` | Execution order. Stages run in ascending order. Composite PK with pipeline_id. |
| `stage_name` | Optional display label (e.g. `'Analysis'`, `'Implementation'`, `'Review'`). |

---

### 13. pipeline_stage_members

```sql
pipeline_stage_members (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pipeline_id UUID NOT NULL,
    stage_number INTEGER NOT NULL,
    workflow_id UUID NOT NULL REFERENCES workflows(id),
    display_order INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (pipeline_id, stage_number)
        REFERENCES pipeline_stages(pipeline_id, stage_number) ON DELETE CASCADE
)
-- idx_pipeline_stage_members_stage(pipeline_id, stage_number)
-- idx_pipeline_stage_members_workflow(workflow_id)
```

Which workflows run in each stage. Multiple members per stage = parallel execution. The stage completes when all members complete.

| Column | Purpose |
|--------|---------|
| `id` | Unique member identifier. |
| `pipeline_id` + `stage_number` | Which stage this member belongs to. FK to pipeline_stages. |
| `workflow_id` | The workflow to execute. Always a workflow — even a single agent is wrapped in a one-step workflow. |
| `display_order` | Rendering order within the stage in the tree UI. Workflows are displayed in ascending order. |

---

### 14. pipeline_runs

```sql
pipeline_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pipeline_id UUID NOT NULL REFERENCES pipelines(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id),
    status TEXT NOT NULL DEFAULT 'running',
    initial_input TEXT NOT NULL,
    current_stage INTEGER NOT NULL DEFAULT 0,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
)
-- idx_pipeline_runs_pipeline(pipeline_id)
-- idx_pipeline_runs_user(user_id)
-- idx_pipeline_runs_status(status)
-- idx_pipeline_runs_started(started_at DESC)
```

One row per execution of a pipeline. Everything below this is the execution tree that the UI renders live.

| Column | Purpose |
|--------|---------|
| `id` | Root of the execution tree. The tree UI fetches everything by run_id. |
| `pipeline_id` | Which pipeline definition this run is executing. |
| `user_id` | Who triggered the run. |
| `status` | `'running'` — stages are executing. `'completed'` — all stages done. `'failed'` — a stage failed and wasn't recovered. `'paused'` — waiting on an interactive step. |
| `initial_input` | The user's prompt, file content, or data that kicks off the pipeline. Fed to stage 1's workflows. |
| `current_stage` | Which stage is currently executing. Used for progress display and resumption after pause. |
| `started_at` | Run start time. Index supports "recent runs" query. |
| `completed_at` | Null while running. Set when the run reaches a terminal state. |

---

### 15. stage_executions

```sql
stage_executions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id UUID NOT NULL REFERENCES pipeline_runs(id) ON DELETE CASCADE,
    pipeline_id UUID NOT NULL,
    stage_number INTEGER NOT NULL,
    stage_member_id UUID NOT NULL REFERENCES pipeline_stage_members(id),
    status TEXT NOT NULL DEFAULT 'running',
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    FOREIGN KEY (pipeline_id, stage_number)
        REFERENCES pipeline_stages(pipeline_id, stage_number)
)
-- idx_stage_executions_run(run_id)
-- idx_stage_executions_status(status)
-- idx_stage_executions_stage(pipeline_id, stage_number)
-- idx_stage_executions_member(stage_member_id)
```

One row per workflow execution within a stage. If a stage has 3 workflow members, there are 3 stage_execution rows. This is a status tracker — the real data lives in agent_executions below.

| Column | Purpose |
|--------|---------|
| `id` | Referenced by agent_executions. |
| `run_id` | Which pipeline run this belongs to. Index supports fetching the full tree for a run. |
| `pipeline_id` + `stage_number` | Which stage definition this is executing. |
| `stage_member_id` | Which specific workflow member this execution corresponds to. Links the runtime row back to the design-time configuration. |
| `status` | `'running'` / `'completed'` / `'failed'`. The stage (as a whole) completes when all its stage_execution rows are `'completed'`. |
| `started_at` | When this workflow started executing within the stage. |
| `completed_at` | When this workflow finished. |

---

### 16. agent_executions

```sql
agent_executions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    stage_execution_id UUID NOT NULL REFERENCES stage_executions(id) ON DELETE CASCADE,
    agent_id UUID NOT NULL REFERENCES agents(id),
    workflow_step_id UUID REFERENCES workflow_steps(id),
    is_interactive BOOLEAN NOT NULL DEFAULT FALSE,
    parent_agent_execution_id UUID REFERENCES agent_executions(id),
    system_prompt_rendered TEXT NOT NULL,
    input TEXT NOT NULL,
    output TEXT,
    structured_output JSONB,
    status TEXT NOT NULL DEFAULT 'running',
    input_tokens BIGINT NOT NULL DEFAULT 0,
    output_tokens BIGINT NOT NULL DEFAULT 0,
    cost_usd REAL NOT NULL DEFAULT 0.0,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
)
-- idx_agent_executions_stage(stage_execution_id)
-- idx_agent_executions_agent(agent_id)
-- idx_agent_executions_step(workflow_step_id)
-- idx_agent_executions_status(status)
-- idx_agent_executions_started(started_at DESC)
-- idx_agent_executions_parent(parent_agent_execution_id)
```

The most important runtime table. One row per actual LLM agent invocation. This is the ground truth for what happened — what the agent saw, what it produced, what it cost.

A workflow step with an `interactive_agent_id` produces **two** agent_execution rows:
1. The **main agent** executes the step's prompt and produces output. `is_interactive = false`.
2. The **interactive agent** receives the main agent's output as input. `is_interactive = true`, `parent_agent_execution_id` points to the main agent's row. Status starts as `'awaiting_user'` — the user sees the interactive agent's initial response in chat and refines the output back and forth. On approval, the interactive agent's final `structured_output` replaces the step's output for downstream consumption.

| Column | Purpose |
|--------|---------|
| `id` | The node ID in the execution tree UI. WebSocket events reference this. |
| `stage_execution_id` | Parent stage execution. CASCADE delete — removing a run removes everything. |
| `agent_id` | Which agent template was used. For main executions, this is `workflow_steps.agent_id`. For interactive executions, this is `workflow_steps.interactive_agent_id`. Join to agents for the name and model config. |
| `workflow_step_id` | Which workflow step this execution corresponds to. Links runtime back to the DAG node. Both the main and interactive executions reference the same step. |
| `is_interactive` | `false` = main agent execution. `true` = interactive review agent execution. The UI renders interactive executions with a chat panel instead of a simple status node. |
| `parent_agent_execution_id` | For interactive executions, points to the main agent execution whose output is being reviewed. Null for main executions. Allows the UI to show the main output alongside the interactive chat. |
| `system_prompt_rendered` | The **exact** system prompt after all context was composed — the agent's base system_prompt plus any attached document content. Stored for reproducibility. You can re-run this exact prompt to debug or verify behavior. |
| `input` | For main executions: the resolved prompt template with `{variable}` refs replaced. For interactive executions: the main agent's output that's being reviewed. |
| `output` | The agent's raw text response. |
| `structured_output` | The agent's response parsed against the output_schema. JSONB so it's queryable. Contains the passdown field if the schema includes one. **For interactive executions**: null if the user approved without changes, non-null if the user made changes during the review chat. See "Interactive Output Resolution" below for how downstream steps resolve the final output. |
| `status` | `'running'` — LLM call in progress. `'awaiting_user'` — interactive agent has responded, waiting for user in chat. `'completed'` — done, output available. `'failed'` — LLM error or validation failure. |
| `input_tokens` | Tokens sent to the LLM. |
| `output_tokens` | Tokens received from the LLM. |
| `cost_usd` | Computed cost for this single invocation. |
| `started_at` | When the LLM call started. |
| `completed_at` | When the response was received and processed. `completed_at - started_at` = latency. |

#### Interactive Output Resolution

When a workflow step has an `interactive_agent_id`, two agent_executions are created: the main agent and the interactive review agent. Downstream steps need the "final" output for that step. The rule:

- **Approve as-is** — user approves without changes. The interactive execution completes with `structured_output = NULL`. The main agent's output is used.
- **Approve with changes** — user refined the output during chat. The interactive execution completes with `structured_output` containing the revised data. The revised output is used.

The runtime resolves the final output with:

```sql
SELECT COALESCE(
    (SELECT structured_output FROM agent_executions
     WHERE parent_agent_execution_id = main.id
       AND status = 'completed'
       AND structured_output IS NOT NULL),
    main.structured_output
)
FROM agent_executions main
WHERE main.workflow_step_id = :step_id
  AND main.stage_execution_id = :stage_exec_id
  AND main.is_interactive = false
```

`COALESCE` picks the interactive output if it exists and is non-null, otherwise falls back to the main output.

**UI display:**
- If the interactive execution has `structured_output IS NOT NULL` → show both outputs with a `⚑ Modified output` flag on the interactive result.
- If `structured_output IS NULL` → show `✅ Approved (no changes)`, display only the main output.

**Impact on for_each:** If a step's output is an array consumed by a downstream `for_each` step, and the interactive review changes the array length (adds/removes items), the for_each runs over the approved array. The pipeline always waits for interactive approval before starting downstream steps, so there are never stale for_each iterations.

---

### 17. execution_messages

```sql
execution_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_execution_id UUID NOT NULL REFERENCES agent_executions(id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    tool_call_id TEXT,
    input_tokens BIGINT NOT NULL DEFAULT 0,
    output_tokens BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
-- idx_execution_messages_execution(agent_execution_id)
-- idx_execution_messages_role(agent_execution_id, role)
-- idx_execution_messages_created(created_at)
```

The full LLM conversation for each agent execution. Every message in the thread is a row. For non-interactive steps this captures the single system→user→assistant exchange. For interactive steps this captures the full multi-turn chat between the user and agent.

| Column | Purpose |
|--------|---------|
| `id` | Message identifier. |
| `agent_execution_id` | Which agent execution this message belongs to. Index supports fetching the full conversation. |
| `role` | `'system'` — the rendered system prompt. `'user'` — the task prompt or user's chat message. `'assistant'` — the agent's response. `'tool'` — a tool call or tool result. |
| `content` | The message text. |
| `tool_call_id` | Links a tool result message back to the tool call that produced it. Null for non-tool messages. |
| `input_tokens` | Tokens consumed by this message when sent to the LLM. |
| `output_tokens` | Tokens produced by the LLM in response to this message. |
| `created_at` | Message timestamp. Ordered by this to reconstruct the conversation. |

---

### 18. token_ledger

```sql
token_ledger (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    agent_execution_id UUID NOT NULL REFERENCES agent_executions(id),
    model_id TEXT NOT NULL,
    input_tokens BIGINT NOT NULL,
    output_tokens BIGINT NOT NULL,
    cost_usd REAL NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
-- idx_token_ledger_user(user_id)
-- idx_token_ledger_agent_exec(agent_execution_id)
-- idx_token_ledger_model(model_id)
-- idx_token_ledger_created(created_at DESC)
-- idx_token_ledger_user_created(user_id, created_at DESC)
```

Single source of truth for all LLM spend. One row per LLM call. Separate from agent_executions so cost queries don't have to touch the large execution table.

| Column | Purpose |
|--------|---------|
| `id` | Ledger entry identifier. |
| `user_id` | Who incurred the cost. Index supports "my total spend" queries without joining through the execution tree. |
| `agent_execution_id` | Which execution produced this cost. Join through stage_executions → pipeline_runs for per-run and per-pipeline cost rollups. |
| `model_id` | Which model was used (e.g. `'claude-sonnet-4-20250514'`). Index supports per-model cost breakdown. |
| `input_tokens` | Tokens sent. |
| `output_tokens` | Tokens received. |
| `cost_usd` | Computed dollar cost for this call. |
| `created_at` | When the cost was incurred. Composite index `(user_id, created_at DESC)` powers time-range queries: "how much did I spend this week." |

#### Token Tracking Rule

**Every LLM request MUST produce a `token_ledger` row.** No exceptions. This includes:

- The initial LLM call for a main agent execution (1 row)
- The initial LLM call for an interactive agent execution (1 row)
- Every subsequent LLM round trip during an interactive chat session (1 row per assistant response)
- Any future LLM calls added to the system (summarizers, validators, etc.)

For interactive sessions with multiple chat turns, there will be **multiple `token_ledger` rows** sharing the same `agent_execution_id`. The per-message token counts on `execution_messages` provide the per-turn breakdown. The `token_ledger` rows provide the cost accounting.

**Summing rules:**
- Total cost for one agent execution: `SUM(cost_usd) WHERE agent_execution_id = :id`
- Total cost for one pipeline run: `SUM(cost_usd)` joined through `agent_executions → stage_executions → pipeline_runs`
- Total cost for a user: `SUM(cost_usd) WHERE user_id = :id`

The `agent_executions.input_tokens`, `output_tokens`, and `cost_usd` fields are **running totals** updated after each LLM call. They must always equal the sum of their corresponding `token_ledger` rows.

---

### 19. results

```sql
results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    agent_execution_id UUID NOT NULL REFERENCES agent_executions(id),
    output_schema_id UUID REFERENCES output_schemas(id),
    name TEXT NOT NULL,
    data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
-- idx_results_user(user_id)
-- idx_results_execution(agent_execution_id)
-- idx_results_schema(output_schema_id)
```

Saved structured outputs from agent executions. Promoted to a standalone entity so results are browsable, selectable, and referenceable in the UI independently of the execution that created them. Can be used as input to future pipeline runs or referenced when configuring new workflows.

| Column | Purpose |
|--------|---------|
| `id` | Result identifier. Selectable in the UI. |
| `user_id` | Owner. Index supports "browse my results" view. |
| `agent_execution_id` | Which execution produced this result. Traceability back to the full execution context. |
| `output_schema_id` | Which schema this result conforms to. Index supports "show all results matching this schema" — useful when the user wants to pick a prior result as input. |
| `name` | User-facing label. Could be auto-generated from the output_variable_name or manually renamed. |
| `data` | The structured output JSONB. Same data as `agent_executions.structured_output` but saved as an independent, browsable entity. |

---

## Lineage

```
users
  ├── agents                              (reusable LLM agent templates)
  │     └── agent_tools                  (N tools per agent)
  ├── tools                               (tool metadata — name, description, parameters)
  ├── output_schemas                      (reusable structured output shapes)
  ├── prompt_templates                    (reusable prompt text with {variable} refs)
  ├── documents                           (attachable context — PRDs, specs, conventions)
  ├── results                             (saved structured outputs, browsable + selectable)
  ├── workflows                           (reusable execution DAGs)
  │     └── workflow_steps                (DAG nodes — agent + config per step)
  │           ├── workflow_step_edges     (DAG edges — execution order + parallelism)
  │           └── step_documents          (attached documents per step)
  ├── pipelines                           (sequential stage orchestration)
  │     └── pipeline_stages               (ordered stages)
  │           └── pipeline_stage_members  (N workflows per stage, parallel)
  └── pipeline_runs                       (one execution of a pipeline)
        └── stage_executions              (one per workflow member per stage)
              └── agent_executions        (one per LLM invocation)
                    ├── execution_messages (full LLM conversation)
                    └── token_ledger      (cost per call)
```

---

## Variable Resolution

When the runtime encounters a `{variable}` reference in a prompt template, it resolves it by:

1. Walk backwards through completed workflow steps (within the same workflow) and completed stage members (from prior stages).
2. Find the step/member whose `output_variable_name` matches the variable name.
3. Pull the `structured_output` JSONB from that step's `agent_execution`.
4. Dot-path access for nested refs: `{features.content.0.name}` navigates into the JSONB.

**Scope rules:**
- Within a workflow, steps can reference any ancestor step's output by variable name.
- Across stages, steps can reference any completed prior stage's member outputs by variable name.
- Variable names must be unique within their scope (the UI validates this at design time).

---

## Table Count: 21

| # | Table | Layer | Purpose |
|---|-------|-------|---------|
| 1 | users | Definition | User accounts |
| 2 | sessions | Definition | Auth sessions |
| 3 | agents | Definition | Reusable agent templates |
| 4 | output_schemas | Definition | Reusable output shapes |
| 5 | prompt_templates | Definition | Reusable prompt text |
| 6 | documents | Definition | Attachable context documents |
| 7 | tools | Definition | Tool metadata (name, description, parameters) |
| 8 | agent_tools | Wiring | Which tools each agent can use |
| 9 | workflows | Definition | Reusable execution DAGs |
| 10 | workflow_steps | Wiring | DAG nodes |
| 11 | workflow_step_edges | Wiring | DAG edges |
| 12 | step_documents | Wiring | Document attachments per step |
| 13 | pipelines | Definition | Stage sequences |
| 14 | pipeline_stages | Wiring | Ordered stages |
| 15 | pipeline_stage_members | Wiring | Workflows per stage |
| 16 | pipeline_runs | Execution | Pipeline run instance |
| 17 | stage_executions | Execution | Workflow execution per stage |
| 18 | agent_executions | Execution | LLM invocation record |
| 19 | execution_messages | Execution | Full LLM conversation |
| 20 | token_ledger | Execution | Cost tracking |
| 21 | results | Execution | Saved structured outputs |
