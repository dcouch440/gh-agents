# System Store — Vision

## What It Is

Every workflow becomes a **system** — a self-contained unit with its own filesystem, metadata index, and semantic search. The store serves two purposes: it's the **designer's workspace** for building agent prompts iteratively, and it's the **shared state** where running agents write their output for downstream steps to discover.

The user's project is their project — a GitHub repo, a folder of documents, whatever they're working on. The workflow operates on it. The system store lives alongside it as `.system/`.

## Why This Matters

Three problems converge here:

**1. The designer is one-shot.** Today, the Agent Designer generates system prompts for every agent in a single LLM call. For 3 agents this works. For 10+ agents, prompt quality degrades because the LLM can't hold all inter-agent coordination logic in one pass. It can't self-correct — if agent 5's design conflicts with agent 2's, it has no way to go back and fix agent 2.

**2. Data between steps is blind.** Data flows as untyped JSON in `StepExecutionEnvelope.data`. Tools are static — `read_file` says "Read the contents of a file" with no knowledge of what files exist or what they contain. If an image generation step writes a PNG, the video step has no way to discover it exists, what it depicts, or where to find it.

**3. Multi-modal pipelines can't work without an artifact layer.** You can't build a story-to-movie pipeline when steps pass JSON blobs. You need files with descriptions, types, and semantic searchability — images, video, audio, documents all discoverable by downstream agents.

The store fixes all three. The designer writes prompts to files it can read back and revise. Running agents write artifacts as real files with metadata. The designer wires explicit paths between agents, with similarity search as a fallback for discovery.

## The Unified Namespace

One filesystem. One `read_file`. One `write_file`. The `.system/` directory is a real directory on disk, volume-mounted into the Docker container. Agents see the whole tree:

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

For workflows that aren't operating on a code repo — like a story-to-movie pipeline — `.system/` itself is the project. The generated assets, documents, and media all live there.

### System Files vs Project Files

Every agent has an explicit understanding of this distinction:

- **Project files** (`src/`, `docs/`, etc.) — **the goal**. This is what you're building. The actual deliverable. The code, the pages, the assets the user cares about.
- **System files** (`.system/`) — **supporting material**. These exist to help agents complete the goal. Drafts, research notes, intermediate data, reference docs. They're not the deliverable — they're the scaffolding.

The designer reinforces this in every agent's prompt. The naming itself teaches the concept — `.system/` signals "this is infrastructure, not output."

The designer orchestrates the **transfer** — early agents produce working documents in `.system/`, later agents refine them, and the final agent applies the deliverable to the real project.

```
Agent 1 (Researcher):  writes .system/artifacts/legal/tos_research.md   (supporting)
Agent 2 (Drafter):     reads .system/artifacts/legal/tos_research.md
                       writes .system/artifacts/legal/tos_draft.md       (supporting)
Agent 3 (Reviewer):    reads .system/artifacts/legal/tos_draft.md
                       writes .system/artifacts/legal/tos_final.md       (supporting)
Agent 4 (Publisher):   reads .system/artifacts/legal/tos_final.md
                       writes src/pages/terms-of-use.md                  (THE GOAL)
```

Same `write_file` tool for both. The path tells you which is which. The designer's assignment for Agent 4: "Read the final Terms of Use at `.system/artifacts/legal/tos_final.md` and publish it to `src/pages/terms-of-use.md`."

### Implicit Tools

Every agent always has `read_file` and `write_file` available — these are implicit, like web search and X search. The designer doesn't need to include them in the tools list. When the designer assigns `tools: ["content_search"]`, the agent actually gets `read_file` + `write_file` + `content_search` + web search + X search.

This means agents with `tools: []` can still write system files and read upstream artifacts. The tools list only contains *additional* capabilities beyond the baseline.

### Real Files, Not Virtual

`.system/` is a real directory on disk. Volume-mounted into the Docker container:

```
Host: /data/workflows/{workflow_id}/system/ → Container: /app/.system/
```

No virtual filesystem. No FUSE. No materialization step. When an agent runs `shell: ffmpeg -i .system/artifacts/art/scene_01.mp4 ...`, it reads a real file. When an agent calls `write_file(".system/artifacts/report.md", content)`, it writes a real file.

Postgres tracks **metadata** as a sidecar index — descriptions, embeddings, tags, who produced the file, media type. But the files themselves are just files on disk. The store service is a thin layer: write the real file, update the metadata row.

```
write_file(".system/artifacts/research/findings.md", content)
  1. Write file to disk at /data/workflows/{id}/system/artifacts/research/findings.md
  2. INSERT/UPDATE system_files SET
       path = 'artifacts/research/findings.md',
       media_type = 'text/markdown',
       produced_by = step_id,
       size_bytes = len(content),
       version = version + 1
  3. Queue async: generate description + embedding (background Haiku call)
```

## Storage Architecture

Real files on disk. Postgres tracks metadata. No virtual filesystem, no content-addressing layer.

