# System Store — Vision

## What It Is

Every workflow becomes a **system** — a self-contained unit with its own filesystem and metadata index. The store serves two purposes: it's the **designer's workspace** for building agent prompts iteratively, and it's the **shared state** where running agents write their output for downstream steps to discover.

The user's project is their project — a GitHub repo, a folder of documents, whatever they're working on. The workflow operates on it. The system store lives alongside it as `.system/`.

## Why This Matters

Three problems converge here:

**1. The designer is one-shot.** Today, the Agent Designer generates system prompts for every agent in a single LLM call. For 3 agents this works. For 10+ agents, prompt quality degrades because the LLM can't hold all inter-agent coordination logic in one pass. It can't self-correct — if agent 5's design conflicts with agent 2's, it has no way to go back and fix agent 2.

**2. Data between steps is blind.** Data flows as untyped JSON in `StepExecutionEnvelope.data`. Tools are static — `read_file` says "Read the contents of a file" with no knowledge of what files exist or what they contain. If a research step writes findings, the report step has no way to discover they exist, what they cover, or where to find them.

**3. Agent output is invisible to other agents.** When steps pass JSON blobs, downstream agents can't discover what upstream agents produced. You need files with descriptions, types, and discoverability — all visible to downstream agents through a shared namespace.

The store fixes all three. The designer writes prompts to files it can read back and revise. Running agents write artifacts with metadata. A runtime manifest automatically presents upstream artifacts to downstream agents — the designer shapes what agents produce and consume, the system handles discovery.

## The Unified Namespace

One filesystem. One `read_file`. One `write_file`. The `.system/` namespace is managed by the store service backed by S3 (MinIO in dev, real S3 in production). Agents see the whole tree:

```
my-project/                         # THE GOAL — the user's actual project
├── src/                            # project code — the deliverable
├── docs/                           # project docs — the deliverable
├── assets/                         # project assets
└── .system/                        # SUPPORTING — exists to help agents do their job
    ├── design/                     # agent configs (auto-scoped per step)
    │   ├── {step_id_a}/agents/     # Step A's agent configs
    │   └── {step_id_b}/agents/     # Step B's agent configs
    ├── artifacts/                  # working files (global namespace, shared)
    └── refs/                       # reference material
```

**Auto-scoping**: The `design/` directory is transparently scoped by step ID. The designer writes `design/agents/scanner.json` — the runtime stores it as `design/{step_id}/agents/scanner.json`. The designer never sees the prefix. The executor reads from the same scoped path. This prevents collisions when multiple workforce steps in the same workflow have agents with the same name.

`artifacts/` is **not** scoped — it's a shared global namespace. Agents in any step can read artifacts from any other step. The designer picks paths that make sense for the task and avoids collisions by seeing what files already exist.

For workflows that aren't operating on a code repo — like a report-generation pipeline — `.system/` itself is the project. The generated documents and data all live there.

### System Files vs Project Files

Every agent has an explicit understanding of this distinction:

- **Project files** (`src/`, `docs/`, etc.) — **the goal**. This is what you're building. The actual deliverable. The code, the pages, the assets the user cares about.
- **System files** (`.system/`) — **supporting material**. These exist to help agents complete the goal. Drafts, research notes, intermediate data, reference docs. They're not the deliverable — they're the scaffolding.

The designer reinforces this in every agent's prompt. The naming itself teaches the concept — `.system/` signals "this is infrastructure, not output."

The designer orchestrates the **transfer** — early agents produce working documents in `.system/`, later agents refine them, and the final agent applies the deliverable to the real project.

```
Agent 1 (Researcher):  writes research notes to .system/artifacts/      (supporting)
Agent 2 (Drafter):     reviews Researcher's artifacts
                       writes draft to .system/artifacts/                (supporting)
Agent 3 (Reviewer):    reviews Drafter's artifacts
                       writes final version to .system/artifacts/        (supporting)
Agent 4 (Publisher):   reviews Reviewer's artifacts
                       writes src/pages/terms-of-use.md                  (THE GOAL)
```

Same `write_file` tool for both. The path prefix tells you which is which — `.system/` is scaffolding, project paths are the deliverable. The designer's assignment for Agent 4: "Review the Reviewer's final Terms of Use artifacts and publish to `src/pages/terms-of-use.md`." The runtime manifest shows exactly which files the Reviewer produced.

### Implicit Tools

Every agent always has `read_file` and `write_file` available — these are implicit, like web search and X search. The designer doesn't need to include them in the tools list. When the designer assigns `tools: ["content_search"]`, the agent actually gets `read_file` + `write_file` + `content_search` + web search + X search.

This means agents with `tools: []` can still write system files and read upstream artifacts. The tools list only contains *additional* capabilities beyond the baseline.

### Storage and Container Access

