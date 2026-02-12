# Dynamic Task Nodes: Agent-Designed Execution

## Vision

The canvas is not a collection of predefined step types. Every node starts **blank**. The user drops a node, talks to an agent, and the agent designs what that node becomes. No type picker, no forms, no dials — just conversation.

The agent decides whether this node should scan code, write documents, run migrations, or anything else. It assembles a task force of sub-agents with the right capabilities, creates a mission brief, and the system executes it.

## Canvas Node Types

```
┌──────────────────────────────────────────────────────┐
│  Resource Nodes (static, user-configured)            │
│    [GitHub: org/repo]  [Database: staging]  [S3]     │
│                                                      │
│  Task Nodes (dynamic, assistant-designed)             │
│    [blank] → chat → configured → executed            │
│                                                      │
│  Protocol Nodes (specialized behavior)               │
│    [Belief Capture]  [Room/Meeting]                   │
│    (also configured from blank via assistant)         │
└──────────────────────────────────────────────────────┘
```

### Resource Nodes

Static infrastructure providers. User configures by hand. No AI involved.

- **GitHub**: repo URL, branch, pre-execution actions (create branch, checkout commit)
- **Database**: connection string, schema access level
- **Files/S3**: paths, read/write permissions
- **API**: endpoint, credentials, SDK availability

Resource nodes don't process anything. They provision environment for downstream task nodes. An edge from a resource node means: "make this available in the execution container."

```
[GitHub: org/my-app, branch: main]
  on_execute: create branch "feat/security-fix" from main
```

### Task Nodes

Dynamic work units. Start blank. Configured through conversation with the assistant. The assistant creates a mission brief. At runtime, a planner creates a detailed plan. Agents execute their slices.

```
[blank] → user chats → assistant configures → [Task: "Fix SQL injection"]
```

### Protocol Nodes

Specialized behavior configured from blank nodes via the assistant:

- **Belief Capture**: "I need to capture what we learned" → assistant configures as belief capture
- **Room/Meeting**: "Let's discuss the results" → assistant configures as a room

These are task nodes with specialized execution behavior. The assistant recognizes when the user's intent maps to a protocol and configures accordingly.

---

## Node Lifecycle

```
blank → configuring (chat active) → configured (ready to run) → executed (has results)
```

### Blank State
- Just a dot on the canvas with a chat bubble
- No type, no config, no edges required
- User can connect resource nodes before or after configuring

### Configuring State
- Chat is open, user talking to assistant
- Assistant sees: full workflow graph, connected resources, available capabilities
- Assistant creates mission brief through tool calls
- Node's visual representation updates in real-time as config takes shape

### Configured State
- Mission brief is complete
- Node shows: task name, agent roster, capability summary
- Ready to execute when workflow runs

### Executed State
- Has results/artifacts from the task force
- Outputs available for downstream nodes (belief capture, other tasks, rooms)

---

## The Three Layers

### Layer 1: Assistant (Design Time)

The assistant is a single workflow-level agent. When the user selects any node on the canvas, the assistant has context about that node AND the full graph.

