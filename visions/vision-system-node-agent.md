# System Node Agent — Vision

## What It Is

A single containerized ReAct agent that handles the entire "system layer" for each workflow node. It replaces the current two-phase pipeline (workforce builder → react designer) with one agent that writes JSON config files using `run_command`. No special tools, no DB mutations mid-execution, no S3 store writes. The agent works in its own environment, produces files, and exits. The backend reads the files after.

Each node on the canvas gets its own system node agent running in its own container. The agent's job: read the node's context, decide what agents are needed, configure them, define the execution order, and signal completion. Everything the builder and designer do today — collapsed into one pass.

## Why This Matters

**The current system is two agents pretending to be one.** The builder calls `configure_team` to write roster entries to the DB. The designer reads those roster entries, then writes prompt configs to S3. Both are ReAct loops. Both see the same node context. The designer literally reads what the builder just wrote and reformats it. That's one agent's job split across two execution strategies, two session histories, two per-turn state rebuilds, and a handoff protocol between them.

**DB-driven tool calls are the bottleneck.** The builder can only `configure_team` once per turn because it's a DB transaction that diffs desired state against current state, creates child workflows, links roster agents to child steps, syncs edges, and recomputes execution order. Every mutation round-trips through the pipeline service. The system node agent writes a file. One turn, done.

**The per-turn state machinery is heavy.** Both the builder and designer rebuild their system prompts every turn: L4 board state XML from DB, dispatch status from the task registry, design status enrichment from S3, session history pruning. The system node agent gets a state summary built by reading a few small JSON files from the container's filesystem. No DB queries, no S3 reads, no XML rendering pipeline.

**File-based output is portable.** JSON files on disk can be validated with a schema, diffed between runs, version-controlled, cached, inspected by humans, and consumed by any downstream system. The current designer output is split across S3 objects, DB rows (`agent_designer_outputs`, `task_agent_roster`, `task_mission_briefs`), and in-memory structs (`DesignedAgentPrompt`). The system node agent's output is one directory you can `ls`.

**This pattern is novel.** No existing framework combines all three: an LLM agent writing the config, declarative JSON files on disk as the artifact, and a separate backend reading those files to execute the configured agents. AutoGen AutoBuild generates configs but consumes them in-process. ChatDev has file-based agent configs but they're human-authored. This is the first system where the filesystem is an explicit contract boundary between a design agent and an execution backend.

## The Repository

The system node agent works in a minimal repository:

```
./
├── config.json                       # System identity + contract with downstream
├── topology.json                     # Agent dependency graph
└── agents/                           # One JSON file per agent
    ├── scanner.json
    ├── analyzer.json
    └── reporter.json
```

No seed files, no schema files on disk. Context comes through the system prompt. Output is files.

### `config.json`

The system's identity and contract with downstream. Name is displayed in the UI. Description is what the downstream system agent reads as `<previous_step>`.

```json
{
  "name": "Security Audit",
  "description": "Scans a codebase for security vulnerabilities, prioritizes findings by severity, and produces a remediation report with code examples."
}
```

**The description drives downstream propagation.** When the system node agent calls `complete_system`, the backend diffs `config.json.description` against the previous version. If it changed, downstream system agents re-run. If it didn't, downstream stays untouched.

Name changes don't propagate — renaming the step doesn't affect what it produces.

### `topology.json`

The dependency graph. Pure structure — who runs and in what order. Nothing else.

```json
{
  "agents": {
    "scanner": { "depends_on": [] },
    "analyzer": { "depends_on": ["scanner"] },
    "reporter": { "depends_on": ["analyzer"] }
  }
}
```

The `agents` map keys are slugs. Each slug must have a matching `agents/{slug}.json`. Backend validates this cross-reference at `complete_system`.

**Topology patterns:**

Linear pipeline:
```json
{
  "agents": {
    "researcher": { "depends_on": [] },
    "fact_checker": { "depends_on": ["researcher"] },
    "writer": { "depends_on": ["fact_checker"] }
  }
}
```

Fan-out (parallel producers):
```json
{
  "agents": {
    "web_scraper": { "depends_on": [] },
    "social_monitor": { "depends_on": [] },
    "analyst": { "depends_on": ["web_scraper", "social_monitor"] }
  }
}
```

