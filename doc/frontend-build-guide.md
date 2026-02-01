# Nexor Frontend Build Guide

This document covers the full frontend implementation for the pipeline builder and execution system. It maps every database table to API endpoints, UI pages, components, and real-time behavior.

---

## Pages Overview

| Page | Route | Purpose |
|------|-------|---------|
| Agents | `/agents` | CRUD agent templates |
| Agent Detail | `/agents/:id` | Edit agent system prompt, model config |
| Output Schemas | `/schemas` | CRUD reusable output shapes |
| Schema Detail | `/schemas/:id` | Edit schema fields |
| Prompt Templates | `/prompts` | CRUD reusable prompt text |
| Prompt Detail | `/prompts/:id` | Edit prompt content with `{variable}` preview |
| Documents | `/documents` | CRUD context documents |
| Document Detail | `/documents/:id` | Edit document content |
| Workflows | `/workflows` | List user's workflows |
| Workflow Editor | `/workflows/:id` | Build/edit workflow DAG (tree UI) |
| Pipelines | `/pipelines` | List user's pipelines |
| Pipeline Editor | `/pipelines/:id` | Build/edit pipeline stages + members (tree UI) |
| Pipeline Run | `/pipelines/:pipelineId/runs/:runId` | Live execution tree with status updates |
| Results | `/results` | Browse saved structured outputs |
| Cost Dashboard | `/costs` | Token usage and spend breakdown |

---

## API Endpoints

All endpoints are prefixed with `/api`. All require `Authorization: Bearer <token>` header. All return JSON. All list endpoints support `?page=1&per_page=50`.

### Auth

| Method | Path | Body | Returns | Notes |
|--------|------|------|---------|-------|
| `POST` | `/auth/login` | `{ email, password }` | `{ token, user }` | Returns bearer token |
| `POST` | `/auth/register` | `{ email, password }` | `{ token, user }` | |
| `POST` | `/auth/logout` | — | `204` | Invalidates session |
| `GET` | `/auth/me` | — | `{ user }` | Current user from token |

### Agents

| Method | Path | Body | Returns |
|--------|------|------|---------|
| `GET` | `/agents` | — | `Agent[]` |
| `POST` | `/agents` | `{ name, system_prompt, model_provider, model_id, model_max_tokens, model_temperature }` | `Agent` |
| `GET` | `/agents/:id` | — | `Agent` |
| `PUT` | `/agents/:id` | `{ name?, system_prompt?, model_provider?, model_id?, model_max_tokens?, model_temperature? }` | `Agent` |
| `DELETE` | `/agents/:id` | — | `204` |

### Output Schemas

| Method | Path | Body | Returns |
|--------|------|------|---------|
| `GET` | `/output-schemas` | — | `OutputSchema[]` |
| `POST` | `/output-schemas` | `{ name, schema }` | `OutputSchema` |
| `GET` | `/output-schemas/:id` | — | `OutputSchema` |
| `PUT` | `/output-schemas/:id` | `{ name?, schema? }` | `OutputSchema` |
| `DELETE` | `/output-schemas/:id` | — | `204` |

### Prompt Templates

| Method | Path | Body | Returns |
|--------|------|------|---------|
| `GET` | `/prompt-templates` | — | `PromptTemplate[]` |
| `POST` | `/prompt-templates` | `{ name, content }` | `PromptTemplate` |
| `GET` | `/prompt-templates/:id` | — | `PromptTemplate` |
| `PUT` | `/prompt-templates/:id` | `{ name?, content? }` | `PromptTemplate` |
| `DELETE` | `/prompt-templates/:id` | — | `204` |

### Documents

| Method | Path | Body | Returns |
|--------|------|------|---------|
| `GET` | `/documents` | — | `Document[]` |
| `POST` | `/documents` | `{ name, content }` | `Document` |
| `GET` | `/documents/:id` | — | `Document` |
| `PUT` | `/documents/:id` | `{ name?, content? }` | `Document` |
| `DELETE` | `/documents/:id` | — | `204` |

### Workflows

| Method | Path | Body | Returns |
|--------|------|------|---------|
| `GET` | `/workflows` | — | `Workflow[]` |
| `POST` | `/workflows` | `{ name, description }` | `Workflow` |
| `GET` | `/workflows/:id` | — | `Workflow` (with steps, edges, step_documents) |
| `PUT` | `/workflows/:id` | `{ name?, description? }` | `Workflow` |
| `DELETE` | `/workflows/:id` | — | `204` |

### Workflow Steps

