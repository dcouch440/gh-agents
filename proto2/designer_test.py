"""
Agent Designer Prototype v2 — Pre-lifecycle prompt generator with validation.

Phase 1: Designer generates (tools, system_prompt, task_prompt) per agent
Phase 2: Run the first agent against mock files to validate prompt quality

The belief format ([tag | confidence] one-sentence findings) is adapted from:
  Belief-Oriented Conversation Architecture (BOCA) — Couch, 2026
  See: proto/paper.md

Improvements over v1:
  - Meta few-shot: worked example in system prompt (code review domain)
  - Verification access: tool assignment guidance for evaluation agents
  - Upstream context scenario: tests verified_upstream belief
  - Phase 2: actually executes Scanner agent with real tool_use calls
  - Planted contradictions: objective scoring of agent output

Usage:
    python3 proto2/designer_test.py                              # stories, phase 1
    python3 proto2/designer_test.py -s security                  # security, phase 1
    python3 proto2/designer_test.py --run-agent                  # stories + execute Scanner
    python3 proto2/designer_test.py -s stories_upstream          # with upstream context
"""

import argparse
import json
import re
import shutil
import tempfile
import time
from datetime import datetime
from pathlib import Path
from textwrap import dedent

import anthropic
from dotenv import load_dotenv

load_dotenv(Path(__file__).resolve().parent.parent / ".env")

# ===========================================================================
# CONFIG
# ===========================================================================

MODEL = "claude-sonnet-4-5-20250929"
AGENT_MODEL = "claude-sonnet-4-5-20250929"  # model for Phase 2 agent execution
MAX_TOKENS = 16384
TEMPERATURE = 0.4

RESULTS_DIR = Path(__file__).resolve().parent
LOG_FILE = RESULTS_DIR / "designer_test.log"
OUTPUT_FILE = RESULTS_DIR / "designer_test_output.json"

client = anthropic.Anthropic()

# ===========================================================================
# LOGGING
# ===========================================================================

LOG_FILE.write_text("")


def log(msg: str, level: str = "INFO"):
    ts = datetime.now().strftime("%H:%M:%S.%f")[:-3]
    line = f"[{ts}] [{level:>5}] {msg}"
    print(line, flush=True)
    with open(LOG_FILE, "a") as f:
        f.write(line + "\n")


def log_sep(title: str):
    log("")
    log("=" * 72)
    log(f"  {title}")
    log("=" * 72)


def log_block(title: str, content: str, max_lines: int = 0):
    """Log a large block of text with a header."""
    lines = content.strip().split("\n")
    truncated = False
    if max_lines and len(lines) > max_lines:
        lines = lines[:max_lines]
        truncated = True
    log(f"--- {title} ({len(content)} chars) ---")
    for line in lines:
        with open(LOG_FILE, "a") as f:
            f.write(f"  | {line}\n")
        print(f"  | {line}", flush=True)
    if truncated:
        total = len(content.strip().split("\n"))
        log(f"  ... ({total - max_lines} more lines, see log file)")
    log(f"--- end {title} ---")


# ===========================================================================
# SYSTEM PROMPT (21 beliefs + worked example)
# ===========================================================================

DESIGNER_SYSTEM_PROMPT = dedent("""\
<identity>
You are the Agent Designer. You transform mission briefs and agent rosters
into optimized prompt pairs (system prompt + task prompt) for each agent
in a task force. Your output directly determines how well the crew performs.
</identity>

<beliefs>
These are your operating beliefs — internalized findings from prompt engineering research, formatted as BOCA-style belief slices (see: Belief-Oriented Conversation Architecture, Couch 2026). Each carries a confidence weight reflecting the strength of evidence behind it.

[identity_specificity | 0.90] Agents with a named role, domain, and expertise level ("a security engineer specializing in auth flow analysis") produce more focused output than generic identities.

[user_as_authority | 0.85] Task context and work assignments belong in the user message, not the system prompt — models treat user-provided content as ground truth with higher attention weight.

[positive_framing | 0.80] Positive instructions ("return raw JSON only") outperform negative instructions ("don't wrap in markdown") — negatives can paradoxically increase the unwanted behavior.

[consequence_context | 0.80] Pairing instructions with their WHY ("output is parsed by JSON.parse(), wrapper text causes errors") helps models generalize the rule to novel situations.

[moderate_verbs | 0.85] Moderately specific verbs (analyze, evaluate, review) outperform maximally specific verbs (microscopically dissect, exhaustively enumerate) with -0.89 correlation to over-specificity.

[xml_structuring | 0.75] XML tags (<context>, <assignment>, <output_format>) clearly delineate prompt sections, reducing misinterpretation and enabling agents to reference sections by name.

[queries_at_bottom | 0.90] Place context and data first, the actual task instruction last — end-of-context positioning improves output quality by up to 30%.

[explanation_first | 0.80] Structure output so reasoning precedes conclusions — forces the model to think before deciding, yielding more thorough analysis (33% -> 92% with schema field ordering).

[tool_least_privilege | 0.85] Reference only the tools each agent actually has — mentioning unavailable tools causes confusion and hallucinated tool calls.

[pipeline_position | 0.80] Agents that understand their position ("you receive Scanner's findings, your analysis feeds to Reporter") scope their work appropriately and avoid over-reaching.

[downstream_consumers | 0.75] Specifying who consumes an agent's output and how ("the Analyzer cannot re-read files, so include enough quoted context") produces more usable deliverables.

[clear_deliverables | 0.85] Defining what "done" looks like — output format, structure, content expectations — prevents agents from producing vague or unusable results.

[effort_calibration | 0.75] Match effort framing to task scope: "scan and list" for extraction, "methodically evaluate each case" for analysis — miscalibrated effort wastes tokens or produces shallow results.

[heuristic_over_rigid | 0.80] Encode judgment frameworks and strategies, not if-else checklists — models generalize better from heuristics describing how a skilled practitioner approaches the work.

[exploratory_prompts | 0.85] When the environment is unknown, guide agents to discover using their tools ("use grep to find auth-related files, then examine each") rather than asserting specifics you cannot verify.

[verified_upstream | 0.85] When upstream agents have produced real findings from the environment, reference those specifics freely — they are verified ground truth, not hallucination.

[few_shot_examples | 0.80] 3-5 diverse examples improve structured output accuracy by 15-40% — include examples when the task involves novel formats or complex classification.

[tool_usage_patterns | 0.80] Describing tool usage patterns with 1-5 examples per tool improves accuracy from 72% to 90% — show agents how to use tools, not just that they exist.

[tone_moderation | 0.75] "Use X when..." outperforms "CRITICAL: you MUST..." on Claude 4.x — moderate directive tone produces higher compliance than urgent imperatives.

[context_budget | 0.80] Minimize low-signal tokens — context rot degrades recall as token count grows; find the smallest set of high-signal tokens that maximize the desired outcome.

[description_routing | 0.75] Agent descriptions for routing ("retrieves capital cities for countries") serve a different purpose than system prompts — keep them third-person, under 20 words, capability-focused.
</beliefs>

<what_you_produce>
For each agent in the roster, assign tools and generate a system prompt and task prompt.

TOOL ASSIGNMENT:
- Review the available_capabilities pool and each agent's role description
- Assign each agent ONLY the tools they need for their specific role
- An agent that searches needs grep + file_read; one that writes output needs file_write
- Consider verification access: agents that evaluate upstream findings benefit from
  read-only tools (file_read, grep) to spot-check quoted passages, even when upstream
  output is nominally complete. Unverifiable claims degrade trust in the pipeline.
- Never assign tools an agent's role doesn't require — unused tools waste context

The SYSTEM PROMPT contains:
- Role identity: specific, domain-aware, with expertise level
- Behavioral guidelines: how to approach work, what quality looks like
- Tool usage instructions: for their assigned tools ONLY, with concrete usage patterns
- When the task involves classification or structured output, include 1-2 concrete
  examples showing what good output looks like — this improves accuracy by 15-40%
- Pair key instructions with consequences ("include file paths because the Patcher
  uses your exact references — incorrect locations cause failed patches")
- Collaboration context: who comes before them (inputs), who comes after (consumers)
- Encode heuristics and judgment frameworks, not rigid templates or checklists
- 200-600 tokens. Enough for identity and behavior, not overloaded with context.

The TASK PROMPT contains:
- Mission context rendered as project briefing (what the team is doing and why)
- Upstream outputs from previous agents (if not first agent), presented as inputs to build on
- Their specific assignment within the mission
- Expected deliverable description
- The actual task instruction at the END of the prompt
- 300-2000 tokens depending on context richness. This is where the work lives.

Design reasoning: For each agent, include a brief note on why you made the
design choices you did — tool assignment rationale, identity framing, verb
selection, context ordering. This is for observability and debugging.
</what_you_produce>

<example>
This is one well-designed agent from a code review task force. Notice: identity
specificity, tool usage patterns with examples, an embedded output example,
consequence context on key instructions, and heuristic framing over rigid templates.

Agent: Reviewer (2nd of 3 agents, receives Linter output, feeds to Patcher)
Tools: [file_read, grep]

SYSTEM PROMPT:
"You are Reviewer, a senior code quality analyst specializing in maintainability
and correctness review for backend services.

You have access to:
- grep: Search for patterns across the codebase. Use this to check if a flagged
  issue is isolated or systemic. Example: grep 'unwrap()' src/**/*.rs
- file_read: Read file contents for deeper analysis. Use this when grep results
  need surrounding context to evaluate properly.

You receive flagged issues from the Linter. For each, evaluate severity and
recommend action. Structure your evaluation as reasoning first, then verdict:

<example_evaluation>
Issue: Unnecessary clone() in hot path (src/api/handlers.rs:47)
Reasoning: The cloned value is a String passed to a function that accepts &str.
  The clone allocates on every request. At ~1000 req/s, this creates measurable
  GC pressure.
Severity: MODERATE
Action: Replace .clone() with .as_str() — zero allocation, same semantics.
</example_evaluation>

Produce structured evaluations the Patcher can act on directly. Include file
paths and line numbers — the Patcher applies fixes using your exact references,
so incorrect locations cause failed patches."

TASK PROMPT:
"<context>
The team is reviewing a Rust API service before release. The Linter completed
static analysis and flagged 23 issues across 8 files.
</context>

<linter_findings>
{upstream output injected here}
</linter_findings>

<assignment>
Review each flagged issue. For issues in shared modules, use grep to check if
the pattern appears elsewhere. Use file_read when the Linter's snippet needs
more context.

For each issue: reasoning, severity (HIGH/MODERATE/LOW), and a specific action.
Group related issues when they share a root cause.

Produce evaluations as a structured list the Patcher can process sequentially.
</assignment>"
</example>

<output_schema>
Respond with a JSON object. The output is parsed directly by a JSON parser.
Wrapper text, markdown fences, or explanatory prose outside the JSON will
cause parsing errors.

{
  "agents": [
    {
      "agent_id": "<id from roster>",
      "agent_name": "<name from roster>",
      "tools": ["<capability from available pool>", "..."],
      "system_prompt": "<the generated system prompt>",
      "task_prompt": "<the generated task prompt>",
      "reasoning": "<tool assignment rationale + prompt design choices>"
    }
  ]
}

Every tool in "tools" MUST come from the available_capabilities pool.
Produce one entry per agent in the roster, in execution_order.
</output_schema>
""")

