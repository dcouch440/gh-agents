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
What the agent sees (merged view):
  /workspace/              ← looks like one filesystem
    my_app/                ← from JuiceFS (previous steps)
    results/               ← from JuiceFS (previous steps)
    new_output/            ← written this step (lives in overlay)
    node_modules/          ← installed this step (lives in overlay, will be filtered)

What's actually happening:
  JuiceFS (read-only lower) ← shared workspace, previous steps' files
  OverlayFS upper (writable) ← all writes from this step go here
```

The agent sees one filesystem and writes freely — `pip install`, `git clone`, build artifacts, whatever. All writes go to the local OverlayFS upper layer at native filesystem speed. The agent has no idea anything special is happening.

**When the step completes**, the system diffs the overlay against the base and filters out junk before persisting clean output back to JuiceFS:

```
Overlay diff → filter → persist to JuiceFS

Filtered out (denylist, same patterns as .gitignore):
  .git/
  __pycache__/, .pytest_cache/
  node_modules/, .npm/
  target/, dist/, build/
  .venv/, venv/
  .cache/
  *.pyc, *.o, *.so
```

The denylist is a static config. Anything not denied gets persisted. Only clean output files make it to the shared workspace.

This solves three problems at once:
- **Workspace pollution**: junk files (caches, installed packages, build artifacts) never reach JuiceFS
- **Metadata pressure**: the small-file storm from `pip install` hits the local overlay (native speed), never touches Postgres metadata
- **Agent freedom**: agents write wherever they want — the system handles cleanup

Environment variables still steer common package managers to `/tmp/` as a first line of defense:

```dockerfile
ENV VIRTUAL_ENV=/tmp/venv
ENV PATH="/tmp/venv/bin:$PATH"
ENV PIP_CACHE_DIR=/tmp/pip-cache
ENV PYTHONDONTWRITEBYTECODE=1
ENV npm_config_prefix=/tmp/npm
ENV npm_config_cache=/tmp/npm-cache
ENV CARGO_HOME=/tmp/cargo
ENV XDG_CACHE_HOME=/tmp/cache
```

But even if an agent bypasses these (installs to the workspace directly, clones a repo, runs a build), the OverlayFS filter catches it. Belt and suspenders.

### Container Lifecycle

```
Sequential step:
  → Container launches
  → JuiceFS mounted read-only at /workspace/ (lower layer)
  → OverlayFS writable upper layer on top
  → Agent sees merged view, writes freely
  → Step completes
  → Overlay diff filtered (denylist removes junk)
  → Clean output persisted to JuiceFS
  → Container torn down
  → Next step sees clean workspace

Parallel steps:
  → Multiple containers, each with own overlay on same JuiceFS base
  → Each writes to its own overlay (no cross-container conflicts)
  → All complete
  → Each overlay filtered and diffed
  → Clean diffs merged (see Merge Strategy below)
  → Next batch sees merged workspace
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
┌──────────────────────────────┐
│         Container            │
│                              │
│  /workspace/ (merged view)   │
│    ┌───────────────────┐     │
│    │  OverlayFS upper  │ ← agent writes here (local, fast)
│    ├───────────────────┤     │
│    │  JuiceFS lower    │ ← previous steps' files (read-only)
│    └────────┬──────────┘     │
│             │                │
│  /tmp/ (local, disposable)   │
└─────────────┼────────────────┘
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

Step completes:
  overlay diff → denylist filter → persist clean files → JuiceFS