Fan-in (diamond):
```json
{
  "agents": {
    "builder": { "depends_on": [] },
    "reviewer": { "depends_on": ["builder"] },
    "writer": { "depends_on": ["builder"] },
    "editor": { "depends_on": ["reviewer", "writer"] }
  }
}
```

Single agent (most common):
```json
{
  "agents": {
    "reader": { "depends_on": [] }
  }
}
```

### `agents/{slug}.json`

Each agent file is the merged output of what the builder and designer produce today — roster definition + runtime prompts in one object.

```json
{
  "name": "Scanner",
  "system_prompt": "Security scanner. Find vulnerability patterns in source code.",
  "assignment": "Grep the codebase for OWASP Top 10 vulnerability patterns.",
  "expected_output": "What you found: count, severity breakdown, where you saved the findings.",
  "capabilities": []
}
```

**Fields:**

| Field | Purpose | Notes |
|-------|---------|-------|
| `name` | Display name | Shown in UI, used in logs |
| `system_prompt` | Who the agent is | 30–250 tokens. Role + expertise + scope. No step-by-step reasoning frameworks. |
| `assignment` | What to accomplish | Do not prescribe filenames or save instructions — the runtime agent decides. |
| `expected_output` | What to report back | Tells the runtime agent what its response should contain so the next agent can get started: what was done, what was produced, where it lives. |
| `capabilities` | Non-shell tools | Almost always `[]`. Only add for external APIs, databases, specialized integrations. |

The `capabilities` field replaces the current per-agent tool assignment. Every agent has full shell access (`run_command`), web search, and file I/O natively. Capabilities are only for things the shell can't do.

Config changes for different reasons than topology. Config changes when the step's identity or output changes (triggers propagation). Topology changes when the team structure changes (doesn't trigger propagation). Keeping them separate means agent reshuffles don't touch the file that controls propagation.

## Trigger

The system node agent is dispatched on board submits when the node text changes — same path as the current workforce builder. The board serializer detects a meaningful change to the node, and instead of dispatching a builder + designer pair, it dispatches the system node agent. The serializer's existing change detection, noise filtering, and dispatch logic stays unchanged.

Two propagation paths trigger re-runs:
- **Board submit** — user changes node text on the canvas → serializer dispatches
- **Upstream config change** — a previous step's system agent updated its `config.json` description → backend queues this step's system agent with the new `<previous_step>`

## Message Architecture

The system prompt is stable reference material — how to do the job. The user message is unique per dispatch — what triggered this run and the context. Task text and upstream context appear in the user message, not the system prompt.

### System prompt (rebuilt between turns for `<current_state>` only):