# ===========================================================================
# TEST SCENARIOS
# ===========================================================================

CAPABILITY_DESCRIPTIONS = {
    "file_read": "file_read: Read file contents from the repository",
    "file_write": "file_write: Create or modify files in the repository",
    "grep": "grep: Search file contents with regex patterns",
    "shell": "shell: Execute shell commands in a sandboxed environment",
    "git": "git: Run git operations (status, diff, log, commit, branch)",
    "github_api": "github_api: Interact with GitHub API (issues, PRs, reviews)",
    "web_search": "web_search: Search the web for information",
    "database_query": "database_query: Execute read-only SQL queries",
}


def format_capabilities(caps: list[str]) -> str:
    return "\n".join(f"- {CAPABILITY_DESCRIPTIONS.get(c, c)}" for c in caps)


# Synthetic Scanner output for upstream context testing
SCANNER_UPSTREAM_OUTPUT = dedent("""\
<upstream_step name="Scanner">
## Character Reference Inventory

### Elena References
- story1.txt (lines 3, 8, 15): "bright blue eyes", teacher for 10 years, age 32
- story2.txt (lines 2, 7, 12): "dark brown eyes", Dr. Elena, medical practice
- story4.txt (line 4): "pushed her glasses up — a habit from her teaching days"

### Marcus References
- story1.txt (lines 10, 16): "just turned forty", gray at temples
- story3.txt (lines 3, 8): "at thirty-five", set 2 years after story1
- story4.txt (line 5): "now forty-two"

## Potential Contradictions Flagged

1. EYES — Elena's eye color conflicts:
   story1.txt:3 "her bright blue eyes scanning the room"
   story2.txt:7 "She looked up with her dark brown eyes"

2. PROFESSION — Elena's career conflicts:
   story1.txt:8 "spent a decade as a teacher"
   story2.txt:2 "Dr. Elena adjusted her stethoscope. After years in her medical practice"
   story4.txt:4 "a habit from her teaching days" (supports story1)

3. AGE — Marcus age timeline impossible:
   story1.txt:10 "He'd just turned forty"
   story3.txt:3 "At thirty-five, Marcus still had the same restless energy"
   story3.txt is set 2 YEARS AFTER story1, but Marcus is 5 years younger.
   story4.txt:5 "now forty-two" (consistent with story1 + 2 years)

4. LOCATION — Cafe location conflicts:
   story1.txt:15 "the cafe on Main Street"
   story3.txt:8 "their usual spot on Oak Avenue"

Coverage: All 4 files in /stories/ scanned. Elena appears in 3/4, Marcus in 3/4.
</upstream_step>
""")