| Method | Path | Body | Returns |
|--------|------|------|---------|
| `POST` | `/workflows/:id/steps` | `{ agent_id, execution_mode, for_each_ref?, prompt_template_id?, prompt_template?, output_schema_id?, output_variable_name?, interactive_agent_id?, display_order }` | `WorkflowStep` |
| `PUT` | `/workflows/:wid/steps/:sid` | (any field) | `WorkflowStep` |
| `DELETE` | `/workflows/:wid/steps/:sid` | — | `204` |

### Workflow Step Edges

| Method | Path | Body | Returns |
|--------|------|------|---------|
| `POST` | `/workflows/:id/edges` | `{ from_step_id, to_step_id }` | `WorkflowStepEdge` |
| `DELETE` | `/workflows/:id/edges` | `{ from_step_id, to_step_id }` | `204` |

### Step Documents

| Method | Path | Body | Returns |
|--------|------|------|---------|
| `POST` | `/workflows/:wid/steps/:sid/documents` | `{ document_id }` | `StepDocument` |
| `DELETE` | `/workflows/:wid/steps/:sid/documents/:did` | — | `204` |

### Pipelines

| Method | Path | Body | Returns |
|--------|------|------|---------|
| `GET` | `/pipelines` | — | `Pipeline[]` |
| `POST` | `/pipelines` | `{ name, description }` | `Pipeline` |
| `GET` | `/pipelines/:id` | — | `Pipeline` (with stages, members) |
| `PUT` | `/pipelines/:id` | `{ name?, description? }` | `Pipeline` |
| `DELETE` | `/pipelines/:id` | — | `204` |

### Pipeline Stages

| Method | Path | Body | Returns |
|--------|------|------|---------|
| `POST` | `/pipelines/:id/stages` | `{ stage_number, stage_name }` | `PipelineStage` |
| `PUT` | `/pipelines/:pid/stages/:num` | `{ stage_name? }` | `PipelineStage` |
| `DELETE` | `/pipelines/:pid/stages/:num` | — | `204` |

### Pipeline Stage Members

| Method | Path | Body | Returns |
|--------|------|------|---------|
| `POST` | `/pipelines/:pid/stages/:num/members` | `{ workflow_id, display_order }` | `PipelineStageMember` |
| `PUT` | `/pipelines/:pid/stages/:num/members/:mid` | `{ display_order? }` | `PipelineStageMember` |
| `DELETE` | `/pipelines/:pid/stages/:num/members/:mid` | — | `204` |

### Pipeline Runs

| Method | Path | Body | Returns |
|--------|------|------|---------|
| `POST` | `/pipelines/:id/runs` | `{ initial_input }` | `PipelineRun` |
| `GET` | `/pipelines/:pid/runs` | — | `PipelineRun[]` |
| `GET` | `/pipelines/:pid/runs/:rid` | — | `PipelineRun` (summary) |
| `GET` | `/pipelines/:pid/runs/:rid/tree` | — | Full execution tree (see below) |

### Interactive Chat

| Method | Path | Body | Returns |
|--------|------|------|---------|
| `GET` | `/agent-executions/:id/messages` | — | `ExecutionMessage[]` |
| `POST` | `/agent-executions/:id/messages` | `{ content }` | `ExecutionMessage` |
| `POST` | `/agent-executions/:id/approve` | `{ structured_output? }` | `AgentExecution` |

`approve` with no body or `structured_output: null` = approve as-is. With `structured_output` = approve with changes.

### Variable Intellisense

| Method | Path | Returns |
|--------|------|---------|
| `GET` | `/pipelines/:id/stages/:num/available-variables` | `AvailableVariable[]` |
| `GET` | `/workflows/:id/steps/:sid/available-variables` | `AvailableVariable[]` |

```typescript
type AvailableVariable = {
    ref: string               // 'stage.1.workflow_name.step_name'
    variable_name: string     // 'features'
    schema: Record<string, { type: string; description: string }> | null
    is_array: boolean
}
```

### Results

| Method | Path | Body | Returns |
|--------|------|------|---------|
| `GET` | `/results` | — | `Result[]` |
| `GET` | `/results?schema_id=:id` | — | `Result[]` (filtered by schema) |
| `GET` | `/results/:id` | — | `Result` |
| `DELETE` | `/results/:id` | — | `204` |

### Token Ledger / Cost

| Method | Path | Returns |
|--------|------|---------|
| `GET` | `/costs?from=:date&to=:date` | `{ total_input_tokens, total_output_tokens, total_cost_usd, by_model: [...] }` |
| `GET` | `/costs/runs/:rid` | `{ total_input_tokens, total_output_tokens, total_cost_usd, by_agent: [...] }` |

---

## WebSocket Events

Connect to `ws://<host>/ws?token=<bearer_token>`.

### Server → Client Events

**Pipeline run status change:**
```json
{
    "event": "pipeline_run_update",
    "run_id": "uuid",
    "status": "running",
    "current_stage": 2
}
```