```

## Merge Strategy for Parallel Steps

When parallel steps write to different files — no conflict, auto-merge. When they modify the same file — three-way merge with LLM conflict resolution.

### The Flow

**1. Parallel steps execute.** Each container gets its own OverlayFS upper on the same JuiceFS base. No snapshot needed — the overlay IS the diff.

**2. After all parallel steps complete.** Filter each overlay (denylist), then compare the clean diffs:
- **New files from one step**: accept (no conflict)
- **Modified by one step only**: accept (no conflict)
- **Modified by multiple steps**: run three-way merge

**3. Three-way merge** (`diff3`, standard Unix tool):
```
Lines unchanged by both     → keep base           (automatic)
Lines changed by A only     → take A's version    (automatic)
Lines changed by B only     → take B's version    (automatic)
Lines changed by both       → CONFLICT            (LLM resolves)
```

90%+ of the file merges automatically. Only conflict hunks go to the LLM.

**4. Conflict resolution.** Send just the conflicting lines (plus surrounding context) to Haiku:

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

**5. Binary file conflicts.** Can't line-merge images or data. Policy: last-write-wins, keep-both (rename), or flag for user. Rare in practice — parallel steps producing binary files usually produce different files.

### Why Conflicts Are Rare

If the builder designs the DAG well, parallel steps do genuinely different work:
- **Different files modified**: most common, auto-merge, zero cost
- **Same file, different sections**: auto-merge, zero cost
- **Same file, same lines**: rare, one Haiku call
- The merge system is a safety net, not the primary path

## Tool Model

Every agent runs in a container with a shell. Most tools are implicit — always available, never listed by the designer.

### Implicit (always available, designer never assigns these)

- **Shell access** — `ls`, `cat`, `grep`, `python`, `pip`, `npm`, `curl`, any CLI tool. The agent has a real terminal in a sandboxed container. Blast radius is the container — can only affect `/workspace/` (shared) and `/tmp/` (disposable).
- **Web search / X search** — model-native. The agent searches the web and social media naturally. No tool definition needed — just natural language in the assignment: "Search the web for..."
- **Store tools** — `store_read_file`, `store_write_file` remain available alongside raw filesystem access for backward compatibility and metadata tracking.
- **Think** — reason without acting.

Shell access subsumes most traditional agent tools:
- `ls` / `find` replaces `store_list_files`
- `cat` replaces `store_read_file`
- File writes via shell/python replaces `store_write_file`
- `pip install` / `npm install` / `python main.py` just work
- `grep` / `rg` replaces `content_search`

### Designer-Assigned Tools (rare)

The designer's `tools` list is almost always empty. It only adds capabilities that neither the shell nor the model provide — domain-specific integrations, API access to internal systems, specialized tooling.

For most agents, a shell and a brain is all they need.

## How the Builder and Designer Change

### Builder — From Routing to Scheduling

The builder's prompt includes this workspace context:

```
Agents run in containers with a shared workspace at /workspace/.
Every step sees files from all previous steps. Agents have full
shell access — they can install, build, and run programs directly.
Most tasks need zero explicit capabilities.
```

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

The designer's prompt includes this workspace context:

```
Agents have /workspace/ mounted with all previous steps' files.
They have shell access. Write assignments that reference the
workspace and the previous step's handoff directly.
```

The designer runs in **step order** — same order as execution. Each step's designer sees the previous step's handoff and the next step's box text. The narrative threads naturally because each handoff feeds into the next step's design context.

```
Builder creates all nodes (full context, detailed descriptions)
                    ↓
Designer runs topologically:
  Step 1: builder desc + downstream desc
        → designs agents + expected_output
  Step 2: builder desc + step 1's expected_output + downstream desc
        → designs agents + expected_output
  Step 3: builder desc + step 2's expected_output + downstream desc
        → designs agents + expected_output
  Step 4: builder desc + step 3's expected_output
        → designs agents + expected_output
```

Each designer receives:
- **Builder's node description** — detailed, specific (the builder had full context)
- **Upstream expected_outputs** — what previous steps promised to hand off (already built + designed, has step name and contract)
- **Downstream box text** — raw text from the user's canvas (not built or designed yet, just the user's words)

The designer's key tool is `expected_output` — it shapes the agent's text response into an orientation handoff targeted at the specific downstream step.

**How to write `expected_output`:**

1. **What did you produce?** — the nature of the output (an app, data, a report)
2. **Where is it?** — paths in the workspace
3. **How does the next step use it?** — run instructions, data format, key files

The `expected_output` is an instruction TO the agent about what its text response should contain. It's NOT the output itself — it's the template.

Each `expected_output` is a contract. The next step's designer reads that contract and writes an assignment that references it. The narrative chains: step 1's expected_output → step 2's assignment → step 2's expected_output → step 3's assignment. No guessing. No gaps.

If the user edits step 1's design, the system re-runs the designer for downstream steps — the chain is invalidated and rebuilt. Same topological propagation as execution.

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

  "tools": []
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

  "tools": []
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

  "tools": []
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

### The Narrative Principle

The designer writes the workflow like a book. Each step's assignment reads like the next chapter — it references what came before and sets up what comes next. The agent never has to guess or explore blindly.

Three layers of context, each more specific:
1. **Assignment** — tells the agent what exists and what to do (designer writes this)
2. **Previous step handoff** — tells the agent where to find things (previous agent wrote this)
3. **Workspace** — has the actual files (previous agents produced these)

The assignment carries the narrative arc. Step 4 knows the full story — research, scrape, analyze — even though it only has an edge from step 3. The designer wrote that context in because it knows the whole workflow.

```
Step 1 assignment:
  "Build a Python CLI web scraper that crawls URLs and extracts
   article content into structured JSON. Write all source files
   to the workspace."