SCENARIOS = {
    "stories": {
        "name": "Story Contradiction Audit",
        "mission": {
            "task_description": "Audit the /stories directory for narrative contradictions across shared characters. The anthology has 12 short stories by multiple authors. Characters 'Elena' and 'Marcus' appear across 4 stories as shared universe characters. Inconsistencies have accumulated over months of independent writing.",
            "failure_mode": "fail_fast",
            "downstream_context": "The final report will be used by the editorial team for their pre-publication review pass.",
        },
        "allowed_capabilities": ["file_read", "grep", "file_write"],
        "roster": [
            {"id": "agent-scanner-001", "name": "Scanner", "role": "Systematically search story files for shared character references and flag potential contradictions", "order": 1},
            {"id": "agent-analyzer-002", "name": "Analyzer", "role": "Evaluate flagged contradictions for severity, categorize by type, and determine root causes", "order": 2},
            {"id": "agent-reporter-003", "name": "Reporter", "role": "Produce a structured contradiction report with findings, severity ratings, and recommended fixes", "order": 3},
        ],
        "upstream_context": "No upstream outputs available. This is the first step in the workflow.",
    },
    "stories_upstream": {
        "name": "Story Contradiction Audit (with upstream Scanner findings)",
        "mission": {
            "task_description": "Audit the /stories directory for narrative contradictions across shared characters. The anthology has 12 short stories by multiple authors. Characters 'Elena' and 'Marcus' appear across 4 stories as shared universe characters. Inconsistencies have accumulated over months of independent writing.",
            "failure_mode": "fail_fast",
            "downstream_context": "The final report will be used by the editorial team for their pre-publication review pass.",
        },
        "allowed_capabilities": ["file_read", "grep", "file_write"],
        "roster": [
            {"id": "agent-analyzer-002", "name": "Analyzer", "role": "Evaluate flagged contradictions for severity, categorize by type, and determine root causes", "order": 1},
            {"id": "agent-reporter-003", "name": "Reporter", "role": "Produce a structured contradiction report with findings, severity ratings, and recommended fixes", "order": 2},
        ],
        "upstream_context": SCANNER_UPSTREAM_OUTPUT,
    },
    "security": {
        "name": "Security Vulnerability Audit",
        "mission": {
            "task_description": "Audit the /src directory for common security vulnerabilities (OWASP Top 10). Focus on SQL injection, XSS, authentication bypasses, and insecure deserialization. The codebase is a Node.js Express API with PostgreSQL.",
            "failure_mode": "continue_on_error",
            "downstream_context": "Findings will be filed as GitHub issues and assigned to the security team for remediation.",
        },
        "allowed_capabilities": ["file_read", "grep", "shell", "github_api"],
        "roster": [
            {"id": "agent-recon-001", "name": "Recon", "role": "Map the attack surface: identify entry points, authentication boundaries, data flow paths, and external integrations", "order": 1},
            {"id": "agent-auditor-002", "name": "Auditor", "role": "Analyze each identified entry point for OWASP Top 10 vulnerabilities with severity ratings", "order": 2},
            {"id": "agent-filer-003", "name": "Filer", "role": "Create GitHub issues for each confirmed vulnerability with reproduction steps and remediation guidance", "order": 3},
        ],
        "upstream_context": "No upstream outputs available. This is the first step in the workflow.",
    },
    "docs": {
        "name": "API Documentation Generation",
        "mission": {
            "task_description": "Generate comprehensive API documentation for the /src/api directory. The codebase uses Rust with Axum. Each endpoint needs method, path, request/response schemas, authentication requirements, and usage examples documented.",
            "failure_mode": "fail_fast",
            "downstream_context": "Documentation will be published to the developer portal and must follow OpenAPI 3.0 conventions.",
        },
        "allowed_capabilities": ["file_read", "grep", "file_write"],
        "roster": [
            {"id": "agent-mapper-001", "name": "Mapper", "role": "Discover all API endpoints, extract route definitions, handler signatures, and middleware chains", "order": 1},
            {"id": "agent-documenter-002", "name": "Documenter", "role": "For each endpoint, analyze the handler code and produce detailed documentation including schemas, auth requirements, and error responses", "order": 2},
            {"id": "agent-writer-003", "name": "Writer", "role": "Compile all endpoint documentation into a cohesive API reference with table of contents, authentication guide, and usage examples", "order": 3},
        ],
        "upstream_context": "No upstream outputs available. This is the first step in the workflow.",
    },
    "codebase": {
        "name": "Pre-Release Security & Quality Audit",
        "mission": {
            "task_description": "Audit the /src directory of a Node.js Express microservice that handles authentication and payment processing. The service processes real transactions and stores PII. Identify security vulnerabilities, performance issues, and code quality problems before the v2.2 release. All findings must be actionable with specific file references and remediation steps.",
            "failure_mode": "fail_fast",
            "downstream_context": "Findings block the release pipeline. The engineering lead triages by severity — CRITICAL blocks release, HIGH must have a remediation plan, MODERATE gets tracked.",
        },
        "allowed_capabilities": ["file_read", "grep", "shell", "file_write", "git", "database_query"],
        "roster": [
            {"id": "agent-mapper-101", "name": "Mapper", "role": "Discover codebase structure, map all entry points and route handlers, trace dependency graph and data flow boundaries between modules", "order": 1},
            {"id": "agent-secaudit-102", "name": "SecurityAuditor", "role": "Analyze entry points for OWASP Top 10 vulnerabilities including injection, broken authentication, broken access control, and credential exposure", "order": 2},
            {"id": "agent-perfanalyst-103", "name": "PerformanceAnalyst", "role": "Identify N+1 query patterns, connection pool issues, algorithmic inefficiencies, and database interaction bottlenecks", "order": 3},
            {"id": "agent-integreview-104", "name": "IntegrationReviewer", "role": "Analyze cross-module data flows for transaction boundary issues, error propagation gaps, race conditions, and state consistency problems", "order": 4},
            {"id": "agent-reportwriter-105", "name": "ReportWriter", "role": "Compile all findings into a prioritized remediation plan with severity ratings, effort estimates, and specific fix examples", "order": 5},
        ],
        "upstream_context": "No upstream outputs available. This is the first step in the workflow.",
    },
}


def build_user_prompt(scenario: dict) -> str:
    mission = scenario["mission"]
    caps = scenario["allowed_capabilities"]
    roster = scenario["roster"]
    upstream = scenario["upstream_context"]

    roster_text = ""
    for i, agent in enumerate(roster):
        roster_text += f"{i+1}. {agent['name']} (id: {agent['id']})\n"
        roster_text += f"   Role: {agent['role']}\n"
        roster_text += f"   Execution Order: {agent['order']}\n\n"

    return dedent(f"""\
<mission>
{mission['task_description']}

Failure mode: {mission['failure_mode']}
{mission.get('downstream_context', '')}
</mission>

<roster>
{roster_text.strip()}
</roster>

<upstream_context>
{upstream}
</upstream_context>

<available_capabilities>
These are the tools authorized for this task force. Assign a subset to each
agent based on their role — not every agent needs every tool.

{format_capabilities(caps)}
</available_capabilities>

For each agent in the roster, assign tools from the available pool and
design a (system prompt, task prompt) pair. Each agent's task prompt should
be written as a direct, contextual work assignment — as if a knowledgeable
team lead is handing them a brief with the right tools for the job.
""")


# ===========================================================================
# ANALYSIS — belief detection in generated output
# ===========================================================================

BELIEF_SIGNALS = {
    "identity_specificity": ["specializing in", "specialist", "expert in", "with expertise"],
    "user_as_authority": [],  # structural — task context in user message
    "positive_framing": ["produce", "return", "output", "generate"],
    "consequence_context": ["because", "since", "so that", "this ensures", "otherwise", "causes"],
    "moderate_verbs": ["analyze", "evaluate", "review", "consider", "identify", "examine"],
    "xml_structuring": ["<context>", "<assignment>", "<output", "<upstream", "<findings", "<mission", "<example"],
    "queries_at_bottom": [],  # structural check below
    "explanation_first": ["reasoning before", "explain why", "present your reasoning", "analysis before", "reasoning first"],
    "tool_least_privilege": [],  # check tools match role
    "pipeline_position": ["receives", "feeds to", "your output", "downstream", "previous agent", "from the"],
    "downstream_consumers": ["consumed by", "will be used by", "cannot re-read", "depends on", "the patcher", "the reporter", "editorial team"],
    "clear_deliverables": ["produce a", "output as", "format the", "structure it", "write to", "structured list", "structured analysis"],
    "effort_calibration": ["scan and list", "methodically", "thorough", "comprehensive", "sweep", "systematically"],
    "heuristic_over_rigid": ["approach", "strategy", "judgment", "consider whether", "framework"],
    "exploratory_prompts": ["use grep to", "use grep", "search for", "identify which", "find all", "discover"],
    "verified_upstream": ["scanner's findings", "scanner has", "linter's", "from the scanner", "from upstream", "flagged"],
    "few_shot_examples": ["<example", "example_", "for example:", "example:"],
    "tool_usage_patterns": ["use this to", "use grep", "use file_read", "use file_write", "example:"],
    "tone_moderation": [],  # absence of MUST/CRITICAL
    "context_budget": [],  # token count of generated prompts
    "description_routing": [],  # not relevant per-agent
}