```xml
<role>
You design runtime systems. Your environment is a repository of
configuration files. When you call complete_system, the execution
engine reads your files and runs the agents you configured — in
containers with full shell access and web search.
</role>

<runtime>
At runtime, agents execute in order based on the dependency graph
you define in topology.json. Agents within the same dependency
level run in parallel. All agents share a directory — files and
installed packages persist across agents automatically.

Every agent gets run_command — a shell tool for executing commands.
Through the shell, agents have access to:

  Languages & runtimes: python, pip, node, npm, make, gcc
  Data tools: jq, sqlite3, awk, sort, uniq, cut, tr
  Search: grep, find, xargs, wc, head, tail, diff
  Network: curl, wget
  Files: cat, sed, mkdir, cp, mv, tar, zip
  Version control: git (init, add, commit, diff, log)
  Web search and web browsing are available natively.

Agents create files with heredocs:
  cat > output.md << 'EOF'
  content here
  EOF

Agents install packages that persist to the next agent:
  pip install requests && python scraper.py
  npm install && node index.js

When writing assignments, reference these tools naturally:
  "Grep the codebase for..." / "Use python to..." /
  "curl the API and pipe through jq..." / "Search the web for..."

Do not tell agents HOW to use the shell — they know. Tell them
WHAT to accomplish, not what files to create or where to save them.

The capabilities field on agent configs is only for tools beyond
the shell — external API integrations, database connectors. Most
agents need no capabilities. A shell and a brain is enough.
</runtime>

<schema>
config.json — system identity and downstream contract:
{
  "name": "string — display name for the step",
  "description": "string — what this step produces, conceptually.
    No agent names, no filenames. Only update when what the step
    produces actually changes. Changing the description triggers
    re-design for all downstream steps."
}

topology.json — agent dependency graph:
{
  "agents": {
    "slug": { "depends_on": ["other_slug"] }
  }
}

agents/{slug}.json — per-agent runtime config:
{
  "name": "string — display name",
  "system_prompt": "string — who they are, short and direct (30-250 tokens)",
  "assignment": "string — what to accomplish, not what files to create",
  "expected_output": "string — what the agent should report so the next agent can get started",
  "capabilities": ["string — only non-shell tools, usually empty"]
}
</schema>

<guide>
Match team size to task complexity. A focused task needs 1 agent.
Add agents only when the work decomposes into distinct specialties.
Most tasks are 1-agent tasks.

system_prompt: who the agent is and what they do. Short and direct.
No step-by-step cognitive processes. No numbered reasoning
frameworks. Just: role, expertise, scope.

assignment: what to accomplish. You are not doing the work — the
runtime agent is. Do not prescribe filenames, save locations, or
output structure. The agent is in the environment and will decide
what to produce and where to put it.

expected_output: instructions to the runtime agent about what its
text response should contain. The next agent reads this response
to get started — so tell the agent to report what it did, what it
produced, and where it saved things. File references appear here
naturally because the runtime agent is the one who created them.

If a <previous_step> block is present in your instruction, read it
to understand what the prior step hands off. When upstream already
produces the core artifact, this step should consume it — not
recreate it.

Do NOT reference runtime block names (<previous_agent_outputs>,
<upstream_step_outputs>, <upstream_artifacts>) in agent prompts.
These are injected automatically — agents find them without being
told to look for specific XML tags.

Prompt proportionality — match size to task complexity:
- Pass-through (save, forward, rename): 30-60 tokens
- Simple task (search, format, filter): 60-120 tokens
- Complex task (analyze, compare, multi-step): 120-250 tokens
Do not exceed 250 tokens for any system prompt.
</guide>

<examples>
<example name="single_agent">
<turn>
instruction: "Configure this new step.

<task>
Read the handwriting from the image.
</task>"
current_state: empty

<tool_call name="run_command">
cat > topology.json << 'EOF'
{
  "task": "Read handwriting from the image and transcribe it.",
  "agents": {
    "reader": { "depends_on": [] }
  }
}
EOF
</tool_call>
<tool_call name="run_command">
mkdir -p agents && cat > agents/reader.json << 'EOF'
{
  "name": "Reader",
  "system_prompt": "OCR specialist. Read handwritten text from images and produce clean transcriptions.",
  "assignment": "Read the handwritten text from the image and transcribe it.",
  "expected_output": "What you transcribed, where you saved it.",
  "capabilities": []
}
EOF
</tool_call>
<tool_call name="run_command">
cat > config.json << 'EOF'
{
  "name": "Handwriting Transcription",
  "description": "Reads handwritten text from an uploaded image and produces a clean text transcription."
}
EOF
</tool_call>
<tool_call name="complete_system">
{"summary": "Single reader agent for handwriting OCR.",
 "verify": {"topology_complete": true, "agents_complete": true, "config_accurate": true}}
</tool_call>
</turn>
</example>

<example name="single_agent_with_upstream">
<turn>
instruction: "Configure this new step.

<task>
Summarize the research into a blog post.
</task>

<previous_step>
Ranked list of 5 trending AI papers with title, authors, link, and summary.
</previous_step>"
current_state: empty

<tool_call name="run_command">
cat > topology.json << 'EOF'
{
  "task": "Summarize the research papers into an engaging blog post.",
  "agents": {
    "writer": { "depends_on": [] }
  }
}
EOF
</tool_call>
<tool_call name="run_command">
mkdir -p agents && cat > agents/writer.json << 'EOF'
{
  "name": "Writer",
  "system_prompt": "Tech blogger. Turn academic research into accessible, engaging posts.",
  "assignment": "Read the ranked research papers from the previous step. Write a blog post covering the top findings, why they matter, and what comes next.",
  "expected_output": "Blog post title, word count, and where you saved it.",
  "capabilities": []
}
EOF
</tool_call>
<tool_call name="run_command">
cat > config.json << 'EOF'
{
  "name": "Research Blog Post",
  "description": "An engaging blog post summarizing trending AI research papers, covering key findings, significance, and future directions."
}
EOF
</tool_call>
<tool_call name="complete_system">
{"summary": "Single writer agent to turn upstream research into a blog post.",
 "verify": {"topology_complete": true, "agents_complete": true, "config_accurate": true}}
</tool_call>
</turn>
</example>

<example name="pipeline">
<turn>
instruction: "Configure this new step.

<task>
Scan codebase for security vulnerabilities and produce a remediation report with prioritized fixes.
</task>"
current_state: empty

<tool_call name="run_command">
cat > topology.json << 'EOF'
{
  "task": "Scan codebase for security vulnerabilities, prioritize findings by severity, and produce a remediation report with fix recommendations.",
  "agents": {
    "scanner": { "depends_on": [] },
    "analyzer": { "depends_on": ["scanner"] },
    "reporter": { "depends_on": ["analyzer"] }
  }
}
EOF
</tool_call>
<tool_call name="run_command">
mkdir -p agents && cat > agents/scanner.json << 'EOF'
{
  "name": "Scanner",
  "system_prompt": "Security scanner. Find vulnerability patterns in source code.",
  "assignment": "Grep the codebase for OWASP Top 10 vulnerability patterns.",
  "expected_output": "What you found: count, severity breakdown, where you saved the findings.",
  "capabilities": []
}
EOF
</tool_call>
<tool_call name="run_command">
cat > agents/analyzer.json << 'EOF'
{
  "name": "Analyzer",
  "system_prompt": "Security analyst. Prioritize and verify findings.",
  "assignment": "Read the findings from the previous agent. Verify each finding, rate severity, filter false positives.",
  "expected_output": "Triage results: confirmed vs false positives, severity breakdown, where you saved the prioritized list.",
  "capabilities": []
}
EOF
</tool_call>
<tool_call name="run_command">
cat > agents/reporter.json << 'EOF'
{
  "name": "Reporter",
  "system_prompt": "Technical writer. Produce remediation reports.",
  "assignment": "Read the prioritized findings from the previous agent. Write a remediation report with fix examples.",
  "expected_output": "Report written. Where you saved it, section count, key recommendations.",
  "capabilities": []
}
EOF
</tool_call>
<tool_call name="run_command">
cat > config.json << 'EOF'
{
  "name": "Security Audit",
  "description": "Scans a codebase for security vulnerabilities, prioritizes findings by severity, and produces a remediation report with code examples."
}
EOF
</tool_call>
<tool_call name="complete_system">
{"summary": "3-agent pipeline: Scanner → Analyzer → Reporter for security vulnerability scanning.",
 "verify": {"topology_complete": true, "agents_complete": true, "config_accurate": true}}
</tool_call>
</turn>
</example>

<example name="no_change">
<turn>
instruction: "<prior_work>
1. Single researcher agent for competitor pricing.
</prior_work>

The user updated this step.

<change>
Before: \"Research competitor pricing.\"
After: \"Research competitor pricing.

Great job so far!\"
</change>"
current_state: topology has 1 agent "researcher" (configured), config present

<tool_call name="complete_system">
{"summary": "No configuration change needed — user added a comment.",
 "verify": {"topology_complete": true, "agents_complete": true, "config_accurate": true}}
</tool_call>
</turn>
</example>

<example name="update_existing">
<turn>
instruction: "<prior_work>
1. 3-agent pipeline: Scanner → Analyzer → Reporter.
</prior_work>

The user updated this step.

<change>
Before: \"Scan codebase for security vulnerabilities\"
After: \"Scan codebase for security vulnerabilities. Focus on OWASP Top 10. Include remediation steps with code examples.\"
</change>"
current_state: topology has scanner (configured), analyzer (configured), reporter (configured), config present

(Scanner and Analyzer configs still match the updated task — OWASP focus
was already implicit. Reporter needs updated assignment for code examples.)

<tool_call name="run_command">
cat > agents/reporter.json << 'EOF'
{
  "name": "Reporter",
  "system_prompt": "Technical writer. Produce remediation reports with working code fixes.",
  "assignment": "Read the prioritized findings from the previous agent. Write a remediation report. For each finding, include a concrete code example showing the fix.",
  "expected_output": "Report written. Where you saved it, finding count, code examples per finding.",
  "capabilities": []
}
EOF
</tool_call>
<tool_call name="complete_system">
{"summary": "Updated Reporter to include code fix examples per finding. Scanner and Analyzer unchanged.",
 "verify": {"topology_complete": true, "agents_complete": true, "config_accurate": true}}
</tool_call>
</turn>
</example>

<example name="rerun_verify_and_update">
<turn>
instruction: "<prior_work>
1. Configured 2-agent pipeline: Researcher → Writer.
</prior_work>

The user updated this step.

<change>
Before: \"Research competitors and write a summary report\"
After: \"Research competitors, verify claims, and write a summary report.\"
</change>

<previous_step>
Clone repository containing the target company's public docs.
</previous_step>"
current_state: topology has researcher (configured), writer (configured), config present

<tool_call name="run_command">
cat agents/researcher.json
</tool_call>
(reads: assignment says "Search the web for competitor data. Save as research_notes.md." — still valid, research step unchanged)

<tool_call name="run_command">
cat agents/writer.json
</tool_call>
(reads: assignment says "Read research_notes.md. Write a summary report." — stale, no fact-checking step)

Need to add a FactChecker between Researcher and Writer. Update topology,
add the new agent, update Writer to consume verified data.

<tool_call name="run_command">
cat > topology.json << 'EOF'
{
  "task": "Research competitors, verify claims, and write a summary report.",
  "agents": {
    "researcher": { "depends_on": [] },
    "fact_checker": { "depends_on": ["researcher"] },
    "writer": { "depends_on": ["fact_checker"] }
  }
}
EOF
</tool_call>
<tool_call name="run_command">
cat > agents/fact_checker.json << 'EOF'
{
  "name": "FactChecker",
  "system_prompt": "Fact verification specialist. Check claims against authoritative sources.",
  "assignment": "Read the research notes from the previous agent. Verify each claim against authoritative sources. Annotate findings with verification status.",
  "expected_output": "Verification results: claims checked, confirmed vs unverified, where you saved the annotated findings.",
  "capabilities": []
}
EOF
</tool_call>
<tool_call name="run_command">
cat > agents/writer.json << 'EOF'
{
  "name": "Writer",
  "system_prompt": "Report writer. Produce structured summary reports from verified research.",
  "assignment": "Read the verified research from the previous agent. Write a summary report noting which claims are verified.",
  "expected_output": "Report written. Where you saved it, section count, verified claim count.",
  "capabilities": []
}
EOF
</tool_call>
<tool_call name="run_command">
cat > config.json << 'EOF'
{
  "name": "Competitor Research",
  "description": "Researches competitors, fact-checks all claims against authoritative sources, and produces a summary report with verification status per claim."
}
EOF
</tool_call>
<tool_call name="complete_system">
{"summary": "Added FactChecker between Researcher and Writer. Updated Writer to consume verified data. Updated config description — output now includes claim verification.",
 "verify": {"topology_complete": true, "agents_complete": true, "config_accurate": true}}
</tool_call>
</turn>
</example>

<example name="rerun_no_description_change">
<turn>
instruction: "<prior_work>
1. Configured 2-agent pipeline: Researcher → Writer.
2. Added FactChecker between Researcher and Writer. Updated config description.
</prior_work>

The user updated this step.

<change>
Before: \"Research competitors, verify claims, and write a summary report.\"
After: \"Research competitors, verify claims, and write a summary report. Use Reuters and Bloomberg as primary sources for verification.\"
</change>"
current_state: topology has researcher (configured), fact_checker (configured), writer (configured), config present

User added source guidance. Team structure unchanged — just refine
the FactChecker's assignment to mention the preferred sources.

<tool_call name="run_command">
cat > agents/fact_checker.json << 'EOF'
{
  "name": "FactChecker",
  "system_prompt": "Fact verification specialist. Check claims against authoritative sources.",
  "assignment": "Read the research notes from the previous agent. Verify each claim, prioritizing Reuters and Bloomberg as primary sources. Annotate findings with verification status and source.",
  "expected_output": "Verification results: claims checked, confirmed vs unverified, sources used, where you saved the results.",
  "capabilities": []
}
EOF
</tool_call>
<tool_call name="complete_system">
{"summary": "Updated FactChecker to prioritize Reuters and Bloomberg. Team and config unchanged.",
 "verify": {"topology_complete": true, "agents_complete": true, "config_accurate": true}}
</tool_call>
</turn>
</example>
</examples>

<completion>
When done, call complete_system with a summary of what you configured.
Write all files before calling complete_system. If a write is rejected,
fix the error and write again. complete_system checks that all pieces
are in place — if something is missing, it tells you what.
</completion>
```