Step 2 assignment:
  "A previous step built a web scraper. Read the previous step's
   handoff to find where it lives and how to run it. Install its
   dependencies and execute it against the target URLs. Save
   results to the workspace."

Step 3 assignment:
  "Previous steps built a web scraper and ran it against target
   URLs. Read the previous step's handoff to find the results
   data. Analyze for pricing trends, anomalies, and market
   positioning. Write your analysis to the workspace."

Step 4 assignment:
  "Previous steps researched competitor pricing, executed a
   scraper, and analyzed the results. Read the previous step's
   handoff to find the analysis. Write an executive report for
   stakeholders that covers trends, anomalies, and recommendations.
   Write the report to the workspace."
```

Each assignment gets richer as the workflow progresses. Step 1 has no history. Step 4 has the full arc. The designer builds this narrative because it sees the whole workflow topology — it knows what every step does and how they connect.

### Agent Prompt Pattern

The system prompt always grounds the agent in the workspace:

```
You are a [role]. Your working directory is /workspace/ where other
steps have contributed and will contribute after you.
```

The assignment carries the narrative and references the handoff:

```
[Narrative context — what previous steps did]. Read the previous
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

## The Agent Prompt

The agent's prompt collapses from 11 XML blocks to 3. The workspace replaces most of what used to be injected into the prompt.

### Current Agent Prompt (7 blocks in user message)

**System message:**
```
You are Scanner, a specialist agent executing as part of a workforce team.
<role>Security scanner who greps for vulnerability patterns...</role>
<mission>Scan codebase for security vulnerabilities...</mission>
<team>Scanner → Analyzer → Reporter</team>
<upstream_outputs>...prior step data...</upstream_outputs>
<instructions>Execute your assigned role. Use your tools...</instructions>
```

**User message:**
```
<user_notes>...context node outputs...</user_notes>
<context>...task description...</context>
<assignment>...designer's assignment...</assignment>
<expected_output>Store: X. Response: Y.</expected_output>
<upstream_artifacts>...XML file manifest...</upstream_artifacts>
<previous_agent_outputs>...prior agents' text...</previous_agent_outputs>
<upstream_step_outputs>...DAG step outputs...</upstream_step_outputs>
```

### File-First Agent Prompt (3 blocks in user message)

**System message:**
```
You are Scanner. [designer's system_prompt]
```

Short. Role and perspective. No mission block, no team roster, no upstream data dump.

**User message:**
```
<previous_step>
  Scanned 500 files, found 7 vulnerabilities. Findings written
  to /workspace/findings/ with one file per vulnerability.
  2 critical, 3 medium, 2 low.
</previous_step>

<assignment>
  Review the vulnerability findings in the workspace. Verify each
  finding in context, assess severity, filter false positives.
  Write your triage to the workspace.
</assignment>

<expected_output>
  Describe the triage results: how many confirmed vs false positives,
  severity breakdown, and where the prioritized findings live in
  the workspace.
</expected_output>
```

Three blocks. That's it.

### What Each Block Does

**`<previous_step>`** — orientation from whoever ran before this agent. Could be a prior agent in the same workforce or the previous step in the workflow. Same tag, same concept. Tells the agent what was done and where to find it in the workspace.

**`<assignment>`** — what to do. Written by the designer, references the handoff and the workspace. The agent's specific task.

**`<expected_output>`** — what to say when done. Shapes the agent's text response as an orientation handoff for whoever comes next. Pattern: what you produced, where it lives, how to use it.