def analyze_agent_output(agent: dict, allowed_caps: list[str], has_upstream: bool = False) -> dict:
    """Analyze a single agent's generated prompts for belief adherence."""
    sys = agent.get("system_prompt", "")
    task = agent.get("task_prompt", "")
    tools = agent.get("tools", [])
    combined = sys + " " + task

    analysis = {
        "name": agent["agent_name"],
        "tools_assigned": tools,
        "system_prompt_words": len(sys.split()),
        "task_prompt_words": len(task.split()),
        "beliefs_detected": [],
        "beliefs_notably_applied": [],
        "warnings": [],
    }

    # Check tool validity
    for t in tools:
        if t not in allowed_caps:
            analysis["warnings"].append(f"Tool '{t}' not in allowed pool!")

    # Check for MUST/CRITICAL in non-label contexts (tone_moderation)
    # Allow MUST/CRITICAL as severity labels but flag as directive language
    must_count = len(re.findall(r'\bMUST\b', sys))
    critical_count = len(re.findall(r'\bCRITICAL\b', sys))
    if must_count > 0 or critical_count > 0:
        # Check if they're used as severity labels vs directives
        severity_uses = len(re.findall(r'(?:severity|tier|rating|level).*?(?:MUST|CRITICAL)|(?:MUST|CRITICAL).*?(?:severity|tier|fix)', sys, re.IGNORECASE))
        directive_uses = (must_count + critical_count) - severity_uses
        if directive_uses > 0:
            analysis["warnings"].append(f"System prompt has {directive_uses} directive MUST/CRITICAL usage(s) — tone_moderation suggests softer framing")
        else:
            analysis["beliefs_notably_applied"].append("tone_moderation: MUST/CRITICAL used only as severity labels, not directives")

    # Check prompt sizes
    if analysis["system_prompt_words"] < 50:
        analysis["warnings"].append(f"System prompt very short ({analysis['system_prompt_words']} words)")
    if analysis["system_prompt_words"] > 800:
        analysis["warnings"].append(f"System prompt long ({analysis['system_prompt_words']} words) — may exceed 600 token target")
    if analysis["task_prompt_words"] < 30:
        analysis["warnings"].append(f"Task prompt very short ({analysis['task_prompt_words']} words)")

    # Detect belief signals
    for belief, signals in BELIEF_SIGNALS.items():
        for signal in signals:
            if signal.lower() in combined.lower():
                if belief not in analysis["beliefs_detected"]:
                    analysis["beliefs_detected"].append(belief)
                break

    # Structural: queries_at_bottom
    task_lines = task.strip().split("\n")
    if task_lines:
        last_section = "\n".join(task_lines[-10:]).lower()
        if any(w in last_section for w in ["scan", "evaluate", "produce", "analyze", "write", "create", "review", "begin", "start"]):
            if "queries_at_bottom" not in analysis["beliefs_detected"]:
                analysis["beliefs_detected"].append("queries_at_bottom")

    # Structural: xml_structuring in task prompt
    if re.search(r"<\w+>", task):
        if "xml_structuring" not in analysis["beliefs_detected"]:
            analysis["beliefs_detected"].append("xml_structuring")

    # Structural: few_shot_examples
    if re.search(r"<example|example_\w+>|Example:|For example:", combined, re.IGNORECASE):
        if "few_shot_examples" not in analysis["beliefs_detected"]:
            analysis["beliefs_detected"].append("few_shot_examples")
            analysis["beliefs_notably_applied"].append("few_shot_examples: embedded example in prompt")

    # Structural: user_as_authority (task context is in task prompt, identity in system)
    if len(task) > len(sys) * 0.5:  # task prompt has substantial content
        if "user_as_authority" not in analysis["beliefs_detected"]:
            analysis["beliefs_detected"].append("user_as_authority")

    # Structural: tone_moderation (absence of directives)
    if must_count == 0 and critical_count == 0:
        if "tone_moderation" not in analysis["beliefs_detected"]:
            analysis["beliefs_detected"].append("tone_moderation")

    # Structural: context_budget (prompt is reasonably sized)
    total_words = analysis["system_prompt_words"] + analysis["task_prompt_words"]
    if 100 < total_words < 1200:
        if "context_budget" not in analysis["beliefs_detected"]:
            analysis["beliefs_detected"].append("context_budget")

    # Structural: verified_upstream (only when upstream exists)
    if has_upstream and any(s.lower() in combined.lower() for s in ["scanner", "linter", "upstream", "findings"]):
        if "verified_upstream" not in analysis["beliefs_detected"]:
            analysis["beliefs_detected"].append("verified_upstream")

    return analysis


# ===========================================================================
# PHASE 2: MOCK FILES + AGENT EXECUTION
# ===========================================================================

MOCK_STORIES = {
    "story1.txt": dedent("""\
        The Meeting
        by Author A

        Elena walked into the cafe on Main Street, her bright blue eyes scanning
        the room for a familiar face. At just thirty-two, she carried the tired
        confidence of someone who had spent a decade as a teacher, shaping young
        minds while her own life quietly stalled.

        She spotted him in the corner. Marcus sat at his usual table, nursing a
        cold espresso. He'd just turned forty, and the gray at his temples showed
        it. But his smile hadn't aged a day.

        "You're late," he said.

        "Traffic," she lied. She'd been sitting in the parking lot for ten minutes,
        rehearsing what to say. The cafe on Main Street had always been their place,
        ever since university.
    """),
    "story2.txt": dedent("""\
        The Discovery
        by Author B

        Dr. Elena adjusted her stethoscope and reviewed the chart one more time.
        After years in her medical practice, she had learned to read between the
        lines of patient complaints.

        The clinic was quiet today. She preferred it that way — fewer distractions
        meant better focus. She looked up with her dark brown eyes, studying the
        waiting room through the glass partition.

        Her phone buzzed. A message from Marcus: "We need to talk. Same place
        as always."

        She sighed. Whatever he wanted, it could wait until after her shift.
        Medicine didn't pause for old friends.
    """),
    "story3.txt": dedent("""\
        The Return
        by Author C
        (Set two years after "The Meeting")

        Two years had passed since they'd last spoken. Marcus ordered his usual
        coffee at their usual spot on Oak Avenue, watching the morning crowd
        shuffle past the window.

        At thirty-five, Marcus still had the same restless energy that had
        defined him in his twenties. He tapped the table impatiently, checking
        his watch every few seconds.

        The door opened. Elena. She looked different — more confident somehow,
        like she'd finally settled into who she was meant to be.

        "You haven't changed," she said, sliding into the booth across from him.
    """),
    "story4.txt": dedent("""\
        The Resolution
        by Author D
        (Set two years after "The Meeting")

        Elena pushed her glasses up — a habit from her teaching days that she
        still hadn't broken. Marcus, now forty-two, sat across from her looking
        older but calmer than she remembered.

        "I read your letter," she said. "All of it."

        He nodded. The cafe was the same one they'd always gone to, though the
        menu had changed twice since their university days.

        "I just needed you to know," he said quietly.

        She reached across the table and squeezed his hand. Some things didn't
        need words.
    """),
}

EXPECTED_CONTRADICTIONS = [
    {"id": "eyes", "description": "Elena eye color: blue (story1) vs brown (story2)", "files": ["story1.txt", "story2.txt"]},
    {"id": "profession", "description": "Elena profession: teacher (story1/4) vs doctor (story2)", "files": ["story1.txt", "story2.txt"]},
    {"id": "age", "description": "Marcus age: 40 (story1) -> 35 (story3, set 2yr later) vs 42 (story4)", "files": ["story1.txt", "story3.txt"]},
    {"id": "location", "description": "Cafe: Main Street (story1) vs Oak Avenue (story3)", "files": ["story1.txt", "story3.txt"]},
]


# ===========================================================================
# CODEBASE SCENARIO — Mock Node.js microservice with planted issues
# ===========================================================================