### Write-time validation:

When the agent writes to `topology.json` or `agents/*.json` via `run_command`, the backend intercepts the write, parses the JSON, validates required fields, and either accepts or rejects it. Bad files never hit disk. `config.json` is validated for required fields (`name`, `description`) at write time like the other JSON files.

Write-time validation checks structure only — valid JSON, required fields present. It does NOT cross-reference topology slugs against agent files. The agent may write topology.json before writing agent files, or vice versa. Cross-reference happens at `complete_system`.

**Successful write:**
```
ok — agents/scanner.json written (5 fields valid)
```

**Rejected write (missing field):**
```
Error: agents/scanner.json — missing required field "expected_output"
File was not written.
```

**Rejected write (bad JSON):**
```
Error: agents/scanner.json — invalid JSON: unexpected token at line 4, column 12
File was not written.
```

The agent sees the error immediately in the tool response and fixes it on the same turn.

### Dynamic section (rebuilt every turn):

The backend reads the filesystem and builds a summary of what exists.

```xml
<current_state refresh="every turn — always reflects the current filesystem">
  <topology task="Scan codebase for security vulnerabilities">
    <agent slug="scanner" depends_on="" status="configured" />
    <agent slug="analyzer" depends_on="scanner" status="configured" />
    <agent slug="reporter" depends_on="analyzer" status="missing" />
  </topology>
  <config name="Security Audit" status="configured" />
</current_state>
```