All file operations go through the `SystemStore` service, which delegates to S3 (MinIO in dev, real S3 in production). See [Storage Architecture](#storage-architecture) for details.

When agents run in Docker containers, the store pre-syncs relevant files from S3 into the container's working directory before execution. Shell commands (`ffmpeg`, etc.) operate on the local working directory. Files written during execution are synced back to S3 on completion.

Postgres tracks **metadata** as a sidecar index — descriptions, tags, who produced the file, media type. The store service is a thin layer: write via the backend, update the metadata row.

```
write_file("findings.md", content, description: "Competitive pricing research notes")
  1. Store service resolves path → .system/artifacts/findings.md
  2. Writes to S3 (MinIO in dev, real S3 in prod)
  3. INSERT/UPDATE system_files SET
       path = 'artifacts/findings.md',
       media_type = 'text/markdown',
       description = 'Competitive pricing research notes',
       produced_by = step_id,
       produced_by_agent = agent_name,
       size_bytes = len(content),
       version = version + 1
  4. File immediately visible in workforce manifest + downstream on completion
```

## Storage Architecture

Postgres tracks metadata. File content lives in S3-compatible object storage. One code path — same `S3Backend` in dev and prod, different endpoint.

### The Backend

```rust
struct S3Backend {
    client: aws_sdk_s3::Client,
    bucket: String,
}

impl S3Backend {
    async fn read(&self, key: &str) -> Result<Vec<u8>>;
    async fn write(&self, key: &str, bytes: &[u8]) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn list(&self, prefix: &str) -> Result<Vec<String>>;
    async fn exists(&self, key: &str) -> Result<bool>;
}
```

**Development**: MinIO in docker-compose, `S3_ENDPOINT=http://minio:9000`. Same S3 API, runs locally.

**Production**: Real S3 (or R2/GCS). Swap the endpoint and credentials. No code change.

```yaml
# docker-compose.yml
minio:
  image: minio/minio
  command: server /data
  ports:
    - "9000:9000"
    - "9001:9001"  # console
  environment:
    MINIO_ROOT_USER: minioadmin
    MINIO_ROOT_PASSWORD: minioadmin
```

### The Namespace

```
s3://bucket/workflows/{workflow_id}/system/
  ├── design/{step_id}/agents/            # agent configs (JSON files)
  ├── artifacts/                          # working files
  └── refs/                              # reference material
```

The `SystemStore` service wraps the `S3Backend` and manages metadata in Postgres. Everything above this layer doesn't know or care about S3.

### Metadata Index (Postgres)

```
system_files
├── system_id        uuid
├── path             text                   # relative to .system/
├── media_type       text
├── description      text                   # agent-provided on write
├── tags             text[]
├── produced_by      uuid (step that wrote)
├── version          int
├── size_bytes       int
├── created_at       timestamptz
└── updated_at       timestamptz

system_snapshots
├── id               uuid
├── system_id        uuid
├── label            text
├── file_versions    jsonb                  # path → version mapping
└── created_at       timestamptz

system_mounts
├── system_id        uuid
├── target_id        uuid
├── mount_point      text
└── access           text (read/read_write)
```

Postgres is a **sidecar index**, not the source of truth for file content. S3 holds the actual bytes. Postgres tracks who produced each file, its description, tags, and media type.

### The Store Service

A thin layer over S3 + metadata:

```rust
SystemStore {
    s3: Arc<S3Backend>,

    // File ops — read/write via S3, update metadata
    read(path) -> bytes
    write(path, bytes, meta) -> writes to S3 + metadata row
    edit(path, find, replace) -> edits via S3 + bumps version
    list(prefix) -> entries from metadata index

    // Artifact flow — runtime manifest for downstream agents
    artifacts_for_step(step_id, run_id) -> files written by upstream steps
    artifacts_for_workforce(step_id, run_id) -> files written by any agent in this step

    // Versioning — lightweight bookmarks
    snapshot(label) -> snapshot_id
    restore(snapshot_id) -> reverts files to snapshot versions

    // Cross-system — mounted stores
    mount(target_system_id, mount_point, access)
}
```

### Store Lifecycle

**Creation**: When a workflow is created, the namespace `workflows/{workflow_id}/system/` is initialized as an S3 prefix. Empty until the first builder → designer cycle or user upload.

**Cleanup on node deletion**: When a DAG step is removed, its `design/{step_id}/` prefix is deleted (files removed from backend, metadata rows deleted). Artifacts the step's agents produced remain in `artifacts/` — downstream steps may still reference them. The `produced_by` column lets the user find and clean up orphaned artifacts.

**Cleanup on workflow deletion**: The entire namespace is removed. All `system_files` metadata rows for that `system_id` are deleted. All snapshots and mounts for that system are deleted.

**Re-execution**: Files persist across runs. Artifacts from previous executions remain unless explicitly overwritten. Agents writing to the same path create a new version. Input-hash caching can skip steps whose inputs haven't changed.

### Snapshots

```
Before execution:  snapshot("pre-execution")
  → records current version of every file (path → version in jsonb)
  → copies current files to snapshot directory

Something breaks:  restore("pre-execution")
  → reverts all files to snapshot state
```

Lightweight bookmarks. No git, no diff algorithm.

## The Designer — From One-Shot to ReAct Builder

### The Current System

The workforce builder and designer are a two-stage pipeline:

```
Builder (ReAct, 12 rounds, has tools)
  → configure_team: sets roster, dependencies, capabilities
  → complete_task: writes plan + summary as Passdown
  → stored in DB
              ↓
Designer (one-shot, 1 round, no tools)
  → reads: roster, plan, capabilities, upstream context
  → generates: { system_prompt, assignment, tools } per agent in ONE JSON blob
  → parsed, saved to designer_outputs table, used from memory
              ↓
Agents execute with designed prompts
```

The builder is already a ReAct agent with 12 rounds and tools. It can self-correct. The designer is the odd one out — a single LLM call that generates all prompts at once, can't read its own output, can't revise.

The builder writes tight role descriptions ("1-2 sentences defining WHO the agent is") and a structured plan that's the only context the designer sees. The plan is the handoff — "if it is not in the plan, it does not exist."

### What Changes

The designer becomes a ReAct agent with store access. Instead of generating all prompts in one JSON blob, it writes one agent's prompt at a time to the store, reads back prior prompts for consistency, and revises as needed.

```
Builder (unchanged — ReAct, 12 rounds, has tools)
  → configure_team: sets roster, dependencies, capabilities
  → complete_task: writes plan + summary as Passdown
  → stored in DB
              ↓
Designer (NEW — ReAct, multi-turn, has store tools)
  → reads roster + plan from DB
  → writes design/agents/researcher.json to store
  → reads it back while designing next agent
  → writes design/agents/writer.json to store
  → reviews all, edits any inconsistencies
  → done
              ↓
Executor reads prompts from store (not from memory)
```

### What The Designer Writes

The builder already owns the roster (names, roles, dependencies, capabilities, execution order). The designer's job is: given this roster, write good prompts. Four fields per agent:

```json
{
  "tools": ["file_read", "content_search"],
  "system_prompt": "You are a senior security scanner specializing in OWASP Top 10 vulnerability detection...",
  "assignment": "Scan the codebase for security vulnerabilities. Write detailed findings to the store. Respond with a compact findings list.",
  "expected_output": "Numbered list of findings. Each: file:line, vuln type, severity estimate. Full details in store."
}
```

Same shape as the current `DesignedAgentPrompt` minus the fields the builder already set (roster_entry_id, name, execution_order, receives_from), plus `expected_output`.

**`expected_output`** is the key addition. It serves three purposes:
1. **Pipeline coherence** — the designer reads it back when writing the next agent. If Agent A's expected output doesn't match what Agent B's assignment expects, the designer catches it.
2. **User visibility** — the config panel shows it. The user sees at a glance what each agent produces without reading the full system prompt.
3. **Self-documentation** — the designer uses it as a contract with itself across turns.

### The Designer's Prompt

The designer's context is split between the system prompt (rebuilt each turn) and the instruction (static per run). The builder already processed the board state and distilled it into the plan — the designer trusts the builder's interpretation and does not see raw board state.

#### System Prompt

```xml
<role>
You are the agent designer for "{{node_name}}". The builder
configured a team — names, roles, capabilities, dependencies.
Your job: write each agent's runtime prompt.

You think in cognitive patterns — how each agent reasons,
what it notices, how its output serves the next agent's input.
The builder decided WHO. You decide HOW they think.
</role>

{{roster_status}}

<tools>
write_file(path, content)
  Write agent config to design/agents/{slug}.json
  Content: { tools, system_prompt, assignment, expected_output }

read_file(path)
  Read a config you already wrote. Use this to verify
  the format chain connects across agents.

complete_design(summary)
  Signal completion. Summary: topology shape, format chain,
  key decisions. No tools after this.
</tools>

<guidelines>
One agent at a time. Your tool history has every config
you wrote this run — use it to verify the format chain.
On re-triggers, read existing configs from prior runs first.

Each config has four fields:
- tools: capabilities beyond baseline (read_file/write_file
  are always implicit)
- system_prompt: who they are, how they think, what they
  write to the store
- assignment: the task, referencing <previous_agent_outputs>
  for upstream text and upstream agent artifacts for depth
- expected_output: what the response looks like — the text
  flowing to downstream agents. Keep it lean. Full work
  goes to the store.

Shape data flow through the prompts:
1. Full work → store (via write_file)
2. Response → lean, structured
3. Downstream reviews upstream agents' artifacts for depth

If <builder_action> says no changes and all agents are
designed, call complete_design immediately.
</guidelines>
```

**`{{roster_status}}`** is rebuilt each turn via `rebuild_system_prompt()` so the designer sees its progress after each `write_file`:

```xml
<roster_status>
  ✓ Scanner        — designed (v1)
  · FactChecker    — pending
  ✓ Analyzer       — designed (v1)

  Designed: 2/3
  Dependencies: Scanner → FactChecker → Analyzer
</roster_status>
```

```rust
fn build_roster_status(roster: &[TaskAgentRosterRow], store: &SystemStore) -> String {
    for agent in roster {
        let path = format!("design/agents/{}.json", slug(&agent.name));
        match store.read(&path) {
            Some(file) => format!("✓ {} — designed (v{})", agent.name, file.version),
            None       => format!("· {} — pending", agent.name),
        }
    }
}
```

#### Instruction (User Message)

The per-run context. On first design, no `<prior_design>` block. On re-trigger, prior design summaries appear first.

```xml
<prior_design>
1. Designed 3-agent linear pipeline. Scanner → Analyzer → Reporter.
   Format chain: numbered findings → prioritized list → full report.
   Each agent writes full work to store, responds lean. Downstream
   agents reference upstream artifacts by agent name via manifest.
</prior_design>

<plan>
## Objective
Scan codebase for OWASP Top 10 vulnerabilities, prioritize
by severity, produce remediation report.

## Agent Guidance
### Scanner
- Systematic grep by vulnerability category
- Write raw findings with code snippets to store

### Analyzer
- Severity: CRITICAL/HIGH/MEDIUM/LOW
- Flag false positives explicitly

### Reporter
- Executive summary, findings by severity, fix examples
</plan>

<roster>
Scanner
  role: "Security scanner who greps for vulnerability
        patterns and confirms findings."
  capabilities: [file_read, content_search]
  receives_from: []

Analyzer
  role: "Security analyst who verifies findings and
        assesses severity."
  capabilities: [file_read, content_search]
  receives_from: [Scanner]

Reporter
  role: "Technical writer who produces remediation docs."
  capabilities: []
  receives_from: [Analyzer]
</roster>

<builder_action>
Configured 3-agent pipeline: Scanner → Analyzer → Reporter
for security vulnerability scanning with OWASP Top 10 focus.
</builder_action>
```

The designer does NOT see:
- **Board state** — the builder already interpreted it into the plan
- **Canvas text** — the builder's domain
- **Upstream DAG step details** — cross-step routing is handled by the DAG at runtime, not by the designer
- **Store file listings** — no runtime artifacts exist at design time; the designer tells agents what to produce, the runtime manifest handles discovery
- **Beliefs** — beliefs are a chat-path concept; the dispatch flow uses DAG topology for cross-node context

On initial design (new node), roster_status shows all agents as pending. The designer writes all configs from scratch using the plan as its guide.

On re-design (roster changed), roster_status shows a mix of designed and pending. The designer reads existing configs, writes new configs for pending agents, checks whether existing agents need updates. The `<builder_action>` tells it what changed.

### How The Designer Solves Context Doubling

The DAG already handles routing — `receives_from` is set by the builder via dependencies. The designer doesn't control which agents' outputs get injected. What it controls is **what those outputs look like**.

The current problem: every agent's full text response flows to downstream agents via `<previous_agent_outputs>`. Agent 5 sees Agents 1-4's complete outputs, most of it redundant because Agent 3 already synthesized 1+2's work.

The designer solves this through prompt engineering, not runtime machinery:

1. **Tell agents to write full work to the store**: "Write your complete analysis to the store."
2. **Tell agents to keep responses lean**: "Respond with the prioritized list only — findings sorted by severity, false positives removed."
3. **Tell downstream agents whose artifacts to review**: "Review the Analyzer's triage artifacts for full context on each finding."

The agent's response becomes the natural handoff — concise, structured, designed for the next agent. The full work product lives in the store, discoverable via the `<upstream_artifacts>` manifest. No `<handoff>` tags, no runtime extraction, no new routing abstractions. The designer shapes the data flow through the prompts themselves.

### The Designer's Tool Loop

The designer works one agent at a time. On the first run, no read-backs are needed — the tool call history has every config the designer just wrote. On re-triggers, the designer reads existing configs from prior runs before making updates.

#### Example: 3-Agent Security Audit (First Run)

```
Roster: Scanner → Analyzer → Reporter
Plan: "Scan codebase for OWASP Top 10 vulnerabilities, prioritize, produce remediation report."

Turn 1: [roster_status: 0/3 designed]
  Think: "Linear pipeline. Scanner produces raw findings, Analyzer
         filters and prioritizes, Reporter writes the doc. Scanner
         should write full findings to store so Analyzer can review them
         via the artifact manifest. Scanner's response should be compact."

  write_file("design/agents/scanner.json", {
    tools: ["file_read", "content_search"],
    system_prompt: "You are a security scanner specializing in OWASP Top 10
      vulnerability detection for backend codebases.

      You have access to:
      - content_search: Grep for vulnerability patterns across the codebase.
      - file_read: Read file contents to confirm findings in context.

      Scan systematically by category: hardcoded secrets, SQL injection,
      XSS, auth bypasses, insecure deserialization, path traversal.
      For each finding: file path, line number, vulnerability type,
      the offending code snippet (3-5 lines).

      Write your complete findings to the store with full code context.
      Keep your response to a structured list — one line per finding with
      path:line, type, and severity estimate.",
    assignment: "Scan the codebase for security vulnerabilities. Focus on
      OWASP Top 10 categories. Write detailed findings to the store.
      Respond with a compact findings list.",
    expected_output: "Numbered list of findings. Each: file:line, vuln type,
      severity estimate. Full details written to store."
  })

Turn 2: [roster_status: 1/3 designed]
  // Scanner's config is in tool history from Turn 1 — no read needed

  write_file("design/agents/analyzer.json", {
    tools: ["file_read", "content_search"],
    system_prompt: "You are a senior application security analyst who
      triages vulnerability reports for false positives and severity.

      You have access to:
      - file_read: Read source files to verify findings in full context.
      - content_search: Check if a pattern is isolated or systemic.

      For each finding from the Scanner:
      1. Read the file to see full context around the flagged line
      2. Determine true positive vs false positive
      3. Use content_search to check if the pattern appears elsewhere
      4. Assign severity: CRITICAL / HIGH / MEDIUM / LOW

      Write your full triage to the store. Your response should be the
      prioritized list only — findings sorted by severity, false positives
      removed.",
    assignment: "Triage the Scanner's findings in <previous_agent_outputs>.
      Verify in source code, assess severity. Review the Scanner's artifacts
      for full code context. Write triage to the store. Respond with
      prioritized list.",
    expected_output: "Prioritized findings sorted by severity. False positives
      removed. Each: severity, file:line, type, one-line justification."
  })

Turn 3: [roster_status: 2/3 designed]
  write_file("design/agents/reporter.json", {
    tools: [],
    system_prompt: "You are a technical writer specializing in security
      remediation documentation for engineering teams.

      Each finding gets: risk description, affected code with file path,
      concrete fix with example code, estimated effort. Structure the
      report: executive summary (3 sentences), findings grouped by
      severity (CRITICAL first), remediation priority checklist.

      Write the report to the store.",
    assignment: "Write a remediation report from the Analyzer's prioritized
      findings in <previous_agent_outputs>. Review the Analyzer's artifacts
      for full context on each finding. Write the report to the store.",
    expected_output: "Complete remediation report with executive summary,
      findings by severity with fix examples, priority checklist."
  })

Turn 4: [roster_status: 3/3 designed]
  complete_design({
    summary: "Designed 3-agent linear pipeline. Scanner writes raw findings
      to store, responds with compact list. Analyzer triages with source
      verification, writes triage to store. Reporter produces remediation
      doc. Format chain: numbered findings → prioritized list → full report.
      Each agent writes full work to store, responds lean. Downstream agents
      reference upstream artifacts by agent name via the manifest."
  })
```

#### Example: 5-Agent Research Pipeline (Diamond Shape, First Run)

```
Roster: WebResearcher ──→ FactChecker → Synthesizer → Writer
        AcademicResearcher ─┘
Plan: "Research nuclear fusion energy. Web for recent news, academic for
       foundational claims. Verify all claims. Synthesize into briefing."

Turn 1: [roster_status: 0/5 designed]
  Think: "Diamond shape — two parallel researchers feed FactChecker.
         Both researchers must produce compatible output formats so
         FactChecker can process them uniformly. Same structure:
         claim + source + date."

  write_file("design/agents/web_researcher.json", {
    tools: [],
    system_prompt: "You are a web research specialist focused on current
      events in emerging technology. Search the web for recent news,
      press releases, funding announcements. For each finding: the
      specific claim, source URL, publication date, source type
      (primary/secondary).

      Write complete notes to the store. Your response: structured
      claims list, one per line.",
    assignment: "Search the web for recent nuclear fusion developments.
      Last 12 months: milestones, funding, policy, commercial progress.
      Write full notes to the store. Respond with structured claims list.",
    expected_output: "Numbered claims list. Each: claim text, source
      (primary/secondary), date. 10-20 claims."
  })

Turn 2: [roster_status: 1/5 designed]
  write_file("design/agents/academic_researcher.json", {
    tools: [],
    system_prompt: "You are an academic research specialist focused on
      peer-reviewed publications and institutional reports. Distinguish
      between demonstrated results (net energy gain, sustained plasma)
      and projected timelines (commercialization estimates).

      Write full notes to the store. Your response: structured claims
      list — same format as any research agent. Mark each as
      'demonstrated' or 'projected'.",
    assignment: "Search for academic publications on nuclear fusion.
      Focus on demonstrated results and credible projections. Write
      full notes to the store. Respond with claims list.",
    expected_output: "Numbered claims list. Each: claim text, source,
      date, demonstrated/projected. 10-20 claims."
  })

Turn 3: [roster_status: 2/5 designed]
  // Both researchers' configs are in tool history — compatible format verified

  write_file("design/agents/fact_checker.json", {
    tools: [],
    system_prompt: "You are a fact verification specialist who
      cross-references claims across multiple sources.

      For each claim:
      1. Check if it appears in both sources (cross-confirmed)
      2. Check for contradictions between sources
      3. Flag single-source claims as unverified
      4. Rate: HIGH (cross-confirmed, primary), MEDIUM (single,
         primary), LOW (single, secondary)

      Write full verification to the store. Respond with verified
      list only — debunked removed, contradictions flagged.",
    assignment: "Verify claims from both researchers in
      <previous_agent_outputs>. Cross-reference and rate confidence.
      Review the WebResearcher's and AcademicResearcher's artifacts
      for source context. Write verification to the store.",
    expected_output: "Verified claims list. Each: claim, confidence
      (HIGH/MEDIUM/LOW), verification note. Contradictions flagged."
  })

Turn 4: [roster_status: 3/5 designed]
  write_file("design/agents/synthesizer.json", {
    tools: [],
    system_prompt: "You are a research synthesizer who distills verified
      findings into coherent narrative structure. Group by theme,
      lead with HIGH confidence claims. Identify 3-5 key takeaways.

      Write outline to the store.",
    assignment: "Synthesize verified claims from FactChecker in
      <previous_agent_outputs>. Group by theme, lead with strongest
      evidence. Write outline to the store. Respond with outline and
      top takeaways.",
    expected_output: "Thematic outline with 3-5 key takeaways. Each
      section: theme, key claims, evidence strength."
  })

Turn 5: [roster_status: 4/5 designed]
  write_file("design/agents/writer.json", {
    tools: [],
    system_prompt: "You are a technical briefing writer for executive
      and technical audiences. Expand outlines into complete documents.
      Executive summary (5 sentences), sections per theme, conclusion.
      1500-2500 words. Authoritative, no speculation beyond evidence.

      Write the briefing to the store.",
    assignment: "Write the briefing from the Synthesizer's outline in
      <previous_agent_outputs>. Review the FactChecker's artifacts for
      claim details. Write the briefing to the store.",
    expected_output: "Complete briefing document, 1500-2500 words.
      Executive summary, thematic sections, conclusion."
  })

Turn 6: [roster_status: 5/5 designed]
  complete_design({
    summary: "Designed 5-agent diamond pipeline. Two parallel researchers
      (web + academic) produce compatible claims lists. FactChecker
      cross-references at merge point, rates confidence. Synthesizer
      groups by theme. Writer produces full briefing. Format chain verified:
      claims lists → verified list → outline → briefing. Each agent
      writes full work to store, downstream references by agent name
      via the artifact manifest."
  })
```

#### Example: 4-Agent Content Marketing Pipeline (First Run)

```
Roster: Researcher → SEOAnalyst → Writer → Editor
Plan: "Research topic, analyze SEO landscape, write optimized article,
       editorial review and fact-check."

Turn 1: [roster_status: 0/4 designed]
  Think: "Linear pipeline with a consistency constraint. SEOAnalyst must
         establish a keyword brief and editorial style guide that Writer
         follows — like a style bible. Researcher writes full notes to
         store, responds lean. Editor reviews all upstream artifacts."

  write_file("design/agents/researcher.json", {
    tools: [],
    system_prompt: "You are a topic research specialist who gathers
      comprehensive background on a subject. For each finding: the claim,
      source URL, publication date, relevance to the topic. Separate
      hard facts from opinions and estimates.

      Write complete research notes to the store — organized by subtopic
      with full source citations. Keep your response to a structured
      findings summary only.",
    assignment: "Research the assigned topic thoroughly. Gather recent
      data, expert opinions, and key statistics. Write full notes to the
      store. Respond with a structured findings summary.",
    expected_output: "Numbered findings summary. Each: key point, source,
      relevance. 10-15 findings. Full notes written to store."
  })

Turn 2: [roster_status: 1/4 designed]
  write_file("design/agents/seo_analyst.json", {
    tools: [],
    system_prompt: "You are an SEO content strategist who analyzes
      search landscape and produces editorial briefs.

      Produce two deliverables:
      1. Keyword brief — primary keyword, secondary keywords, search
         intent, competitor content gaps, target word count.
      2. Editorial style guide — tone, audience level, structure
         requirements, key points to cover, points to avoid.

      Write both as separate files to the store. Your response should
      be the keyword brief summary only.",
    assignment: "Analyze the SEO landscape for the topic based on the
      Researcher's findings in <previous_agent_outputs>. Review the
      Researcher's artifacts for source material. Write keyword brief
      and editorial style guide to the store.",
    expected_output: "Keyword brief: primary keyword, 5-8 secondary
      keywords, search intent, target word count, content gaps."
  })

Turn 3: [roster_status: 2/4 designed]
  write_file("design/agents/writer.json", {
    tools: [],
    system_prompt: "You are a content writer who produces SEO-optimized
      articles following editorial briefs precisely.

      Before writing, review the SEOAnalyst's editorial style guide
      and keyword brief from the store. Follow the style guide for
      tone, structure, and audience level. Naturally incorporate
      keywords from the brief — no keyword stuffing.

      Write the complete article to the store.",
    assignment: "Write the article following the SEOAnalyst's keyword
      brief in <previous_agent_outputs>. Review the SEOAnalyst's
      artifacts for the editorial style guide and the Researcher's
      artifacts for source material. Write the article to the store.",
    expected_output: "Complete article following the editorial brief.
      SEO-optimized, properly structured, target word count met."
  })

Turn 4: [roster_status: 3/4 designed]
  write_file("design/agents/editor.json", {
    tools: [],
    system_prompt: "You are a senior editor who reviews content for
      accuracy, style compliance, and SEO alignment.

      Review process:
      1. Check factual claims against the Researcher's source notes
      2. Verify style guide compliance from the SEOAnalyst's brief
      3. Check keyword usage and density
      4. Flag any unsupported claims or style violations

      Write your editorial review to the store. Your response is the
      final edited article with a brief editorial note.",
    assignment: "Review the Writer's article in <previous_agent_outputs>.
      Review the Researcher's artifacts for fact-checking, the
      SEOAnalyst's artifacts for style guide compliance. Write editorial
      review to the store. Respond with the final article.",
    expected_output: "Final edited article with editorial note summarizing
      changes made, claims verified, and style compliance status."
  })

Turn 5: [roster_status: 4/4 designed]
  complete_design({
    summary: "Designed 4-agent content marketing pipeline. Researcher
      writes full notes to store, responds with findings summary.
      SEOAnalyst establishes keyword brief + editorial style guide
      (written to store) that Writer must follow. Writer produces
      article per the brief. Editor fact-checks against Researcher's
      sources and verifies style guide compliance. Each agent writes
      full work to store, downstream agents reference by name via
      artifact manifest."
  })
```

#### Example: 1-Agent Simple Task

```
Roster: Reader (no capabilities)
Plan: "Read handwriting from image and transcribe."

Turn 1: [roster_status: 0/1 designed]
  write_file("design/agents/reader.json", {
    tools: [],
    system_prompt: "You are an OCR specialist who reads handwritten text
      and produces clean transcriptions. Preserve line breaks, layout,
      emphasis. Mark ambiguous text with [illegible] or [unclear: guess].
      Write the transcription to the store.",
    assignment: "Read and transcribe all handwritten text from the
      provided image. Write the transcription to the store.",
    expected_output: "Clean transcription with layout preserved."
  })

Turn 2: [roster_status: 1/1 designed]
  complete_design({
    summary: "Designed single OCR agent. Reads handwritten text, preserves
      layout, marks ambiguous text. Writes transcription to store."
  })
```

### The Pattern

Every designer loop follows the same rhythm:
1. **Think about the data flow** — what each agent produces and what the next one needs
2. **Write one config** — system prompt, assignment, tools, expected output
3. **Verify the format chain** — the designer's own tool history has every config it just wrote. On re-triggers, it reads existing configs from prior runs.
4. **Catch misalignment** — edit if Agent N's output doesn't match Agent N+1's expectations
5. **Complete** — call `complete_design` with a summary of the full design

The designer catches its own mistakes because its tool history contains every config from this run. On re-triggers, it reads prior configs from the store to understand what exists before making changes. One-shot can't self-correct either way.

### Designer Completion and Session History

The designer follows the same completion and session pattern as the builder.

**Completion tool: `complete_design`**

```json
{
  "summary": "What was designed — topology shape, format chain, key decisions (1-5 sentences)."
}
```

When the designer calls `complete_design`:
1. The engine stops (`should_stop() = true`)
2. The summary is persisted as an assistant message in the designer's session
3. The design run is marked complete

**Persistent session**: Each step has a designer session (keyed by step_id, role = "designer"), separate from the builder session. Summaries accumulate across design runs.

**Prior design injection**: On re-trigger, the designer's `build_messages` fetches the last 3 summaries from the session and injects them as `<prior_design>`:

```xml
<prior_design>
1. Designed 3-agent linear pipeline. Scanner → Analyzer → Reporter.
   Format chain: numbered findings → prioritized list → full report.
   Each agent writes full work to store, responds lean.
2. Added FactChecker between Scanner and Analyzer. Updated Analyzer
   assignment to reference FactChecker's artifacts. Adjusted format
   chain: findings → fact-checked findings → prioritized list → report.
3. Expanded Reporter to include executive summary. Updated expected_output.
</prior_design>
```

The designer sees what it designed before without replaying tool calls. Combined with `roster_status` (what exists now) and the store files (readable via `read_file`), it has full context for incremental changes.

**Parallel to the builder**: The builder has `complete_task(plan, summary)` → `<prior_work>`. The designer has `complete_design(summary)` → `<prior_design>`. Same infrastructure, same session persistence, same pruning logic.

### When The Designer Runs

The designer runs at **design time** — triggered after each builder dispatch, not at execution time. Every board submit that touches a node triggers: Phase 0 → builder dispatch → designer dispatch.

```
Board submit
  → Phase 0: structural changes (agentless)
  → Builder dispatch: configures roster, plan
  → Builder calls complete_task
  → Designer dispatch: writes per-agent prompts to store
  → Designer calls complete_design
  → Prompts appear in config panel for user review
  → User edits, refines, approves
  → User triggers execution
  → Executor reads prompts from store — no designer needed
```

Benefits:
- **User reviews before execution** — the config panel shows designed prompts with `expected_output` for each agent. The user can edit system prompts, adjust tools, change assignments.
- **Re-execution reuses prompts** — run the same workflow ten times without redesigning. Prompts persist in the store.
- **Redesign follows every builder change** — any builder dispatch (roster change, plan change, capability change) triggers a designer re-run. The designer sees `<prior_design>` for continuity.

### Designer Frontend Events

The designer reuses the dispatch infrastructure — same `DispatchStreamSink`, same WebSocket event bus. Builder and designer events are grouped under the same node in the dispatch tab as sequential phases. The designer picks up where the builder left off:

```
┌ Research Team ──────────────────────────────────────┐
│ Builder: Configured 3-agent pipeline (Scanner →     │
│          Analyzer → Reporter)                     ✓ │
│ Designer: Scanner designed                          │
│ Designer: Analyzer designed                         │
│ Designer: Reporter — designing...                 ◐ │
└─────────────────────────────────────────────────────┘
┌ Write Report ───────────────────────────────────────┐
│ Builder: waiting...                               ○ │
└─────────────────────────────────────────────────────┘
```

The designer's raw token stream is not shown — the user doesn't need to watch the LLM think. Instead, per-agent completion events fire as the designer writes each config to the store. The backend emits a `designer_agent_designed` event (agent name + step ID) after each successful agent config write. The dispatch tab renders these as progress lines within the node's section.

**Tree status**: The tree tab shows design status per node, driven by these events:

```
── Research Pipeline
   ├── Research Team          ◐ designing (2/3)
   ├── Validation             ● designed
   └── Write Report           ○ pending
```

- **○ pending** — awaiting design
- **◐ designing** — designer is active, with agent count progress (2/6, 4/6...)
- **● designed** — designer completed all agents. Prompts are in the store.
- **● designed (edited)** — user manually edited a prompt in the config panel after design

**Config panel**: When the designer completes, the user clicks a node in the tree and sees every designed agent — system prompt, assignment, expected_output, tools. These are read directly from the store (`design/agents/*.json`). The user can edit any field before triggering execution. Edits write back to the store, and the node's status changes to "designed (edited)" so the user knows it diverged from the designer's output.

**Event flow**:

```
Builder dispatch starts     → dispatch tab: "Builder: configuring..."
Builder calls complete_task → dispatch tab: "Builder: ✓ Configured 3-agent pipeline"
Designer dispatch starts    → dispatch tab: "Designer: designing..."
Designer writes scanner.json → dispatch tab: "Designer: Scanner designed"
                              tree: ◐ designing (1/3)
Designer writes analyzer.json → dispatch tab: "Designer: Analyzer designed"
                               tree: ◐ designing (2/3)
Designer writes reporter.json → dispatch tab: "Designer: Reporter designed"
Designer calls complete_design → dispatch tab: "Designer: ✓ All agents designed"
                                tree: ● designed
                                config panel: agent configs available for review
```

The dispatch tab shows one continuous stream per node — builder phase then designer phase. No separate sections, no accordion nesting. The user reads top to bottom: roster was configured, then each agent was designed.

### Partial Design Recovery

The designer either completes ALL agents or the step enters an error state. No half-designed pipelines reach the executor.

```
Designer fails at agent 4/6:
  1. Retry: roster_status shows 3/6 designed, designer picks up at agent 4
  2. Retry fails: clear all partial configs, fall back to one-shot (current system)
  3. Fallback fails: step errors out, user notified in config panel
```

**Tree status on failure**: The tree shows **◐ designing (3/6) — error** so the user sees exactly where the designer stopped. The 3 completed configs stay in the store — a retry picks up at agent 4, not from scratch. The step won't execute until all agents have prompts.

The user can:
- Trigger a redesign (retry the designer from where it stopped)
- Edit the prompts manually in the config panel
- Change the roster via the builder and redesign

### Cost Comparison

```
                    One-shot          ReAct (10 agents)
LLM calls:         1                 ~15-20
Time:              ~8s               ~40-60s
Output quality:    degrades at 10+   consistent
Self-correction:   none              built-in
Partial failure:   all or nothing    9/10 saved, redo 1
Cost:              ~$0.05            ~$0.15-0.20
```

For a team of 15 agents about to execute a $5 workflow, spending an extra $0.15 and 40 seconds on better prompts is worth it.

### The Executor Reads From Store

Current: executor uses prompts from an in-memory vec. They're saved to `designer_outputs` for logging but never read back.

New: executor reads prompts from the store.

```rust
// Current
for prompt in &phase_output.designed_prompts {
    execute_single_agent(prompt).await;
}

// New
for agent in &roster {
    let path = format!("design/agents/{}.json", slug(&agent.name));
    let prompt_json = store.read(&path)?;
    let prompt: DesignedAgentPrompt = serde_json::from_slice(&prompt_json)?;
    execute_single_agent(&prompt).await;
}
```

This means:
- **Re-run without re-designing**: read the same files. Same prompts, deterministic.
- **User edits a prompt before re-running**: edit the file in the config panel. The store versions it.
- **Designer failed on one agent**: the other 9 are in the store. Re-run designer for just the one.
- **Rollback**: restore a snapshot, get the prompts from two executions ago.

### Runtime Prompt Assembly

At execution time, the four designer fields map to the LLM call:

**System message** → `designed.system_prompt`

**User message** → assembled by the executor:

```xml
<context>
{task_description from mission brief}
</context>

<assignment>
{designed.assignment}
</assignment>

<expected_output>
{designed.expected_output}
</expected_output>

<refs>
  <file path=".system/refs/style_guide.md" type="text/markdown">
    Visual style rules, color palette, typography.
  </file>
  <file path=".system/refs/character_bible.md" type="text/markdown">
    Character descriptions and personality traits.
  </file>
</refs>

<upstream_artifacts>
  <step name="Research Team">
    <file path=".system/artifacts/data/research.md" type="text/markdown" by="Web Searcher">
      Competitive pricing analysis across 4 providers.
    </file>
  </step>
</upstream_artifacts>

<previous_agent_outputs>
{filtered upstream agent responses}
</previous_agent_outputs>

<upstream_step_outputs>
{upstream DAG step outputs}
</upstream_step_outputs>

{user_notes}
```

The block order: what to do → what to produce → reference material → upstream files → upstream text → user notes. `<refs>` lists user-uploaded reference material available to all agents in the workflow. `<upstream_artifacts>` lists files produced during this run by upstream agents/steps (workforce-local files plus direct-edge upstream files). Both are built by the executor — no dynamic tool descriptions needed.

## Builder → Designer Handoff

### Current Handoff

The builder calls `complete_task` with a `Passdown { plan, summary, question }`. The plan is a free-form markdown string. The designer receives it as an `<upstream>` context block.

```
Builder: complete_task({
  plan: "## Objective\nScan codebase for security vulnerabilities...\n\n## Agent-Specific Guidance\n### Scanner\n- Systematic grep...",
  summary: "Configured 3-agent pipeline: Scanner → Analyzer → Reporter"
})
```

The designer parses `### AgentName` markdown headers to find per-agent guidance. If the builder formats differently, the designer misses it.

### New Handoff

The builder still calls `configure_team` + `complete_task` as today. The roster, dependencies, and capabilities still live in the DB. The plan still flows via the Passdown.

What changes: when the builder re-runs on a task pivot, the handoff carries **what changed**. The designer reads existing prompts for unchanged agents and only redesigns what's needed.

```
User changes node from "write a story about a cat"
                    to "write a report on the attached github repo"

Builder:
  → configure_team: tears down old roster, builds new roster
  → complete_task: new plan + summary
  → Passdown stored with change context

Designer:
  → Reads roster_status: all pending (fresh team, builder replaced everyone)
  → Designs all prompts from scratch

---

User adds a fact-checker to existing team:

Builder:
  → configure_team: adds FactChecker to roster, wires dependencies
  → complete_task: updated plan

Designer:
  → Reads roster_status: 3 designed, 1 pending (FactChecker is new)
  → Reads existing prompts for the 3 designed agents
  → Designs FactChecker prompt, checking coordination with existing agents
  → Checks if existing agents need edits to route output to FactChecker
  → Edits Report Writer's prompt if needed
```

The roster_status injection handles this naturally. The designer sees what's already designed and what's new. No special "change-aware" logic — just reading the store.

### Builder Brevity

The builder's prompt already enforces tight role descriptions: "1-2 sentences defining WHO the agent is — domain expertise, scope boundary, and output type. Everything else goes in the plan."

This is critical for the store model. The builder writes tight configs. The designer expands them into full prompts. The builder should never over-explain in role descriptions — that's the designer's job.

Examples from the current builder prompt:

```
Good: "Security scanner who greps for vulnerability patterns and confirms findings. Outputs a raw findings list with file paths, line numbers, and vulnerability type."

Good: "OCR specialist who reads handwritten text from images and produces a clean transcription."

Bad: "A comprehensive security analysis agent that systematically searches through the entire codebase looking for various types of security vulnerabilities including but not limited to SQL injection, XSS, CSRF, authentication bypasses, hardcoded credentials, and insecure API endpoints, then produces a detailed report..."
```

## Artifact Flow — Runtime Discovery

Agents discover upstream artifacts through a **runtime manifest**, not semantic search. The system tracks every file written during a run and presents upstream artifacts to downstream agents automatically. The designer doesn't need to predict exact file paths — it tells agents what to produce and what to expect, and the runtime handles delivery.

### Two Scopes

**Within a workforce step (shared workspace):** All agents see all files written by any agent in the same workforce, regardless of `receives_from` dependencies. The workforce is a team with a shared desk — if the Web Searcher writes a file, the Fact Checker can see it even without a direct dependency edge.

**Across DAG steps (direct edges only):** A downstream step sees artifacts only from steps with a direct edge to it. If A → B → C with no A → C edge, Step C sees Step B's artifacts but not Step A's. If Step C needs Step A's files, the builder wires an edge A → C.

### The Manifest

Before an agent executes, the runtime builds an `<upstream_artifacts>` block from the file save history:

```xml
<upstream_artifacts>
  <step name="Research Team">
    <file path=".system/artifacts/data/research.md" type="text/markdown" by="Web Searcher">
      Competitive pricing analysis across 4 providers, Q3-Q4 year-over-year.
    </file>
    <file path=".system/artifacts/data/raw_data.csv" type="text/csv" by="Web Searcher">
      Raw pricing data, 847 rows across 4 competitors.
    </file>
    <file path=".system/artifacts/images/chart.png" type="image/png" by="Analyst">
      Pricing comparison bar chart, enterprise tier highlighted.
    </file>
  </step>
</upstream_artifacts>
```

The agent sees paths and descriptions, not file content. It calls `read_file` on whatever it needs. The description comes from the agent at write time — `write_file` takes an optional `description` parameter.

### How The Designer Accounts For This

The designer knows that `<upstream_artifacts>` exists as a runtime mechanism. Its system prompt documents it:

> "Agents with upstream dependencies receive an `<upstream_artifacts>` block at runtime containing a manifest of all files produced by prior steps. You don't need to wire specific file paths — tell agents what to produce via expected_output, and tell downstream agents to look for upstream artifacts."

The designer's job is intent, not plumbing:

- **expected_output**: "Write your findings as markdown files with descriptive names. Include a CSV of raw data."
- **assignment** (downstream): "Review the upstream research artifacts and synthesize a report. Focus on year-over-year trends."

The runtime delivers the manifest. The designer shapes what agents produce and how they consume.

### No Cross-Run Artifacts

Artifacts from previous executions are **not** included in the manifest. Each run starts clean. The manifest only contains files written during the current execution. This prevents stale data from leaking into new runs.

### File Descriptions

When an agent writes a file via `write_file`, it provides an inline description. This description populates the manifest immediately — no async processing needed for runtime discovery.

No background processing needed for artifact discovery.

## Step-to-Step Communication

### Three Layers

Data flows between agents through three channels:

1. **DAG routing** (existing) — the agent's text response flows to downstream agents via `<previous_agent_outputs>`, controlled by `receives_from` edges set by the builder.
2. **Artifact manifest** (new) — `<upstream_artifacts>` lists all files written by upstream steps/agents. The agent sees paths and descriptions, calls `read_file` on what it needs.
3. **Store files** (new) — agents write full work product to the store. Downstream agents read via `read_file` after discovering paths in the manifest.

The designer shapes what travels through each channel:

```
[Scanner]
  → writes full findings to store                      (full work, 2000 words)
  → response: compact numbered findings list           (lean handoff, 200 words)
          ↓ (DAG routing: response text)
          ↓ (Artifact manifest: Scanner's files listed with descriptions)
[Analyzer]
  → receives Scanner's compact list in <previous_agent_outputs>
  → sees Scanner's files in <upstream_artifacts> manifest
  → calls read_file to get full findings when needed
  → writes triage to store
  → response: prioritized list sorted by severity
```

The designer tells Scanner: "write full findings to the store, respond with a compact list." The designer tells Analyzer: "review upstream artifacts and the compact findings list." The manifest handles discovery automatically — the designer doesn't need to hardcode paths.

### Context Doubling — Solved By Design

The designer knows the full pipeline. It designs outputs with the downstream consumer in mind:

- **Agent 1** (Researcher): response is a compact findings list
- **Agent 2** (Fact Checker): response is verified findings with confidence scores
- **Agent 3** (Writer): only receives Agent 2's output (Agent 2 already incorporates Agent 1's work)

