# File-First Data Transport — Vision

## What It Is

The system store becomes a **real POSIX filesystem** backed by JuiceFS (Postgres metadata + S3 storage). Every agent runs in a container with the workspace mounted at `/workspace/`. Agents read files, write files, install dependencies, and execute programs — like a developer on a real machine. The workspace is the primary data transport between steps. No passdowns, no upstream artifacts XML, no edge-bound data flow.

The workflow is a scheduler. The workspace is a shared filesystem. Steps are workers that read and write to the filesystem. Edges define execution order, not data flow.

## Why This Matters

**Context explosion.** A workforce step that produces a Python application writes 15 files. Today, all 15 show up in `upstream_artifacts` XML for every downstream step. The agent tasked with "run the scraper against these URLs" sees `parsers.py`, `models.py`, `test_engine.py` — files it will never touch. Token waste. Cognitive noise. The agent's attention scatters across irrelevant detail.

**Edge-bound propagation.** Data flows along DAG edges. If step A produces an app, step B runs it, and step C analyzes the results — step C has no edge to A. The app files vanish from C's context. The user has to manually wire edges for context propagation, which defeats the point of a DAG.

**Blind handoffs.** The transport layer dumps everything from direct parents and nothing from grandparents. No intelligence about relevance, no awareness of the full workflow state.

**Pipeline thinking.** The designer currently shapes agent responses as handoffs — "keep your response lean, full work to store." Agents are trained to think in input/output pipelines. But the real model is simpler: agents are contributors to a shared project. They read what they need and write what they produce.

**Files aren't executable.** The current system store is text blobs in S3/Postgres accessed via tool calls. An agent can't `pip install -r requirements.txt` or `python main.py`. Programs exist as inert strings, not runnable code.

File-first transport fixes all five. A real filesystem. A shared workspace. Agents pull exactly the depth they need. No edge dependency for awareness. Programs are executable.

## The Workspace

Every agent's container mounts the workspace at `/workspace/`. It's a real POSIX filesystem — `ls`, `cat`, `pip install`, `python main.py` all work. The workspace is shared across all steps in a workflow. Files written by step A are visible to step B after A completes.

```
/workspace/                          ← JuiceFS mount, shared across all steps
  my_app/
    main.py
    scraper/engine.py
    scraper/parsers.py
    scraper/models.py
    requirements.txt
    tests/test_engine.py
  results/
    scraped_data.json
    errors.log
  reports/
    analysis.md
```

The workspace is a living filesystem that grows as steps complete. The first step sees an empty workspace. Each subsequent step sees everything previous steps produced. Multiple steps can contribute to the same directory — step 1 scaffolds `my_app/`, step 2 adds authentication to `my_app/`, step 3 adds database layer to `my_app/`.

### Workspace Context in Agent Prompts

Every agent's prompt includes a `<workspace>` block listing completed namespaces. The description comes from the builder's step name — no LLM needed.

```xml
<workspace goal="Security audit of the customer portal">
  You are one step in a multi-step workflow. All steps share this
  workspace — a real filesystem mounted at /workspace/. You can
  ls, cat, and execute files directly. Use store_list_files to
  see what's available.

  <namespace dir="write_scraper" files="8" desc="Write Scraper"/>
  <namespace dir="run_scraper" files="2" desc="Run Scraper"/>
</workspace>
```

5-10 tokens per namespace. A 20-step workflow is 100-200 tokens. The agent sees what directories exist, decides what's relevant, and explores with standard filesystem tools or `store_list_files`/`store_read_file`.

### Agent-Driven Depth

The agent decides how deep to go:

```
Level 0 — Workspace listing (automatic, in prompt)
  Sees directory names and file counts.

Level 1 — Directory listing (ls or store_list_files)
  ls /workspace/write_scraper/
  → main.py  engine.py  parsers.py  models.py  requirements.txt

Level 2 — File contents (cat or store_read_file)
  cat /workspace/write_scraper/main.py
  → (full source code, executable)
```