### The Directory

```
Host filesystem:
/data/workflows/{workflow_id}/system/     → mounted as /app/.system/ in container
  ├── design/{step_id}/agents/            # agent configs (real JSON files)
  ├── artifacts/                          # working files (real files)
  └── refs/                              # reference material (real files)
```

### Metadata Index (Postgres)

```
system_files
├── system_id        uuid
├── path             text                   # relative to .system/
├── media_type       text
├── description      text                   # auto-generated summary
├── tags             text[]
├── embedding        vector(384)            # for similarity search
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

Postgres is a **sidecar index**, not the source of truth for file content. The files on disk are the source of truth. Postgres tracks who produced each file, its description, tags, and embedding vector for similarity search.

### The Store Service

A thin layer over the real filesystem + metadata:

```rust
SystemStore {
    // File ops — read/write real files, update metadata
    read(path) -> bytes
    write(path, bytes, meta) -> updates file on disk + metadata row
    edit(path, find, replace) -> edits file on disk + bumps version
    list(prefix) -> entries from metadata index

    // Discovery — embedding similarity search
    search(query, scope) -> ranked results from pgvector

    // Versioning — lightweight bookmarks
    snapshot(label) -> snapshot_id
    restore(snapshot_id) -> reverts files to snapshot versions

    // Cross-system — federated search
    mount(target_system_id, mount_point, access)
}
```

### Store Lifecycle

**Creation**: When a workflow is created, the host directory `/data/workflows/{workflow_id}/system/` is created. Empty until the first builder → designer cycle or user upload.

**Cleanup on node deletion**: When a DAG step is removed, its `design/{step_id}/` directory is deleted (real files removed, metadata rows deleted). Artifacts the step's agents produced remain in `.system/artifacts/` — downstream steps may still reference them. The `produced_by` column lets the user find and clean up orphaned artifacts.

**Cleanup on workflow deletion**: The entire directory is removed (`rm -rf`). All `system_files` metadata rows for that `system_id` are deleted. All snapshots and mounts for that system are deleted.

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
  "assignment": "Scan the codebase for security vulnerabilities. Write detailed findings to .system/artifacts/security/raw_findings.md. Respond with a compact findings list.",
  "expected_output": "Numbered list of findings. Each: file:line, vuln type, severity estimate. Full details in store."
}
```

Same shape as the current `DesignedAgentPrompt` minus the fields the builder already set (roster_entry_id, name, execution_order, receives_from), plus `expected_output`.

**`expected_output`** is the key addition. It serves three purposes:
1. **Pipeline coherence** — the designer reads it back when writing the next agent. If Agent A's expected output doesn't match what Agent B's assignment expects, the designer catches it.
2. **User visibility** — the config panel shows it. The user sees at a glance what each agent produces without reading the full system prompt.
3. **Self-documentation** — the designer uses it as a contract with itself across turns.

### What The Designer Sees

The designer receives:

1. **Roster** — agent names, roles, dependencies, capabilities (from DB, set by builder)
2. **Plan** — the builder's execution blueprint (from Passdown). This is the designer's primary context. The original node text is the builder's domain — the designer trusts the builder's interpretation.
3. **Available capabilities** — the pool of tools agents can be assigned
4. **Roster status** — which agents already have designs in the store (from store state)
5. **Existing store files** — listing of `.system/artifacts/` files from prior steps or previous runs
6. **Upstream step info** — what upstream DAG steps produce, for cross-step artifact references

The designer does NOT see:
- The original canvas text (the builder already interpreted it)
- An explicit changeset (the designer infers what changed by comparing roster_status against existing configs)

On initial design (new node), roster_status shows all agents as pending. The designer writes all configs from scratch using the plan as its guide.

On re-design (roster changed), roster_status shows a mix of designed and pending. The designer reads existing configs for designed agents, writes new configs for pending agents, and checks whether existing configs need updates to coordinate with new agents. No explicit "what changed" — the store state IS the truth.

### The Designer's Understanding of the Pipeline

The designer understands the distinction between **working files** and **deliverables** — the same way a tech lead understands that a PR draft isn't the merged commit:

1. **Choose non-colliding paths** — it sees existing store files and picks paths that make sense.
2. **Reference upstream artifacts** — it knows which store paths were written by prior steps and tells its agents exactly where to read.
3. **Orchestrate transfer** — working files flow between agents in the store, the final agent applies the deliverable to the real project.

The designer tells each agent what to read from the previous agent and what to produce for the next one.

### How The Designer Solves Context Doubling

The DAG already handles routing — `receives_from` is set by the builder via dependencies. The designer doesn't control which agents' outputs get injected. What it controls is **what those outputs look like**.

The current problem: every agent's full text response flows to downstream agents via `<previous_agent_outputs>`. Agent 5 sees Agents 1-4's complete outputs, most of it redundant because Agent 3 already synthesized 1+2's work.

The designer solves this through prompt engineering, not runtime machinery:

1. **Tell agents to write full work to the store**: "Write your complete analysis to `.system/artifacts/security/triage.md`."
2. **Tell agents to keep responses lean**: "Respond with the prioritized list only — findings sorted by severity, false positives removed."
3. **Tell downstream agents where to find depth**: "For full context on each finding, read `.system/artifacts/security/triage.md`."

The agent's response becomes the natural handoff — concise, structured, designed for the next agent. The full work product lives in the store for anyone who needs depth. No `<handoff>` tags, no runtime extraction, no new routing abstractions. The designer is smart enough to shape the data flow through the prompts themselves.

### Roster Status Injection

Each turn, the designer's system prompt includes the current state — built from the store, not from conversation history:

```xml
<roster_status>
  ✓ Lead Researcher    — designed (v1)
  ✓ Market Scanner     — designed (v1)
  ✓ Data Analyst       — designed (v2, revised)
  · Fact Checker       — pending
  · Report Writer      — pending
  · Editor             — pending

  Designed: 3/6
  Dependencies: Lead Researcher → Data Analyst → Report Writer
                Market Scanner → Fact Checker → Report Writer
                Report Writer → Editor
</roster_status>
```

The designer doesn't have to reconstruct state from its own tool call history. It sees the truth every turn.

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

### The Designer's Tool Loop

The designer works one agent at a time, reading back prior configs to verify format alignment across the pipeline.

#### Example: 3-Agent Security Audit

```
Roster: Scanner → Analyzer → Reporter
Plan: "Scan codebase for OWASP Top 10 vulnerabilities, prioritize, produce remediation report."

Turn 1: [roster_status: 0/3 designed]
  Think: "Linear pipeline. Scanner produces raw findings, Analyzer
         filters and prioritizes, Reporter writes the doc. Scanner
         should write full findings to store so Analyzer can reference
         specific files. Scanner's response should be compact."

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

      Write complete findings to .system/artifacts/security/raw_findings.md
      with full code context. Keep your response to a structured list —
      one line per finding with path:line, type, and severity estimate.",
    assignment: "Scan the codebase for security vulnerabilities. Focus on
      OWASP Top 10 categories. Write detailed findings to
      .system/artifacts/security/raw_findings.md. Respond with a compact
      findings list.",
    expected_output: "Numbered list of findings. Each: file:line, vuln type,
      severity estimate. Full details in store."
  })

Turn 2: [roster_status: 1/3 designed]
  read_file("design/agents/scanner.json")
  → Scanner outputs numbered list with file:line references
  → Full findings in .system/artifacts/security/raw_findings.md

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

      Write full triage to .system/artifacts/security/triage.md. Your response
      should be the prioritized list only — findings sorted by severity,
      false positives removed.",
    assignment: "Triage the Scanner's findings in <previous_agent_outputs>.
      Verify in source code, assess severity. Read full findings at
      .system/artifacts/security/raw_findings.md for code context. Write triage
      to .system/artifacts/security/triage.md. Respond with prioritized list.",
    expected_output: "Prioritized findings sorted by severity. False positives
      removed. Each: severity, file:line, type, one-line justification."
  })

Turn 3: [roster_status: 2/3 designed]
  read_file("design/agents/analyzer.json")
  → Analyzer outputs prioritized list sorted by severity

  write_file("design/agents/reporter.json", {
    tools: [],
    system_prompt: "You are a technical writer specializing in security
      remediation documentation for engineering teams.

      Each finding gets: risk description, affected code with file path,
      concrete fix with example code, estimated effort. Structure the
      report: executive summary (3 sentences), findings grouped by
      severity (CRITICAL first), remediation priority checklist.

      Write the report to .system/artifacts/security/remediation_report.md.",
    assignment: "Write a remediation report from the Analyzer's prioritized
      findings in <previous_agent_outputs>. For full context on each
      finding, read .system/artifacts/security/triage.md. Write to
      .system/artifacts/security/remediation_report.md.",
    expected_output: "Complete remediation report with executive summary,
      findings by severity with fix examples, priority checklist."
  })

Turn 4: [roster_status: 3/3 designed]
  done()
```

#### Example: 5-Agent Research Pipeline (Diamond Shape)

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

      Write complete notes to .system/artifacts/research/web_findings.md.
      Your response: structured claims list, one per line.",
    assignment: "Search the web for recent nuclear fusion developments.
      Last 12 months: milestones, funding, policy, commercial progress.
      Write full notes to .system/artifacts/research/web_findings.md.
      Respond with structured claims list.",
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

      Write full notes to .system/artifacts/research/academic_findings.md.
      Your response: structured claims list — same format as any
      research agent. Mark each as 'demonstrated' or 'projected'.",
    assignment: "Search for academic publications on nuclear fusion.
      Focus on demonstrated results and credible projections. Write to
      .system/artifacts/research/academic_findings.md. Respond with claims list.",
    expected_output: "Numbered claims list. Each: claim text, source,
      date, demonstrated/projected. 10-20 claims."
  })