**What the assistant sees:**
- Full workflow graph (all nodes, all edges)
- Selected node's current configuration
- Upstream resource nodes and their capabilities
- Upstream task nodes and their outputs
- Downstream nodes (what will consume this node's output)
- Available agent templates and tool registry

**What the assistant produces — the Mission Brief:**

```json
{
  "task": "Find and fix SQL injection vulnerabilities",
  "resources": {
    "github": { "repo": "org/my-app", "branch": "feat/security-fix" },
    "container": { "image": "rust:latest", "tools": ["cargo", "rustc"] }
  },
  "available_capabilities": [
    "file_read", "file_write", "grep", "shell", "git", "github_api"
  ],
  "agent_roster": [
    {
      "name": "Scanner",
      "role": "Find all SQL injection vulnerabilities",
      "capabilities": ["file_read", "grep"]
    },
    {
      "name": "Analyzer",
      "role": "Assess severity and determine fix approach",
      "capabilities": ["file_read"]
    },
    {
      "name": "Developer",
      "role": "Implement fixes and ensure code quality",
      "capabilities": ["file_read", "file_write", "shell"]
    },
    {
      "name": "Tester",
      "role": "Validate fixes and add test coverage",
      "capabilities": ["shell"]
    },
    {
      "name": "Submitter",
      "role": "Create pull request with all changes",
      "capabilities": ["git", "github_api"]
    }
  ],
  "downstream_context": "Results feed into belief capture and review meeting"
}
```

The assistant creates this through tool calls during the conversation:

```
set_task(description)
set_capabilities(capabilities[])
add_agent(name, role, capabilities[])
remove_agent(name)
update_agent(name, changes)
```

### Layer 2: Planner (Runtime)

One LLM call at the start of execution. The planner reads the mission brief and creates a detailed execution plan.

**What the planner sees:**
- The full mission brief
- The actual repo structure (container is already provisioned)
- The agent roster with capabilities

**What the planner produces:**

```json
{
  "plan": [
    {
      "step": 1,
      "description": "Scan src/db/ for raw SQL query construction",
      "assigned_to": "Scanner"
    },
    {
      "step": 2,
      "description": "Scan src/api/ for user input in SQL statements",
      "assigned_to": "Scanner"
    },
    {
      "step": 3,
      "description": "Check ORM layer for unsafe raw query escapes",
      "assigned_to": "Scanner"
    },
    {
      "step": 4,
      "description": "Assess severity of each finding (critical/high/medium/low)",
      "assigned_to": "Analyzer"
    },
    {
      "step": 5,
      "description": "Determine fix approach per vulnerability",
      "assigned_to": "Analyzer"
    },
    {
      "step": 6,
      "description": "Replace string concatenation with parameterized queries",
      "assigned_to": "Developer"
    },
    {
      "step": 7,
      "description": "Add input validation at API boundaries",
      "assigned_to": "Developer"
    },
    {
      "step": 8,
      "description": "Run existing test suite, verify no regressions",
      "assigned_to": "Tester"
    },
    {
      "step": 9,
      "description": "Add SQL injection test cases for each fixed endpoint",
      "assigned_to": "Tester"
    },
    {
      "step": 10,
      "description": "Commit changes and create PR with findings summary",
      "assigned_to": "Submitter"
    }
  ],
  "execution_order": [
    { "phase": 1, "agents": ["Scanner"], "mode": "sequential" },
    { "phase": 2, "agents": ["Analyzer"], "mode": "sequential" },
    { "phase": 3, "agents": ["Developer"], "mode": "sequential" },
    { "phase": 4, "agents": ["Tester"], "mode": "sequential" },
    { "phase": 5, "agents": ["Submitter"], "mode": "sequential" }
  ]
}
```

**Key**: Every agent receives the FULL plan but only executes their assigned steps. They all understand the big picture.

### Layer 3: Agents (Runtime)

Each agent runs in the provisioned container with:
- The **full plan** (complete context of what everyone is doing)
- Their **assigned slice** (which steps they execute)
- **Previous agents' outputs** (results to build on)
- Their **capabilities** resolved to actual tools

```
Scanner receives:
  System: "You are Scanner. Here is the full plan: [plan].
           Your assigned steps: [1, 2, 3].
           You have: file_read, grep.
           Complete your steps and report findings."

Analyzer receives:
  System: "You are Analyzer. Here is the full plan: [plan].
           Your assigned steps: [4, 5].
           Scanner's findings: [scanner_output].
           You have: file_read.
           Assess each finding and determine fix approaches."
```

---

## Resource Node → Task Node: Capability Propagation

When a resource node connects to a task node, it provisions the execution environment:

```
[GitHub: org/my-app] → [Task Node]

At execution time:
  1. Docker container created
  2. git clone org/my-app
  3. cd into repo
  4. Agents start — they're IN the code
```

Multiple resources compose:

```
[GitHub: org/my-app]     → container gets: repo checkout
[Database: staging]      → container gets: connection string as env var
[S3: artifacts-bucket]   → container gets: write credentials
         ↓
    [Task Node]
    All agents have access to repo + database + S3
```

Resource provisioning happens BEFORE the planner runs. The planner sees the live environment and can plan accordingly (e.g., "I see a Cargo.toml, this is a Rust project, I'll plan cargo-specific commands").

---

## Editing Configured Nodes

The user can always reopen the chat on a configured node. The assistant sees the current configuration and can modify it:

```
User: "Actually, also add a security auditor that runs cargo-audit"
Assistant:
  → Adds "Auditor" agent with shell capability
  → Updates mission brief
  → "Done. Added Auditor agent with shell access.
     It'll run cargo-audit as part of the plan."
```

The assistant can also reconfigure the node type entirely:

```
User: "Actually, this should be a belief capture instead"
Assistant:
  → Clears task force config
  → Configures as belief capture protocol
  → Sets up extraction plan based on upstream context
```

---

## Cross-Node Awareness

When the user modifies the graph (disconnects a node, adds a new one), the assistant can update affected nodes:

```
User disconnects Database resource from Task Node

Assistant (if user asks):
  → "The staging database was disconnected. The Developer agent
     had shell capability for running migrations — want me to
     remove the migration steps from the mission brief?"
```

The assistant has graph-wide context but only modifies when the user engages. It doesn't silently reconfigure nodes — it flags changes and lets the user decide.

---

## Data Model

### Mission Brief (stored on workflow_steps or new table)

```sql
CREATE TABLE task_mission_briefs (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    step_id uuid NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    task_description text NOT NULL,
    available_capabilities text[] NOT NULL DEFAULT '{}',
    downstream_context text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE task_agent_roster (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    mission_brief_id uuid NOT NULL REFERENCES task_mission_briefs(id) ON DELETE CASCADE,
    name text NOT NULL,
    role_description text NOT NULL,
    capabilities text[] NOT NULL DEFAULT '{}',
    execution_order integer NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT now()
);
```

### Execution Plan (created at runtime, stored for observability)

```sql
CREATE TABLE task_execution_plans (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    step_id uuid NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    workflow_execution_id uuid NOT NULL,
    plan_json jsonb NOT NULL,
    planner_model text NOT NULL,
    planner_tokens_in integer NOT NULL DEFAULT 0,
    planner_tokens_out integer NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT now()
);
```

### Agent Execution (reuse existing agent_executions table)

Each agent in the task force creates an `agent_execution` record when it runs. Links to the step via `workflow_step_id` and the workflow via `workflow_execution_id`. This means belief capture can find all agent outputs using existing queries.

---

## Integration with Belief System

The belief capture node's content normalization handles task nodes:

```
"task" → load combined agent outputs from task force
         (each agent's final output/report, concatenated by agent name)
```

Task force agents should produce clear, summarizable output — not just silent code changes. The planner can include this in each agent's instructions: "After completing your steps, write a brief summary of what you found/did."

This summary becomes the artifact that belief capture reads. Each agent's summary becomes a separate content block with the agent name as source label.

---

## Assistant Toolset

The workflow assistant needs tools for all node configurations:

```
// Universal
set_node_name(name)
set_node_description(description)

// Task node
set_task(description)
set_capabilities(capabilities[])
add_agent(name, role, capabilities[])
remove_agent(name)
update_agent(name, changes)

// Belief capture (protocol)
create_extraction_plan(tag_vocabulary, focus_guidance)
update_extraction_plan(changes)

// Room (protocol)
configure_room(members, max_turns, tools_enabled)
add_room_member(agent_id, role)

// Graph operations
connect_to(upstream_step_id)
disconnect_from(step_id)
suggest_downstream(description)
```

---

## Execution Flow (end to end)

```
1. User builds graph:
   [GitHub: org/my-app] → [Task: "Fix SQL injection"] → [Belief Capture] → [Room: Review]

2. User hits "Run"

3. Resource provisioning:
   - Docker container spins up
   - org/my-app cloned, branch created

4. Task node execution:
   a. Planner reads mission brief + live repo → creates detailed plan
   b. Scanner runs: finds vulnerabilities, produces findings report
   c. Analyzer runs: reads Scanner's output + full plan, assesses severity
   d. Developer runs: reads Analyzer's output + full plan, implements fixes
   e. Tester runs: reads Developer's output + full plan, validates
   f. Submitter runs: creates PR

5. Belief capture:
   - Reads each agent's output summary as content blocks
   - Runs gatekeeper per block → extracts beliefs
   - Stores beliefs with agent name as source label

6. Room meeting:
   - Beliefs injected into room agent system prompts
   - Agents discuss the security review findings
   - User joins and asks questions
```

---

## Implementation Considerations

**Container management**: Need a container orchestration layer. Each task node execution gets a container. Container lifecycle: create → provision resources → run agents → capture outputs → destroy.

**Agent isolation**: All agents in a task force share the same container (same repo checkout). This means Agent 3 sees Agent 2's code changes. This is intentional — they're working on the same codebase.

**Failure handling**: If an agent fails its slice, the system can:
- Stop the whole task force (fail-fast)
- Skip and continue with remaining agents
- Retry the failed agent
- Let the planner decide (include failure handling in the plan)

**Replanning**: If an agent discovers something unexpected (the codebase uses a different ORM than expected), can it trigger a replan? This is a future enhancement — v1 would just have the agent adapt within its slice.

---

## Build Path

### v1: Mission Brief + Simple Execution
- Task node with mission brief stored as JSON
- No planner — assistant creates the full plan at design time
- Sequential agent execution in shared container
- Each agent gets previous agent's output
- Basic container provisioning (Docker)

### v2: Runtime Planner
- Planner phase at execution time
- Plan stored for observability
- Full plan shared with all agents
- Planner adapts to actual repo structure

### v3: Workflow Assistant
- Single assistant for all nodes
- Dynamic node configuration through conversation
- Cross-node awareness
- Graph modification tools

### v4: Resource Node Types
- GitHub, Database, S3, API resource nodes
- Automatic capability propagation
- Container provisioning from resource config