Most agents stay at Level 1. The executor agent reads a few files to understand how to run the app, then runs it. The refactoring agent reads source files and modifies them in place.

## The Container Model

Every agent runs in a Docker container with the workspace mounted as a real filesystem.

### Filesystem Layout

```
Container filesystem:
  /workspace/              ← JuiceFS mount (shared, persists across steps)
    my_app/                ← workspace files, readable/writable
    results/
    ...

  /tmp/                    ← local to container (disposable)
    venv/                  ← pip install goes here
    node_modules/          ← npm install goes here
    __pycache__/           ← bytecode cache
```

**Workspace** (`/workspace/`): shared across steps via JuiceFS. Source code, data, outputs — anything that should persist and be visible to other steps.

**Local** (`/tmp/`): disposable, local to the container. Installation artifacts (`venv/`, `node_modules/`), build caches, bytecode. Never touches the shared filesystem. Torn down with the container.

This separation means the metadata-heavy small-file storm from `pip install` (thousands of files, symlinks, `.pyc` files) never hits JuiceFS. The workspace only sees code files, data, and outputs — larger, fewer, less metadata pressure.

### Container Lifecycle

```
Sequential step:
  → Container launches with JuiceFS mounted at /workspace/
  → Previous steps' files are visible (close-to-open consistency)
  → Agent installs dependencies locally (/tmp/venv/)
  → Agent reads workspace files, executes programs, writes output
  → Step completes, container torn down
  → Next step's container sees all files

Parallel steps:
  → Multiple containers mount same JuiceFS simultaneously
  → Each sees the same workspace snapshot
  → Each writes output (see Merge Strategy below)
  → All complete, merge if needed, next batch starts
```

## Infrastructure: JuiceFS + Postgres + S3

**JuiceFS** provides a POSIX-compatible filesystem with metadata in Postgres and data in S3. Clients mount via FUSE. Full random read/write. Close-to-open consistency — once a file is written and closed, all other clients see the update on next open.

### Why JuiceFS

- **Full POSIX**: `ls`, `cat`, `chmod`, `pip install`, `python main.py` — everything works
- **Close-to-open consistency**: step B sees step A's files immediately after A completes
- **Multi-writer**: multiple containers mount simultaneously via Kubernetes CSI driver
- **S3 data storage**: uses our existing MinIO/S3 infrastructure
- **Postgres metadata**: uses our existing Postgres — no new dependencies
- **Production-ready**: heavily deployed for AI workloads, Apache-2.0

### Why Not Redis for Metadata

JuiceFS with Redis metadata is 2-4x faster for small file operations. But installation artifacts (`pip install`, `npm install`) go to `/tmp/`, not the workspace. The workspace only sees code and data files — Postgres handles that fine. No new dependency needed.

### Why Not Other Options

- **S3 FUSE mounts** (Mountpoint, s3fs, goofys): no random write support, can't run dev tools
- **SeaweedFS**: less POSIX-compliant, more infrastructure to manage
- **LakeFS**: great for branching/merging, but not a POSIX filesystem — can't run `pip install`
- **Git**: can't handle binary files, 1GB limit, merge conflicts with parallel agents
- **Raw S3 sync**: no real filesystem, agents can't execute programs

### Architecture

```
┌─────────────────┐     ┌─────────────────┐
│  Container A    │     │  Container B    │
│  /workspace/ ←──┼─────┼──→ /workspace/  │
│  (FUSE mount)   │     │  (FUSE mount)   │
└────────┬────────┘     └────────┬────────┘
         │                       │
         └───────────┬───────────┘
                     │
              ┌──────┴──────┐
              │   JuiceFS   │
              │  Metadata   │
              │ (Postgres)  │
              └──────┬──────┘
                     │
              ┌──────┴──────┐
              │   JuiceFS   │
              │    Data     │
              │  (S3/MinIO) │
              └─────────────┘
```