Turn 3: [roster_status: 2/5 designed]
  read_file("design/agents/web_researcher.json")
  read_file("design/agents/academic_researcher.json")
  → Both produce numbered claims lists — compatible format. Good.
  → FactChecker receives both in <previous_agent_outputs>

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

      Write full verification to .system/artifacts/research/verification.md.
      Respond with verified list only — debunked removed,
      contradictions flagged.",
    assignment: "Verify claims from both researchers in
      <previous_agent_outputs>. Cross-reference and rate confidence.
      Full analysis at .system/artifacts/research/verification.md. For source
      context read .system/artifacts/research/web_findings.md and
      .system/artifacts/research/academic_findings.md.",
    expected_output: "Verified claims list. Each: claim, confidence
      (HIGH/MEDIUM/LOW), verification note. Contradictions flagged."
  })

Turn 4: [roster_status: 3/5 designed]
  write_file("design/agents/synthesizer.json", {
    tools: [],
    system_prompt: "You are a research synthesizer who distills verified
      findings into coherent narrative structure. Group by theme,
      lead with HIGH confidence claims. Identify 3-5 key takeaways.

      Write outline to .system/artifacts/synthesis/outline.md.",
    assignment: "Synthesize verified claims from FactChecker in
      <previous_agent_outputs>. Group by theme, lead with strongest
      evidence. Write to .system/artifacts/synthesis/outline.md. Respond with
      outline and top takeaways.",
    expected_output: "Thematic outline with 3-5 key takeaways. Each
      section: theme, key claims, evidence strength."
  })

Turn 5: [roster_status: 4/5 designed]
  read_file("design/agents/synthesizer.json")
  → Outline with takeaways — good input for Writer

  write_file("design/agents/writer.json", {
    tools: [],
    system_prompt: "You are a technical briefing writer for executive
      and technical audiences. Expand outlines into complete documents.
      Executive summary (5 sentences), sections per theme, conclusion.
      1500-2500 words. Authoritative, no speculation beyond evidence.

      Write to .system/artifacts/reports/fusion_briefing.md.",
    assignment: "Write the briefing from the Synthesizer's outline in
      <previous_agent_outputs>. For claim details, read
      .system/artifacts/research/verification.md. Write to
      .system/artifacts/reports/fusion_briefing.md.",
    expected_output: "Complete briefing document, 1500-2500 words.
      Executive summary, thematic sections, conclusion."
  })

Turn 6: [roster_status: 5/5 designed]
  Think: "Format chain check:
         WebResearcher → claims list ✓
         AcademicResearcher → claims list (same format) ✓
         FactChecker expects claims → verified list ✓
         Synthesizer expects verified list → outline ✓
         Writer expects outline → full document ✓
         Store paths distinct, no collisions.
         FactChecker references both researcher store paths."
  done()
```

#### Example: 4-Agent Story-to-Images Pipeline

```
Roster: StoryWriter → ScenePlanner → ArtDirector → ImageGenerator
Plan: "Write children's story, break into scenes, define visual direction,
       generate illustrations."

Turn 1: [roster_status: 0/4 designed]
  Think: "Creative pipeline. Key: visual consistency. StoryWriter must
         include character physical descriptions since this feeds image
         generation downstream. ArtDirector establishes style bible."

  write_file("design/agents/story_writer.json", {
    tools: [],
    system_prompt: "You are a children's story writer for ages 6-10.
      Vivid, visual prose. Every character gets a physical description
      (hair, clothing, distinguishing features) — this story will be
      illustrated. Structure: title, characters section (name +
      appearance + personality), then story with scene breaks (---).

      Write to .system/artifacts/story/story.md.",
    assignment: "Write a children's story about a girl who discovers her
      saxophone can talk to animals. 5-7 scenes. Include detailed
      character descriptions. Write to .system/artifacts/story/story.md.",
    expected_output: "Complete story with character descriptions and
      clear scene breaks. 5-7 scenes."
  })

Turn 2: [roster_status: 1/4 designed]
  write_file("design/agents/scene_planner.json", {
    tools: [],
    system_prompt: "You are a storyboard planner who breaks narratives
      into visual scenes. For each scene: scene_number, setting,
      characters_present, action, emotion, composition_note.

      Write as JSON to .system/artifacts/story/scenes.json.",
    assignment: "Break the story in <previous_agent_outputs> into
      illustrated scenes. Character descriptions are in
      .system/artifacts/story/story.md. Write to .system/artifacts/story/scenes.json.",
    expected_output: "JSON array of scenes with visual direction."
  })