**Stage execution status change:**
```json
{
    "event": "stage_execution_update",
    "run_id": "uuid",
    "stage_execution_id": "uuid",
    "stage_number": 1,
    "status": "completed"
}
```

**Agent execution status change (the main one — powers live tree):**
```json
{
    "event": "agent_execution_update",
    "run_id": "uuid",
    "agent_execution_id": "uuid",
    "workflow_step_id": "uuid",
    "agent_name": "Dave",
    "is_interactive": false,
    "status": "completed",
    "structured_output": { "name": "features", "content": [...], "passdown": "Found 6 features" },
    "input_tokens": 1200,
    "output_tokens": 340,
    "cost_usd": 0.02
}
```

**Interactive chat message (new message from agent in review):**
```json
{
    "event": "execution_message",
    "agent_execution_id": "uuid",
    "message": {
        "id": "uuid",
        "role": "assistant",
        "content": "I've reviewed the ticket. The acceptance criteria are missing...",
        "created_at": "2025-01-15T10:00:00Z"
    }
}
```

**For_each expansion (new agent executions spawned):**
```json
{
    "event": "for_each_spawned",
    "run_id": "uuid",
    "stage_execution_id": "uuid",
    "workflow_step_id": "uuid",
    "agent_executions": [
        { "id": "uuid", "agent_name": "Ticket Writer", "status": "pending", "iteration_index": 0 },
        { "id": "uuid", "agent_name": "Ticket Writer", "status": "pending", "iteration_index": 1 },
        { "id": "uuid", "agent_name": "Ticket Writer", "status": "pending", "iteration_index": 2 }
    ]
}
```

### Client → Server Events

**Subscribe to a run (start receiving events for this run):**
```json
{
    "action": "subscribe_run",
    "run_id": "uuid"
}
```

**Unsubscribe:**
```json
{
    "action": "unsubscribe_run",
    "run_id": "uuid"
}
```

---

## Full Execution Tree Response

`GET /api/pipelines/:pid/runs/:rid/tree`

This is the single most important endpoint. It returns the complete execution state that the tree UI renders.

