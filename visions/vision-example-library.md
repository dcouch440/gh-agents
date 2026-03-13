# Example Library — Vision

## What It Is

A curated library of agent orchestration examples, embedded in pgvector, retrieved by similarity at design time. The builder and designer don't learn from static examples hardcoded in their system prompts — they learn from the most relevant examples for the task at hand, pulled fresh every time.

Each example is a complete case: the task, the team structure, the `.system/` file tree, the full designer output, and the knowledge transfer pattern between agents. The examples are organized by **orchestration concept**, not by domain. A security audit and a research pipeline may both use the same "sequential refinement" pattern — the example teaches the pattern, not the domain.

## Why This Matters

### The Current Problem

The builder has 3 static examples in its system prompt. The designer has 1. These cover simple/pipeline/incremental but can't cover the combinatorial space of real tasks. A 4-agent diamond pattern for competitive analysis requires fundamentally different design thinking than a 4-agent diamond for code review — but both share structural DNA that the current static examples don't capture.

### The Knowledge Transfer Problem

The deeper issue isn't team structure — it's how agents brief each other. This is the same problem humans solve instinctively. When you brief a senior engineer, you don't explain what a database is. You say "the schema's in the repo, table X has the issue." You respect what they already know and transfer only the delta.

LLMs have the same property. Claude knows what a security vulnerability is. Claude knows how to write markdown. Claude knows what JSON looks like. When the designer over-explains these things in a system prompt — "JSON is a structured data format using key-value pairs, you should format your output as..." — it signals the agent needs hand-holding. The agent responds by being cautious, hedging, over-explaining its own output. Worse, it may hallucinate because the verbose instructions crowd out the actual task.

Cognition (the team behind Devin) discovered this empirically: they call it **context anxiety**. When agents perceive their context is bloated or constrained, they take shortcuts and produce incomplete work. The solution isn't better compression — it's not putting the junk in there in the first place.

The examples must teach the designer this craft: **brief agents like colleagues, not interns**.

### What The Research Says

The industry is converging on a clear set of principles:

**1. Pass references, not content.** (Anthropic, Manus, MetaGPT)
Every production multi-agent system that works at scale stores full work product externally and passes lightweight references between agents. Anthropic's own multi-agent research system uses "artifact bypass" — subagents store outputs in external systems and pass references back. Manus externalizes memory to files. MetaGPT forces structured intermediate outputs published to a shared pool.

**2. Respect training knowledge.** (Cognition, Anthropic)
Don't re-explain what the model already knows. A focused 300-token context often outperforms an unfocused 3,000-token context. What you remove from context can matter as much as what you keep.

**3. Structure beats prose.** (MetaGPT, Anthropic)
MetaGPT's key finding: requiring structured intermediate outputs (not free-form text) between agents dramatically improved success rates. Structure maintains consistency and minimizes ambiguity. Anthropic's subagent briefing uses four components: specific objective, output format, tool guidance, task boundaries.

**4. The file system is the coordination layer.** (Manus, Anthropic)
For long-running workflows, files on disk become the source of truth. Manus creates `todo.md` files and intermediate result files. Anthropic uses `claude-progress.txt`. The file system survives context window resets, doesn't bloat conversation history, and is natively accessible by shell tools.

**5. Few-shot retrieval outperforms static examples.** (Emerging consensus)
Dynamic few-shot prompting — retrieving relevant examples via embedding similarity at inference time — produces better results than fixed examples because the examples match the task semantics. No framework does this for agent configuration yet.

## The Transfer Principle

This is the core concept the examples must encode. Not "here's how to do a security audit" but "here's how knowledge flows between agents without loss or bloat."

### What Transfer Means

In human teams, knowledge transfer is natural. A researcher doesn't dump their entire notebook on the writer's desk. They produce a brief with the key findings, and the writer knows to ask for the notebook if they need detail. Both parties understand what the other knows by default and what needs to be explicitly communicated.

Agent transfer works the same way:

```
Agent 1 (Researcher):
  - KNOWS by default: how to search, how to evaluate sources, how to write markdown
  - NEEDS to be told: what to research, where to put the output, what format the next agent expects
  - PRODUCES: full research in .system/, compact claims list as response

Agent 2 (Fact Checker):
  - KNOWS by default: how to verify claims, how to rate confidence, how to search
  - NEEDS to be told: where the claims are, where the full context is, what format to output
  - PRODUCES: verified list with confidence scores, full analysis in .system/

Agent 3 (Writer):
  - KNOWS by default: how to write, how to structure a document, how to read files
  - NEEDS to be told: where the verified data is, what the deliverable format is, where to put it
  - PRODUCES: the deliverable
```