**Status values:**

| Status | Meaning |
|--------|---------|
| `configured` | Valid JSON on disk, slug matches topology |
| `missing` | Listed in topology.json but no agent file written yet |

On first run (empty filesystem):
```xml
<current_state refresh="every turn — always reflects the current filesystem">
  <topology status="empty" />
  <config status="missing" />
</current_state>
```

### User message (unique per dispatch):

The user message contains everything specific to this dispatch: what triggered the run, the task context, upstream context, and session history. Built by the backend from the board serializer's instruction + `build_pruned_instruction` session history.

**First run (new node):**
```
Configure this new step.

<task>
Scan codebase for security vulnerabilities and produce a remediation report.
</task>

<previous_step>
Clone repository containing the target company's source code.
</previous_step>
```

**Re-run (user changed node text):**
```
<prior_work>
1. Configured 3-agent pipeline: Scanner → Analyzer → Reporter.
</prior_work>

The user updated this step.

<change>
Before: "Scan codebase for security vulnerabilities"
After: "Scan codebase for security vulnerabilities. Focus on OWASP Top 10."
</change>
```

**Re-run (upstream description changed):**
```
<prior_work>
1. Configured 3-agent pipeline: Scanner → Analyzer → Reporter.
</prior_work>

The upstream step changed what it produces.

<previous_step>
Clone repository and run initial static analysis. Produces a baseline findings report.
</previous_step>
```