CODEBASE_MOCK_FILES = {
    "src/auth/login.js": dedent("""\
        const express = require('express');
        const router = express.Router();
        const db = require('../db/queries');

        // POST /auth/login
        router.post('/login', async (req, res) => {
          const { email, password } = req.body;

          if (!email || !password) {
            return res.status(400).json({ error: 'Email and password required' });
          }

          try {
            const user = await db.findUserByEmail(email);
            if (!user) {
              return res.status(401).json({ error: 'Invalid credentials' });
            }

            // Verify password
            if (user.password_hash === password) {
              const token = generateToken(user.id);
              return res.json({ token, user: { id: user.id, email: user.email } });
            }

            return res.status(401).json({ error: 'Invalid credentials' });
          } catch (err) {
            console.error('Login error:', err);
            return res.status(500).json({ error: 'Internal server error' });
          }
        });

        function generateToken(userId) {
          const jwt = require('jsonwebtoken');
          return jwt.sign({ userId }, process.env.JWT_SECRET, { expiresIn: '24h' });
        }

        module.exports = router;
    """),

    "src/auth/middleware.js": dedent("""\
        const jwt = require('jsonwebtoken');

        // Authentication middleware — validates JWT and attaches user to request
        function authenticate(req, res, next) {
          const authHeader = req.headers.authorization;
          if (!authHeader || !authHeader.startsWith('Bearer ')) {
            return res.status(401).json({ error: 'No token provided' });
          }

          const token = authHeader.split(' ')[1];
          try {
            const decoded = jwt.verify(token, process.env.JWT_SECRET);
            req.user = decoded;
            next();
          } catch (err) {
            return res.status(401).json({ error: 'Invalid token' });
          }
        }

        // Token refresh handler — debounced to prevent concurrent refresh storms.
        // When multiple requests arrive with near-expiry tokens, only the first
        // triggers a refresh; others wait for the result. The 50ms delay is
        // intentional to batch concurrent refresh attempts.
        let refreshPending = null;
        function refreshToken(req, res) {
          if (refreshPending) {
            return refreshPending.then(token => res.json({ token }));
          }
          refreshPending = new Promise(resolve => {
            setTimeout(() => {
              const newToken = jwt.sign(
                { userId: req.user.userId },
                process.env.JWT_SECRET,
                { expiresIn: '24h' }
              );
              refreshPending = null;
              resolve(newToken);
            }, 50);
          });
          return refreshPending.then(token => res.json({ token }));
        }

        module.exports = { authenticate, refreshToken };
    """),

    "src/payments/checkout.js": dedent("""\
        const express = require('express');
        const router = express.Router();
        const { authenticate } = require('../auth/middleware');
        const db = require('../db/queries');

        // POST /payments/checkout
        router.post('/checkout', authenticate, async (req, res) => {
          const { orderId, amount } = req.body;
          const userId = req.user.userId;

          try {
            // Check user's balance
            const balance = await db.getUserBalance(userId);
            if (balance < amount) {
              return res.status(400).json({ error: 'Insufficient balance' });
            }

            // Charge the user
            await db.deductBalance(userId, amount);

            // Record the transaction
            const txn = await db.query(
              `INSERT INTO transactions (user_id, order_id, amount, status)
               VALUES (${userId}, '${orderId}', ${amount}, 'completed')
               RETURNING id, created_at`
            );

            // Update order status
            await db.updateOrderStatus(orderId, 'paid');

            return res.json({
              success: true,
              transactionId: txn.rows[0].id,
              remainingBalance: balance - amount,
            });
          } catch (err) {
            console.error('Checkout error:', err);
            return res.status(500).json({ error: 'Payment processing failed' });
          }
        });

        module.exports = router;
    """),

    "src/payments/refund.js": dedent("""\
        const express = require('express');
        const router = express.Router();
        const { authenticate } = require('../auth/middleware');
        const db = require('../db/queries');

        // POST /payments/refund
        router.post('/refund', authenticate, async (req, res) => {
          const { transactionId } = req.body;

          try {
            // Look up the transaction
            const txn = await db.getTransaction(transactionId);
            if (!txn) {
              return res.status(404).json({ error: 'Transaction not found' });
            }

            if (txn.status === 'refunded') {
              return res.status(400).json({ error: 'Already refunded' });
            }

            // Process refund — credit user and update status
            await db.addBalance(txn.user_id, txn.amount);
            await db.updateTransactionStatus(transactionId, 'refunded');

            return res.json({
              success: true,
              refundedAmount: txn.amount,
            });
          } catch (err) {
            console.error('Refund error:', err);
            return res.status(500).json({ error: 'Refund processing failed' });
          }
        });

        module.exports = router;
    """),

    "src/db/queries.js": dedent("""\
        const { Pool } = require('pg');
        const pool = require('../config/database');

        // Parameterized queries — safe from SQL injection
        async function findUserByEmail(email) {
          const result = await pool.query(
            'SELECT id, email, password_hash FROM users WHERE email = $1',
            [email]
          );
          return result.rows[0] || null;
        }

        async function getUserBalance(userId) {
          const result = await pool.query(
            'SELECT balance FROM accounts WHERE user_id = $1',
            [userId]
          );
          return result.rows[0]?.balance || 0;
        }

        async function deductBalance(userId, amount) {
          return pool.query(
            'UPDATE accounts SET balance = balance - $1 WHERE user_id = $2',
            [amount, userId]
          );
        }

        async function addBalance(userId, amount) {
          return pool.query(
            'UPDATE accounts SET balance = balance + $1 WHERE user_id = $2',
            [amount, userId]
          );
        }

        async function getTransaction(txnId) {
          const result = await pool.query(
            'SELECT * FROM transactions WHERE id = $1',
            [txnId]
          );
          return result.rows[0] || null;
        }

        async function updateTransactionStatus(txnId, status) {
          return pool.query(
            'UPDATE transactions SET status = $1 WHERE id = $2',
            [status, txnId]
          );
        }

        async function updateOrderStatus(orderId, status) {
          return pool.query(
            'UPDATE orders SET status = $1 WHERE id = $2',
            [status, orderId]
          );
        }

        // Fetch order with all line items
        async function fetchOrderWithItems(orderId) {
          const order = await pool.query(
            'SELECT * FROM orders WHERE id = $1',
            [orderId]
          );
          if (!order.rows[0]) return null;

          const items = await pool.query(
            'SELECT id FROM order_items WHERE order_id = $1',
            [orderId]
          );

          // Load full details for each item
          const fullItems = [];
          for (const item of items.rows) {
            const detail = await pool.query(
              'SELECT * FROM order_items WHERE id = $1',
              [item.id]
            );
            fullItems.push(detail.rows[0]);
          }

          return { ...order.rows[0], items: fullItems };
        }

        // Generic query helper (used by checkout for transaction inserts)
        async function query(sql, params) {
          return pool.query(sql, params);
        }

        module.exports = {
          findUserByEmail, getUserBalance, deductBalance, addBalance,
          getTransaction, updateTransactionStatus, updateOrderStatus,
          fetchOrderWithItems, query,
        };
    """),

    "src/config/database.js": dedent("""\
        const { Pool } = require('pg');

        // Database connection pool.
        // Pool size of 5 is intentional — this service handles ~50 req/s peak,
        // and each query completes in <10ms. 5 connections gives us 500 queries/s
        // throughput with headroom. Larger pools waste server-side memory and
        // increase connection overhead on the PostgreSQL side.
        const pool = new Pool({
          connectionString: 'postgresql://payments_svc:sk_live_xR7mK9pQ2wN4@db.internal:5432/payments',
          max: 5,
          idleTimeoutMillis: 30000,
          connectionTimeoutMillis: 5000,
        });

        pool.on('error', (err) => {
          console.error('Unexpected pool error:', err);
        });

        module.exports = pool;
    """),

    ".env.example": dedent("""\
        # Application
        NODE_ENV=production
        PORT=3000

        # Database — use environment-specific connection strings
        DATABASE_URL=postgresql://user:password@host:5432/dbname

        # Auth
        JWT_SECRET=your-secret-here

        # Payment provider
        STRIPE_SECRET_KEY=sk_test_...
        STRIPE_WEBHOOK_SECRET=whsec_...
    """),

    "package.json": dedent("""\
        {
          "name": "payments-service",
          "version": "2.1.0",
          "description": "Payment processing microservice",
          "main": "src/index.js",
          "scripts": {
            "start": "node src/index.js",
            "test": "jest --coverage",
            "lint": "eslint src/"
          },
          "dependencies": {
            "express": "4.17.1",
            "pg": "8.11.3",
            "jsonwebtoken": "9.0.2",
            "dotenv": "16.3.1",
            "cors": "2.8.5",
            "body-parser": "1.20.2"
          },
          "devDependencies": {
            "jest": "29.7.0",
            "eslint": "8.56.0"
          }
        }
    """),

    # Entry point file — the app wires routes together
    "src/index.js": dedent("""\
        const express = require('express');
        const cors = require('cors');
        const bodyParser = require('body-parser');
        const loginRouter = require('./auth/login');
        const { authenticate, refreshTokenDebounce } = require('./auth/middleware');
        const checkoutRouter = require('./payments/checkout');
        const refundRouter = require('./payments/refund');
        const db = require('./db/queries');

        const app = express();

        app.use(cors());
        app.use(bodyParser.json());

        // Public routes
        app.use('/auth', loginRouter);

        // Protected routes
        app.use('/payments/checkout', authenticate, checkoutRouter);
        app.use('/payments/refund', authenticate, refundRouter);

        // Health check
        app.get('/health', (req, res) => res.json({ status: 'ok' }));

        const PORT = process.env.PORT || 3000;
        app.listen(PORT, () => {
          console.log(`Service running on port ${PORT}`);
        });

        module.exports = app;
    """),
}

EXPECTED_ISSUES = [
    {"id": "sql_injection", "description": "SQL injection via string interpolation in checkout.js"},
    {"id": "timing_attack", "description": "Non-constant-time password comparison in login.js"},
    {"id": "no_rate_limit", "description": "No rate limiting on login endpoint"},
    {"id": "broken_authz", "description": "Missing ownership check on refund endpoint (IDOR)"},
    {"id": "no_transaction", "description": "Missing transaction boundary in checkout payment flow"},
    {"id": "n_plus_one", "description": "N+1 query pattern in fetchOrderWithItems"},
    {"id": "hardcoded_creds", "description": "Hardcoded database credentials in config/database.js"},
    {"id": "outdated_dep", "description": "Outdated Express version, missing helmet/security middleware"},
]