Agent 3 never sees Agent 1's raw data — it's redundant because Agent 2 synthesized it. The designer knows this because it read back Agent 1 and Agent 2's configs before writing Agent 3's.

The artifact manifest is lightweight — just paths and one-line descriptions. The agent decides what to actually read. Context grows linearly with manifest entries, not quadratically with full content.

```
                    Current                 New (designer-shaped)
Agent 3 receives:   Agent 1 + 2 full text   Agent 2 lean response + manifest
                    ~4,000 tokens            ~300 tokens
Agent 3 can read:   nothing else             any artifact via read_file

5-agent pipeline:   O(n²) context growth     O(n) context growth
```

### Workforce Shared Workspace

Within a workforce step, all agents share the same artifact view. Every file written by any agent in the step is visible to every other agent — no dependency edges required.

```
[Research Team] (workforce step)
  Web Searcher → writes research.md, raw_data.csv
  Paper Reviewer → writes paper_analysis.md (can see research.md in manifest)
  Fact Checker → writes verification.md (can see all 3 files above)
```

This is the shared desk model. The workforce is a team working on the same task — hiding files within the team would be counterproductive.

### Cross-Step Artifacts (DAG Edges)

Across DAG steps, artifacts flow only through direct edges:

```
              ┌─→ [Web Research]  ──┐
[Planning] ──┤                      ├──→ [Synthesis]
              └─→ [Paper Review]  ──┘
```