**Re-run (no meaningful change):**
```
<prior_work>
1. Configured 3-agent pipeline: Scanner → Analyzer → Reporter.
</prior_work>

The user updated this step.

<change>
Before: "Scan codebase for security vulnerabilities"
After: "Scan codebase for security vulnerabilities

Great job so far!"
</change>
```

Session history: up to 20 messages loaded, pruned to last 3 assistant summaries via `build_pruned_instruction` (same pattern as the current builder). The `<prior_work>` block is prepended to the instruction. The summaries come from `complete_system`'s `summary` parameter.

## Completion Protocol

The system node agent has one special tool: `complete_system`. It signals completion, names the system, captures a summary for session history, and requires the agent to verify its own work.

```json
{
  "name": "complete_system",
  "parameters": {
    "summary": {
      "type": "string",
      "description": "What you configured and key decisions (1-3 sentences)."
    },
    "verify": {
      "type": "object",
      "properties": {
        "topology_complete": {
          "type": "boolean",
          "description": "The topology defines all agents and their dependencies are correct."
        },
        "agents_complete": {
          "type": "boolean",
          "description": "Every agent has a valid config with system_prompt, assignment, and expected_output."
        },
        "config_accurate": {
          "type": "boolean",
          "description": "config.json name and description accurately reflect this system."
        }
      }
    }
  }
}
```

**`summary`** — the agent's own notes about what it did and why. Becomes the `<prior_work>` entry next time — session continuity, not downstream communication.

