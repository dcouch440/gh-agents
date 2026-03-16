# File-First Data Transport — Vision

## What It Is

The system store becomes a **real POSIX filesystem** backed by JuiceFS (Postgres metadata + S3 storage). Every agent runs in a container with the workspace mounted at `/workspace/`. Agents read files, write files, install dependencies, and execute programs — like a developer on a real machine. The workspace is the primary data transport between steps. No passdowns, no upstream artifacts XML, no edge-bound data flow.

The workflow is a scheduler. The workspace is a shared filesystem. Steps are workers that read and write to the filesystem. Edges define execution order, not data flow.

## Why This Matters

**Context explosion.** A workforce step that produces a Python application writes 15 files. Today, all 15 show up in `upstream_artifacts` XML for every downstream step. The agent tasked with "run the scraper against these URLs" sees `parsers.py`, `models.py`, `test_engine.py` — files it will never touch. Token waste. Cognitive noise.

**Edge-bound propagation.** Data flows along DAG edges. If step A produces an app, step B runs it, and step C analyzes the results — step C has no edge to A. The app files vanish from C's context. The user has to manually wire edges for context propagation.

**Blind handoffs.** The transport layer dumps everything from direct parents and nothing from grandparents. No intelligence about relevance, no awareness of the full workflow state.

**Pipeline thinking.** The designer shapes agent responses as handoffs — "keep your response lean, full work to store." But agents are contributors to a shared project, not links in a chain.

**Files aren't executable.** The current system store is text blobs in S3/Postgres accessed via tool calls. An agent can't `pip install -r requirements.txt` or `python main.py`. Programs exist as inert strings, not runnable code.

File-first transport fixes all five. A real filesystem. A shared workspace. Programs are executable. Agents explore what's relevant and ignore what isn't. No edge dependency for awareness.

## The Workspace

Every agent's container mounts the workspace at `/workspace/`. It's a real POSIX filesystem — `ls`, `cat`, `pip install`, `python main.py` all work. The workspace is shared across all steps in a workflow. Files written by step A are visible to step B after A completes.

```
/workspace/
  my_app/
    main.py
    scraper/
      engine.py
      parsers.py
      models.py
    requirements.txt
    tests/
      test_engine.py
  results/
    scraped_data.json
    errors.log
  reports/
    analysis.md
```

The workspace is a living filesystem that grows as steps complete. The first step sees an empty `/workspace/`. Each subsequent step sees everything previous steps produced. Multiple steps contribute to the same directories — step 1 scaffolds `my_app/`, step 2 adds authentication to `my_app/`, step 3 adds a database layer to `my_app/`.

Agents create directories dynamically based on their task. There are no pre-defined namespaces, no step-bound directories, no system-managed structure. The agents decide how to organize their work — just like developers on a shared machine.

### Workspace Context in Agent Prompts

The agent's system prompt tells it the workspace exists. That's it.

```
Your working directory is /workspace/. Other steps in this workflow
may have produced files there. Explore what exists and do your job.
```

No XML blocks. No directory listings injected into the prompt. No namespace metadata. The agent runs `ls /workspace/` if it wants to see what's there. It reads files if it needs them. It writes files where they make sense.

The workspace is a filesystem, not a data structure in a prompt. The system mounts it and gets out of the way.

## The Container Model

Every agent runs in a Docker container with the workspace mounted as a real filesystem.

### Filesystem Layout

```
Container filesystem:
  /workspace/              ← JuiceFS mount (shared, persists across steps)
    my_app/                ← whatever agents created
    results/
    ...

  /tmp/                    ← local to container (disposable)
    venv/                  ← pip install goes here
    node_modules/          ← npm install goes here
    __pycache__/           ← bytecode cache
```

**Workspace** (`/workspace/`): shared across steps via JuiceFS. Source code, data, outputs — anything that should persist and be visible to other steps.

**Local** (`/tmp/`): disposable, local to the container. Installation artifacts (`venv/`, `node_modules/`), build caches, bytecode. Never touches the shared filesystem. Torn down with the container.

Container environment variables ensure all package managers default to `/tmp/`. The agent never has to think about it:

```dockerfile
# Python — venv and caches go to /tmp/
ENV VIRTUAL_ENV=/tmp/venv
ENV PATH="/tmp/venv/bin:$PATH"
ENV PIP_CACHE_DIR=/tmp/pip-cache
ENV PYTHONDONTWRITEBYTECODE=1

# Node — packages and caches go to /tmp/
ENV npm_config_prefix=/tmp/npm
ENV npm_config_cache=/tmp/npm-cache
ENV NODE_PATH=/tmp/node_modules

# Rust — cargo home goes to /tmp/
ENV CARGO_HOME=/tmp/cargo

# General — XDG cache goes to /tmp/
ENV XDG_CACHE_HOME=/tmp/cache
```

Agent runs `pip install -r requirements.txt` — goes to `/tmp/venv/`. Agent runs `npm install` — goes to `/tmp/node_modules/`. No agent instructions needed, no `.gitignore` equivalent. The tools just write to the right place by default.

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

### Run Isolation

Each workflow run starts with a **fresh workspace**. No files carry over from previous runs. The workspace is created empty when the run begins and torn down when it ends.

**Exception: pinned steps.** When a step is pinned, its workspace files are sealed and persisted. On the next run, pinned files are pre-loaded into the fresh workspace before execution begins. The pinned step replays instantly (zero tokens) and downstream steps see its files as if it just ran.

```
Run N:
  → Fresh /workspace/ (empty)
  → Pinned step files pre-loaded from sealed state
  → Steps execute, workspace grows
  → Run completes

Run N+1:
  → Fresh /workspace/ (empty again)
  → Same pinned files pre-loaded
  → Unpinned steps re-execute with latest logic
```

### User Input

Users can upload files to the workspace before or during a run — reference data, seed files, configuration. These appear in `/workspace/` alongside agent-produced files.

Agents never see the user's original prompt or chat messages. They only see the designer's assignment and whatever files exist in the workspace. The designer is the sole author of what agents know about their task.

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

**2. Parallel steps execute.** Each container mounts the same JuiceFS.

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

**Old mental model:**
```
"Step A receives data from upstream. Step B needs that data.
 Wire an edge so data flows from A to B. Configure the team
 to process this input and produce this output."
```

**New mental model:**
```
"Step A does work and leaves it in the workspace. Step B does
 its work after A finishes. Wire an edge so B runs after A.
 The workspace is shared — B can see whatever A produced."
```

The builder thinks about:
1. **What work needs to happen** — each step's purpose
2. **What order** — which steps depend on others finishing first
3. **Node content** — the description that guides the designer

The builder does NOT think about:
- What data flows between steps (the workspace handles it)
- What files step A produces for step B (the handoff handles it)
- How to format output for downstream consumption (the designer handles it)

**Example: builder creates a 4-step workflow**

```
Step 1: "Build Web Scraper"
  → Creates the scraper application

Step 2: "Run Scraper"
  → Executes the scraper against target URLs
  → Edge from Step 1 (needs the app to exist)

Step 3: "Analyze Results"
  → Statistical analysis of scraped data
  → Edge from Step 2 (needs results to exist)

Step 4: "Write Executive Report"
  → Final deliverable for stakeholders
  → Edge from Step 3 (needs analysis to exist)
```

Edges express "needs to exist before I start." Nothing about data format, file paths, or output structure. The builder configured what each step does and when it runs. The designer takes it from here.

### Designer — Shaping the Handoff

The designer writes per-agent prompts for each step's workforce. In the file-first model, the designer's key tool is `expected_output` — it shapes the agent's text response into an orientation handoff targeted at the specific downstream step.

**The designer knows both sides.** It designed step 2's agents AND knows what step 3 needs. So it writes step 2's `expected_output` to contain exactly what step 3's agents need to orient themselves in the workspace.

**How to write `expected_output`:**

1. **What did you produce?** — the nature of the output (an app, data, a report)
2. **Where is it?** — paths in the workspace
3. **How does the next step use it?** — run instructions, data format, key files

The `expected_output` is an instruction TO the agent about what its text response should contain. It's NOT the output itself — it's the template.

### Full Designer Example: 4-Step Scraper Workflow

**Step 1: "Build Web Scraper"** (next step needs to run the app)