```json
{
    "run": {
        "id": "uuid",
        "pipeline_id": "uuid",
        "pipeline_name": "Feature Builder",
        "status": "running",
        "initial_input": "Build a component library for the design system",
        "current_stage": 2,
        "started_at": "2025-01-15T10:00:00Z",
        "completed_at": null,
        "total_input_tokens": 14200,
        "total_output_tokens": 3800,
        "total_cost_usd": 0.24
    },
    "stages": [
        {
            "stage_number": 1,
            "stage_name": "Analysis",
            "status": "completed",
            "stage_executions": [
                {
                    "id": "uuid",
                    "workflow_name": "Project Review",
                    "status": "completed",
                    "agent_executions": [
                        {
                            "id": "uuid",
                            "agent_name": "Project Analyst",
                            "workflow_step_id": "uuid",
                            "is_interactive": false,
                            "status": "completed",
                            "structured_output": {
                                "conventions": "strict TypeScript, function components...",
                                "passdown": "Found 12 conventions, strict TS required"
                            },
                            "input_tokens": 1200,
                            "output_tokens": 340,
                            "cost_usd": 0.02,
                            "started_at": "2025-01-15T10:00:01Z",
                            "completed_at": "2025-01-15T10:00:08Z",
                            "interactive_review": null
                        }
                    ]
                },
                {
                    "id": "uuid",
                    "workflow_name": "Codebase Scan",
                    "status": "completed",
                    "agent_executions": [
                        {
                            "id": "uuid",
                            "agent_name": "Scanner",
                            "workflow_step_id": "uuid",
                            "is_interactive": false,
                            "status": "completed",
                            "structured_output": {
                                "existing_components": ["Button", "Card", "Modal"],
                                "passdown": "Scanned 48 files, 3 existing components"
                            },
                            "input_tokens": 2100,
                            "output_tokens": 520,
                            "cost_usd": 0.04,
                            "started_at": "2025-01-15T10:00:01Z",
                            "completed_at": "2025-01-15T10:00:12Z",
                            "interactive_review": null
                        }
                    ]
                }
            ]
        },
        {
            "stage_number": 2,
            "stage_name": "Decomposition",
            "status": "running",
            "stage_executions": [
                {
                    "id": "uuid",
                    "workflow_name": "Feature Decomposer",
                    "status": "running",
                    "agent_executions": [
                        {
                            "id": "uuid",
                            "agent_name": "Analyst",
                            "workflow_step_id": "uuid",
                            "is_interactive": false,
                            "status": "completed",
                            "structured_output": {
                                "features": [
                                    { "name": "Reusable Button", "language": "React" },
                                    { "name": "Data Table", "language": "React" },
                                    { "name": "Form Input", "language": "React" },
                                    { "name": "Toast Notifications", "language": "React" },
                                    { "name": "Dropdown Menu", "language": "React" },
                                    { "name": "Tabs Component", "language": "React" }
                                ],
                                "passdown": "Decomposed into 6 features"
                            },
                            "input_tokens": 1800,
                            "output_tokens": 420,
                            "cost_usd": 0.03,
                            "started_at": "2025-01-15T10:00:13Z",
                            "completed_at": "2025-01-15T10:00:20Z",
                            "interactive_review": {
                                "id": "uuid",
                                "agent_name": "Feature Reviewer",
                                "is_interactive": true,
                                "status": "completed",
                                "structured_output": null,
                                "modified": false,
                                "input_tokens": 900,
                                "output_tokens": 180,
                                "cost_usd": 0.01
                            }
                        },
                        {
                            "id": "uuid",
                            "agent_name": "Ticket Writer",
                            "workflow_step_id": "uuid",
                            "is_interactive": false,
                            "status": "completed",
                            "for_each_index": 0,
                            "for_each_label": "Reusable Button",
                            "structured_output": {
                                "ticket": { "title": "Implement Reusable Button", "description": "..." },
                                "passdown": "Button component ticket ready"
                            },
                            "input_tokens": 800,
                            "output_tokens": 200,
                            "cost_usd": 0.01,
                            "started_at": "2025-01-15T10:00:21Z",
                            "completed_at": "2025-01-15T10:00:28Z",
                            "interactive_review": null
                        },
                        {
                            "id": "uuid",
                            "agent_name": "Ticket Writer",
                            "workflow_step_id": "uuid",
                            "is_interactive": false,
                            "status": "running",
                            "for_each_index": 1,
                            "for_each_label": "Data Table",
                            "structured_output": null,
                            "input_tokens": 800,
                            "output_tokens": 0,
                            "cost_usd": 0.0,
                            "started_at": "2025-01-15T10:00:21Z",
                            "completed_at": null,
                            "interactive_review": null
                        },
                        {
                            "id": "uuid",
                            "agent_name": "Ticket Writer",
                            "workflow_step_id": "uuid",
                            "is_interactive": false,
                            "status": "running",
                            "for_each_index": 2,
                            "for_each_label": "Form Input",
                            "structured_output": null,
                            "input_tokens": 800,
                            "output_tokens": 0,
                            "cost_usd": 0.0,
                            "started_at": "2025-01-15T10:00:21Z",
                            "completed_at": null,
                            "interactive_review": null
                        },
                        {
                            "id": "uuid",
                            "agent_name": "Ticket Writer",
                            "workflow_step_id": "uuid",
                            "is_interactive": false,
                            "status": "pending",
                            "for_each_index": 3,
                            "for_each_label": "Toast Notifications",
                            "structured_output": null,
                            "input_tokens": 0,
                            "output_tokens": 0,
                            "cost_usd": 0.0,
                            "started_at": null,
                            "completed_at": null,
                            "interactive_review": null
                        },
                        {
                            "id": "uuid",
                            "agent_name": "Ticket Writer",
                            "workflow_step_id": "uuid",
                            "is_interactive": false,
                            "status": "pending",
                            "for_each_index": 4,
                            "for_each_label": "Dropdown Menu",
                            "structured_output": null,
                            "input_tokens": 0,
                            "output_tokens": 0,
                            "cost_usd": 0.0,
                            "started_at": null,
                            "completed_at": null,
                            "interactive_review": null
                        },
                        {
                            "id": "uuid",
                            "agent_name": "Ticket Writer",
                            "workflow_step_id": "uuid",
                            "is_interactive": false,
                            "status": "pending",
                            "for_each_index": 5,
                            "for_each_label": "Tabs Component",
                            "structured_output": null,
                            "input_tokens": 0,
                            "output_tokens": 0,
                            "cost_usd": 0.0,
                            "started_at": null,
                            "completed_at": null,
                            "interactive_review": null
                        }
                    ]
                }
            ]
        },
        {
            "stage_number": 3,
            "stage_name": "Implementation",
            "status": "pending",
            "stage_executions": []
        }
    ]
}
```

---

## Execution Tree UI

### Tree Layout (Pipeline Run Page)

The run page renders a vertical tree from the `/tree` response. Every node is an `agent_execution`.