Turn 3: [roster_status: 2/4 designed]
  read_file("design/agents/scene_planner.json")
  → JSON scenes — ArtDirector needs this plus character details

  write_file("design/agents/art_director.json", {
    tools: [],
    system_prompt: "You are an art director for children's illustration.
      First establish a style bible: art style, color palette, character
      consistency rules. Then for each scene write a generation prompt
      including style, character appearance, composition, lighting.

      Style bible to .system/artifacts/art/style_bible.md.
      Prompts to .system/artifacts/art/prompts.json (scene_number, prompt, aspect_ratio).",
    assignment: "Read scenes in <previous_agent_outputs> and full story
      at .system/artifacts/story/story.md for character details. Create style
      bible at .system/artifacts/art/style_bible.md. Write per-scene generation
      prompts to .system/artifacts/art/prompts.json.",
    expected_output: "Style bible + per-scene generation prompts in JSON."
  })

Turn 4: [roster_status: 3/4 designed]
  read_file("design/agents/art_director.json")
  → Outputs prompts.json — ImageGenerator reads it

  write_file("design/agents/image_generator.json", {
    tools: ["generate_image"],
    system_prompt: "You are an illustration producer. For each scene
      prompt, call generate_image. After each generation, check the
      result against the style bible. If a character looks wrong,
      adjust and regenerate. Save to .system/artifacts/art/illustrations/
      as scene_01.png, scene_02.png, etc.",
    assignment: "Read prompts at .system/artifacts/art/prompts.json and style
      bible at .system/artifacts/art/style_bible.md. Generate an illustration
      per scene. Validate consistency. Save to
      .system/artifacts/art/illustrations/.",
    expected_output: "One illustration per scene. Summary of each with
      consistency notes."
  })

Turn 5: [roster_status: 4/4 designed]
  done()
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
      Write to .system/artifacts/transcription.md.",
    assignment: "Read and transcribe all handwritten text from the
      provided image. Write to .system/artifacts/transcription.md.",
    expected_output: "Clean transcription with layout preserved."
  })

Turn 2: [roster_status: 1/1 designed]
  done()
```

### The Pattern

Every designer loop follows the same rhythm:
1. **Think about the data flow** — what each agent produces and what the next one needs
2. **Write one config** — system prompt, assignment, tools, expected output
3. **Read back prior configs** — verify the format chain connects
4. **Catch misalignment** — edit if Agent N's output doesn't match Agent N+1's expectations
5. **Verify at the end** — spot-check the chain before calling done()

The designer catches its own mistakes. It reads back what it wrote, spots coordination issues, and fixes them. One-shot can't do this.

### When The Designer Runs

The designer runs at **design time** — triggered by board submit, not at execution time. This is a change from the current system where the designer runs at the start of each execution.

```
Board submit / chat dispatch
  → Builder configures roster, dependencies, plan
  → Designer runs async, writes prompts to store
  → Prompts appear in config panel for user review
  → User edits, refines, approves
  → User triggers execution
  → Executor reads prompts from store — no designer needed
```

Benefits:
- **User reviews before execution** — the config panel shows designed prompts with `expected_output` for each agent. The user can edit system prompts, adjust tools, change assignments.
- **Re-execution reuses prompts** — run the same workflow ten times without redesigning. Prompts persist in the store.
- **Redesign is explicit** — if the roster changes (builder re-runs), the designer re-runs. If the user edits a prompt manually, it stays edited. Redesign only happens when the user or the builder triggers it.
- **Design status in the tree** — the sidebar shows which nodes are designed, which are designing, which are pending (already in the visual dispatch vision).

### Partial Design Recovery

The designer either completes ALL agents or NONE count. No half-designed pipelines reach the executor.

```
Designer fails at agent 4/6:
  1. Retry: roster_status shows 3/6 designed, designer picks up at agent 4
  2. Retry fails: clear all partial configs, fall back to one-shot (current system)
  3. Fallback fails: step errors out, user notified in config panel
```

The step is atomic — fully designed or error state. The user sees the error and can:
- Trigger a redesign (retry the designer)
- Edit the prompts manually in the config panel
- Change the roster via the builder and redesign

No orphan nodes. The step exists but won't execute until all agents have prompts.

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

## Semantic Search — The Discovery Layer

The designer handles primary data flow — it tells each agent exactly which store paths to read. Semantic search is the **fallback** for discovering files the designer didn't explicitly reference: user-uploaded refs, cross-mounted artifacts from connected systems, or files from previous executions.

### How It Works

Every file in the store has an embedding vector generated from its description + tags. Before a step executes, similarity search runs against the agent's task and injects relevant refs the designer didn't explicitly wire:

```xml
<store_refs context="similarity" relevance="0.82-0.95">
  <ref path=".system/refs/character_bible.md" type="text/markdown" lines="47">
    Character descriptions, personality traits, visual style guide.
  </ref>

  <ref path=".system/mounts/story-engine/docs/world_building.md" relevance="0.84">
    World rules, magic system, geography.
  </ref>

  2 additional refs found. Use search_repo("query") for more.
