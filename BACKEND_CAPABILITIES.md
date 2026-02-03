# Backend Capabilities & Implementation Status

## Overview

This document describes the current state of the nexor backend, what it can do, what's missing, and the plan to reach 100% completeness.

## Current Architecture (What Works ✅)

### 1. Pipeline → Workflow → Agent Hierarchy

```
Pipeline
  └─> Pipeline Stages (sequential)
      └─> Pipeline Stage Members (parallel workflows)
          └─> Workflows (DAG of steps)
              └─> Workflow Steps (agents with execution modes)
```

**Example:**
```
Pipeline: "Feature Development"

  Stage 1: "Analysis"
    ├─ Workflow A: Requirements Analysis (3 agents in DAG)
    │   ├─ Step 1: planner-agent
    │   ├─ Step 2: research-agent
    │   └─ Step 3: synthesizer-agent
    │
    └─ Workflow B: Validation (1 agent)
        └─ Step 1: validator-agent

  Stage 2: "Design"
    └─ Workflow C: Feature Design (for_each)
        └─ Step 1: designer-agent (executes N times)
```

### 2. Workflow Step Execution Modes

**Mode 1: "single"** (Default)
- One agent, one execution
- Standard workflow step

**Mode 2: "for_each"**
- One agent, multiple executions
- Iterates over an array from previous step output
- Example: Process each file, each requirement, each feature
- Syntax: `for_each_ref: "files"` → iterates over `{files}` array
- Each iteration gets one element via `{files.$}` syntax

**Mode 3: "room"**
- Multiple agents in collaborative conversation
- Turn-based multi-agent discussion
- Gatekeeper can control who speaks
- Example: Design review panel with 3 agents discussing

### 3. Document Context System

**Two levels of document attachment:**

**Level 1: Agent Context** (Global to agent)
```
Agent: "code-reviewer"
└─> agent_context (via PUT /agents/:id/context)
    ├─ style-guide.md
    └─> best-practices.md

These docs should appear in EVERY execution of this agent:
- Every workflow step it's in
- Every for_each iteration
- Every room conversation
```

**Level 2: Step Context** (Specific to workflow step)
```
Workflow Step: "Review code quality"
└─> step_documents (via workflow step attachment)
    ├─ project-requirements.md
    └─> code-standards.md

These docs only appear when THIS specific step executes
```

**Final Prompt Structure:**
```
┌─────────────────────────────────┐
│ Agent System Prompt             │ (agent.system_prompt)
├─────────────────────────────────┤
│ Agent Context Documents         │ ← From agent_context (MISSING ❌)
│   ---                           │
│   ## Style Guide                │
│   {content}                     │
├─────────────────────────────────┤
│ Step Context Documents          │ ← From step_documents (WORKS ✅)
│   ---                           │
│   ## Project Requirements       │
│   {content}                     │
├─────────────────────────────────┤
│ Rendered Prompt with Variables  │ (step.prompt_template)
│   Review this code: {code}      │
└─────────────────────────────────┘
```

### 4. Variable Resolution & Output Mapping

**Workflow steps can reference outputs from:**
- Current workflow (completed steps): `{step_name.field}`
- Prior pipeline stages: `{prior_stage.output.field}`
- Array elements in for_each: `{files.$.name}`

**Dot-path access:**
```
{features.content.0.name}
{requirements[2].description}
{output.list.$.title}  ← In for_each mode
```

**Implementation:** `resolve_variables()` at `dag_executor.rs:128-192`

### 5. Parallel Execution

**Stage-level parallelism:**
- Multiple workflows in a pipeline stage run in parallel
- Implementation: `execute_stage_via_members()` uses `tokio::spawn`

**Step-level parallelism:**
- Workflow steps with no dependencies run in parallel
- DAG edges control execution order
- Implementation: Topological sort + dependency tracking

### 6. Tool Management

**Agent Tools:**
```
Agent → agent_tools (join table) → Tools

Agent can use assigned tools during execution
```

**Tool Routers:**
```
Intelligent tool selection via LLM routing
User request → Tool Router (LLM) → Selects appropriate tool
```

### 7. Structured Outputs

**Output Schemas:**
```
Workflow Step
  └─> output_schema_id → Output Schema (JSON Schema)
      └─> Validated structured output
          └─> Stored in results table
```