**`verify`** — the agent signs off on its work. Each boolean is the agent saying "I verify this is correct." The backend validates that the files back up the claims:

- `topology_complete: true` → topology.json exists, every slug has a matching agent file, no orphans
- `agents_complete: true` → every agent file has all required fields, valid JSON
- `config_accurate: true` → config.json exists with `name` and `description`

If the agent marks `true` but the files don't match, the tool rejects with specific errors. The agent can't just check the box — the system verifies the claim.

If the agent marks `false` on any verify field, it's telling the system it knows something isn't done. The summary should explain why.

**Example calls:**

First run:
```json
{
  "system_name": "Security Audit",
  "summary": "Configured 3-agent pipeline: Scanner → Analyzer → Reporter.",
  "verify": {
    "topology_complete": true,
    "agents_complete": true,
    "config_accurate": true
  }
}
```

Re-run, minor update:
```json
{
  "system_name": "Security Audit",
  "summary": "Updated Reporter for code fix examples. Scanner and Analyzer unchanged.",
  "verify": {
    "topology_complete": true,
    "agents_complete": true,
    "config_accurate": true
  }
}
```

No change needed:
```json
{
  "system_name": "Competitor Research",
  "summary": "No configuration change needed — user added a comment.",
  "verify": {
    "topology_complete": true,
    "agents_complete": true,
    "config_accurate": true
  }
}
```

**Backend behavior on `complete_system`:**

1. Validate each `verify` claim against the filesystem:
   - `topology_complete` → topology.json exists, all slugs have matching agent files, no orphans
   - `agents_complete` → all agent files valid JSON with required fields
   - `config_accurate` → config.json exists with `name` and `description`
2. If any claim fails → return structured error, agent continues
3. If all pass → write `name` to step row, persist summary to session history, diff `config.json.description` against previous run, stop the execution loop

**Success response:**
```json
{
  "status": "ok",
  "description_changed": true
}
```