Each agent is told three things: **where to find input**, **what to produce**, **where to put output**. Everything else — how to search, how to write, how to evaluate — is trusted as training knowledge. The designer doesn't teach the agent its job. It gives the agent its assignment.

### The Anti-Pattern: Over-Briefing

```
BAD system prompt:
"You are a security scanner. Security scanning involves searching through
source code to find potential vulnerabilities. Vulnerabilities are weaknesses
in code that could be exploited by attackers. Common vulnerability types
include SQL injection (where user input is directly concatenated into SQL
queries), Cross-Site Scripting (XSS, where user input is reflected in HTML
without escaping), and hardcoded credentials (where passwords or API keys
are stored directly in source code). When you find a vulnerability, you
should note the file path, the line number, and the type of vulnerability.
A file path is the location of the file in the directory structure..."
```

This prompt wastes ~150 tokens explaining what the model already knows. Worse, the model now believes it's talking to someone who needs this level of explanation, and it mirrors that verbosity in its output. The downstream agent then receives a bloated response full of definitions it also doesn't need.

```
GOOD system prompt:
"You are a senior application security engineer. Scan for OWASP Top 10
categories. For each finding: file:line, vulnerability type, the offending
code snippet (3-5 lines), and severity estimate.

Write complete findings to .system/artifacts/security/raw_findings.md.
Respond with a compact numbered list — one line per finding."
```

40 tokens. The model knows what OWASP Top 10 means. The model knows what a code snippet is. The prompt tells it what to do and where to put it. Nothing more.

### The Transfer Chain

The designer orchestrates a **transfer chain** — each agent produces exactly what the next agent needs to start working, nothing more. The full depth lives in `.system/` for any agent that needs it.

```
Agent 1 response (what flows via DAG):
  "12 findings across 4 categories. 3 critical (SQL injection in auth module),
   5 high, 4 medium. Full findings with code context in .system/artifacts/security/raw_findings.md."

Agent 2 reads this and knows:
  - The scale (12 findings, 4 categories)
  - The shape (3 critical, 5 high, 4 medium)
  - Where to find depth (.system/ path)
  - What to do next (triage, verify, prioritize)

Agent 2 does NOT need:
  - Agent 1's full methodology description
  - Definitions of vulnerability types
  - Explanations of what file:line notation means
  - The complete findings inline (they're in the store)
```

The response is a briefing, not a report. The store has the report. The briefing tells the next agent what happened and where to look.

## The Example Schema

Each example is a complete orchestration case stored as a record:

```
example_id:          uuid
task_description:    text          # embedded for semantic search
agent_count:         int           # metadata filter
topology:            text          # "linear", "diamond", "fan_out", "fan_in", "complex"
pattern:             text          # the abstract concept (see Pattern Catalog)
domain_tags:         text[]        # ["security", "code", "research", "creative", "data"]

# The full case
agents: [{
  name:              text
  role_description:  text          # builder-style, 1-2 sentences
  tools:             text[]
  system_prompt:     text          # the actual prompt
  assignment:        text
  expected_output:   text
}]

dependencies: [{
  from:              text          # agent name
  to:                text          # agent name
}]

system_tree:         text          # the .system/ file tree this pattern produces
plan:                text          # the builder's plan
transfer_notes:      text          # how knowledge flows between agents

quality_score:       float         # curated rating (0-1)
embedding:           vector(384)   # from task_description
```

### What Gets Embedded

The `task_description` field — a natural language description of what the task is. This is the search key. When a builder is configuring a team for "scan this codebase for security vulnerabilities and produce a remediation plan," the embedding similarity finds examples with similar task descriptions.

### What Gets Filtered

The `agent_count` field — a hard metadata filter. If the builder has already decided on 3 agents, retrieve examples with 3 agents (and optionally 2 or 4 for comparison). Don't show a 7-agent example when the task calls for 2.

The `topology` field — optional filter for structural similarity. If the builder set up a diamond dependency pattern, prefer diamond examples.

The `pattern` field — the abstract orchestration concept. Multiple examples may share the same pattern but differ in domain and agent count.

## The Pattern Catalog

Examples are organized by the abstract orchestration concept they demonstrate. A pattern is domain-agnostic — it describes HOW agents coordinate, not WHAT they work on.

### Pattern 1: Sequential Refinement

Each agent takes the previous agent's work and makes it better. The output gets progressively more polished. Each step has a clear transformation: raw → analyzed → formatted → delivered.