```
┌─────────────────────────────────────────────────────────────────┐
│ Pipeline Run: "Feature Builder"                    ● RUNNING    │
│ Input: "Build a component library for the design system"        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Stage 1: Analysis ✅                                           │
│  │                                                              │
│  ├─ Workflow: "Project Review"                                  │
│  │  └─ Project Analyst                    ✅ completed  $0.02   │
│  │     passdown: Found 12 conventions, strict TS required       │
│  │                                                              │
│  └─ Workflow: "Codebase Scan"                                   │
│     └─ Scanner                            ✅ completed  $0.04   │
│        passdown: Scanned 48 files, 3 existing components        │
│                                                                 │
│  ──────────────────────────────────────────────────────────     │
│                                                                 │
│  Stage 2: Decomposition ● running                               │
│  │                                                              │
│  └─ Workflow: "Feature Decomposer"                              │
│     │                                                           │
│     ├─ Analyst                            ✅ completed  $0.03   │
│     │  passdown: Decomposed into 6 features                     │
│     │  ├─ Review: Feature Reviewer        ✅ approved   $0.01   │
│     │  │  ✅ Approved (no changes)                               │
│     │                                                           │
│     └─ for_each features (6 items):                             │
│        ├─ [0] Ticket Writer: Reusable Button  ✅ done   $0.01   │
│        │  passdown: Button component ticket ready                │
│        ├─ [1] Ticket Writer: Data Table       ● running         │
│        ├─ [2] Ticket Writer: Form Input       ● running         │
│        ├─ [3] Ticket Writer: Toast            ○ pending         │
│        ├─ [4] Ticket Writer: Dropdown         ○ pending         │
│        └─ [5] Ticket Writer: Tabs             ○ pending         │
│                                                                 │
│  ──────────────────────────────────────────────────────────     │
│                                                                 │
│  Stage 3: Implementation ○ pending                              │
│  └─ (waiting for stage 2)                                       │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│ Tokens: 14,200 in / 3,800 out │ Cost: $0.24 │ Elapsed: 2m 12s │
└─────────────────────────────────────────────────────────────────┘
```

### Node States

| Status | Icon | Color | Behavior |
|--------|------|-------|----------|
| `pending` | ○ | gray | Waiting for dependencies |
| `running` | ● (animated) | blue | LLM call in progress |
| `awaiting_user` | ⏸ | yellow | Interactive chat open, waiting for user |
| `completed` | ✅ | green | Done, output available |
| `failed` | ✗ | red | Error, click to see details |

### Interactive Review Display

When a step has an interactive review, it renders nested under the main agent:

**No changes (approved as-is):**
```
├─ Analyst                            ✅ completed  $0.03
│  passdown: Decomposed into 6 features
│  ├─ Review: Feature Reviewer        ✅ approved   $0.01
│  │  ✅ Approved (no changes)
```

**With changes:**
```
├─ Analyst                            ✅ completed  $0.03
│  passdown: Decomposed into 6 features
│  output: { features: [A, B, C, D, E] }
│  ├─ Review: Feature Reviewer        ✅ approved   $0.02
│  │  ⚑ Modified output
│  │  output: { features: [A, C, E] }
```

**Awaiting user (chat open):**
```
├─ Ticket Writer                      ✅ completed  $0.01
│  passdown: Button component ticket ready
│  ├─ Review: Ticket Reviewer         ⏸ awaiting user
│  │  ┌──────────────────────────────────────────┐
│  │  │ Agent: The acceptance criteria are        │
│  │  │ missing edge cases for disabled state...  │
│  │  │                                           │
│  │  │ You: Good point, add those.               │
│  │  │                                           │
│  │  │ Agent: Updated. Here's the revised...     │
│  │  │                                           │
│  │  │ [Type a message...]          [Approve]    │
│  │  └──────────────────────────────────────────┘
```

### Clicking a Completed Node

Expands an inspection panel showing:

1. **Agent info** — name, model, system prompt
2. **Input** — the resolved prompt the agent received
3. **Output** — raw text response
4. **Structured output** — parsed JSON with syntax highlighting
5. **Documents attached** — list of documents that were injected
6. **Messages** — full LLM conversation (system, user, assistant, tool calls)
7. **Cost** — input/output tokens, cost USD, latency

---

## Pipeline Builder UI (Editor Pages)

### Workflow Editor (`/workflows/:id`)

A vertical tree editor. Each step is an expandable row.