def setup_mock_files(scenario_key: str = "stories") -> Path:
    """Create a temp directory with mock files for the given scenario."""
    tmpdir = Path(tempfile.mkdtemp(prefix="designer_test_"))

    if scenario_key in ("stories", "stories_upstream"):
        stories_dir = tmpdir / "stories"
        stories_dir.mkdir()
        for name, content in MOCK_STORIES.items():
            (stories_dir / name).write_text(content)
    elif scenario_key == "codebase":
        for relpath, content in CODEBASE_MOCK_FILES.items():
            fpath = tmpdir / relpath
            fpath.parent.mkdir(parents=True, exist_ok=True)
            fpath.write_text(content)

    return tmpdir


def cleanup_mock_files(tmpdir: Path):
    shutil.rmtree(tmpdir, ignore_errors=True)


def handle_tool_call(tool_name: str, tool_input: dict, base_dir: Path) -> str:
    """Execute a mock tool call against the base directory (stories/ or project root)."""
    if tool_name == "grep":
        pattern = tool_input.get("pattern", "")
        path = tool_input.get("path", "")
        log(f"    [TOOL] grep pattern='{pattern}' path='{path}'")
        results = []
        try:
            compiled = re.compile(pattern, re.IGNORECASE)
        except re.error:
            return f"Error: invalid regex pattern '{pattern}'"
        # Search all text-like files recursively
        extensions = {"*.txt", "*.js", "*.json", "*.md", "*.env", "*.env.example"}
        seen = set()
        for ext in extensions:
            for fpath in sorted(base_dir.rglob(ext)):
                if fpath in seen:
                    continue
                seen.add(fpath)
                relpath = fpath.relative_to(base_dir)
                try:
                    for i, line in enumerate(fpath.read_text().split("\n"), 1):
                        if compiled.search(line):
                            results.append(f"{relpath}:{i}: {line.strip()}")
                except (UnicodeDecodeError, PermissionError):
                    pass
        if not results:
            return "No matches found."
        return "\n".join(results)

    elif tool_name == "file_read":
        path = tool_input.get("path", "")
        log(f"    [TOOL] file_read path='{path}'")
        # Try resolving relative to base_dir with various path normalizations
        candidates = [
            base_dir / path,
            base_dir / path.lstrip("/"),
            base_dir / Path(path).name,
        ]
        # Also try stripping leading /src/, /stories/ etc.
        stripped = path.lstrip("/")
        for prefix in ("src/", "stories/"):
            if stripped.startswith(prefix):
                candidates.append(base_dir / stripped)
                candidates.append(base_dir / stripped[len(prefix):])
        for candidate in candidates:
            if candidate.exists() and candidate.is_file():
                return candidate.read_text()
        # Fallback: search by filename
        filename = Path(path).name
        for fpath in base_dir.rglob(filename):
            if fpath.is_file():
                return fpath.read_text()
        return f"Error: file not found: {path}"

    elif tool_name == "file_write":
        path = tool_input.get("path", "")
        content = tool_input.get("content", "")
        log(f"    [TOOL] file_write path='{path}' ({len(content)} chars)")
        fpath = base_dir / path.lstrip("/")
        fpath.parent.mkdir(parents=True, exist_ok=True)
        fpath.write_text(content)
        return f"Written {len(content)} chars to {path}"

    elif tool_name == "shell":
        command = tool_input.get("command", "")
        log(f"    [TOOL] shell command='{command}'")
        # Simulate safe structural commands (find, ls, tree) against mock dir
        import shlex
        parts = shlex.split(command) if command else []
        if not parts:
            return "Error: empty command"
        cmd = parts[0]
        if cmd in ("find", "ls", "tree"):
            # List all files recursively
            results = []
            for fpath in sorted(base_dir.rglob("*")):
                if fpath.is_file():
                    results.append(str(fpath.relative_to(base_dir)))
            return "\n".join(results) if results else "No files found."
        elif cmd == "cat":
            # Redirect cat to file_read
            if len(parts) > 1:
                return handle_tool_call("file_read", {"path": parts[1]}, base_dir)
            return "Error: cat requires a file path"
        elif cmd == "wc":
            # Count lines across all files
            results = []
            for fpath in sorted(base_dir.rglob("*")):
                if fpath.is_file():
                    try:
                        lines = len(fpath.read_text().splitlines())
                        results.append(f"  {lines} {fpath.relative_to(base_dir)}")
                    except (UnicodeDecodeError, PermissionError):
                        pass
            return "\n".join(results) if results else "No files found."
        else:
            return f"Shell command '{cmd}' not available in sandbox. Available: find, ls, tree, cat, wc"

    else:
        return f"Unknown tool: {tool_name}"


def run_agent_phase2(
    agent_data: dict,
    base_dir: Path,
    scenario_key: str = "stories",
    max_rounds: int = 15,
) -> dict:
    """Execute an agent with designer-generated prompts and real tool_use."""
    agent_name = agent_data["agent_name"]
    system_prompt = agent_data["system_prompt"]
    task_prompt = agent_data["task_prompt"]
    assigned_tools = agent_data.get("tools", [])

    log_sep(f"PHASE 2: EXECUTING {agent_name}")
    log(f"Tools: {assigned_tools}")
    log(f"Max rounds: {max_rounds}")

    # Build tool definitions
    tool_defs = []
    if "grep" in assigned_tools:
        tool_defs.append({
            "name": "grep",
            "description": "Search file contents with regex patterns. Returns matching lines with file:line: prefix.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "Regex pattern to search for"},
                    "path": {"type": "string", "description": "Directory to search in (e.g., /stories/)"},
                },
                "required": ["pattern", "path"],
            },
        })
    if "file_read" in assigned_tools:
        tool_defs.append({
            "name": "file_read",
            "description": "Read the full contents of a file.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to file (e.g., /stories/story1.txt)"},
                },
                "required": ["path"],
            },
        })
    if "shell" in assigned_tools:
        tool_defs.append({
            "name": "shell",
            "description": "Execute shell commands in a sandboxed environment. Available commands: find, ls, tree, cat, wc.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "Shell command to execute (e.g., 'find src/ -name *.js')"},
                },
                "required": ["command"],
            },
        })
    if "file_write" in assigned_tools:
        tool_defs.append({
            "name": "file_write",
            "description": "Write content to a file.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to write to"},
                    "content": {"type": "string", "description": "Content to write"},
                },
                "required": ["path", "content"],
            },
        })

    messages = [{"role": "user", "content": task_prompt}]
    total_input = 0
    total_output = 0
    tool_calls = 0
    final_text = ""
    all_text_blocks: list[str] = []  # accumulate all text across rounds for scoring
    round_num = 0

    for round_num in range(1, max_rounds + 1):
        log(f"  Round {round_num}/{max_rounds}...")
        t0 = time.time()

        kwargs = {
            "model": AGENT_MODEL,
            "max_tokens": 8192,
            "temperature": 0.3,
            "system": system_prompt,
            "messages": messages,
        }
        if tool_defs:
            kwargs["tools"] = tool_defs

        resp = client.messages.create(**kwargs)
        ms = int((time.time() - t0) * 1000)
        total_input += resp.usage.input_tokens
        total_output += resp.usage.output_tokens

        log(f"    {resp.usage.input_tokens} in / {resp.usage.output_tokens} out ({ms}ms) stop={resp.stop_reason}")

        # Process response blocks — convert pydantic models to plain dicts
        # so the SDK can re-serialize them on subsequent rounds
        assistant_content = resp.content
        content_dicts = []
        for block in assistant_content:
            if block.type == "text":
                content_dicts.append({"type": "text", "text": block.text})
            elif block.type == "tool_use":
                content_dicts.append({
                    "type": "tool_use",
                    "id": block.id,
                    "name": block.name,
                    "input": block.input,
                })
        messages.append({"role": "assistant", "content": content_dicts})

        # Accumulate all text blocks for scoring
        for block in assistant_content:
            if block.type == "text" and block.text.strip():
                all_text_blocks.append(block.text)

        if resp.stop_reason == "end_turn":
            # Extract final text
            for block in assistant_content:
                if block.type == "text":
                    final_text = block.text
            break

        if resp.stop_reason == "tool_use":
            tool_results = []
            for block in assistant_content:
                if block.type == "tool_use":
                    tool_calls += 1
                    result = handle_tool_call(block.name, block.input, base_dir)
                    tool_results.append({
                        "type": "tool_result",
                        "tool_use_id": block.id,
                        "content": result,
                    })
            messages.append({"role": "user", "content": tool_results})
        else:
            break

    log(f"  Agent completed: {round_num} rounds, {tool_calls} tool calls")
    log(f"  Tokens: {total_input} in / {total_output} out")

    # Use accumulated text for scoring if agent hit max rounds without end_turn
    scoreable_text = final_text if final_text else "\n".join(all_text_blocks)

    # Evaluate output
    log_sep(f"PHASE 2: {agent_name} OUTPUT")
    if final_text:
        log_block(f"{agent_name} final output", final_text)
    else:
        log(f"  (No end_turn — scoring accumulated text from {len(all_text_blocks)} blocks, {len(scoreable_text)} chars)")
        log_block(f"{agent_name} accumulated text", scoreable_text[:2000])

    if scenario_key == "codebase":
        return _score_codebase(agent_name, scoreable_text, round_num, tool_calls, total_input, total_output)
    else:
        return _score_stories(agent_name, scoreable_text, round_num, tool_calls, total_input, total_output)