**Transfer dynamic**: Each agent's response summarizes what changed. The store has the full artifact at each stage. The next agent reads the summary to understand what was done, reads the store file for the actual content.

**`.system/` dynamic**: A trail of progressively refined files. Each agent reads the previous version, writes a new one. The path naming shows the progression: `draft.md` → `reviewed.md` → `final.md`. Or versioned: `report_v1.md` → `report_v2.md`.

**Agent default knowledge respected**: Each agent knows HOW to do its job (analyze, write, review). The designer tells it WHERE to find input, WHAT specifically to transform, and WHERE to put output.

**Typical counts**: 2-4 agents.

```
Examples to build:
  2-agent: Researcher → Writer
  3-agent: Drafter → Reviewer → Finalizer
  4-agent: Extractor → Transformer → Validator → Formatter
```

**Example — 3-agent Sequential Refinement:**

```
Task: "Research a topic and produce a polished briefing document."
Topology: linear
Pattern: sequential_refinement
Agents: Researcher → Synthesizer → Writer

.system/ tree produced:
├── artifacts/
│   ├── research/
│   │   └── raw_findings.md          ← Researcher
│   ├── synthesis/
│   │   └── structured_brief.md      ← Synthesizer
│   └── output/
│       └── briefing.md              ← Writer (deliverable)

Researcher:
  tools: []
  system_prompt: "You are a research specialist. Search for current
    information on the assigned topic. For each finding: the specific
    claim, source, date, reliability assessment.

    Write complete notes to .system/artifacts/research/raw_findings.md.
    Respond with a structured claims list — one line per finding."
  assignment: "Research the assigned topic. Write full notes to
    .system/artifacts/research/raw_findings.md. Respond with a
    compact claims list."
  expected_output: "Numbered claims list. Each: claim, source, date,
    reliability. Full notes in store."

  Transfer note: Response is a compressed index of findings. The store
  file has full context, quotes, source analysis. Synthesizer reads
  the response to understand the shape, reads the store for depth.

Synthesizer:
  tools: []
  system_prompt: "You are a research synthesizer. Organize findings
    into thematic groups. Identify 3-5 key takeaways. Resolve
    contradictions between sources. Flag gaps.

    Read full findings at .system/artifacts/research/raw_findings.md.
    Write structured brief to .system/artifacts/synthesis/structured_brief.md.
    Respond with the outline and takeaways."
  assignment: "Synthesize the findings from <previous_agent_outputs>.
    For full context, read .system/artifacts/research/raw_findings.md.
    Write to .system/artifacts/synthesis/structured_brief.md."
  expected_output: "Thematic outline with 3-5 takeaways. Each section:
    theme, supporting claims, evidence strength."

  Transfer note: Response is the outline — the Writer's roadmap.
  Full structured brief in store has the evidence backing each section.

Writer:
  tools: []
  system_prompt: "You are a document writer. Expand outlines into
    complete, polished documents. Executive summary, thematic sections,
    conclusion. Authoritative tone, no speculation beyond evidence.

    Write to .system/artifacts/output/briefing.md."
  assignment: "Write the briefing from the outline in
    <previous_agent_outputs>. For claim details and evidence, read
    .system/artifacts/synthesis/structured_brief.md. Write to
    .system/artifacts/output/briefing.md."
  expected_output: "Complete briefing document. Executive summary,
    thematic sections, conclusion."

  Transfer note: Final agent. Response IS the deliverable summary.
  Full document in store. No downstream agent — transfer complete.
```

### Pattern 2: Parallel Collection + Merge

Multiple agents gather information independently from different sources or perspectives. A merge agent synthesizes their outputs into a unified result. The parallel agents must produce compatible output formats so the merge agent can process them uniformly.

**Transfer dynamic**: Parallel agents don't know about each other. Each writes to its own path in `.system/`. Each responds with a structured list in the same format. The merge agent receives all responses via DAG routing and can read all store files for depth.

**`.system/` dynamic**: Parallel paths that never collide. Each collector writes to its own subdirectory. The merge agent reads all subdirectories and writes the synthesis. Path naming reflects the source: `web/`, `academic/`, `internal/`.

**Agent default knowledge respected**: Collectors know how to search and evaluate sources. The merge agent knows how to synthesize and resolve contradictions. The designer tells each WHERE to look, WHAT format to use (so outputs are compatible), and WHERE to put results.

**Critical designer decision**: The output format must be compatible across parallel agents. If Web Researcher outputs `claim, source, date` and Academic Researcher outputs `finding, paper, year`, the merge agent has to handle two formats. The designer standardizes this.

