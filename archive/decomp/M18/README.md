# Milestone 18: Typed Subagent System

> Claude Code-style ephemeral typed subagents that the orchestrator spawns on-demand for specialized subtasks, with fan-out/fan-in parallel coordination and manual spawn API.

## Goal

Orchestrator receives a complex task, spawns 3 typed subagents (Explorer + Coder + Tester) in parallel, collects results, synthesizes final output. User can also `/spawn explorer "find auth files"` from chat.

**Checkpoint**: `POST /api/agents/spawn` with `explorer` type creates ephemeral agent, runs task, auto-cleans up. Complex task via chat triggers orchestrator to spawn multiple subagents in parallel, collect results, produce synthesized output.

---

## Scope

- 9 tickets, ~41 slices
- New files: `src/agents/agent_type.rs`, `src/agents/spawner.rs`, `src/agents/coordinator.rs`, `src/agents/toolkit.rs`
- New files: `src/execution/search.rs`, `src/execution/edit.rs`
- Modified files: `src/agents/agent.rs`, `pool.rs`, `executor.rs`, `dispatcher.rs`, `channels.rs`, `mod.rs`
- Modified files: `src/server/api.rs`, `ws.rs`, `state.rs`, `mod.rs`
- Modified files: `src/types/config.rs`, `src/llm/cost.rs`, `src/execution/mod.rs`

## Key Concepts

| Concept | Description |
|---------|-------------|
| `AgentType` | Specialization unit — binds a role to tool permissions, tier, and timeout |
| `AgentTypeRegistry` | In-memory registry of available agent types (5 built-in) |
| Ephemeral Agent | Agent that auto-removes after its task completes or fails |
| `SpawnRequest` | Request to spawn a typed subagent with a task assignment |
| `SubagentCoordinator` | Fan-out/fan-in: spawns N subagents, collects results, synthesizes |
| `TaskAnalyzer` | LLM-based analysis that recommends which subagent types to spawn |
| `AgentToolkit` | Permission-gated tool dispatch — connects agents to the execution layer |

## Built-in Agent Types

| Type | Tier | Permissions | Max Concurrent |
|------|------|-------------|---------------|
| `explorer` | Utility | FileRead, LlmCall | 4 |
| `coder` | Worker | FileRead, FileWrite, LlmCall | 3 |
| `tester` | Worker | FileRead, ShellExec, LlmCall | 2 |
| `reviewer` | Worker | FileRead, LlmCall | 2 |
| `planner` | Orchestrator | LlmCall | 1 |

## Dependency Graph

```
18.1 (Registry)
  ├→ 18.2 (Ephemeral Lifecycle)  ─┐
  └→ 18.3 (Spawn Protocol)       ─┤
                                   ├→ 18.4 (Orchestrator Spawning)
                                   │    └→ 18.5 (Parallel Coordination)
  18.1 ──→ 18.6 (Manual API)      │
  18.2 ──→ 18.7 (Observability) ──┘
  18.8 (Search & Edit) ─┐
  18.1 ────────────────  ├→ 18.9 (Toolkit & Tool Loop)
                        ─┘
```

## Tickets

| Ticket | Title | Slices | Dependencies |
|--------|-------|--------|-------------|
| 18.1 | AgentType Registry | 5 | None |
| 18.2 | Ephemeral Agent Lifecycle | 6 | 18.1 |
| 18.3 | Spawn Request Protocol | 4 | 18.1 |
| 18.4 | Orchestrator Subagent Spawning | 5 | 18.2, 18.3 |
| 18.5 | Parallel Subagent Coordination | 5 | 18.4 |
| 18.6 | Manual Spawn API | 5 | 18.1 |
| 18.7 | Observability | 4 | 18.2, 18.5 |
| 18.8 | Search & Edit Operations | 3 | None |
| 18.9 | Agent Toolkit & Tool Execution Loop | 5 | 18.1, 18.8 |

## Key Design Decisions

1. **Ephemeral by default** — Subagents are short-lived. They exit after task completion and are removed from the pool and dispatcher automatically.
2. **Separate from tier limits** — Ephemeral agents have their own `max_ephemeral` limit and don't count against `max_workers`/`max_utilities`.
3. **Registry is in-memory** — `AgentTypeRegistry` lives in `AppState`. No DB table. Custom types can be registered at startup.
4. **LLM-based task analysis** — The orchestrator uses an LLM call with structured JSON output to decide which subagent types to spawn. Simple tasks bypass analysis.
5. **Fan-out/fan-in** — `SubagentCoordinator` spawns all subagents concurrently, collects results with timeout, handles partial failures gracefully.
6. **Tool permissions are enforced** — `AgentToolkit` (18.9) checks `ToolPermission` before dispatching any tool call. Agents only see tools they're permitted to use.

## Verification

1. `cargo check` — compiles
2. `cargo test` — all new + existing tests pass
3. `cargo clippy` — no warnings
4. Manual: `POST /api/agents/spawn` with `explorer` type → agent runs and auto-cleans up
5. Manual: send complex task via chat → orchestrator spawns multiple subagents in parallel