def _score_stories(agent_name: str, final_text: str, round_num: int, tool_calls: int, total_input: int, total_output: int) -> dict:
    """Score stories scenario output for planted contradictions."""
    log_sep("PHASE 2: CONTRADICTION SCORING")
    found = []
    missed = []
    output_lower = final_text.lower()

    for contra in EXPECTED_CONTRADICTIONS:
        detected = False
        if contra["id"] == "eyes":
            detected = ("blue" in output_lower and "brown" in output_lower) or "eye color" in output_lower or "eye colour" in output_lower
        elif contra["id"] == "profession":
            detected = ("teacher" in output_lower and "doctor" in output_lower) or ("teacher" in output_lower and "medical" in output_lower)
        elif contra["id"] == "age":
            detected = ("forty" in output_lower and "thirty-five" in output_lower) or ("40" in output_lower and "35" in output_lower)
        elif contra["id"] == "location":
            detected = ("main street" in output_lower and "oak avenue" in output_lower)

        if detected:
            found.append(contra)
            log(f"  FOUND: {contra['description']}")
        else:
            missed.append(contra)
            log(f"  MISSED: {contra['description']}", level="WARN")

    score = len(found)
    total = len(EXPECTED_CONTRADICTIONS)
    log(f"")
    log(f"  Score: {score}/{total} contradictions found")

    return {
        "agent_name": agent_name,
        "rounds": round_num,
        "tool_calls": tool_calls,
        "tokens": {"input": total_input, "output": total_output},
        "issues_found": [c["id"] for c in found],
        "issues_missed": [c["id"] for c in missed],
        "score": f"{score}/{total}",
        "final_output_length": len(final_text),
    }


def _score_codebase(agent_name: str, final_text: str, round_num: int, tool_calls: int, total_input: int, total_output: int) -> dict:
    """Score codebase scenario output for planted security/quality issues."""
    log_sep("PHASE 2: ISSUE SCORING")
    found = []
    missed = []
    output_lower = final_text.lower()

    for issue in EXPECTED_ISSUES:
        detected = False
        iid = issue["id"]

        if iid == "sql_injection":
            detected = (
                "sql injection" in output_lower
                or ("interpolat" in output_lower and "sql" in output_lower)
                or ("${" in final_text and "query" in output_lower)
                or "string concatenat" in output_lower
            )
        elif iid == "timing_attack":
            detected = (
                "timing" in output_lower
                or "constant-time" in output_lower
                or "constant time" in output_lower
                or ("===" in final_text and "password" in output_lower)
                or "bcrypt" in output_lower
                or "time-safe" in output_lower
            )
        elif iid == "no_rate_limit":
            detected = (
                "rate limit" in output_lower
                or "brute force" in output_lower
                or "throttl" in output_lower
            )
        elif iid == "broken_authz":
            detected = (
                ("authorization" in output_lower and "refund" in output_lower)
                or "idor" in output_lower
                or ("ownership" in output_lower and "refund" in output_lower)
                or ("any user" in output_lower and "refund" in output_lower)
                or "access control" in output_lower
            )
        elif iid == "no_transaction":
            detected = (
                "transaction" in output_lower
                or "race condition" in output_lower
                or "atomicity" in output_lower
                or ("begin" in output_lower and "commit" in output_lower)
                or "toctou" in output_lower
            )
        elif iid == "n_plus_one":
            detected = (
                "n+1" in output_lower
                or "n + 1" in output_lower
                or ("loop" in output_lower and "query" in output_lower and "fetchorder" in output_lower)
                or ("individual" in output_lower and "query" in output_lower and "loop" in output_lower)
            )
        elif iid == "hardcoded_creds":
            detected = (
                "hardcoded" in output_lower
                or "hard-coded" in output_lower
                or ("credential" in output_lower and "source" in output_lower)
                or ("password" in output_lower and "connection" in output_lower and "string" in output_lower)
                or "sk_live" in output_lower
            )
        elif iid == "outdated_dep":
            detected = (
                "outdated" in output_lower
                or ("express" in output_lower and "4.17" in output_lower)
                or "helmet" in output_lower
                or ("cve" in output_lower and "express" in output_lower)
                or "security middleware" in output_lower
            )

        if detected:
            found.append(issue)
            log(f"  FOUND: {issue['description']}")
        else:
            missed.append(issue)
            log(f"  MISSED: {issue['description']}", level="WARN")

    score = len(found)
    total = len(EXPECTED_ISSUES)
    log(f"")
    log(f"  Score: {score}/{total} issues found")

    return {
        "agent_name": agent_name,
        "rounds": round_num,
        "tool_calls": tool_calls,
        "tokens": {"input": total_input, "output": total_output},
        "issues_found": [i["id"] for i in found],
        "issues_missed": [i["id"] for i in missed],
        "score": f"{score}/{total}",
        "final_output_length": len(final_text),
    }


# ===========================================================================
# PHASE 1: DESIGNER CALL
# ===========================================================================