**Typical counts**: 3-5 agents (2-3 collectors + 1-2 merge/synthesis).

```
Examples to build:
  3-agent: Collector A + Collector B → Merger
  4-agent: 3 Collectors → Synthesizer
  5-agent: 3 Collectors → Validator → Writer
```

**Example — 4-agent Parallel Collection + Merge:**

```
Task: "Gather data from multiple sources and produce a comprehensive analysis."
Topology: fan_in
Pattern: parallel_collection_merge
Agents: SourceA, SourceB, SourceC (parallel) → Analyst

.system/ tree produced:
├── artifacts/
│   ├── collection/
│   │   ├── source_a.md              ← SourceA
│   │   ├── source_b.md              ← SourceB
│   │   └── source_c.md              ← SourceC
│   └── analysis/
│       └── consolidated.md          ← Analyst (deliverable)

SourceA / SourceB / SourceC (same structure, different assignment):
  tools: []
  system_prompt: "You are a data collection specialist focused on
    [source type]. For each data point: the specific finding,
    source reference, confidence level.

    Write complete findings to .system/artifacts/collection/source_[x].md.
    Respond with a structured list — one line per data point.

    Output format (all collectors use this):
    - [finding] | [source] | [confidence: high/medium/low]"
  assignment: "Collect data from [source]. Write to
    .system/artifacts/collection/source_[x].md. Respond with
    structured list using the standard format."
  expected_output: "Structured data points list. Standard format:
    finding | source | confidence."

  Transfer note: All three collectors use identical output format.
  The Analyst receives three compatible lists it can process uniformly.

Analyst:
  tools: []
  system_prompt: "You are an analyst who consolidates multi-source
    data. Cross-reference findings across sources. Identify
    consensus (appears in 2+ sources), unique findings (single
    source), and contradictions. Weight by confidence level.

    Read full collection data from .system/artifacts/collection/.
    Write consolidated analysis to .system/artifacts/analysis/consolidated.md."
  assignment: "Analyze data from all collectors in
    <previous_agent_outputs>. Cross-reference, identify consensus
    and contradictions. For full context read .system/artifacts/collection/.
    Write to .system/artifacts/analysis/consolidated.md."
  expected_output: "Consolidated analysis. Sections: consensus findings,
    unique findings by source, contradictions, confidence assessment."

  Transfer note: Three inputs → one output. The standardized format
  from collectors makes cross-referencing mechanical. The Analyst
  adds judgment — what's confirmed, what's contested, what's novel.
```

### Pattern 3: Produce + Verify

One agent produces work, another verifies it. The verifier's job is narrow: check correctness, flag issues, rate confidence. This pattern is consistently effective because verification requires minimal context transfer — the verifier just needs the output and evaluation criteria.

**Transfer dynamic**: The producer's response is the work product. The verifier's response is a pass/fail assessment with specific issues. No ambiguity in the handoff — "here's the thing, is it correct?"

**`.system/` dynamic**: Producer writes full work to store. Verifier writes verification report to store. The verification report references specific locations in the producer's output.

**Agent default knowledge respected**: The verifier knows how to evaluate quality in its domain. The designer tells it WHAT to verify, WHERE to find it, and WHAT criteria to apply. Never explain how to verify.

**Typical counts**: 2-3 agents.

```
Examples to build:
  2-agent: Producer → Verifier
  3-agent: Producer → Verifier → Corrector
  3-agent: Producer → Verifier → Publisher (verify then deliver)
```

**Example — 2-agent Produce + Verify:**

```
Task: "Produce an output and verify its correctness."
Topology: linear
Pattern: produce_verify
Agents: Producer → Verifier

.system/ tree produced:
├── artifacts/
│   ├── work/
│   │   └── output.md                ← Producer
│   └── verification/
│       └── report.md                ← Verifier

Producer:
  tools: [file_read, content_search]
  system_prompt: "You are a [domain] specialist. [Task-specific
    instructions].

    Write complete output to .system/artifacts/work/output.md.
    Respond with a summary of what you produced and key decisions."
  assignment: "[Specific task]. Write to .system/artifacts/work/output.md.
    Respond with summary."
  expected_output: "Summary of output with key decisions noted.
    Full output in store."

Verifier:
  tools: [file_read]
  system_prompt: "You are a verification specialist. Review the
    Producer's work for correctness, completeness, and quality.

    For each issue found:
    - Location (section/line)
    - Issue type (error/omission/quality)
    - Severity (blocking/minor)
    - Suggested fix (one line)

    Write full verification to .system/artifacts/verification/report.md.
    Respond: PASS (no blocking issues) or FAIL (blocking issues found)
    with issue count."
  assignment: "Verify the work in <previous_agent_outputs>. Read full
    output at .system/artifacts/work/output.md. Write verification
    to .system/artifacts/verification/report.md."
  expected_output: "PASS or FAIL with issue count and summary.
    Full verification report in store."

  Transfer note: Verifier receives the summary, reads the full output
  from store. Its response is a binary verdict + issue count.
  Downstream (if any) gets an unambiguous signal.
```

