# Goal: Conversational Agent Creation & Wiring

Users type into the chat window to create agents, configure their roles, and wire them up to work together.

Examples of what this looks like:
- "Spin up a worker to handle the frontend tests"
- "Create a reviewer agent and have it watch the worker's output"
- "Add a utility agent for formatting"

## Agent Clusters & Tooling

Agents are freeform — an orchestrator or a worker, each with different tools.
The user defines what an agent can do by selecting tools and configuring triggers.
Groups of agents working together form **clusters**.

Tools come from the existing execution layer:
- File operations (read, write, list)
- Git operations (status, diff, commit, branch)
- Test runner (full suite, single test, coverage)
- Sandboxed command execution
- Agent management (create, list, assign, remove)

New tools are just a schema + an execute handler. Users should be able to
compose agents with whatever tool mix fits their workflow.

## Scheduled / Timed Agents & Pipelines

Agents support scheduling and event-driven triggers. Completion of one agent
can trigger the next, forming pipelines.

Examples:
- "Find bugs and commit fixes, then have a reviewer check them"
- "Run the test suite every hour and report failures"
- "After every ticket completion, add tests and fix bugs"
- A ticket only gets accepted after two agents sign off (worker + reviewer)
- Chain agents: worker → reviewer → merge agent

Trigger types:
- **Cron-like** — run on a schedule (hourly, daily, etc.)
- **Event-driven** — fire on completion events, commit events, ticket transitions
- **Approval gates** — ticket/PR only advances after N agents approve

This means agents need: scheduled execution, completion triggers that
spawn/notify downstream agents, pipeline definitions, and acceptance
criteria (e.g. "two agents must pass before merging").

---

## What's Been Built

### Orchestrator Tool Use Loop (DONE)
- Anthropic native tool use in the LLM layer (types, streaming, content blocks)
- Multi-turn tool use loop in the orchestrator (up to 10 rounds per message)
- Proper `tool_result` content blocks (not plain text) for reliable tool conversations
- `Message.content` supports both plain text and structured content blocks (`MessageContent` enum)

### Agent Pool & Dispatcher (DONE)
- `AgentPool` + `Dispatcher` initialized in `AppState` when API key is available
- Pool manages agents by tier (Orchestrator/Worker/Utility) with configurable limits
- Dispatcher routes commands to agents and collects responses via channels

### Agent Management Tools (DONE — 8 tools)
| Tool | Description |
|------|-------------|
| `list_agents` | Pool stats by tier (total, available, max) |
| `list_roles` | All available roles with descriptions, categories, styles, delegation rules |
| `create_agent` | Spawn agent by tier + optional name |
| `assign_task` | Send task to agent with role-aware context (system prompt, required reading, style) |
| `get_task_result` | Poll for task status: pending → started → in_progress → completed/failed |
| `list_pending_approvals` | Show all agents waiting for approval decisions |
| `respond_to_approval` | Approve or deny an agent's pending action |
| `remove_agent` | Remove agent from pool |

### Role System Integration (DONE)
- `RoleManager` in `AppState` loads role prompts from `src/agents/prompts/*.txt`
- 8 predefined roles: orchestrator, worker, utility, reviewer, summarizer, complaint-finder, risk-assessor, scope-definer
- `assign_task` accepts optional `role` param — loads role-specific system prompt, communication style, output format, and required reading files
- Custom roles supported via `RoleLibrary`

### Agent Result Flow (DONE)
- Background response consumer drains dispatcher responses into `task_results` map
- Results keyed by `task_id` — accessible via `get_task_result` tool
- All agent events broadcast to UI via WebSocket channels:
  - `TaskUpdate` — status changes (in_progress, completed, failed) with progress %
  - `AgentUpdate` — agent status changes (working, idle, waiting_for_approval)
  - `FeedUpdate` — progress messages, approval requests

### Approval Gates (DONE)
- Agents can request approval (`ApprovalRequest` with action + details)
- Approval requests broadcast to UI feed as `approval_request` type
- Orchestrator LLM can list pending approvals and approve/deny via tools
- Denied agents receive reason string

---

## What's Left To Build

### 1. Clusters — Agent Grouping & Shared Context

**Concept:** Named groups of agents that share context and work together on related tasks.

**What's needed:**
- `Cluster` struct: id, name, description, member agent IDs, shared context (files, conventions)
- Tools: `create_cluster`, `add_agent_to_cluster`, `remove_agent_from_cluster`, `list_clusters`
- When assigning a task to a clustered agent, inject the cluster's shared context into `TaskContext`
- Cluster-level status: aggregate progress across member agents
- Persist clusters (currently everything is in-memory — clusters should survive restarts)