</store_refs>
```

The agent sees a few relevant things it might not have known about. Each has a description, type, and a path it can follow. This supplements the designer's explicit store path references — it doesn't replace them.

### File Descriptions

When an agent writes a file, the description is generated automatically in the background:

- **Text files**: Haiku reads content, generates 1-2 sentence summary + tags
- **Image files**: Vision model views the image, generates description + tags
- **JSON files**: Haiku reads structure, generates summary + tags
- **Binary (audio/video)**: media_type + size + generation prompt (if from `generate_image`/`generate_video`)

On small edits (< 10% of content changed), the existing description is kept. On major rewrites, the description regenerates async. The agent doesn't wait — the file is searchable with the old description until the new one lands.

### The `search_repo` Tool

```
search_repo: "Search the system store for artifacts by semantic similarity."
  query: string    — "What to search for"
  type: optional   — "image/png", "text/markdown", etc.
  scope: optional  — "artifacts", "refs", "mounts/*"

  Returns: ranked results with descriptions and paths
```

### Embedding Infrastructure

**`fastembed-rs`** — runs `all-MiniLM-L6-v2` locally in Rust via ONNX Runtime. ~80MB model, milliseconds per embedding on CPU.

**`pgvector`** — adds a `vector(384)` column to `system_files`. Similarity search is a SQL query with HNSW indexing.

```sql
SELECT path, description, media_type,
       1 - (embedding <=> $1) as relevance
FROM system_files
WHERE system_id = ANY($2)
AND 1 - (embedding <=> $1) > 0.7
ORDER BY relevance DESC
LIMIT 10;
```

## Dynamic Tool Descriptions

Tools become context-aware by reading from the store at execution time.

### Current Problem

Tool definitions are static:

```rust
Tool {
    name: "read_file",
    description: "Read the contents of a file.",
    input_schema: json!({ "properties": { "path": { "type": "string" } } })
}
```

### The Fix

At step execution time, tool descriptions are built from store metadata:

```
read_file: "Read a file from the system store. Available files:
  .system/refs/style_guide.md — Visual style rules, color palette, typography
  .system/refs/character_bible.md — Character descriptions and personality traits
  .system/artifacts/docs/research.md — Competitive analysis (42KB, by researcher)
  .system/artifacts/images/chart_01.png — Pricing comparison chart (1024x768)
  Use search_repo('query') to find more files."
```

The agent sees a menu with descriptions, not a blank text field.

## Step-to-Step Communication

### Two Layers: DAG Routing + Store

Data flows between agents through two channels:

1. **DAG routing** (existing) — the agent's text response flows to downstream agents via `<previous_agent_outputs>`, controlled by `receives_from` edges set by the builder.
2. **Store** (new) — agents write full work product to files. Downstream agents read from the store when they need depth beyond the response.

The designer controls what travels through each channel by crafting prompts that separate lean responses from detailed store writes:

```
[Scanner]
  → writes .system/artifacts/security/raw_findings.md   (full work, 2000 words)
  → response: compact numbered findings list            (lean handoff, 200 words)
          ↓ (DAG routing — response only)
[Analyzer]
  → receives Scanner's compact list in <previous_agent_outputs>
  → reads .system/artifacts/security/raw_findings.md for code context when needed
  → writes .system/artifacts/security/triage.md
  → response: prioritized list sorted by severity
```

No runtime extraction. No handoff tags. The designer tells Scanner: "write full findings to the store, respond with a compact list." The designer tells Analyzer: "read the full findings at this path if you need context." The intelligence is in the prompt engineering.

### Context Doubling — Solved By Design

The designer knows the full pipeline. It designs outputs with the downstream consumer in mind:

- **Agent 1** (Researcher): response is a compact findings list
- **Agent 2** (Fact Checker): response is verified findings with confidence scores
- **Agent 3** (Writer): only receives Agent 2's output (Agent 2 already incorporates Agent 1's work)

Agent 3 never sees Agent 1's raw data — it's redundant because Agent 2 synthesized it. The designer knows this because it read back Agent 1 and Agent 2's configs before writing Agent 3's.

```
                    Current                 New (designer-shaped)
Agent 3 receives:   Agent 1 + 2 full text   Agent 2 lean response
                    ~4,000 tokens            ~200 tokens
Agent 3 can read:   nothing else             full store via read_file

5-agent pipeline:   O(n²) context growth     O(n) context growth
```

### The Store As Shared State

```
[Research]
  → agent writes .system/artifacts/docs/research_notes.md      (full work)
  → agent writes .system/artifacts/data/competitors.json       (structured data)
  → response: "Found 12 competitors, 4 increased pricing 12-18% in Q4"

[Write Draft]
  → receives Research response (lean summary)
  → reads .system/artifacts/docs/research_notes.md for detail
  → writes .system/artifacts/docs/draft_v1.md
```

Each step sees the cumulative store state. The designer tells each agent exactly which store paths to read — no guessing, no searching for files the designer already knows about.

### Parallel Steps

```
              ┌─→ [Web Research]  ──┐
[Planning] ──┤                      ├──→ [Synthesis]
              └─→ [Paper Review]  ──┘