### Pattern 4: Transform + Deliver

Working files flow through the pipeline in `.system/`, getting transformed at each stage. The final agent "delivers" — it takes the working output and places it where it belongs (the user's project, a specific format, a specific location).

**Transfer dynamic**: The transfer concept in action. Early agents work in `.system/` (supporting). The final agent reads from `.system/` and writes to the project root (the goal). The designer explicitly orchestrates this boundary crossing.

**`.system/` dynamic**: Working files accumulate in `.system/artifacts/`. Each agent reads from the previous stage's path and writes to its own. The final agent reads from `.system/` and writes to the project. The path change signals the conceptual shift from "working on it" to "delivering it."

**Agent default knowledge respected**: Agents know how to read, transform, and write files. The designer tells them the INPUT path, the OUTPUT path, and the TRANSFORMATION. The path itself communicates whether this is working material (.system/) or the deliverable (project root).

**Typical counts**: 2-5 agents.

```
Examples to build:
  2-agent: Generator → Publisher
  3-agent: Researcher → Drafter → Publisher
  4-agent: Planner → Builder → Reviewer → Deployer
```

**Example — 3-agent Transform + Deliver:**

```
Task: "Research, draft, and publish a document to the project."
Topology: linear
Pattern: transform_deliver
Agents: Researcher → Drafter → Publisher

.system/ tree produced:
├── artifacts/
│   ├── research/
│   │   └── notes.md                 ← Researcher (supporting)
│   └── drafts/
│       └── document.md              ← Drafter (supporting)

Project root:
├── docs/
│   └── output.md                    ← Publisher (THE GOAL)

Researcher:
  tools: []
  system_prompt: "You are a research specialist. Gather information
    on the assigned topic. Write notes to
    .system/artifacts/research/notes.md."
  assignment: "Research the topic. Write to
    .system/artifacts/research/notes.md. Respond with key findings."
  expected_output: "Key findings list. Full notes in store."

Drafter:
  tools: []
  system_prompt: "You are a document drafter. Turn research into a
    structured draft. Read research at
    .system/artifacts/research/notes.md. Write draft to
    .system/artifacts/drafts/document.md."
  assignment: "Draft the document from findings in
    <previous_agent_outputs>. Full research at
    .system/artifacts/research/notes.md. Write to
    .system/artifacts/drafts/document.md."
  expected_output: "Draft document. Summary of structure and sections."

Publisher:
  tools: [file_write]
  system_prompt: "You are a publisher. Read the final draft from
    .system/artifacts/drafts/document.md. Apply any final formatting.
    Write to the project at docs/output.md.

    This is the DELIVERY step — you are writing the actual
    deliverable to the user's project, not to .system/."
  assignment: "Read draft at .system/artifacts/drafts/document.md.
    Format and publish to docs/output.md."
  expected_output: "Published document at docs/output.md. Confirmation
    of final format and structure."

  Transfer note: The path change from .system/artifacts/ to docs/
  IS the transfer. The Publisher crosses the boundary from supporting
  work to the actual deliverable. The designer makes this explicit.
```

### Pattern 5: Evidence Chain

Multiple agents produce evidence, each building on the previous. The final agent collates all evidence into a report. Every claim is traceable back through the chain.

**Transfer dynamic**: Each agent's response is a compressed evidence summary. The store has the full evidence with references. The chain is auditable — you can trace any claim in the final report back through each agent's store files.

**`.system/` dynamic**: Structured evidence directories. Each agent writes to a labeled path. The final agent reads ALL evidence paths to produce the collated output. File naming reflects the evidence stage: `findings/`, `verification/`, `analysis/`.

**Agent default knowledge respected**: Agents know how to gather evidence, verify claims, and write reports. The designer tells them WHERE to find upstream evidence, WHAT criteria to apply, and WHERE to write their own.

**Typical counts**: 3-5 agents.

```
Examples to build:
  3-agent: Investigator → Verifier → Reporter
  4-agent: Scanner → Analyzer → Prioritizer → Reporter
  5-agent: 2 Investigators → Correlator → Analyst → Reporter
```

### Pattern 6: Workspace Coordination

Multiple agents work on different parts of the same artifact. Unlike parallel collection (where agents work independently), workspace agents are aware of each other's work areas and must avoid conflicts. The designer assigns non-overlapping zones.

**Transfer dynamic**: Each agent's response summarizes what it contributed. The store has the actual sections/components. A coordinator agent (or the final agent) assembles the pieces.

**`.system/` dynamic**: Shared artifact directory with clear ownership per agent. Each agent writes to its assigned section: `chapters/intro.md`, `chapters/methodology.md`, `chapters/results.md`. The assembler reads all sections.

**Agent default knowledge respected**: Agents know how to write in their domain. The designer assigns SCOPE (which section), CONSTRAINTS (length, style, what to cover), and AWARENESS (what other agents are covering, so they don't duplicate).

**Critical designer decision**: Scope boundaries must be explicit. "You handle sections 1-3, they handle sections 4-6" prevents overlap. The designer also sets a style reference so the assembled document reads cohesively.

**Typical counts**: 3-6 agents (2-4 workers + 1 assembler, optional coordinator).

```
Examples to build:
  3-agent: 2 Section Writers → Assembler
  4-agent: 3 Section Writers → Editor/Assembler
  5-agent: Coordinator → 3 Workers → Assembler
```

### Pattern 7: Progressive Detail

A broad, shallow pass first, then progressively deeper passes. Each pass adds detail to the previous. Unlike sequential refinement (which transforms the output), progressive detail expands it — the broad sketch remains, the detail fills in around it.

**Transfer dynamic**: Each pass's response is the current state at that resolution level. The store has the working artifact. Later passes read the artifact, add detail, and write back (or write a more detailed version).

**`.system/` dynamic**: A single evolving artifact or a set of increasingly detailed files. The naming reflects depth: `outline.md` → `expanded_outline.md` → `full_document.md`. Or versioned: `plan_v1.md` (broad) → `plan_v2.md` (detailed) → `plan_v3.md` (implementation-ready).

**Agent default knowledge respected**: Each agent knows how to work at its assigned level of detail. The designer tells them the CURRENT level, the TARGET level, and what specifically to add. Never explain what "more detail" means — the agent knows.

**Typical counts**: 2-4 agents.

```
Examples to build:
  2-agent: Outliner → Expander
  3-agent: Architect → Designer → Implementer
  4-agent: Surveyor → Planner → Detailer → Executor
```

### Pattern 8: Format Bridge

Data enters in one format and must exit in another. Agents handle the transformation stages. Each agent is a specialist in its format domain — one reads the source format, another structures the intermediate representation, another renders the target format.

**Transfer dynamic**: Each agent's response describes the transformation it applied. The store has the actual data at each stage. The format specialist knows its format natively — the designer just tells it WHERE to read and WHERE to write.

**`.system/` dynamic**: Format-specific files at each stage. Path naming reflects the format: `raw/input.csv` → `structured/data.json` → `output/report.html`. Each file is a complete representation of the data in that format.

**Agent default knowledge respected**: Agents know their format domains. A CSV parser knows how to parse CSV. A JSON structurer knows how to design schemas. An HTML renderer knows how to build pages. The designer tells them WHAT data they're working with and any domain-specific constraints, not how to handle the format.

**Typical counts**: 2-4 agents.

```
Examples to build:
  2-agent: Reader → Renderer
  3-agent: Parser → Transformer → Renderer
  3-agent: Extractor → Structurer → Generator
```

## Retrieval at Design Time

### When Retrieval Happens

Two injection points in the pipeline:

**1. Builder retrieval** — when the builder is about to configure a team. The node's task description is embedded and matched against examples. The builder sees 2-3 relevant examples showing team structures for similar tasks — agent counts, roles, dependencies, plans.

**2. Designer retrieval** — when the designer is about to write prompts. The builder's plan + roster is embedded and matched. The designer sees 2-3 relevant examples showing prompt craft — system prompts, assignments, expected_outputs, transfer patterns.

### How Retrieval Works

```
1. Embed the query
   Builder: embed(node.task_description)
   Designer: embed(builder.plan + roster_summary)

2. Filter by metadata
   agent_count: ±1 of actual count (3-agent task gets 2-4 agent examples)
   topology: prefer matching topology, fall back to any

3. Similarity search
   SELECT * FROM example_library
   WHERE agent_count BETWEEN $count - 1 AND $count + 1
   ORDER BY embedding <=> $query_embedding
   LIMIT 3

4. Inject into prompt
   <relevant_examples>
     <example task="..." agents="3" pattern="sequential_refinement">
       [full example case]
     </example>
     <example task="..." agents="3" pattern="produce_verify">
       [full example case]
     </example>
   </relevant_examples>
```

### What The Builder Sees

The builder receives examples focused on **structure decisions**: how many agents, what roles, what dependencies, what plan format.

```xml
<relevant_examples>
  <example task="Research a topic and produce a report" agents="3"
           pattern="sequential_refinement" topology="linear">
    Agents: Researcher → Synthesizer → Writer
    Dependencies: Researcher→Synthesizer, Synthesizer→Writer
    Plan: "Research assigned topic. Synthesizer groups findings by
      theme. Writer produces final document."
    Transfer: Each agent writes full work to .system/, responds lean.
      Writer delivers to project root.
  </example>
</relevant_examples>
```

The builder sees the structural DNA — team shape, dependency graph, plan style. It doesn't need the full system prompts (that's the designer's job).

### What The Designer Sees

The designer receives examples focused on **prompt craft**: how to write system prompts, how to phrase assignments, how to set expected_output, how to orchestrate file paths.

```xml
<relevant_examples>
  <example task="Research a topic and produce a report" agents="3"
           pattern="sequential_refinement">
    <agent name="Researcher">
      system_prompt: "You are a research specialist. Search for current
        information on the assigned topic. For each finding: the specific
        claim, source, date, reliability.
        Write to .system/artifacts/research/raw_findings.md.
        Respond with structured claims list."
      assignment: "Research the topic. Write to .system/artifacts/research/
        raw_findings.md. Respond with compact claims list."
      expected_output: "Numbered claims list. Each: claim, source, date,
        reliability. Full notes in store."
    </agent>
    ...
  </example>
</relevant_examples>
```

The designer sees the craft — prompt structure, brevity, file path patterns, expected_output format, transfer dynamics. It learns by example, not by instruction.

## Guidelines for Writing Examples

### The Four Rules

**1. Brief like a colleague.**
Every system prompt assumes the agent is a senior professional. No definitions. No explanations of basic concepts. The prompt says WHAT to do, WHERE to find input, WHERE to put output. It never says HOW to do the actual work.

Test: if you removed a sentence from the system prompt and the agent would still know what it means, that sentence shouldn't be there.

**2. Standardize transfer format within a pipeline.**
When multiple agents feed into a merge point, their output formats must be identical. The designer picks a format and enforces it across all feeder agents. The merge agent shouldn't have to parse two different structures.

Test: can the merge agent process all inputs with a single parsing logic?

**3. Make the .system/ tree predictable.**
Path naming should follow a consistent convention within each example. Group by purpose (`research/`, `drafts/`, `output/`), not by agent name. An agent's output path should make sense even if you don't know which agent wrote it.

Test: can you read the .system/ tree and understand the pipeline flow without reading any prompts?

**4. Separate working files from deliverables.**
Everything in `.system/` is supporting material. The deliverable goes to the project root (or is the final agent's response). The designer makes this boundary explicit in the final agent's assignment.

Test: is it clear which file is the actual output the user asked for?

### Quality Criteria

Each example is rated on:

- **Transfer efficiency** — does each agent's response contain exactly what the next agent needs? No more, no less?
- **Prompt brevity** — are system prompts under 150 tokens for simple roles, under 300 for complex ones?
- **Store coherence** — do the .system/ paths form a readable tree? Would a human scanning the directory understand the pipeline flow?
- **Format chain** — does each expected_output match the next agent's assignment expectations?
- **Default knowledge respect** — does any prompt explain something the model already knows?

### Example Count Targets

| Agent Count | Examples | Why |
|-------------|----------|-----|
| 1 | 10-12 | Cover common single-agent tasks. Teach restraint — don't add agents when one will do. |
| 2 | 12-15 | Producer-consumer pairs. The most common real pattern. |
| 3 | 12-15 | Where real design thinking starts. Linear chains, fan-in. |
| 4 | 8-10 | Diamonds, fan-out + merge. Structural complexity begins. |
| 5 | 6-8 | Complex topologies. Parallel specialists + coordination. |
| 6+ | 4-6 | Large teams. Rare but important — scope management, workspace coordination. |
| **Total** | **~55-65** | Enough to cover the combinatorial space without redundancy. |

Each count bracket should cover multiple patterns. Not all patterns appear at all counts — a 1-agent task never uses "parallel collection."

| Count | Patterns |
|-------|----------|
| 1 | Single specialist (no pattern — baseline) |
| 2 | Sequential refinement, produce + verify, transform + deliver, format bridge |
| 3 | Sequential refinement, parallel collection + merge, produce + verify, evidence chain, transform + deliver |
| 4 | Parallel collection + merge, evidence chain, workspace coordination, progressive detail, diamond variants |
| 5+ | Complex combinations: parallel collection → verification → delivery, workspace + coordination, multi-stage evidence |

## Building The Library

### Phase 1: Core Patterns (8-10 examples)

One example per pattern, at the most natural agent count for that pattern. These are the reference implementations — the highest quality, most carefully crafted examples.

### Phase 2: Count Variants (20-25 examples)

Each core pattern expanded to adjacent agent counts. The 3-agent sequential refinement becomes a 2-agent and 4-agent variant. This teaches the builder how patterns scale up and down.

### Phase 3: Domain Coverage (20-25 examples)

The patterns from Phase 1-2 instantiated in specific domains — security, code, research, creative, data. These show domain-specific conventions (security uses severity levels, creative uses style bibles, data uses schemas) while maintaining the abstract pattern structure.

### Phase 4: Edge Cases (5-10 examples)

Complex topologies, large teams, unusual patterns. The 7-agent pipeline with parallel branches and multiple merge points. The workspace coordination with 5 writers and an editor. These are rare but important for handling ambitious user requests.

## Storage Architecture

### Database

```sql
CREATE TABLE example_library (
  example_id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  task_description TEXT NOT NULL,
  agent_count      INT NOT NULL,
  topology         TEXT NOT NULL,
  pattern          TEXT NOT NULL,
  domain_tags      TEXT[] NOT NULL DEFAULT '{}',
  agents           JSONB NOT NULL,
  dependencies     JSONB NOT NULL DEFAULT '[]',
  system_tree      TEXT NOT NULL,
  plan             TEXT NOT NULL,
  transfer_notes   TEXT NOT NULL,
  quality_score    FLOAT NOT NULL DEFAULT 0.8,
  embedding        vector(384) NOT NULL,
  created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_example_library_embedding
  ON example_library USING hnsw (embedding vector_cosine_ops);

CREATE INDEX idx_example_library_agent_count
  ON example_library (agent_count);
```

### Embedding Generation

Same infrastructure as the system store: `fastembed-rs` running `all-MiniLM-L6-v2` locally. Embeddings are generated when examples are created and stored directly in the `embedding` column.

### Retrieval Query

```sql
SELECT example_id, task_description, agents, dependencies,
       system_tree, plan, transfer_notes, pattern, topology,
       1 - (embedding <=> $1) as relevance
FROM example_library
WHERE agent_count BETWEEN $2 AND $3
  AND quality_score >= 0.7
ORDER BY embedding <=> $1
LIMIT 3;
```

## What This Builds On

| Capability | Already built / planned | Example Library adds |
|------------|------------------------|---------------------|
| Builder system prompt | Static examples in `config/archetype/workforce/builder/system.md` | Dynamic example retrieval via similarity |
| Designer system prompt | Static example in `config/designer/system.md` | Dynamic prompt craft examples via similarity |
| pgvector | Planned for system store (system_files.embedding) | Shared infrastructure, new table |
| fastembed-rs | Planned for system store | Shared infrastructure |
| System store (.system/) | Vision doc: real files, Postgres metadata | Examples demonstrate .system/ patterns |
| Designer ReAct loop | Vision doc: multi-turn with store tools | Examples show the loop in action |
| expected_output | Vision doc: 4th field in designer output | Examples demonstrate format chain verification |
| Builder brevity | Current: "1-2 sentences" role descriptions | Examples reinforce this constraint |

## What This Doesn't Change

- The builder's tools and decision-making process — unchanged.
- The designer's ReAct loop — unchanged. Examples are injected context, not new behavior.
- The roster, dependencies, and capabilities — still set by the builder.
- The system store architecture — unchanged. Examples demonstrate patterns, they don't modify the store.
- The DAG execution engine — unchanged.
- The board submit pipeline — unchanged.

## The Goal

A designer that has never seen "build a security audit pipeline" before receives 2-3 examples of similar pipelines and produces prompts indistinguishable from hand-crafted ones. Not because it was taught security — it already knows security. Because it was shown HOW to orchestrate agents, HOW to write file paths, HOW to phrase assignments that respect the next agent's knowledge, HOW to set expected_output that maintains the format chain.

The examples don't teach domain knowledge. They teach orchestration craft.
