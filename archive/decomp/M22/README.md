# Milestone 22: Multi-Agent Docker Isolation

> Per-agent Docker containers with git worktree isolation, enabling multiple agents to work on different branches simultaneously on the same machine.

## Goal

Multiple agents can work on separate tickets/branches concurrently without filesystem or git conflicts. Each agent gets its own Docker container backed by a git worktree, communicates with the orchestrator over WebSocket, and merges results back when done.

**Checkpoint**: Orchestrator assigns 3 tasks on different branches. Each spawns a container with its own worktree. Agents edit files, run tests, commit — all in parallel with zero conflicts. Orchestrator collects results and merges.

---

## Problem

Today, all agents run in-process sharing a single filesystem and git checkout. If Agent A checks out `branch-a` and Agent B checks out `branch-b`, they stomp on each other. The existing `Sandbox` module (M7.4) provides Docker isolation for command execution, but agents themselves aren't containerized.

## Solution: Worktree + Container Per Agent

```
Host Machine
├── /project (main repo, .git/)
│   └── .nexor/worktrees/
│       ├── agent-abc/     ← git worktree on branch ticket-123
│       ├── agent-def/     ← git worktree on branch ticket-456
│       └── agent-ghi/     ← git worktree on branch ticket-789
│
├── Container: agent-abc   ← mounts worktrees/agent-abc as /workspace
├── Container: agent-def   ← mounts worktrees/agent-def as /workspace
└── Container: agent-ghi   ← mounts worktrees/agent-ghi as /workspace
```

**Why git worktrees**: `git worktree add` creates a lightweight checkout sharing the `.git` object database. Each worktree can be on a different branch with zero duplication of history. Creation is instant, cleanup is `git worktree remove`.

## Scope

- 7 tickets, ~30 slices
- New files: `src/execution/worktree.rs`, `src/agents/container.rs`, `src/agents/worker_client.rs`, `src/agents/container_pool.rs`
- Modified files: `src/execution/git.rs`, `src/execution/mod.rs`, `src/agents/pool.rs`, `src/agents/mod.rs`, `src/server/api.rs`
- New: `docker/agent-worker.Dockerfile`, updated `docker/docker-compose.yml`
- Modified: `src/types/config.rs`

## Key Concepts

| Concept | Description |
|---------|-------------|
| `WorktreeManager` | Creates/lists/removes git worktrees under `.nexor/worktrees/` |
| `AgentContainer` | Represents a running Docker container for a single agent |
| `ContainerPool` | Manages container lifecycle: create, monitor, destroy |
| `WorkerClient` | HTTP/WS client that the host orchestrator uses to communicate with containerized agents |
| `AgentWorkerMode` | Standalone binary mode where the nexor binary runs as a worker inside a container, connecting back to the host orchestrator |
| `agent-worker.Dockerfile` | Container image with the nexor binary + language runtimes needed to work on code |

## Architecture

```
┌─────────────────────────────────────────────────┐
│  Host: Orchestrator Process                      │
│  ┌──────────────┐  ┌────────────────────────┐   │
│  │ ContainerPool │  │ WorktreeManager        │   │
│  │ - spawn()     │  │ - create(branch)       │   │
│  │ - destroy()   │  │ - remove(agent_id)     │   │
│  │ - monitor()   │  │ - list()               │   │
│  └──────┬───────┘  └──────────┬─────────────┘   │
│         │                      │                  │
│  ┌──────┴──────────────────────┴───────────────┐ │
│  │           WorkerClient (per agent)           │ │
│  │  - assign_task()                             │ │
│  │  - stream_progress()                         │ │
│  │  - collect_result()                          │ │
│  └──────────────────────────────────────────────┘ │
└──────────────────────┬──────────────────────────┘
                       │ Docker API / HTTP
    ┌──────────────────┼──────────────────────┐
    │                  │                      │
┌───┴───┐         ┌───┴───┐            ┌─────┴──┐
│Agent A│         │Agent B│            │Agent C │
│Worker │         │Worker │            │Worker  │
│Mode   │         │Mode   │            │Mode    │
│       │         │       │            │        │
│/work  │         │/work  │            │/work   │
│space  │         │space  │            │space   │
└───────┘         └───────┘            └────────┘
 worktree/a        worktree/b           worktree/c
 branch-123        branch-456           branch-789
```

## Dependency Graph

```
22.1 (Worktree Manager)
  └→ 22.2 (Agent Worker Dockerfile)
      └→ 22.3 (Container Lifecycle)
          ├→ 22.4 (Worker Mode Binary)
          │    └→ 22.5 (Worker Client Protocol)
          │         └→ 22.6 (Pool Integration)
          └→ 22.7 (Cleanup & Monitoring)
```

## Tickets

| Ticket | Title | Slices | Dependencies |
|--------|-------|--------|-------------|
| 22.1 | Git Worktree Manager | 5 | None |
| 22.2 | Agent Worker Dockerfile | 3 | None (parallel with 22.1) |
| 22.3 | Container Lifecycle Management | 5 | 22.1, 22.2 |
| 22.4 | Agent Worker Mode | 5 | 22.3 |
| 22.5 | Worker Client Protocol | 4 | 22.4 |
| 22.6 | Container Pool Integration | 5 | 22.5 |
| 22.7 | Cleanup, Monitoring & Health | 3 | 22.3 |

## Key Design Decisions

1. **Git worktrees over full clones** — Worktrees share the `.git` object store, making creation instant and disk-efficient. `git worktree add .nexor/worktrees/agent-abc -b ticket-123` takes milliseconds.
2. **Orchestrator stays on host** — Only worker agents run in containers. The orchestrator manages worktrees and containers from the host process, keeping coordination simple.
3. **SQLite stays on host** — Containers don't access the database. All state flows through the orchestrator via HTTP/WS.
4. **Existing Sandbox infrastructure reused** — `SandboxConfig` (resource limits, timeouts, env vars) applies directly to agent containers. The `ContainerPool` wraps the existing Docker execution patterns.
5. **Worker mode is a flag** — `nexor --worker --orchestrator-url ws://host:3000/ws/worker` starts the binary in worker mode. Same binary, different entrypoint.
6. **Merge happens on host** — When an agent finishes, the orchestrator does `git merge` from the worktree on the host. Conflict detection from `src/execution/git.rs` is reused directly.
7. **Container image includes language runtimes** — The worker Dockerfile installs git, Node.js, Python, and common build tools so agents can run tests and builds inside their containers.
8. **Graceful degradation** — If Docker is unavailable, the system falls back to in-process execution (current behavior). Container isolation is opt-in via config.

## Configuration

```toml
[agents.containers]
enabled = true                          # false = in-process (current behavior)
image = "nexor-worker:latest"           # Worker container image
memory_limit = "2g"                     # Per-container memory
cpu_limit = "2.0"                       # Per-container CPU
max_containers = 5                      # Max concurrent containers
network_enabled = true                  # Containers need network for git push
worktree_dir = ".nexor/worktrees"       # Relative to project root
cleanup_on_complete = true              # Remove worktree + container after task
```

## Verification

1. `cargo check` — compiles
2. `cargo test` — all new + existing tests pass
3. `cargo clippy` — no warnings
4. Manual: create worktree, verify isolated branch checkout
5. Manual: spawn container with worktree mount, verify file isolation
6. Manual: run 2+ agents in parallel on different branches, verify no conflicts
7. Manual: agent completes task, worktree + container cleaned up
8. Manual: Docker unavailable → falls back to in-process execution