```
Workflow: "Feature Decomposer"                          [Save] [Run Test]
─────────────────────────────────────────────────────────────────────────

  Step 1                                                    [+ Add Step After]
  ├─ Agent: [▼ Analyst                    ]
  ├─ Prompt: [▼ Use saved template  |  Write custom ▼]
  │  ┌──────────────────────────────────────────────────────────────────┐
  │  │ Review {conventions} and create a list of stateless components   │
  │  │ for the front end in React.                                     │ <- intellisense on {
  │  └──────────────────────────────────────────────────────────────────┘
  ├─ Output Schema: [▼ feature_list       ]
  ├─ Output Variable: [ features          ]
  ├─ Documents: [▼ Project Requirements] [+ Add]
  ├─ Review Agent: [▼ Feature Reviewer    ]        <- dropdown, "None" to disable
  ├─ Execution Mode: (● Single) (○ For Each)
  └─ Depends On: [no dependencies — entry step]

  Step 2                                                    [+ Add Step After]
  ├─ Agent: [▼ Ticket Writer              ]
  ├─ Prompt:
  │  ┌──────────────────────────────────────────────────────────────────┐
  │  │ Create a comprehensive ticket for {features.content.$.name}     │
  │  └──────────────────────────────────────────────────────────────────┘
  ├─ Output Schema: [▼ ticket             ]
  ├─ Output Variable: [ tickets           ]
  ├─ Documents: [none]
  ├─ Review Agent: [▼ Ticket Reviewer     ]
  ├─ Execution Mode: (○ Single) (● For Each)
  │  └─ For Each Ref: [▼ features.content ]     <- dropdown of available arrays
  └─ Depends On: [▼ Step 1 (Analyst)     ]      <- multi-select, defines edges

[+ Add Step]
```

**Depends On** is how edges are created. Selecting "Step 1" as a dependency creates a `workflow_step_edges` row from step 1 to step 2. Multi-select supports merge nodes (step depends on multiple parents).

**The `$` in for_each prompts** is a placeholder for the current iteration element. `{features.content.$.name}` means "for each element in `features.content`, access its `.name`."

### Pipeline Editor (`/pipelines/:id`)

```
Pipeline: "Feature Implementation Pipeline"                [Save] [Run]
──────────────────────────────────────────────────────────────────────

  Stage 1: [ Analysis                     ]         [+ Add Workflow] [✗]
  │
  ├─ Workflow: [▼ Project Review          ]                          [✗]
  └─ Workflow: [▼ Codebase Scan           ]                          [✗]

  Stage 2: [ Decomposition               ]         [+ Add Workflow] [✗]
  │
  └─ Workflow: [▼ Feature Decomposer      ]                          [✗]

  Stage 3: [ Implementation              ]         [+ Add Workflow] [✗]
  │
  └─ Workflow: [▼ Builder                 ]                          [✗]

[+ Add Stage]

Pipeline Input:
┌──────────────────────────────────────────────────────────────────┐
│ Build a component library for the design system                  │
│                                                                  │
│                                            [▶ Run Pipeline]      │
└──────────────────────────────────────────────────────────────────┘
```

Each stage is a collapsible section. Workflows are selected from a dropdown of the user's saved workflows. Stages are reorderable via drag. Workflows within a stage are reorderable.

---

## Intellisense System

When a user types `{` in any prompt template field, the frontend:

1. Calls `GET /api/workflows/:id/steps/:sid/available-variables` (or the pipeline-level equivalent)
2. Shows a dropdown of available variables with their types
3. On selecting a variable, continues with `.` for nested access
4. Schema fields are shown at each level

```
{                           ← user types {
┌──────────────────────┐
│ conventions    Object │    ← from stage 1, step 1
│ dependencies   Array  │    ← from stage 1, step 2
│ features       Object │    ← from current workflow, step 1
└──────────────────────┘

{features.                   ← user selects features, types .
┌──────────────────────┐
│ name          String  │
│ content       Array   │
│ passdown      String  │
└──────────────────────┘

{features.content.           ← user selects content, types .
┌──────────────────────┐
│ $  (for_each item)   │    ← only shown in for_each steps
│ 0  (first element)   │
│ 1  (second element)  │
└──────────────────────┘

{features.content.$.         ← user selects $
┌──────────────────────┐
│ name          String  │
│ language      String  │
│ component_name String │
│ component_path String │
└──────────────────────┘
```

---

## State Management

Following the existing codebase conventions (vanilla React, no external state libraries):

### New Contexts

| Context | State | Purpose |
|---------|-------|---------|
| `WorkflowContext` | Workflows list, current workflow with steps/edges | Workflow CRUD + builder state |
| `PipelineBuilderContext` | Current pipeline with stages/members | Pipeline editor state |
| `PipelineRunContext` | Current run tree, live updates | Execution tree + WebSocket events |
| `OutputSchemaContext` | Schemas list | Schema CRUD |
| `PromptTemplateContext` | Templates list | Template CRUD |
| `ResultContext` | Results list | Browsing saved outputs |

### New Hooks