**Results Storage:**
```
agent_executions
  └─> results
      - name (variable name)
      - data (JSONB)
      - output_schema_id
```

## What's Missing (Backend Gaps ❌)

### 1. Agent Context Not Loaded ❌

**Problem:**
- API endpoints exist: `GET/PUT /agents/:id/context`
- Database table exists: `agent_context` join table
- Documents get stored successfully
- **BUT:** `compose_prompt()` doesn't load them during execution

**Impact:**
- Agent context documents are stored but never used
- Only step documents get loaded
- Agents can't have global knowledge base

**Location:** `dag_executor.rs:229-280` (`compose_prompt()`)

**Current code:**
```rust
// Append attached documents
if let Some(wf_repo) = workflow_repo {
    if let Ok(step_docs) = wf_repo.list_step_documents(step.id).await {
        // Loads step documents ✅
    }
}

// Agent context loading is MISSING ❌
```

### 2. Agent Context in Room Execution ❌

**Location:** `room_executor.rs`

Room-based workflow steps don't load agent context for participating agents.

### 3. Agent Context in Chat Execution ❌

**Location:** `chat_consumer.rs`

Direct chat with orchestrator doesn't load agent context.

## Implementation Plan

### Phase 1: Core Agent Context Loading

**File:** `src/server/dag_executor.rs`

**Changes to `compose_prompt()`:**

```rust
pub async fn compose_prompt(
    step: &WorkflowStepRow,
    prompt_template_repo: Option<&dyn PromptTemplateRepo>,
    doc_repo: Option<&dyn DocumentRepo>,
    workflow_repo: Option<&dyn WorkflowRepo>,
    server_repo: &dyn ServerRepo,  // ← ADD THIS
    outputs: &HashMap<String, JsonValue>,
    prior_outputs: &HashMap<String, JsonValue>,
    for_each_element: Option<&JsonValue>,
) -> String {
    // 1. Resolve template variables
    let prompt = resolve_variables(&raw_prompt, outputs, prior_outputs);

    let mut full_prompt = prompt;

    // 2. NEW: Append agent context documents (global to agent)
    if let Some(d_repo) = doc_repo {
        if let Ok(agent_docs) = server_repo.get_agent_context(step.agent_id).await {
            for doc in &agent_docs {
                full_prompt.push_str(&format!(
                    "\n\n---\n## {} (Agent Context)\n{}",
                    doc.title,
                    doc.content
                ));
            }
        }
    }

    // 3. EXISTING: Append step documents (specific to this step)
    if let Some(wf_repo) = workflow_repo {
        if let Ok(step_docs) = wf_repo.list_step_documents(step.id).await {
            if let Some(d_repo) = doc_repo {
                for sd in &step_docs {
                    if let Ok(Some(doc)) = d_repo.get_document(sd.document_id).await {
                        full_prompt.push_str(&format!(
                            "\n\n---\n## {} (Step Context)\n{}",
                            doc.title,
                            doc.content
                        ));
                    }
                }
            }
        }
    }

    full_prompt
}
```

**Update all callers:**
- `execute_single_step()` - line 179
- `execute_for_each_step()` - line 243
- `dag_executor.rs:execute_workflow()` - lines 688, 744
- `hub/dag.rs` - similar locations

### Phase 2: Room Execution

**File:** `src/server/room_executor.rs`

Add agent context loading when building prompts for room agents.

### Phase 3: Chat Execution

**File:** `src/server/chat_consumer.rs`

Add agent context loading for chat orchestrator (if applicable).

### Phase 4: Tests

**Files to test:**
- `src/server/api/agent_context/tests.rs` - Verify documents are loaded
- Integration tests for workflow execution with agent context
- For_each execution with agent context
- Room execution with agent context

## Database Schema Reference

### Agent Context
```sql
CREATE TABLE agent_context (
    agent_id UUID REFERENCES agents(id) ON DELETE CASCADE,
    document_id UUID REFERENCES documents(id) ON DELETE CASCADE,
    PRIMARY KEY (agent_id, document_id)
);
```

### Step Documents
```sql
CREATE TABLE step_documents (
    step_id UUID REFERENCES workflow_steps(id) ON DELETE CASCADE,
    document_id UUID REFERENCES documents(id) ON DELETE CASCADE,
    PRIMARY KEY (step_id, document_id)
);
```