```

Parallel agents write to their own files. Web Research writes `.system/artifacts/research/web_findings.md`. Paper Review writes `.system/artifacts/research/paper_analysis.md`. No shared files, no conflicts — the DAG structure ensures this by design.

At the merge point, Synthesis receives both agents' lean responses via DAG routing and can read their full store files for depth.

## Vision — Agents That See Images

Agents can view images through vision content blocks. When `read_file` returns an image, it's sent as `ContentBlock::Image` — the agent sees the actual pixels.

### Media-Aware File Reading

```rust
match meta.media_type.as_str() {
    "image/png" | "image/jpeg" | "image/webp" => {
        ContentBlock::Image { source: ImageSource {
            source_type: "base64",
            media_type: meta.media_type,
            data: base64::encode(&bytes),
        }}
    }
    "text/markdown" | "text/plain" | "application/json" => {
        ContentBlock::Text { text: String::from_utf8_lossy(&bytes) }
    }
    "audio/mp3" | "video/mp4" => {
        // Can't "see" these — return description
        ContentBlock::Text {
            text: format!("[{}: {} — {}]", meta.media_type, path, meta.description)
        }
    }
}
```

### Image Context Compaction

Images are ~1,000-1,600 tokens each. In a tool-use loop, every image persists in conversation history. You can't solve this with text descriptions — a description can never substitute for the actual pixels. The base64 IS the content.

The fix: images live for **one round trip**, then compact to a pointer. The agent can always re-read the image if it needs to see the pixels again.

```
Turn 2: Agent calls read_file("character.png")
Turn 3: [IMAGE — 1,500 tokens] Agent responds, analyzing the image
        ── COMPACTION ──
        Image replaced in history with:
        <image-viewed path=".system/artifacts/art/character.png">
          [You viewed this image above. Your analysis is in your prior response.]
          Use read_file(".system/artifacts/art/character.png") to view again.
        </image-viewed>
Turn 4+: Pointer — 30 tokens instead of 1,500
         Agent's own analysis from Turn 3 is still in history as text
Turn 7:  Agent needs to re-check a detail → read_file(".system/artifacts/art/character.png")
         Full image loads for one round trip, then compacts again
```

No description is generated. No summary is attempted. The agent's own response from the turn it viewed the image is the best record of what it saw — and that's already in the conversation history as text. The compaction just removes the raw base64 and leaves a path to re-read.

```
                        Without compaction    With compaction
5 images viewed:        7,500 tokens          250 tokens
10 turns of work:       ×10 round trips       ×10 round trips
Total image cost:       75,000 tokens         1,500 tokens
Agent re-reads 1:       +1,500 for 1 turn     (compacts again next turn)
```

The agent never loses access to the pixels. It just doesn't carry them in context when it's not actively looking at them.

## Multi-Modal Capabilities

### Image Generation

xAI Grok Imagine — same API already integrated:

```
POST https://api.x.ai/v1/images/generations
  model: "grok-imagine-image"       $0.02/image
  model: "grok-imagine-image-pro"   $0.07/image

POST https://api.x.ai/v1/images/edits
  Accepts up to 3 source images for editing/composition
```

### Video Generation

```
POST https://api.x.ai/v1/videos/generations
  model: "grok-imagine-video"       $0.05/second
  Supports text-to-video and image-to-video
  Async: submit → poll → retrieve
  Duration: 1-15 seconds per clip
```

### New Tools

```
generate_image:
  prompt: string
  style_ref: optional path
  aspect_ratio: "16:9" | "1:1" | "9:16"
  → generates image via Grok Imagine
  → stores in system store
  → description auto-generated by vision model viewing the result

generate_video:
  prompt: string
  image_url: optional path (for image-to-video)
  duration: 1-15 seconds
  → submits to Grok Imagine Video (async poll)
  → stores in system store with metadata

search_repo:
  query: string
  type: optional media type filter
  scope: optional path scope
  → embedding similarity search across store + mounts
  → returns ranked results with descriptions and paths
```

### Input Hash Caching

Hash each step's inputs (upstream artifact hashes + prompt + config). If the hash matches a previous execution, skip the step and reuse cached output. Critical when image gen is $0.02 and video gen is $0.05/second.

```rust
let input_hash = blake3::hash(&serialize(&step.prompt, &step.config, &upstream_hashes));
if let Some(cached) = store.get_by_input_hash(input_hash) {
    return cached;  // skip execution entirely
}
```

## Connected Systems — Federated Mounts

When workflows connect, they mount each other's stores.

```sql
INSERT INTO system_mounts (system_id, target_id, mount_point, access)
VALUES ('art-pipeline', 'story-engine', 'mounts/story-engine', 'read');
```

Now `search_repo("character description")` spans both stores. Results come back with mount-prefixed paths:

```xml
<ref path=".system/mounts/story-engine/docs/character_bible.md" relevance="0.94">
  Character descriptions, personality traits, visual style guide.