| Hook | Purpose |
|------|---------|
| `useWorkflowContext` | Access workflow context (throws outside provider) |
| `useWorkflows` | Fetch workflows list |
| `useWorkflowMutations` | Create/update/delete workflows, steps, edges |
| `usePipelineBuilderContext` | Access pipeline builder context |
| `usePipelineRunContext` | Access run tree + live state |
| `usePipelineRunTree` | Fetch full tree, subscribe to WebSocket updates |
| `useOutputSchemaContext` | Access schema context |
| `useOutputSchemas` | Fetch schemas list |
| `useOutputSchemaMutations` | Create/update/delete schemas |
| `usePromptTemplateContext` | Access template context |
| `usePromptTemplates` | Fetch templates list |
| `usePromptTemplateMutations` | Create/update/delete templates |
| `useAvailableVariables` | Fetch available variables for intellisense |
| `useInteractiveChat` | Fetch messages, send messages, approve for an interactive agent execution |
| `useResultContext` | Access result context |
| `useResults` | Fetch results list |

### WebSocket Integration for Live Tree

The `usePipelineRunTree` hook:

1. Fetches the initial tree via `GET /api/pipelines/:pid/runs/:rid/tree`
2. Subscribes to the run via WebSocket `subscribe_run`
3. On each `agent_execution_update` event, patches the tree state
4. On `for_each_spawned`, adds new nodes to the tree
5. On `stage_execution_update`, updates stage status
6. On `pipeline_run_update`, updates the run header
7. On unmount, sends `unsubscribe_run`

```typescript
const usePipelineRunTree = (pipelineId: string, runId: string) => {
    const [tree, dispatch] = useReducer(treeReducer, null)
    const ws = useWebSocket()

    // Initial fetch
    useEffect(() => { ... fetch tree, dispatch({ type: 'SET_TREE', payload }) }, [runId])

    // WebSocket subscription
    useEffect(() => {
        ws.send({ action: 'subscribe_run', run_id: runId })
        ws.on('agent_execution_update', (data) => dispatch({ type: 'UPDATE_AGENT_EXECUTION', payload: data }))
        ws.on('for_each_spawned', (data) => dispatch({ type: 'ADD_FOR_EACH_NODES', payload: data }))
        ws.on('stage_execution_update', (data) => dispatch({ type: 'UPDATE_STAGE_EXECUTION', payload: data }))
        ws.on('pipeline_run_update', (data) => dispatch({ type: 'UPDATE_RUN', payload: data }))
        return () => ws.send({ action: 'unsubscribe_run', run_id: runId })
    }, [runId])

    return tree
}
```

---

## Component Hierarchy

### Pipeline Run Page

```
PipelineRunPage
├── RunHeader (pipeline name, status, input, elapsed time)
├── StageList
│   └── StageSection (one per stage)
│       ├── StageHeader (stage name, status badge)
│       └── WorkflowExecutionList
│           └── WorkflowExecution (one per stage_execution)
│               └── AgentExecutionList
│                   └── AgentExecutionNode (one per agent_execution)
│                       ├── NodeStatusIcon
│                       ├── AgentName
│                       ├── PassdownText
│                       ├── TokenCostBadge
│                       ├── InteractiveReviewSection (if interactive)
│                       │   ├── ReviewStatusBadge (approved / modified / awaiting)
│                       │   ├── OutputComparison (original vs modified, if changed)
│                       │   └── ChatPanel (if awaiting_user)
│                       │       ├── MessageList
│                       │       ├── MessageInput
│                       │       └── ApproveButton
│                       ├── ForEachGroup (if for_each step)
│                       │   └── AgentExecutionNode (one per iteration)
│                       └── ExecutionInspector (expandable on click)
│                           ├── AgentInfoSection
│                           ├── InputSection
│                           ├── OutputSection
│                           ├── DocumentsSection
│                           ├── MessagesSection
│                           └── CostSection
└── RunFooter (total tokens, total cost, elapsed)
```

### Workflow Editor Page

```
WorkflowEditorPage
├── WorkflowHeader (name, description, save/run buttons)
└── StepList
    └── StepEditor (one per workflow_step, expandable)
        ├── AgentSelector (dropdown of user's agents)
        ├── PromptEditor
        │   ├── TemplateSelector (dropdown: saved template or custom)
        │   └── PromptTextArea (with IntellisenseOverlay)
        │       └── VariableDropdown (on { keypress)
        ├── OutputSchemaSelector (dropdown of user's schemas)
        ├── OutputVariableInput (text field)
        ├── DocumentSelector (multi-select of user's documents)
        ├── ReviewAgentSelector (dropdown: None or agent)
        ├── ExecutionModeToggle (single / for_each)
        │   └── ForEachRefSelector (dropdown of available array variables)
        └── DependsOnSelector (multi-select of other steps in this workflow)
```

### Pipeline Editor Page