```json
{
  "system_prompt": "You are a Python developer. Your working directory
    is /workspace/ where other steps will use what you produce.",

  "assignment": "Build a Python CLI web scraper that crawls URLs and
    extracts article content into structured JSON. Write all source
    files to the workspace.",

  "expected_output": "Describe the application you built: where the
    entry point lives in the workspace, how to install its dependencies,
    what CLI arguments it accepts, and what format it returns.",

  "tools": ["run_command"]
}
```

Agent's text response (shaped by expected_output):
```
Built a Python CLI scraper at /workspace/scraper/. Entry point is
main.py. Install: pip install -r requirements.txt. Run:
python main.py --url {url} --depth {n} --format json. Returns a
JSON array of {title, content, url, scraped_at} to stdout. 6 source
files, tested with 3 unit tests.
```

**Step 2: "Run Scraper"** (next step needs to analyze results data)

```json
{
  "system_prompt": "You are a test executor. Your working directory
    is /workspace/ where a previous step built an application.",

  "assignment": "A scraper was built in a previous step. Read the
    previous step's handoff to find it, install its dependencies,
    and run it against these URLs: [url1, url2, url3, url4, url5].
    Save results to the workspace.",

  "expected_output": "Report how many URLs were scraped successfully,
    any failures, and where the results data lives in the workspace.
    Describe the data format so the analysis step can read it.",

  "tools": ["run_command"]
}
```

Agent's text response:
```
Executed the scraper against 5 target URLs. 4 succeeded, 1 timed
out (url3). Results written to /workspace/results/scraped_data.json
— JSON array with 4 entries, each containing title, content, url,
and scraped_at fields. Timeout error logged to
/workspace/results/errors.log.
```

**Step 3: "Analyze Results"** (next step needs key findings for a report)

```json
{
  "system_prompt": "You are a data analyst. Your working directory
    is /workspace/ where previous steps have produced data.",

  "assignment": "Previous steps scraped competitor data. Read the
    previous step's handoff to find the results, then analyze for
    pricing trends, anomalies, and market positioning. Write your
    analysis to the workspace.",

  "expected_output": "Summarize the key findings: how many competitors
    analyzed, main trends, notable anomalies, and confidence level.
    Note where the full analysis and supporting charts live in the
    workspace.",

  "tools": ["run_command"]
}
```

Agent's text response:
```
Analyzed scraped data from 4 competitors. Key findings: 3 of 4
increased enterprise pricing 12-18% YoY. One outlier (CompetitorB)
dropped prices 5% — likely a market grab. High confidence on
pricing data, medium confidence on tier breakdowns (2 sources
had inconsistent tier naming). Full analysis at
/workspace/analysis/report.md. Charts at /workspace/analysis/charts/.
```

**Step 4: "Write Executive Report"** (final step, no downstream)

```json
{
  "system_prompt": "You are a technical writer. Your working directory
    is /workspace/ where previous steps have produced analysis.",

  "assignment": "Previous steps researched and analyzed competitor
    pricing. Read the previous step's handoff to find the analysis,
    then write an executive report for stakeholders. Write the
    report to the workspace.",

  "expected_output": "Confirm the report is written and where it
    lives in the workspace. Note the word count and key sections.",

  "tools": []
}
```

Agent's text response:
```
Executive report written to /workspace/deliverable/pricing-report.md.
1,200 words. Sections: Executive Summary, Pricing Comparison Table,
YoY Trend Analysis, Anomaly Flags, Recommendations. Includes
embedded charts from the analysis step.
```

### The Pattern

Every `expected_output` follows the same structure:

```
1. What you produced (the nature of your output)
2. Where it lives (paths in the workspace)
3. What the next step needs to know (format, key details, how to use it)
```

The designer tailors #3 to the specific downstream consumer:
- Next step **runs** the output → include run instructions
- Next step **analyzes** the output → include data format and location
- Next step **writes about** the output → include key findings and confidence
- No downstream step → just confirm completion and location

### What the Next Step Receives

When step 3 starts, its agents see:
1. **System prompt** — "You are a data analyst. Your working directory is /workspace/..."
2. **Assignment** — "Previous steps scraped competitor data. Read the previous step's handoff to find the results..."
3. **Previous step output** — Step 2's text: "Executed the scraper against 5 URLs... Results at /workspace/results/scraped_data.json..."
4. **The workspace** — `/workspace/` mounted with everything from steps 1 and 2

The previous step output orients the agent to the right files. The workspace has the actual data. The assignment tells the agent what to do. The `expected_output` shapes what this agent will say to step 4.