### Pipeline Stage Members
```sql
CREATE TABLE pipeline_stage_members (
    id UUID PRIMARY KEY,
    pipeline_id UUID,
    stage_number INTEGER,
    workflow_id UUID REFERENCES workflows(id),
    display_order INTEGER,
    FOREIGN KEY (pipeline_id, stage_number)
        REFERENCES pipeline_stages(pipeline_id, stage_number)
        ON DELETE CASCADE
);
```

## API Endpoints (All Implemented ✅)

### Pipeline Management
- `GET /api/pipelines` - List pipelines
- `POST /api/pipelines` - Create pipeline
- `GET /api/pipelines/:id` - Get pipeline
- `POST /api/pipelines/:id/runs` - Start pipeline run

### Workflow Management
- `GET /api/workflows` - List workflows
- `POST /api/workflows` - Create workflow
- `GET /api/workflows/:id/steps` - List workflow steps
- `POST /api/workflows/:id/steps` - Create workflow step
- `POST /api/workflows/:id/edges` - Add step dependency

### Agent Context (API works, execution doesn't load)
- `GET /api/agents/:id/context` - Get agent's documents
- `PUT /api/agents/:id/context` - Set agent's documents

### Step Documents
- `POST /api/workflows/:wid/steps/:sid/documents` - Attach document to step

### Documents
- `GET /api/documents` - List documents
- `POST /api/documents` - Create document
- `GET /api/documents/:id` - Get document

## Example User Flow (When Backend is 100%)

### 1. Create Documents
```bash
# Upload PRD
POST /api/documents
{
  "title": "Product Requirements",
  "content": "...",
  "doc_type": "prd"
}
→ Returns {id: "prd-123"}

# Upload style guide
POST /api/documents
{
  "title": "Code Style Guide",
  "content": "...",
  "doc_type": "reference"
}
→ Returns {id: "style-456"}
```

### 2. Create Agents
```bash
POST /api/agents
{
  "name": "Code Reviewer",
  "system_prompt": "You are an expert code reviewer...",
  "model_id": "claude-sonnet-4-5"
}
→ Returns {id: "agent-reviewer"}

# Attach global context to agent
PUT /api/agents/agent-reviewer/context
{
  "document_ids": ["style-456"]
}
```

### 3. Create Workflow
```bash
POST /api/workflows
{
  "name": "Code Review Workflow",
  "description": "Multi-agent code review"
}
→ Returns {id: "wf-review"}

# Add steps
POST /api/workflows/wf-review/steps
{
  "agent_id": "agent-reviewer",
  "prompt_template": "Review this code: {code}",
  "output_variable_name": "review_result"
}
→ Returns {id: "step-1"}

# Attach step-specific document
POST /api/workflows/wf-review/steps/step-1/documents
{
  "document_id": "prd-123"
}
```

### 4. Create Pipeline
```bash
POST /api/pipelines
{
  "name": "Feature Development",
  "stages": [
    {
      "stage_number": 1,
      "workflows": ["wf-review"]
    }
  ]
}
→ Returns {id: "pipeline-1"}
```

### 5. Run Pipeline
```bash
POST /api/pipelines/pipeline-1/runs
{
  "initial_input": "Review the authentication module"
}
→ Pipeline executes with:
  - Agent gets style-guide.md (agent context) ✅
  - Agent gets prd-123 (step context) ✅
  - Agent processes the review
```

## Completion Checklist

- [ ] Implement agent context loading in `compose_prompt()`
- [ ] Update all callers of `compose_prompt()` to pass `server_repo`
- [ ] Add agent context loading to room execution
- [ ] Add agent context loading to chat execution
- [ ] Write tests for agent context loading
- [ ] Test for_each execution with agent context
- [ ] Test room execution with agent context
- [ ] Verify output mapping works across stages
- [ ] Document the complete flow

## Once Backend is 100%

The system will support:
1. ✅ Start pipeline from document (PRD)
2. ✅ Multiple workflows per pipeline stage (parallel)
3. ✅ Multiple agents per workflow (DAG)
4. ✅ Documents attached to agents (global context)
5. ✅ Documents attached to steps (step context)
6. ✅ Output mapping between stages via variables
7. ✅ For_each execution for array processing
8. ✅ Room-based multi-agent collaboration
9. ✅ Structured outputs with JSON schemas
10. ✅ Tool management and routing

**Then it's just a matter of building a UI to expose all this functionality.**