### What the Workspace Replaces

| Old prompt block | What it contained | File-first equivalent |
|---|---|---|
| `<upstream_artifacts>` | XML manifest of store files | Agent runs `ls /workspace/` |
| `<upstream_step_outputs>` | Prior DAG step output data | `<previous_step>` handoff text |
| `<previous_agent_outputs>` | Prior workforce agent output | `<previous_step>` handoff text |
| `<upstream_outputs>` | Prior step data in system prompt | Gone — workspace has the files |
| `<user_notes>` | Context node outputs | User's files in `/workspace/` |
| `<context>` | Task description | Folded into `<assignment>` |
| `<mission>` | Mission brief in system prompt | Gone — assignment has the task |
| `<team>` | Roster listing in system prompt | Gone — agent doesn't need to know the roster |

The agent doesn't need to know the team roster, the mission brief, or an XML manifest of files. It has an assignment, a handoff from the previous step, and a real filesystem. Everything else is noise the workspace handles.

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
Builder: Topology + node content (per-node, detailed descriptions)
         Thinks in scheduling, not data routing.
                        ↓
Designer: Runs in topological order (same order as execution)
          Each step's designer sees:
            - Builder's node description
            - Upstream steps' expected_outputs (already designed)
            - Downstream step descriptions
          Writes per-agent prompts + expected_output.
          expected_output feeds into the next step's design context.
                        ↓
Execute: DAG runs topologically
         Each step gets a container:
           JuiceFS (read-only lower) + OverlayFS (writable upper)
         Agent sees merged /workspace/, writes freely.
         Previous steps' files are there on disk.
         Programs are executable. Data is real.
                        ↓
Step completes → overlay diff filtered (denylist removes junk)
              → clean output persisted to JuiceFS
              → container torn down
              → parallel steps: clean diffs merged (diff3 + Haiku)
              → next batch starts with expanded workspace
```

## Detailed: Topological Design Pass

This is the most significant change to the existing system. Today, the builder and designer both run **per-node independently** — each node's designer has no knowledge of what other nodes' designers wrote. In the file-first model, the design phase runs in **topological order across the entire workflow**, threading context from step to step.

### What Changes

| | Current | File-First |
|--|---------|-----------|
| Builder | Per-node, independent | Per-node, step order, sees previous step's handoff |
| Designer trigger | After each node's builder completes | Builder then designer per step, in step order |
| Designer context | Node's roster + upstream runtime envelopes | Node's roster + previous step's handoff + next step's box text |
| `expected_output` format | "Store: X. Response: Y." (dual, inward-focused) | Orientation handoff for the next step (outward-focused) |
| Cross-step awareness | None — designer doesn't know other steps' designs | Each designer reads the previous step's designed handoff |
| Re-design propagation | Independent — editing step 1's design doesn't affect step 2 | Sequential — editing step 1 re-runs downstream designers |

### Current System: How the Designer Works Today

**Builder** (per-node, unchanged):
```
Receives: dispatch instruction + board state + upstream topology
Produces: agent roster (names, roles, capabilities, dependencies)
          task description, failure mode, plan
Writes:   roster to DB via configure_team()
```

**Designer** (per-node, current):
```
Receives: roster + mission brief + upstream runtime envelopes + plan
          (from build_workforce_designer_input())
Produces: per-agent JSON configs
          { tools, system_prompt, assignment, expected_output }
Writes:   design/{step_id}/agents/{slug}.json to store
```

The designer currently sees upstream via `format_envelopes_as_upstream()` — which reads `completed_envelopes` (runtime step output data). This is fine for execution-time redesign, but at initial design time there are no runtime envelopes. The designer designs blind to what upstream steps will produce.

The `expected_output` field is currently dual-format: "Store: [artifact]. Response: [lean summary]." It tells the agent what to save and what to say. But it's inward-focused — it describes what THIS agent produces, not what the NEXT step needs to know.

### New System: Topological Design Pass

**For each step, in order:**
```
Step 1:
  Builder receives:
    1. Task + board state (existing)
    2. No previous step (first in chain)
  Builder writes: roster, plan, task description

  Designer receives:
    1. Builder's roster + plan (existing)
    2. No previous step handoff (first in chain)
    3. Next step's box text: "Run the scraper against target URLs"
  Designer writes: per-agent configs + step handoff
  Step 1's handoff is now available.