### Agent Prompt Pattern

The system prompt always grounds the agent in the workspace:

```
You are a [role]. Your working directory is /workspace/ where other
steps have contributed and will contribute after you. Explore what
exists and do your job.
```

The assignment always references the workspace and the previous handoff:

```
[Context from builder about what this step does]. Read the previous
step's handoff to find [what you need]. [Your specific task].
Write your output to the workspace.
```

### Observability

The handoff text IS the per-step observability. The UI shows each step's text output — the orientation shaped by `expected_output`. The user reads a step's output and sees: what was done, what was produced, where it lives. No need to track which files each step created. No need to diff the workspace before/after each step. The handoff tells the story.

```
Step 1 — Write Scraper:
  "Built a Python CLI scraper in /workspace/my_app/. Entry point
   is main.py, accepts --url and --depth. Returns JSON to stdout.
   6 source files, requirements.txt included."

Step 2 — Run Scraper:
  "Executed the scraper against 5 target URLs. Results written to
   /workspace/results/scraped_data.json. 1 URL failed (timeout),
   logged to /workspace/results/errors.log."

Step 3 — Analyze Results:
  "Analysis report written to /workspace/reports/analysis.md.
   Key finding: 3 of 5 competitors increased pricing 12-18% YoY.
   Supporting charts in /workspace/reports/charts/."
```

The user reads the workflow execution like a story — each step's handoff is a paragraph. Click into the workspace to see the actual files.

## What This Replaces

| Old | New |
|-----|-----|
| `upstream_artifacts` XML (edge-bound, lists every file) | Real filesystem at `/workspace/` |
| Passdown data as primary transport | Workspace files are the data; text output is orientation |
| Edge-based data propagation | Edges are scheduling; workspace is global |
| `store_read_file` / `store_write_file` (text blobs) | Real POSIX filesystem (`cat`, `ls`, `python`) |
| Response shaping for downstream consumption | Designer shapes `expected_output` as targeted briefing |
| Step-bound namespaces | Agents create directories dynamically |

## What This Doesn't Replace

- **DAG execution order** — edges still determine topological sort and parallelism
- **Designer per-agent prompts** — reframed from handoffs to workspace contributions
- **Agent text output** — still flows to downstream steps as orientation; designer shapes it via `expected_output`
- **`<previous_agent_outputs>` within workforce** — stays for within-step agent coordination
- **Execution envelopes** — still track metadata (tokens, cost, status); text output carries orientation, not data
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
          "Your working directory is /workspace/."
                        ↓
Execute: DAG runs topologically
         Each step gets a container with /workspace/ mounted.
         Previous steps' files are there on disk.
         Agent installs deps locally, reads/writes workspace.
         Programs are executable. Data is real.
         Agents create directories as they see fit.
                        ↓
Step completes → container torn down
              → workspace persists (JuiceFS)
              → parallel steps: merge if needed (diff3 + Haiku)
              → next batch starts with expanded workspace
```

## Scope: Creation Workflows

This vision focuses on workflows where **agents create things** — the workspace starts empty and agents build applications, research, reports, data pipelines. The workspace grows organically. The handoff chain orients each step to what previous steps produced.

**Repository-based workflows** — where agents operate on an existing codebase (find bugs, add features, refactor, submit PRs) — are a different concern. They require git integration: cloning, branching, committing, merging back to origin. This is Codex/Cursor territory and can be built as an input mechanism on top of file-first transport (system pre-loads `/workspace/` with a repo clone before the run starts). But the git lifecycle (branches, PRs, merge-back) is a future extension, not a core architectural concern for this vision.

## Cross-Workflow Composition (Future)

Each workflow gets its own workspace. Cross-workflow sharing via mounts:

```
Workflow A: /workspace/                   ← Workflow A's filesystem
Workflow B: /workspace/                   ← Workflow B's filesystem
            /workspace/imports/from-a/    ← read-only mount of A's output
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
| Upstream artifacts | Edge-bound file listing in XML | Replaced by shared workspace filesystem |
| Execution envelopes | Step output + metadata | Metadata only; workspace carries data |
| DAG edges | Topological ordering + data routing | Ordering only; workspace handles data |
| Pin system | Replay from sealed namespace | Workspace files persist with pins |