Synthesis sees artifacts from both Web Research and Paper Review (direct edges). It does **not** see Planning's artifacts unless Planning → Synthesis is also wired. The builder controls the topology. The designer writes prompts that account for it.

At the merge point, Synthesis receives:
- Text responses from both steps via `<previous_agent_outputs>`
- File manifest from both steps via `<upstream_artifacts>`
- Full file content on demand via `read_file`

## Store Tools

### list_artifacts

```
list_artifacts:
  scope: optional path scope
  → lists files in the store with descriptions
  → scoped to current step's workspace + refs
```

An explicit tool for agents to browse the store. Complements the `<upstream_artifacts>` manifest — the manifest shows upstream files automatically, `list_artifacts` lets agents explore the full store namespace when needed.

## Connected Systems — Federated Mounts

When workflows connect, they mount each other's stores.

```sql
INSERT INTO system_mounts (system_id, target_id, mount_point, access)
VALUES ('art-pipeline', 'story-engine', 'mounts/story-engine', 'read');
```

Now agents in Art Pipeline can `read_file` any path under `.system/mounts/story-engine/`. The system resolves the mount and reads from Story Engine's store. Mounted artifacts appear in the `<upstream_artifacts>` manifest when the mount source is a direct upstream in the collection DAG.

### Systems of Systems