```
PipelineEditorPage
├── PipelineHeader (name, description, save/run buttons)
├── StageList
│   └── StageEditor (one per stage, collapsible)
│       ├── StageNameInput
│       ├── MemberList
│       │   └── MemberRow (workflow selector dropdown, reorderable)
│       └── AddWorkflowButton
├── AddStageButton
└── PipelineInputArea (text area + run button)
```

---

## Flow of Execution (Full Lifecycle)

### 1. User Builds Definitions

```
User creates agents           → POST /api/agents
User creates output schemas   → POST /api/output-schemas
User creates prompt templates  → POST /api/prompt-templates
User creates documents        → POST /api/documents
```

### 2. User Builds a Workflow

```
User creates workflow          → POST /api/workflows
User adds steps               → POST /api/workflows/:id/steps (for each step)
User draws edges              → POST /api/workflows/:id/edges (for each dependency)
User attaches documents       → POST /api/workflows/:wid/steps/:sid/documents
User saves                    → PUT /api/workflows/:id (updates name/description)
```

### 3. User Builds a Pipeline

```
User creates pipeline          → POST /api/pipelines
User adds stages              → POST /api/pipelines/:id/stages (for each stage)
User adds workflows to stages → POST /api/pipelines/:pid/stages/:num/members
```

### 4. User Runs a Pipeline

```
User enters initial input     → types in the input text area
User clicks "Run Pipeline"    → POST /api/pipelines/:id/runs { initial_input }
                              ← returns { run_id }
Frontend navigates to         → /pipelines/:pid/runs/:rid
```

### 5. Frontend Loads the Run Page

```
Fetch initial tree            → GET /api/pipelines/:pid/runs/:rid/tree
Subscribe to updates          → WS: { action: "subscribe_run", run_id }
Render tree from response     → StageList → WorkflowExecution → AgentExecutionNode
```

### 6. Backend Executes the Pipeline

For each stage (sequentially):

```
Backend creates stage_execution rows for each member
Backend starts each workflow's DAG:
  ├─ Find entry steps (no incoming edges)
  ├─ For each entry step:
  │   ├─ Resolve prompt template ({variables} → actual data from prior outputs)
  │   ├─ Append attached document content
  │   ├─ Create agent_execution row (status: running)
  │   ├─ → WS: agent_execution_update (running)
  │   ├─ Call LLM (agent's system_prompt + resolved prompt)
  │   ├─ Parse structured_output against output_schema
  │   ├─ Save to agent_execution (status: completed)
  │   ├─ → WS: agent_execution_update (completed, with structured_output)
  │   ├─ Create token_ledger row
  │   │
  │   ├─ If step has interactive_agent_id:
  │   │   ├─ Create interactive agent_execution (status: running)
  │   │   ├─ Send main output to interactive agent
  │   │   ├─ Save interactive agent's response
  │   │   ├─ Set status: awaiting_user
  │   │   ├─ → WS: agent_execution_update (awaiting_user)
  │   │   ├─ PAUSE — wait for user approval
  │   │   ├─ ... user chats via POST /agent-executions/:id/messages ...
  │   │   ├─ ... user approves via POST /agent-executions/:id/approve ...
  │   │   ├─ Set status: completed
  │   │   ├─ → WS: agent_execution_update (completed)
  │   │   └─ Use COALESCE logic for final output
  │   │
  │   ├─ Check outgoing edges — find child steps
  │   ├─ For each child step:
  │   │   ├─ Check if ALL parents completed
  │   │   └─ If yes, start the child step (recurse)
  │   │
  │   └─ If child step is for_each:
  │       ├─ Read the for_each_ref array from parent output
  │       ├─ Create N agent_execution rows (one per element)
  │       ├─ → WS: for_each_spawned (all N nodes)
  │       └─ Run all N in parallel
  │
  ├─ When all terminal steps complete → stage_execution status: completed
  ├─ → WS: stage_execution_update (completed)
  │
  └─ When all stage_executions for the stage complete → advance to next stage
      ├─ → WS: pipeline_run_update (current_stage incremented)
      └─ If no more stages → pipeline_run status: completed
```

### 7. Frontend Updates in Real Time

```
On agent_execution_update:
  └─ Find node in tree by agent_execution_id
     ├─ Update status icon (○ → ● → ✅)
     ├─ Show passdown text from structured_output
     ├─ Update token/cost display
     └─ If awaiting_user → show chat panel

On for_each_spawned:
  └─ Find parent step node
     └─ Insert N new child nodes (with iteration labels)

On stage_execution_update:
  └─ Update stage header status badge

On pipeline_run_update:
  └─ Update run header (status, current_stage)
     └─ If completed → show completion state, final cost
```