**Example flow:**
```
User: "Create a frontend cluster with a worker and a reviewer"
  → create_cluster(name: "frontend")
  → create_agent(tier: "worker", name: "frontend-worker")
  → add_agent_to_cluster(cluster: "frontend", agent_id: "...")
  → create_agent(tier: "worker", name: "frontend-reviewer")
  → add_agent_to_cluster(cluster: "frontend", agent_id: "...")
  → assign_task(agent_id: "frontend-worker", role: "worker", title: "Fix login form")
```

### 2. Execution Tools — Connect Agents to the Execution Layer

**Concept:** Agents can actually do things — read/write files, run tests, execute git commands.

**What's needed:**
- Bridge tools from `src/execution/` into the agent tool system:
  - `read_file` / `write_file` / `list_files` (from `src/execution/file_ops.rs`)
  - `git_status` / `git_diff` / `git_commit` / `git_branch` (from `src/execution/git.rs`)
  - `run_tests` / `run_single_test` (from `src/execution/test_runner.rs`)
  - `run_command` (from `src/execution/sandbox.rs`)
- Per-agent tool allowlists: which tools an agent is allowed to use (sandbox safety)
- Tool results flow back through the same `get_task_result` mechanism

**This is critical** — without execution tools, agents can only generate text. With them, agents can actually modify code, run tests, and commit.

### 3. Scheduled Agents & Triggers

**Concept:** Agents that run on schedules or in response to events.

**What's needed:**
- `Schedule` struct: cron expression or interval, agent_id, task template
- `Trigger` struct: event type (task_completed, commit, timer), action (assign_task to agent)
- Background scheduler task (tokio interval) that checks for due schedules
- Tools: `create_schedule`, `list_schedules`, `remove_schedule`, `create_trigger`, `list_triggers`
- Persist schedules to DB (need a `schedules` table)

**Example flow:**
```
User: "Run the test suite every hour and report failures"
  → create_agent(tier: "utility", name: "test-runner")
  → create_schedule(agent_id: "...", cron: "0 * * * *",
      task: {title: "Hourly test run", description: "Run full test suite, report failures"})
```

### 4. Pipelines — Chained Agent Workflows

**Concept:** Completion of one agent's task automatically triggers the next agent.

**What's needed:**
- `Pipeline` struct: name, stages (ordered list of agent_id + task template)
- Modify response consumer: on `TaskCompleted`, check if agent is part of a pipeline, auto-assign next stage
- Tools: `create_pipeline`, `add_stage`, `start_pipeline`, `get_pipeline_status`
- Approval gates as pipeline stages: stage doesn't advance until N agents approve
- Persist pipelines to DB

**Example flow:**
```
User: "Set up a pipeline: worker writes code, reviewer checks it, then merge if approved"
  → create_pipeline(name: "code-review")
  → add_stage(pipeline: "code-review", agent_id: "worker-1", role: "worker")
  → add_stage(pipeline: "code-review", agent_id: "reviewer-1", role: "reviewer",
      approval_required: true)
  → start_pipeline(pipeline: "code-review",
      task: {title: "Implement login", description: "..."})
```

### 5. Persistence — Survive Restarts

**Concept:** Agent definitions, clusters, schedules, and pipelines persist to DB.

**What's needed:**
- New DB tables: `agents`, `clusters`, `cluster_members`, `schedules`, `triggers`, `pipelines`, `pipeline_stages`
- On startup: reload agents from DB, reconstruct pool, restart schedules
- Migration file for the new tables
- `ServerRepo` trait extensions for CRUD operations

### 6. Agent Seeding — Predefined Agent Configurations

**Concept:** Ship default agent configurations that get seeded into the DB on first run.

**What's needed:**
- Seed definitions in a config file or Rust constants (e.g., `src/agents/seeds.rs`)
- Default agents: a general worker, a code reviewer, a test runner, a summarizer
- Each seed defines: tier, role, name, description, allowed tools, default schedule (if any)
- Seeding runs on first startup (check if DB is empty) or via a CLI command
- Users can modify/delete seeded agents — they're just starting points

---

## Recommended Build Order

1. **Execution tools** — agents can actually do work (highest impact)
2. **Clusters** — group agents, share context
3. **Persistence** — survive restarts
4. **Scheduled agents** — cron + event triggers
5. **Pipelines** — chained workflows
6. **Agent seeding** — ship defaults