</ref>
```

The agent can `read_file` that path. The system resolves the mount and reads from Story Engine's store.

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
│                    federated search                      │
└─────────────────────────────────────────────────────────┘
```

Each system is autonomous. Connected systems share artifacts through mounts and federated similarity search.

## The Demo Workflow

```
[Write Story] → [Split Scenes] → [Generate Images] → [Generate Video] → [Assemble]
                                        ↑
                                  [Style Guide]
```

1. **Write Story** — workforce writes `.system/artifacts/docs/story.md` and `.system/artifacts/docs/characters.md`
2. **Style Guide** — agent writes `.system/refs/style.md` with visual rules
3. **Split Scenes** — reads story via similarity search, writes `.system/artifacts/data/scenes.json`
4. **Generate Images** — parallel fan-out, one image per scene via Grok Imagine, each stored with vision-generated description
5. **Generate Video** — image-to-video for each scene via Grok Imagine Video
6. **Assemble** — ffmpeg in Docker container, stitches clips into final video

Each step discovers prior artifacts through similarity search. No explicit port wiring.

**Cost for a 6-scene short film**: ~$0.12 images + ~$1.80 video + ~$2 LLM calls. Under $5.

## Implementation Stack

### What Exists

- Postgres (database)
- xAI/Grok (LLM + web search + X search + image gen + video gen API)
- DAG executor with parallel step support
- Board serializer (canvas to structure)
- Workforce pipeline with builder (ReAct, 12 rounds) + designer (one-shot)
- Vision content blocks (`ContentBlock::Image`)
- Docker container execution
- Builder → Designer handoff via Passdown { plan, summary }

### What's Added

| Component | Implementation | Effort |
|-----------|---------------|--------|
| `pgvector` extension | Extension install + migration | Minimal |
| `fastembed-rs` | `cargo add fastembed` — local ONNX embeddings | Small |
| Workflow filesystem | `/data/workflows/{id}/system/` directory per workflow | Small |
| `system_files` table | One migration | Small |
| `system_snapshots` table | One migration | Small |
| `system_mounts` table | One migration | Small |
| `SystemStore` service | CRUD + search + mount resolution | Medium |
| Designer → ReAct agent | New strategy with store tools, roster status injection, `expected_output` field, runs at design time | Medium |
| Executor reads from store | Replace in-memory vec with store reads | Small |
| Implicit read/write tools | `read_file` + `write_file` available to all agents (store + project) | Small |
| `generate_image` tool | xAI Grok Imagine API call + store write | Small |
| `generate_video` tool | xAI Grok Imagine Video API call (async poll) + store write | Small |
| `search_repo` tool | pgvector similarity query | Small |
| Dynamic tool descriptions | Read store metadata at tool build time | Small |
| Designer-shaped handoffs | Designer crafts lean responses + store writes per agent | No runtime cost — prompt engineering only |
| Ref injection in prompts | Pre-step similarity search + inject `<ref>` blocks | Medium |
| Image context compaction | Post-response hook, replace base64 with re-read pointer | Medium |
| Store lifecycle | Create on workflow create, cleanup on node/workflow delete | Small |
| Design auto-scoping | Transparent `design/{step_id}/` prefix on designer store tools | Small |
| Auto file descriptions | Background Haiku/vision call on write, debounced on edits | Medium |

### What This Builds On

| Capability | Already built | System Store adds |
|------------|--------------|-------------------|
| DAG execution | Orchestrator, parallel steps, envelopes | Shared project state via store |
| Tool dispatch | 15 execution tools, cascade routing | Dynamic descriptions from store metadata |
| Workforce builder | ReAct agent, configure_team, complete_task | Unchanged — still owns roster + plan |
| Workforce designer | One-shot JSON generation | ReAct agent with store, iterative prompt building, expected_output, context-doubling prevention |
| Board serializer | Classify, diff, filter, score | Unchanged — still feeds Phase 0 |
| Beliefs extraction | Haiku extraction, neighbor awareness | Unchanged |
| Vision support | ContentBlock::Image, PNG rasterization | Media-aware read_file + compaction |
| Docker execution | Persistent containers, file ops | `.system/` volume-mounted into container, unified namespace |
| Port system | json_path extraction, edge wiring | DAG routing stays (lean responses), store adds depth layer |

## What This Enables

### Short Term
- Multi-modal workflows (text + image + video generation)
- Self-correcting pipelines (vision agents QA their own output)
- Context-aware tools (agents know what files exist and what they contain)
- Better prompts for large teams (designer builds iteratively, self-corrects)
- O(n) context scaling instead of O(n²) (designer shapes lean responses + store depth)
- Re-run workflows without re-designing (prompts persist in store)
- User can edit designed prompts before execution (edit the file, re-run)

### Medium Term
- Connected workflow systems with federated search
- Workflow templates (clone a system store, swap the refs)
- Input-hash caching (skip unchanged steps in expensive pipelines)

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