## Merge Strategy for Parallel Steps

When parallel steps write to different files — no conflict, auto-merge. When they modify the same file — three-way merge with LLM conflict resolution.

### The Flow

**1. Snapshot before parallel batch.** Record file checksums from workspace.

**2. Parallel steps execute.** Each container mounts the same JuiceFS. JuiceFS close-to-open consistency provides isolation during execution.

**3. After all parallel steps complete.** Diff each step's changes against the snapshot:
- **New files from one step**: accept (no conflict)
- **Modified by one step only**: accept (no conflict)
- **Modified by multiple steps**: run three-way merge

**4. Three-way merge** (`diff3`, standard Unix tool):
```
Lines unchanged by both     → keep base           (automatic)
Lines changed by A only     → take A's version    (automatic)
Lines changed by B only     → take B's version    (automatic)
Lines changed by both       → CONFLICT            (LLM resolves)
```

90%+ of the file merges automatically. Only conflict hunks go to the LLM.

**5. Conflict resolution.** Send just the conflicting lines (plus surrounding context) to Haiku:

```
File: /workspace/my_app/main.py, lines 36-42 conflict.

Context (unchanged):
  from flask import Flask
  app = Flask(__name__)

Agent A wrote:
  from auth import auth_middleware
  app.use(auth_middleware)

Agent B wrote:
  from db import init_database
  init_database(app)

Combine both changes.
```

Haiku returns the merged version. One call per conflict hunk, ~20 lines of context, fractions of a cent.

**6. Binary file conflicts.** Can't line-merge images or data. Policy: last-write-wins, keep-both (rename), or flag for user. Rare in practice — parallel steps producing binary files usually produce different files.

### Why Conflicts Are Rare

If the builder designs the DAG well, parallel steps do genuinely different work:
- **Different files modified**: most common, auto-merge, zero cost
- **Same file, different sections**: auto-merge, zero cost
- **Same file, same lines**: rare, one Haiku call
- The merge system is a safety net, not the primary path

## How the Builder and Designer Change

### Builder — From Routing to Scheduling

**Old:** "Wire edges so data flows from A to B. Configure the team to handle this input."

**New:** "Wire edges so steps run in the right order. Configure the team to do their job in the workspace."

The builder stops thinking about data routing. Edges are scheduling, not plumbing.

### Designer — From Handoffs to Workspace Contributions

**Old:** "Keep responses lean. Full work to store. Shape the response as a handoff."

**New:** "Write everything to the workspace. Read the workspace to understand context. Your response is a brief status."

Four changes:

**1. `expected_output` becomes `expected_files`.** Not "respond with a numbered list." Instead: "Write findings to the workspace with clear descriptions."

**2. Kill `<previous_agent_outputs>`.** Agents don't receive upstream text in their prompt. They read the workspace. Within a workforce, `<previous_agent_outputs>` stays for now — it's small (brief status text) and the designer already coordinates within-workforce agents.

**3. No more response shaping.** Everything goes to files. The text response is a completion signal.

**4. Assignments reference the workspace, not agents.** "Review the security findings in the workspace" not "read the Scanner's output in `<previous_agent_outputs>`."

### Designer Prompt Guidelines

```
All work goes to the workspace:
1. Write everything to the workspace with clear file descriptions
2. Read the workspace to understand what's been done before you
3. Your text response is a brief status, not a handoff
```

### Agent Prompt Pattern

```
You are a security scanner. You work in a shared workspace at
/workspace/ where other steps have contributed and will contribute
after you. Scan the codebase for vulnerabilities. Write your
findings to the workspace.
```

The agent understands: it's a contributor to a shared project, not a link in a processing chain.

## What This Replaces