Step 2:
  Builder receives:
    1. Task + board state (existing)
    2. Previous step's handoff (NEW): "Describe the app, entry point,
       dependencies, arguments, return format"
  Builder writes: roster (knows an app is coming in → single executor)

  Designer receives:
    1. Builder's roster + plan (existing)
    2. Previous step's handoff (NEW): same as above
    3. Next step's box text: "Analyze pricing data for trends"
  Designer writes: per-agent configs + step handoff
  Step 2's handoff is now available.

Step 3:
  ...and so on
```

The previous step's handoff helps the builder decide team composition ("an app is coming in" → executor agent vs "raw data is coming in" → analyst pipeline). The designer uses it to write assignments that reference the handoff, and shapes expected_output for whatever comes next.

### What the Designer Receives: New Context Blocks

Currently the designer's instruction template (`react_prompt.md`) has:
```
{{prior_design}}
<dispatch_instruction>...</dispatch_instruction>
<upstream_topology>...</upstream_topology>
```

Rename and add blocks using plain language:

**`<task>`** (was `dispatch_instruction`) — what this step needs to accomplish:
```xml
<task>
  Execute the scraper against target URLs and save results.
  Roster: [Executor] — single agent with shell access.
</task>
```

**`<step_order>`** (was `upstream_topology`) — where this step sits:
```xml
<step_order>
  Build Web Scraper → [THIS STEP] → Analyze Results
</step_order>
```

**`<previous_step>`** (new) — what the step before this one will tell your agents. This step has already been designed. The handoff description tells you what information will be available when your agents start:
```xml
<previous_step name="Build Web Scraper">
  <handoff>
    Describe the application you built: where the entry point lives
    in the workspace, how to install its dependencies, what CLI
    arguments it accepts, and what format it returns.
  </handoff>
</previous_step>
```

**`<next_step>`** (new) — raw text from the user's canvas. This step hasn't been designed yet — it's just what the user wrote in the box. Read it to understand what the next step needs, and shape your expected_output so your agents' handoff gives that step the right orientation:
```xml
<next_step>
  Analyze competitor pricing across Q3 and Q4, compare
  year-over-year trends, flag anomalies above 10%.
</next_step>
```

### How `expected_output` Changes

**Current format** (inward-focused, dual):
```
"Store: JSON array of findings (file path, line, type). Response: finding count and top 3."
```

**New format** (outward-focused, orientation handoff):
```
"Describe what you produced: where the results data lives in the
workspace, the data format, how many records, and any failures.
The next step will analyze this data."
```

The key shift: `expected_output` stops being about what the agent saves/responds. It becomes instructions for how the agent should orient the next step. The designer writes it knowing what the next step needs because it can see the next step's box text.

### Updated Designer Prompt (react_system.md additions)

Add to guidelines:
```
expected_output — orient the next step:
- The expected_output tells the agent what its text response should
  contain so the NEXT step's agents can find their way.
- Read <previous_step> to understand what the step before this one
  will say. Write assignments that reference that handoff.
- Read <next_step> to understand what comes after. Shape
  expected_output to give the next step what it needs.
- Pattern: what you produced, where it lives, how to use it.
- If there is no next step, just confirm completion and location.
```

### Updated Designer Instruction Template (react_prompt.md)

```
{{prior_design}}

<task>
{{task}}
</task>

<step_order>
{{step_order}}
</step_order>

<previous_step>
{{previous_step}}
</previous_step>

<next_step>
{{next_step}}
</next_step>

Review the board_state. For each agent:
- If design_status="pending", write a new config.
- If design_status="designed", read and verify consistency.

When writing expected_output:
- Read <previous_step> to understand what the agents will hear
  from the step before. Reference it in your assignments.
- Read <next_step> to understand what comes after. Shape
  expected_output to orient the next step.
Then call complete_design.
```

### Updated Designer Example

**Designing step 2 ("Run Scraper")** with previous and next step context:

```xml
<previous_step name="Build Web Scraper">
  <handoff>
    Describe the application: entry point in workspace, dependency
    installation, CLI arguments, return format.
  </handoff>
