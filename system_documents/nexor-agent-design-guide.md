# nexor Agent Design Guide

A practical guide for designing agents and workflows in the nexor multi-agent orchestration system.

---

## 1. Introduction

This guide provides patterns and best practices for creating agents and workflows in nexor. Use it to:
- Design effective agents for specific tasks
- Structure workflows with proper data flow
- Choose appropriate execution modes and configurations
- Understand how documents, schemas, and variables work together

This is a design guide, not a user manual. Focus is on how to create agents and workflows, not how to use the UI.

---

## 2. Agent Design Fundamentals

### 2.1 Agent Anatomy

Every agent has these core components:

**Configuration:**
- `name`: Agent identifier (e.g., "Code Analyzer", "Feature Extractor")
- `system_prompt`: Role definition, task description, constraints, output format
- `model_provider`: Currently only "anthropic" supported
- `model_id`: Specific model (e.g., "claude-sonnet-4-20250514")
- `model_max_tokens`: Token limit per response (default: 4096)
- `model_temperature`: Sampling randomness, 0.0-1.0 (default: 0.7)

**Optional Features:**
- `output_schema_id`: Enforce structured JSON output
- `router_id`: Enable mode-based routing and tool clusters
- Tool assignments via `agent_tools` table

### 2.2 Model Selection

| Model | Cost | Speed | Use For |
|-------|------|-------|---------|
| **haiku** | $ | Fast | Simple tasks, filtering, routing, quick classifications |
| **sonnet** | $$ | Medium | Analysis, coding, general work, most common choice |
| **opus** | $$$ | Slow | Complex reasoning, critical decisions, high-stakes tasks |

**Cost formula**: `(input_tokens / 1M × input_price) + (output_tokens / 1M × output_price)`

### 2.3 Temperature Guide

| Range | Behavior | Use Case |
|-------|----------|----------|
| **0.0-0.3** | Deterministic, focused | Code generation, structured extraction, factual analysis |
| **0.5-0.7** | Balanced | General tasks, most workflows |
| **0.8-1.0** | Creative, varied | Brainstorming, content generation, creative writing |

### 2.4 System Prompt Design

**Effective Pattern:**
```
[ROLE] You are a {specific role} that {primary function}.

[TASK] Your task is to {concrete task description}.

[CONSTRAINTS]
- {constraint 1}
- {constraint 2}
- {constraint 3}

[OUTPUT FORMAT]
{description of expected output structure}
```

**Automatic Additions:**
- Output schema instructions appended if `output_schema_id` is set
- Agent-level documents concatenated with markdown headers
- Step-level documents concatenated (in workflows)
- Room context injected automatically for room-based execution

### 2.5 Tool Assignment Strategy

**Tool Execution:**
- Agents with tools use React-style loops (max 15 rounds)
- Each round: LLM responds → tools execute → results feed back → LLM continues
- No tools = single LLM call (faster, cheaper)

**Best Practices:**
- Minimal tool set per agent (focused capability)
- Tools should match agent's role
- Don't assign all tools to every agent

**Tool Categories:**
- **File operations**: read_file, write_file, edit_file, list_files
- **Git operations**: git_status, git_diff, git_commit, git_branch
- **Execution**: run_tests, run_command
- **Research**: web_search, x_search
- **Meta**: request_assistance (router mode only)

### 2.6 Common Agent Patterns

| Pattern | Temperature | Tools | Output Schema | Use Case |
|---------|------------|-------|---------------|----------|
| **Analyzer** | 0.5-0.7 | None | Yes | Structured analysis of inputs |
| **Executor** | 0.0-0.3 | File/Git | No | Code changes, file operations |
| **Reviewer** | 0.5-0.7 | None | Yes | Approval decisions, quality checks |
| **Router** | 0.3-0.5 | request_assistance | No | Classify and delegate tasks |
| **Researcher** | 0.5-0.7 | Research | Yes | Information gathering, web search |

---

## 3. Workflow Design

### 3.1 Workflow Structure