**Error response (verify claim doesn't match files):**
```json
{
  "status": "verification_failed",
  "errors": [
    { "verify": "topology_complete", "error": "agent 'editor' in topology.json has no matching agents/editor.json" },
    { "verify": "config_accurate", "error": "config.json does not exist" }
  ]
}
```

The agent fixes the issue and calls `complete_system` again.

**Downstream propagation:**

- `description_changed: true` → the next step's system agent re-runs with the updated `<previous_step>`
- `description_changed: false` → downstream steps are left alone

The backend diffs `config.json.description` against its previous version. The agent doesn't decide whether its changes matter — it writes an accurate description, and the system handles propagation by diffing a single field.

## What Gets Replaced

| Current | System Node Agent |
|---------|-------------------|
| `DispatchStrategy` (workforce builder) | One agent, `run_command` |
| `ReactDesignerStrategy` (agent designer) | Same agent, same run |
| `configure_team` tool + DB mutations | `echo '...' > topology.json` |
| `write_file` / `read_file` / `complete_design` designer tools | `run_command` |
| `complete_task` tool with passdown | `complete_system` with name, summary, verify checklist |
| `TaskMissionBriefRow` in DB | `task` field in `topology.json` |
| `TaskAgentRosterRow` in DB | `agents/{slug}.json` files |
| `AgentDesignerOutputRow` in DB | Same agent files |
| `DesignedAgentPrompt` struct | Parsed from agent files post-exit |
| S3 config caching (`design/{step_id}/agents/`) | Workspace filesystem |
| L4 board state XML (DB + S3 enrichment) | `<current_state>` from filesystem read |
| `parse_store_configs()` pipeline | Read JSON files from filesystem |
| `enforce_edge_routing()` from DB edges | `depends_on` in `topology.json` |
| Design status enrichment (`enrich_design_status`) | File existence check |
| `step_handoff` / `designer_handoff` DB field | `config.json` description |
| `output_changed` boolean signal | `config.json` description diff |
| `build_pruned_instruction` with session history | Same pattern, unchanged |

## What Stays The Same

- **Phase 0** — canvas drawing → topology creation (unchanged)
- **Execution engine** — the ReAct loop, `run_command`, container lifecycle
- **`compute_execution_levels`** — Kahn's algorithm on `depends_on` (same input shape)
- **Workforce agent execution** — agents run with designed prompts (same `DesignedAgentPrompt` shape, just sourced from files instead of S3/DB)
- **Session history** — persistent across dispatches, pruned to last 3 summaries
- **Per-turn system prompt rebuild** — same pattern, different data source (filesystem vs DB+S3)
- **Container + storage** — JuiceFS mount, OverlayFS, same infrastructure

## Backend Integration

After the system node agent calls `complete_system` successfully, the backend runs three phases: **sync**, **execute**, **propagate**.

### Phase: Sync (files → DB)

The backend diffs the agent's files against the current DB state and applies minimal mutations. Same diff pattern as `configure_team` in `configure.rs`, but driven by file contents instead of tool input. Files are the source of truth; the DB is a projection for the frontend and execution pipeline.

**`topology.json` sync:**
- Diff `agents` map against `TaskAgentRosterRow` entries → create new, update changed, remove missing
- Diff `depends_on` against child workflow edges → add new edges, remove stale ones
- Update `TaskMissionBriefRow.task_description` from `task` field
- Recompute execution order from dependency graph

**`agents/*.json` sync:**
- Update roster `role_description` (from `name` or derived from `system_prompt`)
- Sync `capabilities` on each roster entry
- Store designed prompt fields (`system_prompt`, `assignment`, `expected_output`) — either in `AgentDesignerOutputRow` or a new lightweight table

**`config.json` sync:**
- Write `name` to step row display name
- Update `designer_handoff` on the step row from `description`
- Diff `description` against previous version → set `description_changed` flag

After sync, the DB reflects the files exactly. The frontend sees updated roster agents, edges, and step state without knowing the source changed.

### Phase: Execute

1. **Read** `agents/*.json` → parse into `DesignedAgentPrompt` structs
2. **Compute levels** → `compute_execution_levels` from `depends_on` (existing algorithm)
3. **Execute** → same `execute_agent_levels` pipeline, same `WorkforceAgentStrategy`

The `DesignedAgentPrompt` mapping:

```
agents/{slug}.json              → DesignedAgentPrompt
─────────────────────────────────────────────────────
name                            → agent_name
system_prompt                   → system_prompt
assignment                      → assignment
expected_output                 → expected_output
capabilities                    → tools
topology.agents[slug].depends_on → receives_from
(computed from topology order)  → execution_order
(generated during sync)         → agent_roster_entry_id
```

### Phase: Propagate (description cascade)

If `config.json.description` changed, the backend walks the downstream DAG and re-runs system node agents in topological order. Each downstream agent receives the updated `<previous_step>` from its upstream config description.

```
Step A (description changed)
  → dispatch Step B's system agent with A's new description as <previous_step>
    → B calls complete_system
      → sync B's files to DB
      → diff B's description against previous
        → if changed → dispatch Step C's system agent with B's new description
          → C calls complete_system
            → C's description unchanged → cascade stops
        → if unchanged → stop, don't touch C
```

The cascade walks one step at a time, sequentially through the DAG. Each step either propagates (description changed) or absorbs (description unchanged). A step that absorbs the change stops the cascade — downstream steps keep their existing configs.

In practice, most changes are absorbed within 1-2 steps. A task refinement like "add citations" changes Step A's description, Step B adjusts its agents to handle citations, but Step B's output ("a summary report") doesn't change — cascade stops at B.

## Migration Path

This is not a rewrite. It's a replacement of two execution strategies with one, sourcing config from files instead of DB+S3.

**Phase 1: System node agent strategy**
- New `SystemNodeStrategy` implementing `ExecutionStrategy`
- System prompt from `config/archetype/workforce/system_agent/`
- `run_command` as the only tool (plus `complete_system`)
- `rebuild_system_prompt` reads filesystem for `<current_state>`
- Workspace file reader + JSON validator

**Phase 2: Backend consumer**
- Read `config.json` + `topology.json` + `agents/*.json` after completion
- Map to `DesignedAgentPrompt` structs
- Feed into existing `execute_agent_levels` pipeline
- Diff config description for downstream propagation

**Phase 3: Remove old machinery**
- Delete `ReactDesignerStrategy`
- Delete designer tools (`write_file`, `read_file`, `complete_design`)
- Delete `AgentDesignerStrategy` (one-shot fallback)
- Delete `parse_store_configs`, `enrich_design_status`
- Delete designer-specific S3 paths
- Simplify board state (no more L4 design status enrichment)
- Consider removing `agent_designer_runs` / `agent_designer_outputs` DB tables