def run_designer(scenario_key: str) -> dict | None:
    scenario = SCENARIOS[scenario_key]
    has_upstream = "No upstream" not in scenario["upstream_context"]

    log_sep(f"PHASE 1: AGENT DESIGNER — {scenario['name']}")
    log(f"Scenario: {scenario_key}")
    log(f"Model: {MODEL}")
    log(f"Temperature: {TEMPERATURE}")
    log(f"Agents: {len(scenario['roster'])}")
    log(f"Allowed capabilities: {scenario['allowed_capabilities']}")
    log(f"Has upstream context: {has_upstream}")

    user_prompt = build_user_prompt(scenario)

    log_sep("SYSTEM PROMPT")
    log_block("system_prompt", DESIGNER_SYSTEM_PROMPT, max_lines=20)

    log_sep("USER PROMPT (full)")
    log_block("user_prompt", user_prompt)

    # Call the API
    log_sep("LLM CALL")
    log(f"Calling {MODEL}...")
    t0 = time.time()

    resp = client.messages.create(
        model=MODEL,
        max_tokens=MAX_TOKENS,
        temperature=TEMPERATURE,
        system=DESIGNER_SYSTEM_PROMPT,
        messages=[{"role": "user", "content": user_prompt}],
    )

    elapsed_ms = int((time.time() - t0) * 1000)
    text_block = next(b for b in resp.content if b.type == "text")
    raw_text = text_block.text

    log(f"Completed in {elapsed_ms}ms")
    log(f"Input tokens:  {resp.usage.input_tokens}")
    log(f"Output tokens: {resp.usage.output_tokens}")
    log(f"Total tokens:  {resp.usage.input_tokens + resp.usage.output_tokens}")

    input_cost = resp.usage.input_tokens * 3.0 / 1_000_000
    output_cost = resp.usage.output_tokens * 15.0 / 1_000_000
    total_cost = input_cost + output_cost
    log(f"Est. cost:     ${total_cost:.4f} (in: ${input_cost:.4f}, out: ${output_cost:.4f})")

    log_sep("RAW OUTPUT")
    log_block("raw_response", raw_text)

    # Parse JSON
    log_sep("PARSING")
    parsed = None
    try:
        parsed = json.loads(raw_text)
        log(f"JSON parsed successfully — {len(parsed.get('agents', []))} agents")
    except json.JSONDecodeError as e:
        log(f"JSON PARSE ERROR: {e}", level="ERROR")
        log("Attempting to extract JSON from markdown fences...")
        match = re.search(r"```(?:json)?\s*\n(.*?)\n```", raw_text, re.DOTALL)
        if match:
            try:
                parsed = json.loads(match.group(1))
                log(f"Extracted from fence — {len(parsed.get('agents', []))} agents")
            except json.JSONDecodeError as e2:
                log(f"Still failed: {e2}", level="ERROR")
        if parsed is None:
            # Brute force: find first { ... } balance
            depth = 0
            start = raw_text.find("{")
            if start >= 0:
                for i in range(start, len(raw_text)):
                    if raw_text[i] == "{":
                        depth += 1
                    elif raw_text[i] == "}":
                        depth -= 1
                        if depth == 0:
                            try:
                                parsed = json.loads(raw_text[start:i+1])
                                log(f"Extracted via brace matching — {len(parsed.get('agents', []))} agents")
                            except json.JSONDecodeError:
                                pass
                            break
        if parsed is None:
            log("No valid JSON found in output", level="ERROR")
            return None

    # Display + analyze each agent
    log_sep("GENERATED PROMPTS")

    all_analysis = []

    for i, agent in enumerate(parsed["agents"]):
        log("")
        log(f"{'─' * 72}")
        log(f"  AGENT {i+1}: {agent['agent_name']}")
        log(f"  Tools: {agent.get('tools', [])}")
        log(f"{'─' * 72}")

        log("")
        log_block(f"{agent['agent_name']} — SYSTEM PROMPT", agent["system_prompt"])
        log("")
        log_block(f"{agent['agent_name']} — TASK PROMPT", agent["task_prompt"])
        log("")
        log_block(f"{agent['agent_name']} — REASONING", agent.get("reasoning", "(none)"))

        analysis = analyze_agent_output(agent, scenario["allowed_capabilities"], has_upstream=has_upstream)
        all_analysis.append(analysis)

    # Summary
    log_sep("BELIEF ANALYSIS")

    all_beliefs_seen = set()
    for a in all_analysis:
        log(f"")
        log(f"  {a['name']}:")
        log(f"    Tools:          {a['tools_assigned']}")
        log(f"    System words:   {a['system_prompt_words']}")
        log(f"    Task words:     {a['task_prompt_words']}")
        log(f"    Beliefs ({len(a['beliefs_detected'])}):  {', '.join(a['beliefs_detected'])}")
        all_beliefs_seen.update(a["beliefs_detected"])
        if a["beliefs_notably_applied"]:
            for n in a["beliefs_notably_applied"]:
                log(f"    NOTABLE: {n}")
        if a["warnings"]:
            for w in a["warnings"]:
                log(f"    WARNING: {w}", level="WARN")

    log("")
    all_belief_tags = list(BELIEF_SIGNALS.keys())
    missing = [b for b in all_belief_tags if b not in all_beliefs_seen]
    log(f"  Total unique beliefs detected: {len(all_beliefs_seen)}/{len(all_belief_tags)}")
    if missing:
        log(f"  Not detected: {', '.join(missing)}")
    else:
        log(f"  All beliefs detected!")

    output = {
        "scenario": scenario_key,
        "scenario_name": scenario["name"],
        "has_upstream": has_upstream,
        "model": MODEL,
        "temperature": TEMPERATURE,
        "timing_ms": elapsed_ms,
        "tokens": {
            "input": resp.usage.input_tokens,
            "output": resp.usage.output_tokens,
            "total": resp.usage.input_tokens + resp.usage.output_tokens,
        },
        "cost_usd": round(total_cost, 4),
        "designer_output": parsed,
        "analysis": all_analysis,
        "beliefs_detected_total": len(all_beliefs_seen),
        "beliefs_missing": missing,
    }

    OUTPUT_FILE.write_text(json.dumps(output, indent=2))
    log("")
    log(f"Full output saved to: {OUTPUT_FILE}")
    log(f"Full log saved to: {LOG_FILE}")

    return output


def _generate_synthetic_mapper_output(mock_dir: Path) -> str:
    """Generate a realistic Mapper output (file listing + entry points) for upstream injection."""
    lines = ["## Codebase Structure\n"]

    # List all files
    files = sorted(f.relative_to(mock_dir) for f in mock_dir.rglob("*") if f.is_file())
    lines.append("### Files discovered:")
    for f in files:
        lines.append(f"- {f}")

    # Identify entry points from index.js
    lines.append("\n### Entry Points:")
    lines.append("- src/index.js — Main application entry, wires Express routes")
    lines.append("- POST /auth/login — Authentication endpoint (src/auth/login.js)")
    lines.append("- POST /payments/checkout — Payment processing (src/payments/checkout.js)")
    lines.append("- POST /payments/refund — Refund handling (src/payments/refund.js)")

    # Data flow boundaries
    lines.append("\n### Data Flow Boundaries:")
    lines.append("- Database interaction: src/db/queries.js (query helpers used by auth and payments)")
    lines.append("- Database config: src/config/database.js (connection pool setup)")
    lines.append("- Auth middleware: src/auth/middleware.js (JWT validation on protected routes)")
    lines.append("- External deps: express 4.17.1, pg, bcrypt, jsonwebtoken, dotenv (see package.json)")

    lines.append("\n### Module Dependencies:")
    lines.append("- src/index.js → src/auth/login.js, src/auth/middleware.js, src/payments/checkout.js, src/payments/refund.js")
    lines.append("- src/auth/login.js → src/db/queries.js")
    lines.append("- src/payments/checkout.js → src/db/queries.js")
    lines.append("- src/payments/refund.js → src/db/queries.js")
    lines.append("- src/db/queries.js → src/config/database.js")

    return "\n".join(lines)


# ===========================================================================
# MAIN
# ===========================================================================

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Agent Designer prototype v2")
    parser.add_argument(
        "--scenario", "-s",
        choices=list(SCENARIOS.keys()),
        default="stories",
        help="Which test scenario to run (default: stories)",
    )
    parser.add_argument(
        "--run-agent",
        action="store_true",
        help="Phase 2: execute the first agent with generated prompts against mock files",
    )
    args = parser.parse_args()

    # Phase 1: Designer
    result = run_designer(args.scenario)

    if result is None:
        log("Designer failed, cannot continue.", level="ERROR")
        exit(1)

    # Phase 2: Agent execution (optional)
    if args.run_agent:
        supported = ("stories", "stories_upstream", "codebase")
        if args.scenario not in supported:
            log(f"Phase 2 only supports {supported} (need mock files)", level="WARN")
        else:
            agents = result["designer_output"]["agents"]

            if args.scenario == "codebase":
                # Run SecurityAuditor (index 1) with synthetic Mapper output
                agent_idx = 1
                agent_data = dict(agents[agent_idx])
                tmpdir = setup_mock_files(args.scenario)
                mock_dir = tmpdir

                # Generate synthetic Mapper output (file listing) for upstream context
                mapper_output = _generate_synthetic_mapper_output(mock_dir)
                agent_data["task_prompt"] = agent_data["task_prompt"].replace(
                    "{Mapper's output will be injected here}", mapper_output
                )
                rounds = 25
            else:
                agent_idx = 0
                agent_data = agents[agent_idx]
                tmpdir = setup_mock_files(args.scenario)
                mock_dir = tmpdir / "stories"
                rounds = 15

            log(f"Mock files created at: {mock_dir}")
            try:
                phase2_result = run_agent_phase2(agent_data, mock_dir, scenario_key=args.scenario, max_rounds=rounds)
                result["phase2"] = phase2_result

                # Re-save with phase 2 results
                OUTPUT_FILE.write_text(json.dumps(result, indent=2))
                log(f"Results updated with Phase 2: {OUTPUT_FILE}")
            finally:
                cleanup_mock_files(tmpdir)

    log_sep("COMPLETE")