```
┌─────────────────────────────────────────────────────────┐
│  PRODUCT STUDIO                                         │
│                                                         │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐ │
│  │ Story Engine │───→│ Art Pipeline│───→│ Film Studio │ │
│  │ .system/    │    │ .system/    │    │ .system/    │ │
│  │ characters  │    │ style guide │    │ final cuts  │ │
│  │ plot arcs   │    │ all renders │    │ audio mixes │ │
│  └─────────────┘    └─────────────┘    └─────────────┘ │
│         │                   │                  │        │
│         └───────────────────┴──────────────────┘        │
│                    mounted stores                         │
└─────────────────────────────────────────────────────────┘
```

Each system is autonomous. Connected systems share artifacts through mounts — read-only access to another system's store via mount-prefixed paths.

## Implementation Stack

### What Exists

- Postgres (database)
- xAI/Grok (LLM + web search + X search)
- DAG executor with parallel step support
- Board serializer (canvas to structure)
- Workforce pipeline with builder (ReAct, 12 rounds) + designer (one-shot)
- Docker container execution
- Builder → Designer handoff via Passdown { plan, summary }

### What's Added

| Component | Implementation | Effort |
|-----------|---------------|--------|
| Workflow filesystem | `s3://bucket/workflows/{id}/system/` prefix per workflow | Small |
| `system_files` table | One migration | Small |
| `system_snapshots` table | One migration | Small |
| `system_mounts` table | One migration | Small |
| `SystemStore` service | CRUD + manifest + mount resolution | Medium |
| Designer → ReAct agent | New strategy with store tools, roster status injection, `expected_output` field, runs at design time | Medium |
| Executor reads from store | Replace in-memory vec with store reads | Small |
| Implicit read/write tools | `read_file` + `write_file` available to all agents (store + project) | Small |
| Artifact manifest | `<upstream_artifacts>` block built from file save history | Small |
| `<refs>` prompt block | Inject user-uploaded refs into agent prompts | Small |
| Designer-shaped handoffs | Designer crafts lean responses + store writes per agent | No runtime cost — prompt engineering only |
| Store lifecycle | Create on workflow create, cleanup on node/workflow delete | Small |
| Design auto-scoping | Transparent `design/{step_id}/` prefix on designer store tools | Small |