| Old | New |
|-----|-----|
| `upstream_artifacts` XML (edge-bound, lists every file) | `<workspace>` (global, directory names only) |
| Passdown data (envelope carries text downstream) | Workspace files (agent reads what it needs) |
| Edge-based data propagation | Edges are scheduling; workspace is global |
| `store_read_file` / `store_write_file` tools (text blobs from S3) | Real POSIX filesystem (`cat`, `ls`, `python main.py`) |
| Response shaping ("keep it lean for handoff") | All work to files, response is status |

## What This Doesn't Replace

- **DAG execution order** — edges still determine topological sort and parallelism
- **Designer per-agent prompts** — reframed from handoffs to workspace contributions
- **`<previous_agent_outputs>` within workforce** — stays for within-step agent coordination (small, brief text)
- **Execution envelopes** — still track metadata (tokens, cost, status), no longer carry data
- **Pin system** — pinned steps replay; their workspace files persist

## The Full Pipeline

```
User draws workflow → Board Submit
                        ↓
Phase 0: Create nodes, wire edges (agentless, instant)
                        ↓
Builder: Topology + node content (per-node)
         Thinks in scheduling, not data routing.
                        ↓
Designer: Per-agent prompts (per-node, writes expected_files)
          Frames agents as workspace contributors.
                        ↓
Execute: DAG runs topologically
         Each step gets a container with /workspace/ mounted.
         Previous steps' files are there on disk.
         Agent installs deps locally, reads/writes workspace.
         Programs are executable. Data is real.
                        ↓
Step completes → container torn down
              → workspace persists (JuiceFS)
              → namespace appears in <workspace> for next batch
              → parallel steps: merge if needed (diff3 + Haiku)
              → next batch starts with expanded workspace
```

## Cross-Workflow Composition (Future)

Each workflow gets its own JuiceFS namespace (prefix). Cross-workflow sharing via mounts:

```
Workflow A: /workspace-a/
Workflow B: /workspace-b/
            /workspace-b/imports/workflow-a/  ← read-only mount of A's output
```

Patterns from industry research (see `research/workspace-filesystem-infrastructure.md`):
- **Typed workflow ports** (Nextflow pattern) — workflows declare input/output directories
- **Alias-based references** (MLflow pattern) — mount "latest" output, not a specific run ID
- **Output manifests** — machine-readable description of output artifacts
- **Content-addressable caching** (Bazel pattern) — skip re-execution if inputs unchanged
- **Isolation by default** (K8s pattern) — each workflow owns its namespace, sharing is opt-in

## Cost Model

| Phase | What | Model | Cost |
|-------|------|-------|------|
| Workspace mount | JuiceFS FUSE mount in container | No LLM — infrastructure | Compute only |
| Workspace injection | Build `<workspace>` XML from step metadata | No LLM — template | Zero |
| Merge (clean) | diff3 auto-merge non-conflicting changes | No LLM — Unix tool | Zero |
| Merge (conflict) | Haiku resolves conflict hunks | Haiku (~20 lines per hunk) | ~$0.0001/hunk |
| File access | Agent reads/writes files directly | No LLM — filesystem | Zero |

No per-step LLM overhead for transport. The workspace is infrastructure. The only LLM cost is conflict resolution during parallel merges, which is rare and cheap.

## What This Builds On

| Capability | Already built | File-first transport adds |
|------------|---------------|--------------------------|
| System store | S3 backend, Postgres metadata | JuiceFS POSIX mount over same S3 + Postgres |
| Container execution | Docker containers for agents | Workspace mounted at /workspace/ |
| Store tools | `store_read_file`, `store_write_file` | Real filesystem access (ls, cat, python) |
| Designer prompts | Per-agent system_prompt, assignment, tools | Reframed: expected_files, workspace mindset |
| Upstream artifacts | Edge-bound file listing in XML | Replaced by global workspace |
| Execution envelopes | Step output + metadata | Metadata only; workspace carries data |
| DAG edges | Topological ordering + data routing | Ordering only; workspace handles data |
| Pin system | Replay from sealed namespace | Workspace files persist with pins |