Workflows are **Directed Acyclic Graphs (DAGs)** of steps:
- **Nodes**: Workflow steps (agent executions)
- **Edges**: Dependencies (from_step → to_step)
- **Execution**: Topological sort (Kahn's algorithm)

**Entry Steps:**
- Steps with no incoming edges
- Execute first, ordered by `display_order`
- Must have explicit ordering for reproducibility

**Dependency Rules:**
- No cycles allowed (validated at execution time)
- Child steps wait for all parents to complete
- Independent steps can run concurrently (parallel mode)

### 3.2 Step Configuration

Each workflow step defines:

```
agent_id                 → Which agent executes
execution_mode           → "single", "for_each", or "room"
prompt_template          → User message with {variable} placeholders
output_variable_name     → Key for storing structured output
output_schema_id         → Enforce JSON Schema structure (optional)
interactive_agent_id     → Review agent for approval (optional)
display_order            → Execution order for entry steps
```

### 3.3 Execution Modes

#### Single Mode (Default)

**Use for:** Sequential processing, decision points, transformations

**Configuration:**
```
execution_mode: "single"
prompt_template: "Analyze this code: {previous_step.code}"
output_variable_name: "analysis"
```

**Behavior:** One agent, one execution, single output

---

#### For-Each Mode (Parallel Iteration)

**Use for:** Batch processing, parallel analysis, multi-target execution

**Configuration:**
```
execution_mode: "for_each"
for_each_ref: "features.items"              # Array to iterate
for_each_label_field: "name"                # Extract label from element
prompt_template: "Extract info from {features.items.$.name}"
output_variable_name: "extracted_items"
```

**Special Syntax:**
- `{features.items}` → resolves to array
- `{features.items.$.name}` → accesses current element's `.name` field
- `$` only valid in for_each mode

**Output:** Array of structured outputs, one per iteration

**Cost Warning:** Iterates ALL elements (LLM cost multiplier). Filter/sample large arrays first.

---

#### Room Mode (Multi-Agent Discussion)

**Use for:** Consensus building, multi-perspective analysis, debate

**Configuration:**
```
execution_mode: "room"
room_id: {uuid}
```

**Behavior:**
- Gatekeeper LLM determines speaker order based on roles and context
- Each speaker gets: room context, transcript so far, user message
- Transcript accumulates across turns
- Sequential speakers (not parallel)

---

### 3.4 Dependency Management

**Edges define execution order:**
- `from_step_id` → `to_step_id`
- Parent must complete before child starts
- All parents must complete before child can begin

**Parallel Potential:**
- Independent steps (no path between them) can run concurrently
- Depends on workflow `execution_mode` setting

**Cycle Detection:**
- Errors on circular dependencies
- Validated during topological sort at execution time

---

## 4. Data Flow Patterns

### 4.1 Variable Resolution

**Syntax:** `{variable_name}` or `{variable_name.nested.path.0.field}`

**Scope:**
- Completed step outputs (current workflow)
- Prior stage outputs (cross-workflow in collections)

**Dot-Path Navigation:**
- Object fields: `{analysis.summary}`
- Array indices: `{features.0.name}`
- Deep nesting: `{data.items.2.metadata.title}`

**Missing Variables:**
- Unresolved variables left as `{variable_name}` (no error)
- Design prompts to handle missing data gracefully

**Examples:**

```
{analysis}                         → Step output "analysis"
{analysis.summary}                 → Nested object field
{features.items}                   → Array (for for_each)
{features.items.0.name}            → First array element's name
{features.items.$.name}            → Current element (for_each only)
{$workflow_research.findings}      → Cross-workflow (collections)
```

### 4.2 Output Schema Usage

**Definition:**
- Stored in `output_schemas` table as JSON Schema (JSONB)
- Assign to agent or step via `output_schema_id`

**How It Works:**
1. Schema injected into system prompt as instructions
2. LLM instructed to respond with valid JSON
3. Response parsed from JSON or markdown code blocks
4. NO runtime validation against schema (instruction-based only)

**Best Practices:**
- Design clear, explicit schemas with descriptions
- Keep structures flat (easier to parse from markdown)
- Always set `output_variable_name` when using schemas
- Test that LLM consistently produces valid output

**Example Schema:**
```json
{
  "type": "object",
  "properties": {
    "summary": {"type": "string", "description": "Brief summary"},
    "confidence": {"type": "number", "minimum": 0, "maximum": 1},
    "findings": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "title": {"type": "string"},
          "description": {"type": "string"}
        },
        "required": ["title", "description"]
      }
    }
  },
  "required": ["summary", "confidence", "findings"]
}
```

### 4.3 For-Each Data Flow

**Pattern:**

```
Step 1: Extract Features
  output_variable_name: "features"
  structured_output: {"items": [{...}, {...}, {...}]}

Step 2: Analyze Each Feature (for_each)
  for_each_ref: "features.items"
  for_each_label_field: "name"
  prompt_template: "Analyze {features.items.$.name}: {features.items.$.description}"
  output_variable_name: "analyses"

  → Spawns N parallel executions
  → Each gets current element via {$.field} syntax
  → Output: array of N structured results

Step 3: Synthesize
  prompt_template: "Synthesize these analyses: {analyses}"
  → Receives aggregated array
```

**Efficiency Tips:**
- Filter large arrays before for_each (cost multiplier)
- Use `for_each_label_field` for tracking iterations in logs
- Consider sequential workflow mode to control concurrency

### 4.4 Cross-Workflow Variables (Collections)

**Syntax:** `{$workflow_name.output_variable_name}`

**How It Works:**
- Collections execute workflows as DAG nodes
- Each workflow produces aggregated step outputs
- Downstream workflows reference completed workflow outputs

**Example:**

```
Collection: "Feature Analysis Pipeline"

Workflow A: "Research"
  → outputs: {"findings": [...], "summary": "..."}

Workflow B: "Analysis" (depends on A)
  → prompt: "Analyze these findings: {$workflow_research.findings}"

Workflow C: "Report" (depends on B)
  → prompt: "Generate report from {$workflow_analysis.report}"
```

---

## 5. Document System

### 5.1 Document Types

**Agent Context (Global):**
- Linked via `agent_context` table
- Applied to ALL executions of that agent
- Use for: agent role definitions, standards, examples

**Step Context (Workflow-Specific):**
- Linked via `step_documents` table
- Applied only to specific workflow step
- Use for: task-specific guidance, specialized instructions

### 5.2 Concatenation Format

Documents appended to prompts with markdown formatting:

```
[Agent's system_prompt]

---
## Document Title (Agent Context)
Document content here...

---
## Another Document (Step Context)
More context here...

[Then: output schema instructions if present]

[Then: user message / prompt_template]
```

### 5.3 When to Use Documents

**Good Uses:**
- Large static context (architecture, standards, examples)
- Shared knowledge across multiple agents
- Step-specific guidance for particular tasks
- Reference implementations, code templates

**Considerations:**
- Documents concatenate with prompts (token cost)
- Large documents can exceed context windows
- Consider splitting large docs or using smaller excerpts

### 5.4 Document Design Patterns

| Pattern | Description | Example |
|---------|-------------|---------|
| **Architecture** | System design, component relationships | "System uses 3-tier architecture: API/Service/DB" |
| **Standards** | Coding conventions, patterns | "Use async/await, prefer composition over inheritance" |
| **Examples** | Reference code, templates | "Example API endpoint structure: [code]" |
| **Context** | Domain knowledge, project info | "This system manages medical records with HIPAA compliance" |

---

## 6. Workflow Collections

### 6.1 Collection Architecture

**Collections are meta-DAGs:**
- Workflows as nodes (not steps)
- `collection_workflow_edges` define dependencies
- Each workflow produces aggregated outputs
- Cross-workflow variable resolution via `{$workflow_name.variable}`

**Execution Modes:**
- `sequential`: One workflow at a time (ordered)
- `parallel`: Concurrent workflows (respects dependencies)

### 6.2 Collection Patterns

**Pipeline (Sequential Stages):**
```
Research → Analysis → Implementation → Review
```
Each stage completes before next begins.

**Fan-Out/Fan-In:**
```
        → Analyze A →
Input   → Analyze B → Synthesize → Report
        → Analyze C →
```
Parallel analysis, then aggregation.

**Staged Processing:**
```
Stage 1: Data Collection (parallel workflows)
Stage 2: Processing (depends on Stage 1)
Stage 3: Reporting (depends on Stage 2)
```

### 6.3 Collection vs Single Workflow

**Use Collection When:**
- Independent workflows with distinct concerns
- Reusable workflow components
- Different teams own different workflows
- Clear stage boundaries

**Use Single Workflow When:**
- Tight coupling between steps
- Shared context across all steps
- Simple linear or branching flow
- Steps are not independently useful

---

## 7. Interactive Agents

### 7.1 Interactive Review Pattern

**How It Works:**
1. Main agent executes step → produces output
2. Interactive agent (via `interactive_agent_id`) reviews output
3. Status: `awaiting_user` → workflow pauses
4. User chats with review agent (Q&A)
5. User approves → workflow resumes
6. User rejects → workflow fails

**Configuration:**
```
Step: "Generate Code"
  agent_id: {code_generator_id}
  interactive_agent_id: {reviewer_id}
  output_variable_name: "generated_code"
```

### 7.2 Review Agent Design

**Effective Reviewer System Prompt:**
```
You are a code reviewer. Evaluate the generated code for:
- Correctness and functionality
- Code quality and style adherence
- Security vulnerabilities
- Test coverage

Provide clear feedback on any issues. If code meets all criteria,
recommend approval. If not, explain what needs to be fixed.
```

**Review Schema Example:**
```json
{
  "type": "object",
  "properties": {
    "approved": {"type": "boolean"},
    "issues": {
      "type": "array",
      "items": {"type": "string"}
    },
    "recommendations": {"type": "string"}
  },
  "required": ["approved", "issues"]
}
```

### 7.3 Use Cases

- Quality gates before deployment
- Human oversight for critical decisions
- Approval workflows for policy compliance
- Iterative refinement with human feedback

---

## 8. Design Best Practices

### 8.1 Schema Design

**Instruction-Based Enforcement:**
- Schemas injected into prompts as instructions
- NO runtime validation (LLM compliance only)
- Design for clarity and explicit structure

**Best Practices:**
- Include `description` fields for all properties
- Prefer flat structures over deep nesting
- Use `required` to enforce critical fields
- Test with actual LLM executions
- Handle malformed outputs gracefully in downstream steps

### 8.2 Variable Resolution

**Best Practices:**
- Use simple, tested variable paths
- Provide fallback text in prompts for missing variables
- Test variable resolution before complex workflows
- Document expected output structure in step descriptions

**Example with Fallback:**
```
Analyze this: {analysis.summary}

If no analysis is available, start fresh based on: {original_input}
```

### 8.3 For-Each Efficiency

**Cost Management:**
- For-each iterates ALL elements (LLM cost multiplier)
- Filter/sample large datasets before iteration
- Consider: Does every element need LLM analysis?
- Alternative: Batch elements, process groups

**Best Practices:**
- Use `for_each_label_field` for tracking (debugging)
- Set reasonable array size limits upstream
- Monitor costs in production workflows

### 8.4 Tool Assignment

**Best Practices:**
- Minimal tool set per agent (focused capability)
- Tools should match agent's role and task
- Don't assign all tools by default
- Test tool execution with realistic scenarios

**Pattern:**
- Analyzer agents: No tools (single LLM call)
- Executor agents: File/Git tools only
- Researcher agents: Research tools only
- Router agents: request_assistance only

### 8.5 Workflow Complexity

**Best Practices:**
- Break large DAGs into collections
- Limit to ~10 steps per workflow for maintainability
- Use `display_order` for entry steps (reproducibility)
- Document workflow purpose and data flow

**When to Split:**
- More than 10-15 steps
- Multiple distinct concerns
- Reusable sub-workflows
- Hard to visualize or debug

---

## 9. Design Checklist

### Agent Design
- [ ] System prompt defines role, task, constraints clearly
- [ ] Model choice matches task (haiku/sonnet/opus)
- [ ] Temperature appropriate for determinism needs
- [ ] Tools limited to essential capabilities
- [ ] Output schema (if any) is clear and flat with descriptions

### Workflow Design
- [ ] No cycles in step edges (will be validated at execution)
- [ ] Entry steps have `display_order` set
- [ ] Variable names consistent across steps
- [ ] For-each steps have `for_each_ref` and `for_each_label_field`
- [ ] Output schemas assigned with `output_variable_name`
- [ ] Interactive agents (if any) have clear approval criteria

### Data Flow
- [ ] Variable paths tested and validated
- [ ] Prompts handle missing variables gracefully
- [ ] Output schemas match downstream input needs
- [ ] For-each arrays are reasonably sized

### Context Management
- [ ] Documents focused and relevant
- [ ] Agent vs step context clearly separated
- [ ] Token budget considered (docs + prompts + outputs)
- [ ] Large documents split or excerpted

---

## 10. Quick Reference

### Variable Resolution Syntax

```
{analysis}                         → Step output "analysis"
{analysis.summary}                 → Nested object field
{features.items}                   → Array for for_each
{features.items.0.name}            → Array element by index
{features.items.$.name}            → Current element (for_each only)
{$workflow_research.findings}      → Cross-workflow (collections)
```

### Execution Mode Matrix

| Mode | Use Case | Output | Concurrency |
|------|----------|--------|-------------|
| **single** | Default step execution | Single result | None |
| **for_each** | Batch processing | Array of results | Per-element parallel |
| **room** | Multi-agent discussion | Transcript | Sequential speakers |

### Model Selection

| Model | Cost | Speed | Best For |
|-------|------|-------|----------|
| **claude-haiku** | $ | Fast | Simple tasks, filtering, routing |
| **claude-sonnet** | $$ | Medium | Analysis, coding, general work |
| **claude-opus** | $$$ | Slow | Complex reasoning, critical decisions |

### Tool Categories

| Category | Tools | Use Case |
|----------|-------|----------|
| **File** | read_file, write_file, edit_file, list_files | Code manipulation, file operations |
| **Git** | git_status, git_diff, git_commit, git_branch | Version control operations |
| **Execution** | run_tests, run_command | Testing, build operations |
| **Research** | web_search, x_search | Information gathering |
| **Meta** | request_assistance | Router mode delegation |

---

## Common Design Questions

**Q: How do I design a workflow that processes files in parallel?**
A: Use a for_each step with `for_each_ref` pointing to an array of file paths. Set `for_each_label_field` to track each file.

**Q: Why isn't my output schema working?**
A: Schemas are instruction-based only (no runtime validation). Check that your agent's system prompt is clear and the LLM is producing valid JSON. Parse errors are handled gracefully.

**Q: Should I use a collection or single workflow?**
A: Use collections when workflows have distinct concerns and can be reused independently. Use single workflow for tightly coupled steps with shared context.

**Q: How do I handle missing variables?**
A: Variables resolve to `{variable}` string if missing (no error). Design prompts with fallback text or instructions for missing data.

**Q: When should agents have no tools?**
A: Analyzer and reviewer agents typically don't need tools (faster, cheaper, single LLM call). Use tools only when agents need to execute actions (files, git, research).

---

## Summary

The nexor agent system provides a powerful multi-agent orchestration platform with:

- **Flexible agent configuration**: Model, temperature, tools, schemas, routing
- **DAG-based workflows**: Topological execution, parallel support, interactive review
- **Rich data flow**: Variable resolution, structured schemas, for-each iteration
- **Document system**: Agent and step-level context injection
- **Workflow collections**: Meta-DAG orchestration, cross-workflow variables

Design agents and workflows with clarity, purpose, and cost-awareness. Test with realistic scenarios. Iterate based on execution results.