</previous_step>

<next_step>
  Analyze competitor pricing data for trends and anomalies.
</next_step>
```

Note: `<previous_step>` has a name and designed handoff (already built + designed). `<next_step>` is just raw box text from the canvas (not built or designed yet).

Designer writes:
```json
{
  "system_prompt": "You are a test executor. Your working directory
    is /workspace/ where a previous step built an application.",

  "assignment": "A previous step built a web scraper. Its handoff
    describes the entry point, how to install dependencies, and
    how to run it. Follow those instructions to install and execute
    the scraper against these target URLs: [url1, url2, ...].
    Save all results to the workspace.",

  "expected_output": "Report the execution results: how many URLs
    scraped successfully, any failures and why, where the results
    data lives in the workspace, and the data format (fields per
    record). The next step will perform statistical analysis on
    this data.",

  "tools": []
}
```

The assignment references what the upstream handoff contract promised ("its handoff describes the entry point, how to install dependencies, and how to run it"). The expected_output is shaped for what the downstream step needs ("where the results data lives, the data format, fields per record").

### Step-Level Expected Output

Each step's designer also writes a **step-level expected output summary** — a one-liner stored as metadata on the step. This is what downstream designers see in `<upstream_handoff_contracts>`.

For a workforce with 3 agents, the step-level summary represents what the STEP as a whole hands off, not what individual agents produce:

```
Workforce agents:
  Scanner expected_output: "List findings with severity..."
  Analyzer expected_output: "Report prioritized findings..."
  Reporter expected_output: "Confirm report location..."

Step-level expected_output (what downstream sees):
  "Describe the security audit results: total findings by severity,
   where the remediation report lives in the workspace, and key
   recommendations."
```

The designer writes this as part of `complete_design()`:
```json
complete_design({
  "summary": "3-agent pipeline: Scanner → Analyzer → Reporter...",
  "step_expected_output": "Describe the security audit results:
    total findings by severity, where the remediation report lives
    in the workspace, and key recommendations."
})
```

### Re-Design Propagation

When the user edits step 1 (changes roster, modifies node text):
1. Step 1's builder + designer re-run
2. Step 1's handoff may change
3. Pass the new handoff to step 2's designer as `<previous_step>`
4. Step 2's designer checks its existing configs against the new handoff
5. If the handoff meaningfully changed → updates configs, writes new handoff
6. If the handoff is still compatible → skips, keeps existing configs
7. Continue to step 3 with step 2's (possibly updated) handoff

The designer's existing verify-and-skip pattern handles this naturally. No special comparison logic — the designer reads the new previous step handoff, looks at its existing assignments, and decides if they still make sense.

### Implementation Changes

**Backend (`designer_input/`):**
- Add `previous_step_handoff: Option<PreviousStepHandoff>` to `DesignerInput`
- Add `next_step_text: Option<String>` to `DesignerInput`
- New struct `PreviousStepHandoff { step_name, handoff_description }`
- Build from step metadata + stored designer outputs

**Backend (builder dispatch):**
- Change builder dispatch to run in step order (board dispatcher already does this)
- Add `<previous_step>` to builder's instruction context
- After each builder + designer completes, handoff is available for next step's builder

**Backend (`pipeline/designer.rs`):**
- Change designer trigger: builder then designer per step, in step order
- After each designer completes, extract step-level handoff from `complete_design`
- Store as step metadata for next step's builder and designer to read
- Re-design: pass new handoff to next designer, let it verify-and-skip or update

**Config (builder `system.md`):**
- Add `<previous_step>` to builder context
- Add guidance: "Read what the previous step will hand off to understand what's coming in"

**Config (`designer/react_prompt.md`):**
- Rename `dispatch_instruction` → `task`
- Rename `upstream_topology` → `step_order`
- Add `{{previous_step}}` and `{{next_step}}` template vars
- Update instructions to use plain language

**Config (`designer/react_system.md`):**
- Add guidelines for writing outward-focused `expected_output`
- Add examples showing previous step handoff reference in assignments
- Update existing examples to include next step awareness

**`complete_design` tool:**
- Add `step_handoff` field to completion payload
- Persist as step metadata

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