### Implementation Slices

Ordered by dependency. Each slice is one plan.

1. **Store Foundation** — `system_files` migration, S3 prefix management (`workflows/{id}/system/`), `SystemStore` service (write, read, list, edit), file save history tracking.
2. **ReAct Designer** — `expected_output` on `DesignedAgentPrompt`, new system prompt, designer strategy as ReAct agent with store tools, roster status injection, partial recovery. Depends on 1.
3. **Executor Reads From Store** — Executor reads `design/{step_id}/agents/*.json` instead of in-memory vec. Auto-scoping. `<upstream_artifacts>` manifest built from file save history and injected into runtime prompts. Depends on 1 + 2.
4. **Implicit Agent Tools** — `read_file` / `write_file` for all executing agents, agents write to `.system/artifacts/`, `<refs>` prompt block for user uploads. Depends on 1.
5. **Advanced** — Snapshots, federated mounts. Future.

Critical path: **1 → 2 → 3**. Slice 4 can parallelize after 1.

### What This Builds On

| Capability | Already built | System Store adds |
|------------|--------------|-------------------|
| DAG execution | Orchestrator, parallel steps, envelopes | Shared project state via store |
| Tool dispatch | 15 execution tools, cascade routing | Implicit read_file/write_file for all agents |
| Workforce builder | ReAct agent, configure_team, complete_task | Unchanged — still owns roster + plan |
| Workforce designer | One-shot JSON generation | ReAct agent with store, iterative prompt building, expected_output, context-doubling prevention |
| Board serializer | Classify, diff, filter, score | Unchanged — still feeds Phase 0 |
| Beliefs extraction | Haiku extraction, neighbor awareness | Unchanged |
| Vision support | ContentBlock::Image, PNG rasterization | Unchanged |
| Docker execution | Persistent containers, file ops | `.system/` pre-synced into container, unified namespace |
| Port system | json_path extraction, edge wiring | DAG routing stays (lean responses), store adds depth layer |

## What This Enables

### Short Term
- Artifact manifest (agents discover upstream files automatically)
- Better prompts for large teams (designer builds iteratively, self-corrects)
- O(n) context scaling instead of O(n²) (designer shapes lean responses + store depth)
- Re-run workflows without re-designing (prompts persist in store)
- User can edit designed prompts before execution (edit the file, re-run)

### Medium Term
- Connected workflow systems with federated mounts
- Workflow templates (clone a system store, swap the refs)

### Long Term
- Always-on systems (workflows that watch for changes and react)
- Systems that build systems (architect workflows that produce new workflows)
- Marketplace (publish and fork system configurations)

## What This Doesn't Change

- The canvas and drawing layer — unchanged
- The board serializer pipeline — unchanged
- The DAG execution engine — extended, not rewritten
- The workforce builder — unchanged, still owns roster, dependencies, capabilities, plan
- The beliefs system — unchanged
- User authentication and authorization — unchanged
- The chat dispatch path — unchanged
